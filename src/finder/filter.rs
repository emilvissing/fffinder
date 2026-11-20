//! Nushell filters over what the window shows, and Smart Folders (saved
//! searches with a filter). fff-search has no size or date filters; a
//! `where` condition in Nushell has all of them.

use crate::theme::ui;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use fffinder_core::{FileEntry, KindFilter, SmartFolder};
use gpui_kit::TestSupportExt as _;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{AppState, Finder, Row, SearchScope};
use crate::actions::{FINDER, ToggleFilter};
use crate::model::Location;
use crate::nushell;

/// Search results fetched while a filter is on, so it has enough to work with.
pub const FILTER_SEARCH_LIMIT: usize = 5000;

/// An item a filter was run over: its path, and the size and date the
/// filter was given for it, which can change while the item stays where it
/// is.
pub(super) type FilterInput = (PathBuf, Option<u64>, Option<SystemTime>);

/// What a window searches with besides the words: the kind, the scope, Name
/// or Contents, and the filter with its bar.
struct SearchSetup {
    kind: KindFilter,
    scope: SearchScope,
    contents: bool,
    filter: String,
    filter_open: bool,
}

/// The search setup of the Smart Folder that's open, and the window's own,
/// which it took the place of and which comes back when its search ends.
pub(super) struct SmartFolderSetup {
    before: SearchSetup,
    /// As the Smart Folder set it. A part that differs by the time the
    /// search ends was changed by hand, and stays as it was changed.
    opened: SearchSetup,
    /// The folder the Smart Folder was saved to search, when it couldn't be
    /// found and This Computer is searched in its place.
    missing: Option<PathBuf>,
}

impl Finder {
    pub(super) fn on_filter_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                // Wait for a pause in typing; each run starts a `nu` process.
                let text = state.read(cx).value().to_string();
                self.filter_debounce = cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(400))
                        .await;
                    this.update(cx, |this, cx| this.set_filter(text, cx)).ok();
                });
            }
            InputEvent::PressEnter { .. } => {
                let text = state.read(cx).value().to_string();
                self.filter_debounce = Task::ready(());
                self.set_filter(text, cx);
            }
            InputEvent::Focus | InputEvent::Blur => {}
        }
    }

    /// Filter what the window shows with a Nushell `where` condition, or a
    /// pipeline starting with `|`. Empty turns the filter off.
    pub fn set_filter(&mut self, text: String, cx: &mut Context<Self>) {
        let text = text.trim().to_string();
        if text == self.filter {
            return;
        }
        let on_off = self.filter.is_empty() != text.is_empty();
        self.filter = text;
        self.filter_keep = None;
        self.filter_source = None;
        self.filter_error = None;
        self.filtering = false;
        self.filter_task = Task::ready(());
        // Turning it on or off changes how many results a search fetches.
        if on_off && self.is_searching() {
            self.run_search(false, cx);
        }
        self.rebuild_rows(cx);
    }

    pub fn filter_text(&self) -> &str {
        &self.filter
    }

    pub fn filter_error(&self) -> Option<&str> {
        self.filter_error.as_deref()
    }

    pub fn is_filtering(&self) -> bool {
        self.filtering
    }

    pub fn filter_bar_open(&self) -> bool {
        self.filter_open || !self.filter.is_empty()
    }

    /// Keep only the rows the filter selected, starting a new run when the
    /// rows changed: when they are other items, or in another order, or one
    /// has another size or date. Rows stay as they are until the first run
    /// finishes. The row of a name being edited in the views stays whatever
    /// the filter selected.
    pub(super) fn apply_filter_to(&mut self, rows: &mut Vec<Row>, cx: &mut Context<Self>) {
        if self.filter.is_empty() {
            return;
        }
        // A search that hasn't answered has no rows of its own yet. A run
        // over none would end at once with nothing kept, and that would hide
        // the search's first results until their own run finished.
        if rows.is_empty() && self.is_searching() && self.search_stats.is_none() {
            return;
        }
        // An item can come back where it was with another size or date: a
        // file that another program saved, or a folder once the index has
        // told its size. The filter may go by those, so what it answered
        // for the item as it was says nothing of the item as it is.
        let given: Vec<FilterInput> = rows
            .iter()
            .map(|r| (r.entry.path.clone(), r.entry.size, r.entry.modified))
            .collect();
        if self.filter_source.as_ref() != Some(&given) {
            self.filter_source = Some(given);
            let entries: Vec<FileEntry> = rows.iter().map(|r| r.entry.clone()).collect();
            self.start_filter(entries, cx);
        }
        if let Some(keep) = &self.filter_keep {
            // A new item is named in its row before the filter can have
            // answered for it, and the answer may be that it doesn't pass.
            // Taking the row away would take the name's field with it, so
            // the item is held to the filter once its name is given.
            let named = self
                .renaming
                .as_ref()
                .filter(|renaming| !renaming.in_sidebar)
                .map(|renaming| renaming.path.as_path());
            rows.retain(|r| keep.contains(&r.entry.path) || named == Some(r.entry.path.as_path()));
        }
    }

    fn start_filter(&mut self, entries: Vec<FileEntry>, cx: &mut Context<Self>) {
        let Some(nu) = cx.global::<AppState>().nu.clone() else {
            self.filter_error = Some(
                "Nushell (nu) was not found, so filters can't run. Set its path in Settings."
                    .to_string(),
            );
            return;
        };
        let filter = self.filter.clone();
        let cwd = self
            .current_folder()
            .map(Path::to_path_buf)
            .or_else(dirs::home_dir)
            .unwrap_or_else(std::env::temp_dir);
        self.filtering = true;
        // Replacing the task cancels a run for rows that are gone.
        self.filter_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    if entries.is_empty() {
                        return Ok(Vec::new());
                    }
                    nushell::filter_paths(&nu, &filter, &entries, &cwd)
                })
                .await;
            this.update(cx, |this, cx| {
                this.filtering = false;
                match result {
                    Ok(paths) => {
                        this.filter_keep = Some(paths.into_iter().collect::<HashSet<_>>());
                        this.filter_error = None;
                    }
                    Err(err) => {
                        this.filter_keep = None;
                        this.filter_error = Some(err);
                    }
                }
                this.rebuild_rows(cx);
            })
            .ok();
        });
    }

    pub(super) fn toggle_filter(
        &mut self,
        _: &ToggleFilter,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.filter_bar_open() {
            self.close_filter(window, cx);
        } else {
            self.filter_open = true;
            self.filter_input.update(cx, |s, cx| s.focus(window, cx));
            cx.notify();
        }
    }

    pub fn close_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.filter_open = false;
        self.filter_debounce = Task::ready(());
        self.filter_input
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.set_filter(String::new(), cx);
        self.focus_files(window, cx);
        cx.notify();
    }

    // ----- Smart Folders --------------------------------------------------------------

    pub fn smart_folders(&self) -> &[SmartFolder] {
        &self.config.smart_folders
    }

    /// The current search and filter, ready to save.
    pub fn current_smart_folder(&self, name: String) -> Option<SmartFolder> {
        if !self.is_searching() {
            return None;
        }
        Some(SmartFolder {
            name,
            query: self.query.clone(),
            kind: self.config.kind_filter,
            scope: match self.scope {
                SearchScope::Folder => self.current_folder().map(Path::to_path_buf),
                SearchScope::Computer => None,
            },
            contents: self.search_contents,
            filter: (!self.filter.is_empty()).then(|| self.filter.clone()),
        })
    }

    /// Save `folder`, in place of one with the same name if there is one,
    /// which keeps its row where it was in the sidebar.
    pub fn add_smart_folder(&mut self, folder: SmartFolder, cx: &mut Context<Self>) {
        let folders = &mut self.config.smart_folders;
        let at = folders.iter().position(|f| f.name == folder.name);
        folders.retain(|f| f.name != folder.name);
        folders.insert(at.unwrap_or(folders.len()), folder);
        self.settings_changed(cx);
        cx.notify();
    }

    pub fn remove_smart_folder(&mut self, name: &str, cx: &mut Context<Self>) {
        self.config.smart_folders.retain(|f| f.name != name);
        self.settings_changed(cx);
        cx.notify();
    }

    /// Whether `folder`'s search is the one showing, all of it: the words,
    /// the kind, the place, Name or Contents, and the filter. Two that differ
    /// in any of those are never both the one showing.
    pub(super) fn shows_smart_folder(&self, folder: &SmartFolder) -> bool {
        let Some(mut showing) = self.current_smart_folder(folder.name.clone()) else {
            return false;
        };
        // One whose folder couldn't be found searches This Computer in its
        // place, and that is its search until the scope is changed by hand.
        let missing = self
            .smart_folder_setup
            .as_ref()
            .and_then(|setup| setup.missing.as_ref());
        if showing.scope.is_none() && missing.is_some() && missing == folder.scope.as_ref() {
            showing.scope = folder.scope.clone();
        }
        showing == *folder
    }

    fn search_setup(&self, cx: &App) -> SearchSetup {
        SearchSetup {
            kind: self.config.kind_filter,
            scope: self.scope,
            contents: self.search_contents,
            // As the bar's field has it, which is the filter but for the
            // pause before what's typed there is applied.
            filter: self.filter_input.read(cx).value().trim().to_string(),
            filter_open: self.filter_open,
        }
    }

    /// Put `filter` in the bar's field and apply it, with the bar open or
    /// closed.
    fn replace_filter(
        &mut self,
        filter: String,
        open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.filter_open = open;
        // What was typed there a moment ago would be applied over it.
        self.filter_debounce = Task::ready(());
        self.filter_input
            .update(cx, |s, cx| s.set_value(filter.clone(), window, cx));
        self.set_filter(filter, cx);
        // A bar that goes away may have the keyboard in its field, and keys
        // pressed in a field that isn't drawn go nowhere.
        let field = self.filter_input.read(cx).focus_handle(cx);
        if field.is_focused(window) && !self.filter_bar_open() {
            self.focus_files(window, cx);
        }
    }

    /// A frame is about to be drawn. The bar's field may have the keyboard
    /// though the bar isn't showing: a dialog gives the keyboard back to
    /// what had it when the dialog opened, and the bar can go in the
    /// meantime, as when Go to Folder is called from a Smart Folder's filter
    /// field and ends that search. Keys pressed in a field that isn't drawn
    /// go nowhere, and [`Self::keyboard_lost`] can't be the one to act: what
    /// stopped being drawn with the keyboard was the dialog, no part of
    /// this browser, and GPUI reports nothing lost when the keyboard then
    /// lands on a field that isn't drawn. The files take the keyboard
    /// before anything of this browser is drawn, so this frame draws them
    /// with it and no other frame has to be asked for.
    pub(super) fn check_filter_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.filter_bar_open() {
            return;
        }
        let field = self.filter_input.read(cx).focus_handle(cx);
        if field.is_focused(window) {
            self.focus_files(window, cx);
        }
    }

    /// Run a saved search: its place, kind, filter, and words.
    pub fn open_smart_folder(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(folder) = self.config.smart_folders.get(ix).cloned() else {
            return;
        };
        let (scope, missing) = match folder.scope.as_ref().filter(|s| s.is_dir()) {
            Some(scope) => {
                self.navigate(Location::Folder(scope.clone()), window, cx);
                (SearchScope::Folder, None)
            }
            None => {
                // The search is wider than the one that was saved, which
                // the results alone wouldn't show.
                if let Some(missing) = &folder.scope {
                    window.push_notification(
                        (
                            gpui_kit::component::notification::NotificationType::Warning,
                            format!(
                                "{} can’t be found, so “{}” is searching This Computer",
                                missing.display(),
                                folder.name
                            ),
                        ),
                        cx,
                    );
                }
                self.end_search(window, cx);
                (SearchScope::Computer, folder.scope.clone())
            }
        };
        let filter = folder.filter.unwrap_or_default();
        self.enter_smart_folder(folder.kind, scope, folder.contents, filter, window, cx);
        // The sidebar marks it as the one showing all the same.
        if let Some(setup) = &mut self.smart_folder_setup {
            setup.missing = missing;
        }
        self.search_input
            .update(cx, |s, cx| s.set_value(folder.query.clone(), window, cx));
        self.set_query(folder.query, true, cx);
        cx.notify();
    }

    /// Search with a Smart Folder's kind, scope, Name or Contents, and filter
    /// until the search ends, and then with the window's own again (see
    /// [`Self::leave_smart_folder`]). No search is showing when this is
    /// called: the one before has ended, and with it a Smart Folder that was
    /// open, so what the window searches with by now is its own.
    pub(super) fn enter_smart_folder(
        &mut self,
        kind: KindFilter,
        scope: SearchScope,
        contents: bool,
        filter: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let before = self.search_setup(cx);
        self.scope = scope;
        self.config.kind_filter = kind;
        // Opening a Smart Folder isn't choosing a kind for the searches after
        // it or for the next tab, so there's nothing to publish.
        self.published.kind_filter = kind;
        self.search_contents = contents;
        let open = !filter.is_empty();
        self.replace_filter(filter, open, window, cx);
        self.smart_folder_setup = Some(SmartFolderSetup {
            before,
            opened: self.search_setup(cx),
            missing: None,
        });
    }

    /// The search a Smart Folder ran is ending: the window gets back the kind,
    /// scope, Name or Contents, and filter it had before, so the saved ones
    /// don't stay on for the folders and searches after it. A part that was
    /// changed by hand while the Smart Folder was open stays as changed.
    pub(super) fn leave_smart_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(SmartFolderSetup { before, opened, .. }) = self.smart_folder_setup.take() else {
            return;
        };
        let now = self.search_setup(cx);
        // Set as they are, not through their setters, which would search
        // again for words that are about to go.
        if now.kind == opened.kind {
            self.config.kind_filter = before.kind;
            // No more of a choice to publish than the Smart Folder's was.
            self.published.kind_filter = before.kind;
        }
        if now.scope == opened.scope {
            self.scope = before.scope;
        }
        if now.contents == opened.contents {
            self.search_contents = before.contents;
        }
        if now.filter == opened.filter && now.filter_open == opened.filter_open {
            // Through the filter's own setter, which knows what else to
            // forget. It starts the search over when the filter goes on or
            // off, and the end of the search, which comes next, calls that
            // off.
            self.replace_filter(before.filter, before.filter_open, window, cx);
        }
        // The toolbar shows the scope even when nothing is searched.
        cx.notify();
    }

    // ----- the bar ------------------------------------------------------------------

    pub(super) fn render_filter_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let error = self.filter_error.clone();
        v_flex()
            .flex_shrink_0()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.tab_bar)
            .child(
                h_flex()
                    .h(ui(36.))
                    .px(ui(14.))
                    .gap(ui(8.))
                    .text_size(ui(12.))
                    // As in the search field, Escape empties the field, and
                    // in one that's already empty it gives the files the
                    // keyboard. The bar stays open.
                    .capture_action(cx.listener(
                        |this, _: &gpui_kit::component::input::Escape, window, cx| {
                            if this.filter_input.read(cx).value().is_empty() {
                                this.focus_files(window, cx);
                                cx.stop_propagation();
                            }
                        },
                    ))
                    .child(
                        Icon::new(Lucide::Funnel)
                            .with_size(ui(14.))
                            .text_color(muted),
                    )
                    .child(
                        div()
                            .font_family(theme.mono_font_family.clone())
                            .text_color(muted)
                            .child("where"),
                    )
                    .child(
                        Input::new(&self.filter_input)
                            .id("filter-input")
                            .small()
                            .flex_1()
                            .font_family(theme.mono_font_family.clone()),
                    )
                    .when(self.filtering, |this| {
                        this.child(Spinner::new().xsmall().color(muted))
                    })
                    .child(
                        Button::new("close-filter")
                            .small()
                            .ghost()
                            .icon(IconName::Close)
                            .tooltip_with_action("Remove the filter", &ToggleFilter, Some(FINDER))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.close_filter(window, cx)
                            })),
                    ),
            )
            .when_some(error, |this, error| {
                this.child(
                    div()
                        .id("filter-error-scroll")
                        .max_h(ui(90.))
                        .overflow_y_scrollbar()
                        .mx(ui(14.))
                        .mb(ui(8.))
                        .p(ui(8.))
                        .rounded(ui(6.))
                        .bg(theme.danger.opacity(0.12))
                        .text_color(theme.danger)
                        .font_family(theme.mono_font_family.clone())
                        .text_size(ui(11.))
                        .child(div().id("filter-error").test_support().child(error)),
                )
            })
    }
}
