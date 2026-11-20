//! File previews for the gallery, the column view, and Quick Look. Structured
//! files that are text (CSV, JSON, TOML, YAML, …) are highlighted like code;
//! binary ones (SQLite, spreadsheets, …) are opened with Nushell and shown as
//! tables, and Quick Look can explore either (see `explore.rs`). Code is
//! highlighted, Markdown rendered, archives listed, fonts shown in their own
//! type, PDFs drawn page by page (see `pdf.rs`), and videos and documents
//! show the system's thumbnail of them.

use super::explore::{QuickLookMode, mode_buttons};
use super::pdf::Surface;
use super::preview_images::PreviewImages;
use super::views::file_icon;
use super::{AppState, ContentHit, Finder};
use crate::actions::{
    ExploreBack, ExploreInto, MoveDown, MoveLeft, MoveRight, MoveUp, NextPdfPage,
    PreviewTextBigger, PreviewTextReset, PreviewTextSmaller, PreviousPdfPage, QUICK_LOOK,
    QuickLook, QuickLookExplore,
};
use crate::nushell::{self, NuOutput};
use crate::theme::palette;
use crate::theme::ui;
use fffinder_core::config::{DEFAULT_PREVIEW_FONT_SIZE, FONT_SIZE_RANGE};
use fffinder_core::{FileEntry, FileKind, ViewMode, format};
use gpui_kit::TestSupportExt as _;
use gpui_kit::base::ScrollBounce;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::text::TextView;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::Cell;
use std::io::Read as _;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const TEXT_LIMIT: usize = 64 * 1024;
const TABLE_ROWS: usize = 200;
/// Font files bigger than this aren't loaded to show them.
const FONT_LIMIT: u64 = 50_000_000;
/// A preview that opens at a matching line keeps up to this many of the
/// lines before it in view.
const MATCH_CONTEXT_LINES: usize = 2;
/// The most those lines may add up to, in bytes. Long lines wrap, and would
/// push the match out of a small preview.
const MATCH_CONTEXT_BYTES: usize = 160;

#[derive(Clone)]
pub enum Preview {
    Loading,
    Image,
    Text {
        text: SharedString,
        truncated: bool,
    },
    /// Source colored the way the user's Nushell colors it.
    Code {
        text: SharedString,
        spans: Vec<(std::ops::Range<usize>, HighlightStyle)>,
        /// Comments, which `nu-highlight` leaves plain; drawn in the theme's
        /// secondary text color.
        comments: Vec<std::ops::Range<usize>>,
        truncated: bool,
    },
    Table {
        columns: Vec<SharedString>,
        rows: Vec<Vec<SharedString>>,
        /// A line under the table, such as where it came from.
        footer: Option<SharedString>,
    },
    /// Markdown, rendered.
    Markdown {
        text: SharedString,
    },
    /// A font file, drawn in its own type once `data` is loaded into the
    /// text system, in the weight and style of the face it holds.
    Font {
        family: SharedString,
        weight: FontWeight,
        italic: bool,
        data: Option<Arc<Vec<u8>>>,
    },
    /// Audio or video, with its running time when the header says.
    Media {
        duration: Option<Duration>,
    },
    Folder {
        items: usize,
    },
    Unavailable,
}

impl Preview {
    /// Whether the preview shows text, so its size can be changed.
    pub(super) fn has_text(&self) -> bool {
        matches!(
            self,
            Preview::Text { .. }
                | Preview::Code { .. }
                | Preview::Table { .. }
                | Preview::Markdown { .. }
                | Preview::Font { .. }
        )
    }
}

/// The family name stored in a font file, and its face's weight and
/// whether it slants: `Inter-Bold.ttf` is Inter at 700, upright.
fn font_face(data: &[u8]) -> Option<(String, FontWeight, bool)> {
    use ttf_parser::name_id::{FAMILY, TYPOGRAPHIC_FAMILY};
    let face = ttf_parser::Face::parse(data, 0).ok()?;
    let family = [TYPOGRAPHIC_FAMILY, FAMILY].into_iter().find_map(|id| {
        face.names()
            .into_iter()
            .find(|name| name.name_id == id && name.is_unicode())
            .and_then(|name| name.to_string())
    })?;
    let weight = FontWeight(f32::from(face.weight().to_number()));
    Some((family, weight, face.is_italic() || face.is_oblique()))
}

/// Font files already given to the text system, which keeps every font it's
/// given for good, by path and modification time.
#[derive(Default)]
struct LoadedFonts(std::collections::HashSet<(std::path::PathBuf, Option<std::time::SystemTime>)>);

impl Global for LoadedFonts {}

fn extension(entry: &FileEntry) -> String {
    entry
        .path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// What's inside a zip or tar archive, as a table.
fn archive_table(entry: &FileEntry) -> Option<Preview> {
    let (listed, more) = fffinder_core::archive::list(&entry.path, TABLE_ROWS).ok()?;
    let count = listed.len();
    let rows = listed
        .into_iter()
        .map(|item| {
            let size = if item.is_dir {
                "--".to_string()
            } else {
                format::human_size(item.size)
            };
            vec![item.name.into(), size.into()]
        })
        .collect();
    let footer = match (count, more) {
        (n, true) => format!("The first {n} items"),
        (1, false) => "1 item".to_string(),
        (n, false) => format!("{n} items"),
    };
    Some(Preview::Table {
        columns: vec!["Name".into(), "Size".into()],
        rows,
        footer: Some(footer.into()),
    })
}

/// Work out the preview for `entry`; `dark` picks the code colors.
fn load_preview(entry: &FileEntry, nu: Option<&Path>, dark: bool) -> Preview {
    if entry.is_dir {
        let items = std::fs::read_dir(&entry.path)
            .map(|d| d.count())
            .unwrap_or(0);
        return Preview::Folder { items };
    }
    let ext = extension(entry);
    if let Some(nu) = nu
        && matches!(ext.as_str(), "nu" | "nuon")
        && let Some((source, truncated)) = read_text(&entry.path)
        && let Ok(colored) = nushell::highlight_nu(nu, &source)
    {
        let (text, spans) = crate::ansi::parse(&colored);
        let styled: Vec<_> = spans.iter().map(|(range, _)| range.clone()).collect();
        return Preview::Code {
            comments: nushell::comment_ranges(&text, &styled),
            text: text.into(),
            spans: spans
                .into_iter()
                .map(|(range, style)| (range, style.to_highlight()))
                .collect(),
            truncated,
        };
    }
    // Structured files that are text show as text, colored. The others
    // (SQLite, spreadsheets, MessagePack, binary plists) open with Nushell.
    let structured = nushell::is_structured(&entry.path);
    let structured_text = structured.then(|| read_text(&entry.path)).flatten();
    if let Some(nu) = nu
        && structured
        && structured_text.is_none()
        && entry.size.unwrap_or(0) < nushell::OPEN_LIMIT
    {
        let run = nushell::preview_file(nu, &entry.path);
        match run.output {
            NuOutput::Table { columns, rows } => {
                let total = rows.len();
                let footer = format!(
                    "Opened with Nushell · {total} row{}{}",
                    if total == 1 { "" } else { "s" },
                    if total > TABLE_ROWS {
                        ", the first 200 shown"
                    } else {
                        ""
                    }
                );
                return Preview::Table {
                    columns: columns.into_iter().map(Into::into).collect(),
                    rows: rows
                        .into_iter()
                        .take(TABLE_ROWS)
                        .map(|r| r.into_iter().map(Into::into).collect())
                        .collect(),
                    footer: Some(footer.into()),
                };
            }
            NuOutput::Text(text) => {
                return Preview::Text {
                    truncated: text.len() > TEXT_LIMIT,
                    text: text.chars().take(TEXT_LIMIT).collect::<String>().into(),
                };
            }
            NuOutput::Empty => {}
        }
    }
    if fffinder_core::archive::format(&entry.path).is_some()
        && let Some(table) = archive_table(entry)
    {
        return table;
    }
    if matches!(ext.as_str(), "ttf" | "otf" | "ttc")
        && entry.size.unwrap_or(0) < FONT_LIMIT
        && let Ok(data) = std::fs::read(&entry.path)
        && let Some((family, weight, italic)) = font_face(&data)
    {
        return Preview::Font {
            family: family.into(),
            weight,
            italic,
            data: Some(Arc::new(data)),
        };
    }
    match entry.kind {
        FileKind::Image => return Preview::Image,
        FileKind::Video | FileKind::Audio => {
            return Preview::Media {
                duration: fffinder_core::media::duration(&entry.path),
            };
        }
        _ => {}
    }
    if let Some((text, truncated)) = structured_text.or_else(|| {
        matches!(
            entry.kind,
            FileKind::Text
                | FileKind::Code
                | FileKind::Data
                | FileKind::Other
                | FileKind::Executable
        )
        .then(|| read_text(&entry.path))
        .flatten()
    }) {
        if matches!(ext.as_str(), "md" | "markdown" | "mdown" | "mkd") {
            return Preview::Markdown { text: text.into() };
        }
        if let Some(spans) = crate::highlight::highlight(&text, &entry.path, dark) {
            return Preview::Code {
                text: text.into(),
                spans,
                comments: Vec::new(),
                truncated,
            };
        }
        return Preview::Text {
            text: text.into(),
            truncated,
        };
    }
    Preview::Unavailable
}

/// The start of a file as text, or `None` when it looks binary.
pub(super) fn read_text(path: &Path) -> Option<(String, bool)> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; TEXT_LIMIT + 1];
    let mut len = 0;
    while len < buf.len() {
        match file.read(&mut buf[len..]) {
            Ok(0) => break,
            Ok(n) => len += n,
            Err(_) => return None,
        }
    }
    buf.truncate(len);
    if buf.iter().take(8192).any(|&b| b == 0) {
        return None;
    }
    let truncated = buf.len() > TEXT_LIMIT;
    buf.truncate(TEXT_LIMIT);
    let text = String::from_utf8_lossy(&buf).into_owned();
    // Lossy decoding of a binary file produces many replacement characters.
    let bad = text.chars().take(4096).filter(|&c| c == '\u{FFFD}').count();
    (bad < 16).then_some((text, truncated))
}

/// What an item's row says of it that its preview depends on: when it was
/// last modified, and how big a file is. A preview made for one stamp is
/// out of date for an item whose row has another.
pub(super) type PreviewStamp = (Option<SystemTime>, Option<u64>);

/// The stamp of `entry`, from nothing but the entry. A row's date needn't
/// be the disk's: a search result has the index's, in whole seconds. Held
/// against a date read from the disk, such a row would never match its
/// preview, and the gallery, which asks for its preview with every frame,
/// would have it read over and over. A folder's size is left out because it
/// isn't the row's own: it's filled in from the index some time after the
/// folder is listed.
fn preview_stamp(entry: &FileEntry) -> PreviewStamp {
    (entry.modified, entry.size.filter(|_| !entry.is_dir))
}

impl Finder {
    /// Start loading the preview for `entry` unless it is cached or loading
    /// for the item as `entry` has it. A preview made before the item
    /// changed stays up until the new one is read, so nothing spins in
    /// between.
    pub fn ensure_preview(&mut self, entry: &FileEntry, cx: &mut Context<Self>) {
        let path = entry.path.clone();
        let stamp = preview_stamp(entry);
        // What's being read is what the cache is about to hold.
        let current = match self.preview_tasks.get(&path) {
            Some((loading, _)) => Some(loading),
            None => self.previews.get(&path).map(|(cached, _)| cached),
        };
        if current == Some(&stamp) {
            return;
        }
        if self.previews.len() > 256 {
            self.previews.retain(|cached, _| *cached == path);
        }
        let nu = cx.global::<AppState>().nu.clone();
        let dark = cx.theme().is_dark();
        let entry = entry.clone();
        let task = cx.spawn(async move |this, cx| {
            let mut preview = cx
                .background_spawn({
                    let entry = entry.clone();
                    async move { load_preview(&entry, nu.as_deref(), dark) }
                })
                .await;
            this.update(cx, |this, cx| {
                // A font is drawn in its own type once the text system has
                // it, which only needs to be told once.
                if let Preview::Font { data, .. } = &mut preview
                    && let Some(bytes) = data.take()
                    && cx
                        .default_global::<LoadedFonts>()
                        .0
                        .insert((entry.path.clone(), entry.modified))
                {
                    let bytes = Arc::unwrap_or_clone(bytes);
                    if let Err(err) = cx.text_system().add_fonts(vec![bytes.into()]) {
                        log::warn!("loading a font to preview it: {err}");
                    }
                }
                this.preview_tasks.remove(&entry.path);
                this.previews.insert(entry.path.clone(), (stamp, preview));
                cx.notify();
            })
            .ok();
            cx.refresh();
        });
        // A load for the item as it was before is dropped here, which stops
        // it: the one that lands is for the item as it is now.
        self.preview_tasks.insert(path, (stamp, task));
    }

    pub fn preview_of(&self, entry: &FileEntry) -> Preview {
        self.previews
            .get(&entry.path)
            .map(|(_, preview)| preview.clone())
            .unwrap_or(Preview::Loading)
    }

    /// Forget every preview, the ones still being read too, and read the
    /// shown item's again. Only the gallery asks for its preview as it
    /// draws: anywhere else the item would wait for one until another item
    /// is selected.
    pub(super) fn forget_previews(&mut self, cx: &mut Context<Self>) {
        self.previews.clear();
        self.preview_tasks.clear();
        self.selection_changed(cx);
    }

    /// The text `path`'s preview shows, when it has been read and is text.
    #[doc(hidden)]
    pub fn preview_text_for_test(&self, path: &Path) -> Option<String> {
        match &self.previews.get(path)?.1 {
            Preview::Text { text, .. }
            | Preview::Code { text, .. }
            | Preview::Markdown { text } => Some(text.to_string()),
            _ => None,
        }
    }

    /// The line of `path`'s preview that's marked as what the content
    /// search found, as the preview has it.
    #[doc(hidden)]
    pub fn preview_match_for_test(&self, path: &Path) -> Option<String> {
        let (_, Preview::Text { text, .. } | Preview::Code { text, .. }) =
            self.previews.get(path)?
        else {
            return None;
        };
        let mark = hit_range(text, self.content_hit(path)?)?;
        Some(text[mark].to_string())
    }

    /// Space or Cmd+Y: show the selected item in Quick Look, a structured
    /// file as its text, or close it.
    pub fn quick_look(&mut self, _: &QuickLook, window: &mut Window, cx: &mut Context<Self>) {
        self.quick_look_as(QuickLookMode::Text, window, cx);
    }

    /// Cmd+Space: show the selected item in Quick Look, a structured file as
    /// data to explore, or close it.
    pub fn quick_look_explore(
        &mut self,
        _: &QuickLookExplore,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.quick_look_as(QuickLookMode::Explore, window, cx);
    }

    /// Open Quick Look in `mode`, switch it to `mode` when it's open in the
    /// other, or close it. Quick Look is a window of its own, except in
    /// `--term` mode, where the terminal shows one window and it's a dialog
    /// in it.
    fn quick_look_as(&mut self, mode: QuickLookMode, window: &mut Window, cx: &mut Context<Self>) {
        let term = crate::term::active();
        let open = self.quick_look_open && (!term || window.has_active_dialog(cx));
        if open && self.quick_look_mode != mode {
            self.set_quick_look_mode(mode, cx);
            window.refresh();
            return;
        }
        self.quick_look_mode = mode;
        if term {
            self.quick_look_dialog(window, cx);
        } else {
            self.toggle_quick_look_window(window, cx);
        }
        self.ensure_explore(cx);
    }

    /// Forget what Quick Look showed, once it's closed.
    pub(super) fn quick_look_closed(&mut self) {
        self.quick_look_open = false;
        self.explore = None;
    }

    fn quick_look_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.quick_look_open {
            self.quick_look_closed();
            // The flag can outlive the dialog; only close what is open.
            if window.has_active_dialog(cx) {
                window.close_dialog(cx);
                return;
            }
        }
        let Some(entry) = self.primary_entry() else {
            return;
        };
        self.focus_before_dialog(window, cx);
        self.ensure_preview(&entry, cx);
        self.ensure_pdf(&entry, cx);
        self.quick_look_open = true;
        let view = cx.entity();
        let focus = self.quick_look_focus.clone();
        let first = std::cell::Cell::new(true);
        window.open_dialog(cx, move |dialog, window, cx| {
            // Focus the preview so Space (bound in "QuickLook") closes it again.
            if first.replace(false) {
                focus.focus(window, cx);
            }
            let finder = view.read(cx);
            let Some(entry) = finder.primary_entry() else {
                return dialog;
            };
            let open_path = entry.path.clone();
            let open_view = view.clone();
            let close_view = view.clone();
            let text_size = finder
                .preview_of(&entry)
                .has_text()
                .then(|| finder.config.preview_font_size());
            let modes = Finder::can_explore(&entry, cx).then_some(finder.quick_look_mode());
            // Quick Look takes most of the window, so a bigger window shows
            // a bigger preview, and never gets smaller than it used to be.
            let viewport = window.viewport_size();
            let (wide, tall) = (f32::from(viewport.width), f32::from(viewport.height));
            let width = (wide * 0.85).max(f32::from(ui(860.))).min(wide - 32.);
            let height = (tall * 0.85 - f32::from(ui(150.))).max(f32::from(ui(300.)));
            dialog
                .title(
                    h_flex()
                        .gap(ui(8.))
                        .child(file_icon(&entry, ui(20.), false, &palette(cx)))
                        .child(div().truncate().child(entry.name.clone())),
                )
                .w(px(width))
                .child(
                    div()
                        .track_focus(&focus)
                        .key_context(crate::actions::QUICK_LOOK)
                        .on_action(mode_handler::<QuickLook>(&view, QuickLookMode::Text))
                        .on_action(mode_handler::<QuickLookExplore>(
                            &view,
                            QuickLookMode::Explore,
                        ))
                        .on_action({
                            let view = view.clone();
                            move |_: &ExploreInto, window, cx| {
                                view.update(cx, |this, cx| this.explore_into(cx));
                                window.refresh();
                            }
                        })
                        .on_action({
                            let view = view.clone();
                            move |_: &ExploreBack, window, cx| {
                                if !view.update(cx, |this, cx| this.explore_back(cx)) {
                                    window.close_dialog(cx);
                                    view.update(cx, |this, _| this.quick_look_closed());
                                }
                                window.refresh();
                            }
                        })
                        .on_action(text_size_handler::<PreviewTextBigger>(&view, Some(1.)))
                        .on_action(text_size_handler::<PreviewTextSmaller>(&view, Some(-1.)))
                        .on_action(text_size_handler::<PreviewTextReset>(&view, None))
                        .on_action(step_handler::<MoveUp>(&view, 0, -1))
                        .on_action(step_handler::<MoveDown>(&view, 0, 1))
                        .on_action(step_handler::<MoveLeft>(&view, -1, 0))
                        .on_action(step_handler::<MoveRight>(&view, 1, 0))
                        .on_action(page_handler::<NextPdfPage>(&view, 1))
                        .on_action(page_handler::<PreviousPdfPage>(&view, -1))
                        .h(px(height))
                        .child(render_preview_large(
                            finder,
                            &view,
                            &entry,
                            Surface::QuickLook,
                            cx,
                        )),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            h_flex()
                                .flex_1()
                                .gap(ui(8.))
                                .when_some(text_size, |this, size| {
                                    this.child(text_size_buttons(&view, size))
                                })
                                .when_some(modes, |this, mode| {
                                    this.child(mode_buttons(&view, mode))
                                }),
                        )
                        .child(
                            Button::new("ql-reveal")
                                .label(super::menus::reveal_label())
                                .on_click({
                                    let path = entry.path.clone();
                                    move |_, _, cx| crate::system::reveal(&path, cx)
                                }),
                        )
                        .child(Button::new("ql-open").primary().label("Open").on_click(
                            move |_, window, cx| {
                                // `close_dialog` doesn't run `on_close`.
                                window.close_dialog(cx);
                                open_view.update(cx, |this, cx| {
                                    this.quick_look_closed();
                                    // A file no app claims opens in an editor.
                                    this.open_files(vec![open_path.clone()], window, cx);
                                });
                            },
                        )),
                )
                .on_close(move |_, _, cx| {
                    close_view.update(cx, |this, _| this.quick_look_closed());
                })
        });
    }

    /// Quick Look's arrow keys, as in Finder. With several items selected
    /// they step through those and keep the selection. Otherwise they move
    /// the selection the way the view does, except that in the list and in
    /// columns Left and Right go to the previous and next item. Quick Look
    /// then shows the new item.
    pub(super) fn quick_look_step(
        &mut self,
        dx: isize,
        dy: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.explore_move(dx, dy, cx) {
            return;
        }
        let view = self.effective_view();
        if view != ViewMode::Columns && self.selection.len() > 1 {
            self.selection.cycle_cursor(dx + dy);
            if let Some(ix) = self.selection.cursor() {
                self.scroll_to(ix);
            }
            cx.notify();
        } else if matches!(view, ViewMode::List | ViewMode::Columns) {
            self.move_selection(0, dx + dy, false, window, cx);
        } else {
            self.move_selection(dx, dy, false, window, cx);
        }
        if let Some(entry) = self.primary_entry() {
            self.ensure_preview(&entry, cx);
            self.ensure_pdf(&entry, cx);
            self.ensure_explore(cx);
        }
        // Selecting in a column focuses the files; keep the keys coming to
        // the dialog. Quick Look's own window keeps its focus.
        if self.quick_look_window.is_none() {
            self.quick_look_focus.focus(window, cx);
        }
    }
}

/// Space or Cmd+Space in Quick Look's dialog: the mode it's in closes it,
/// and the other switches to that mode.
fn mode_handler<A: Action>(
    view: &Entity<Finder>,
    mode: QuickLookMode,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    let view = view.clone();
    move |_, window, cx| {
        if view.read(cx).quick_look_mode() == mode {
            window.close_dialog(cx);
            view.update(cx, |this, _| this.quick_look_closed());
        } else {
            view.update(cx, |this, cx| this.set_quick_look_mode(mode, cx));
        }
        window.refresh();
    }
}

/// Pages through a PDF from Quick Look, `by` pages at a time.
fn page_handler<A: Action>(
    view: &Entity<Finder>,
    by: isize,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    let view = view.clone();
    move |_, _, cx| {
        view.update(cx, |this, cx| {
            this.step_pdf_page(by, Surface::QuickLook, cx)
        })
    }
}

/// Moves through the items from Quick Look by `dx` and `dy`.
fn step_handler<A: Action>(
    view: &Entity<Finder>,
    dx: isize,
    dy: isize,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    let view = view.clone();
    move |_, window, cx| {
        view.update(cx, |this, cx| this.quick_look_step(dx, dy, window, cx));
        // The dialog is drawn by the window's root, not by the Finder view.
        window.refresh();
    }
}

/// Steps the preview text size by `steps` points, or resets it for `None`.
pub(super) fn step_text_size(
    view: &Entity<Finder>,
    steps: Option<f32>,
    window: &mut Window,
    cx: &mut App,
) {
    view.update(cx, |this, cx| {
        this.step_preview_size(steps, Surface::QuickLook, cx)
    });
    // The dialog is drawn by the window's root, not by the Finder view.
    window.refresh();
}

pub(super) fn text_size_handler<A: Action>(
    view: &Entity<Finder>,
    steps: Option<f32>,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    let view = view.clone();
    move |_, window, cx| step_text_size(&view, steps, window, cx)
}

/// Quick Look's smaller/bigger text buttons, and the size, which resets it.
pub(super) fn text_size_buttons(view: &Entity<Finder>, size: f32) -> impl IntoElement {
    let (min, max) = FONT_SIZE_RANGE;
    let view = view.clone();
    size_stepper(
        "ql-text",
        format!("{size} pt"),
        [size <= min, size >= max, size == DEFAULT_PREVIEW_FONT_SIZE],
        Some([&PreviewTextSmaller, &PreviewTextBigger, &PreviewTextReset]),
        move |steps, window, cx| step_text_size(&view, steps, window, cx),
    )
}

/// A small "A" (smaller), a big "A" (bigger), and the current size, which
/// resets it; `step` gets -1, +1, or `None`. The buttons are `{id}-smaller`,
/// `{id}-bigger` and `{id}-reset`; `disabled` follows the same order, and
/// `keys` are Quick Look actions whose shortcuts the tooltips show.
pub(super) fn size_stepper(
    id: &str,
    label: String,
    disabled: [bool; 3],
    keys: Option<[&dyn Action; 3]>,
    step: impl Fn(Option<f32>, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let step = std::rc::Rc::new(step);
    let tips = ["Smaller Text", "Bigger Text", "Actual Size"];
    let buttons = [
        ("smaller", Some(-1.)),
        ("bigger", Some(1.)),
        ("reset", None),
    ];
    h_flex()
        .gap(ui(2.))
        .children(buttons.into_iter().enumerate().map(|(ix, (name, steps))| {
            let step = step.clone();
            let button = Button::new(SharedString::from(format!("{id}-{name}")))
                .ghost()
                .small()
                .disabled(disabled[ix])
                .on_click(move |_, window, cx| step(steps, window, cx));
            let button = match ix {
                0 => button.child(div().text_size(ui(11.)).child("A")),
                1 => button.child(div().text_size(ui(16.)).child("A")),
                _ => button.label(label.clone()),
            };
            match keys {
                Some(keys) => button.tooltip_with_action(tips[ix], keys[ix], Some(QUICK_LOOK)),
                None => button.tooltip(tips[ix]),
            }
        }))
}

/// A big preview with a caption: the gallery's main area, and Quick Look in
/// `--term` mode.
pub fn render_preview_large(
    finder: &Finder,
    view: &Entity<Finder>,
    entry: &FileEntry,
    surface: Surface,
    cx: &App,
) -> AnyElement {
    v_flex()
        .size_full()
        .gap(ui(10.))
        .child(preview_area(finder, view, entry, surface, cx))
        .child(
            h_flex()
                .justify_center()
                .child(preview_caption(finder, entry, cx)),
        )
        .into_any_element()
}

/// The item's name, then its kind, size, and date.
pub(super) fn preview_caption(finder: &Finder, entry: &FileEntry, cx: &App) -> impl IntoElement {
    // The details go under the name when both don't fit side by side.
    h_flex()
        .flex_wrap()
        .gap_x(ui(8.))
        .text_size(ui(12.))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(entry.name.clone()),
        )
        .child(
            div()
                .text_color(cx.theme().muted_foreground)
                .child(info_line(finder, entry)),
        )
}

/// The preview itself, filling its space; a PDF with its toolbar.
pub(super) fn preview_area(
    finder: &Finder,
    view: &Entity<Finder>,
    entry: &FileEntry,
    surface: Surface,
    cx: &App,
) -> AnyElement {
    if surface == Surface::QuickLook && finder.explores(entry, cx) {
        return super::explore::render_explore(finder, view, entry, cx);
    }
    let pdf = finder.pdf_view(entry);
    let body = preview_body(
        finder,
        entry,
        ui(220.),
        finder.config.preview_font_size(),
        cx,
    );
    // A PDF shows the system's thumbnail of its first page until hayro has
    // drawn the page.
    match pdf {
        Some(pdf) => finder.render_pdf(pdf, view, surface, body, cx),
        None => div()
            .size_full()
            .rounded(ui(8.))
            .overflow_hidden()
            .child(body)
            .into_any_element(),
    }
}

/// A compact card for the column view's last column.
pub fn render_preview_card(finder: &Finder, entry: &FileEntry, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.muted_foreground;
    let row = |label: &'static str, value: String| {
        let value = SharedString::from(value);
        let shown = StyledText::new(value.clone());
        h_flex()
            .gap(ui(8.))
            .text_size(ui(11.))
            .child(
                div()
                    .w(ui(70.))
                    .flex_shrink_0()
                    .text_color(muted)
                    .child(label),
            )
            .child(super::fit::with_cut_off_text(
                div().id(label).flex_1().truncate(),
                shown,
                value,
                |shown| shown,
            ))
    };
    v_flex()
        .size_full()
        .gap(ui(12.))
        .child(
            div()
                .h(ui(240.))
                .w_full()
                .rounded(ui(8.))
                .overflow_hidden()
                .child(preview_body(
                    finder,
                    entry,
                    ui(120.),
                    DEFAULT_PREVIEW_FONT_SIZE,
                    cx,
                )),
        )
        .child(
            div()
                .text_size(ui(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(entry.name.clone()),
        )
        .child(row("Kind", entry.kind_label()))
        .when_some(entry.size, |this, size| {
            this.child(row("Size", format::human_size(size)))
        })
        .when_some(
            match finder.preview_of(entry) {
                Preview::Media { duration } => duration,
                _ => None,
            },
            |this, duration| {
                this.child(row(
                    "Duration",
                    fffinder_core::media::format_duration(duration),
                ))
            },
        )
        .when_some(entry.modified, |this, t| {
            this.child(row("Modified", format::human_date(t)))
        })
        .child(row(
            "Where",
            entry.parent().map(super::tildify).unwrap_or_default(),
        ))
        .into_any_element()
}

fn info_line(finder: &Finder, entry: &FileEntry) -> String {
    let mut parts = vec![entry.kind_label()];
    if let Some(pages) = finder.pdf_view(entry).map(|pdf| pdf.page_count()) {
        parts.push(match pages {
            1 => "1 page".to_string(),
            n => format!("{n} pages"),
        });
    }
    if let Preview::Media {
        duration: Some(duration),
    } = finder.preview_of(entry)
    {
        parts.push(fffinder_core::media::format_duration(duration));
    }
    if let Some(size) = entry.size {
        parts.push(format::human_size(size));
    }
    if let Some(time) = entry.modified {
        parts.push(format::human_date(time));
    }
    parts.join(" · ")
}

/// The preview itself; `text_size` is in points.
fn preview_body(
    finder: &Finder,
    entry: &FileEntry,
    icon: Pixels,
    text_size: f32,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let pal = palette(cx);
    let centered_icon = |extra: Option<String>| {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap(ui(10.))
            .child(file_icon(entry, icon, false, &pal))
            .when_some(extra, |this, text| {
                this.child(
                    div()
                        .text_size(ui(12.))
                        .text_color(theme.muted_foreground)
                        .child(text),
                )
            })
            .into_any_element()
    };
    // QuickLook's picture of the file (macOS), or its icon.
    let picture = |caption: Option<String>| {
        let (shown, muted) = (entry.clone(), theme.muted_foreground);
        large_thumbnail(entry, move || {
            icon_with_caption(&shown, icon, &pal, muted, caption.clone())
        })
    };
    let is_image = entry.kind == FileKind::Image;
    match finder.preview_of(entry) {
        _ if is_image => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                img(PreviewImages::source(&finder.preview_images, entry))
                    .size_full()
                    .object_fit(ObjectFit::Contain)
                    .with_fallback({
                        // Formats GPUI can't decode (HEIC, RAW, PSD) use QuickLook's.
                        let (entry, muted) = (entry.clone(), theme.muted_foreground);
                        move || {
                            let shown = entry.clone();
                            large_thumbnail(&entry, move || {
                                icon_with_caption(&shown, icon, &pal, muted, None)
                            })
                        }
                    }),
            )
            .into_any_element(),
        Preview::Loading => v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child(Spinner::new().small())
            .into_any_element(),
        Preview::Image | Preview::Unavailable => picture(None),
        Preview::Media { duration } => picture(duration.map(fffinder_core::media::format_duration)),
        Preview::Markdown { text } => div()
            .id(("markdown-preview", super::views::path_id(&entry.path)))
            .size_full()
            .bg(theme.background)
            .px(ui(16.))
            .py(ui(12.))
            .text_size(ui(text_size + 1.5))
            .child(
                TextView::markdown(("markdown", super::views::path_id(&entry.path)), text)
                    .selectable(true)
                    .scrollable(true)
                    .size_full(),
            )
            .into_any_element(),
        Preview::Font {
            family,
            weight,
            italic,
            ..
        } => {
            let samples = [
                (text_size + 2., "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
                (text_size + 2., "abcdefghijklmnopqrstuvwxyz"),
                (text_size + 2., "0123456789 !?&@#%*()[]{}"),
                (
                    text_size * 2.4,
                    "The quick brown fox jumps over the lazy dog",
                ),
            ];
            v_flex()
                .size_full()
                .bg(theme.background)
                .p(ui(16.))
                .gap(ui(10.))
                .overflow_y_scrollbar()
                // The scrollbar wrapper keeps the scroll position under its own
                // ID, not the element's. Made from the file's path, it starts
                // each preview at its top, not where the last one was left.
                .id(("font-preview", super::views::path_id(&entry.path)))
                .child(
                    div()
                        .text_size(ui(12.))
                        .text_color(theme.muted_foreground)
                        .child(family.clone()),
                )
                .children(samples.into_iter().map(|(size, sample)| {
                    div()
                        .font_family(family.clone())
                        .font_weight(weight)
                        .when(italic, |this| this.italic())
                        .text_size(ui(size))
                        .child(sample)
                }))
                .into_any_element()
        }
        Preview::Folder { items } => centered_icon(Some(if items == 1 {
            "1 item".into()
        } else {
            format!("{items} items")
        })),
        Preview::Text { text, truncated } => text_preview(
            ("text-preview", super::views::path_id(&entry.path)),
            text,
            Vec::new(),
            truncated,
            finder.content_hit(&entry.path),
            text_size,
            cx,
        ),
        Preview::Code {
            text,
            mut spans,
            comments,
            truncated,
        } => {
            let comment = HighlightStyle {
                color: Some(theme.muted_foreground),
                ..Default::default()
            };
            spans.extend(comments.into_iter().map(|range| (range, comment)));
            spans.sort_by_key(|(range, _)| range.start);
            text_preview(
                ("code-preview", super::views::path_id(&entry.path)),
                text,
                spans,
                truncated,
                finder.content_hit(&entry.path),
                text_size,
                cx,
            )
        }
        Preview::Table {
            columns,
            rows,
            footer,
        } => {
            let header_bg = theme.muted;
            let border = theme.border;
            let ncols = columns.len().max(1);
            // Columns widen with the text.
            let cell = 140. * text_size / DEFAULT_PREVIEW_FONT_SIZE;
            let width = ui((ncols as f32 * cell).max(300.));
            v_flex()
                .size_full()
                .bg(theme.background)
                .border_1()
                .border_color(border)
                .rounded(ui(8.))
                .overflow_hidden()
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        // The wrapper that scrolls both ways lays this out in
                        // a row, which stretches it to the scroll area's
                        // height with the table spilling out of it, and an
                        // area scrolls no further than its own children
                        // reach. Aligned to the top, it's as tall as the table.
                        .self_start()
                        .overflow_scrollbar()
                        // The wrapper's ID, so that each file has its own
                        // scroll position.
                        .id(("table-preview", super::views::path_id(&entry.path)))
                        .child(
                            v_flex()
                                .id("preview-table")
                                .test_support()
                                .min_w(width)
                                .text_size(ui(text_size))
                                .child(
                                    h_flex()
                                        .bg(header_bg)
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .children(
                                            columns.iter().map(|c| table_cell(c.clone(), cell)),
                                        ),
                                )
                                .children(rows.iter().enumerate().map(|(ix, row)| {
                                    h_flex()
                                        .when(ix % 2 == 1, |this| this.bg(pal.stripe))
                                        .children(row.iter().map(|c| table_cell(c.clone(), cell)))
                                })),
                        ),
                )
                .when_some(footer, |this, footer| {
                    this.child(
                        h_flex()
                            .id("preview-table-footer")
                            .test_support()
                            .px(ui(10.))
                            .py(ui(4.))
                            .border_t_1()
                            .border_color(border)
                            .text_size(ui(10.5))
                            .text_color(theme.muted_foreground)
                            .child(footer),
                    )
                })
                .into_any_element()
        }
    }
}

/// The colors and marks of a preview's text, by the bytes they cover, in
/// order and apart.
type Spans = Vec<(Range<usize>, HighlightStyle)>;

/// The text of a text or code preview, scrolling when it's longer than its
/// space. `hit` is the line a content search found in the file: it's marked,
/// and the preview opens with it in view rather than at the file's start.
fn text_preview(
    id: (&'static str, usize),
    text: SharedString,
    spans: Spans,
    truncated: bool,
    hit: Option<&ContentHit>,
    text_size: f32,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let mark = hit.and_then(|hit| hit_range(&text, hit));
    // A long file's match can be past the part of it that was read.
    let beyond = truncated && hit.is_some_and(|hit| line_range(&text, hit.line).is_none());
    let spans = match &mark {
        Some(mark) => with_mark(spans, mark, palette(cx).highlight),
        None => spans,
    };
    let note = truncated.then(|| {
        let note = SharedString::from(match hit.filter(|_| beyond) {
            Some(ContentHit { line, .. }) => {
                format!("… (preview truncated; the match on line {line} is further down)")
            }
            None => "… (preview truncated)".to_string(),
        });
        div()
            .id("preview-note")
            .aria_label(note.clone())
            .pt(ui(8.))
            .text_color(theme.muted_foreground)
            .child(note)
            .test_support()
    });
    let body = div()
        .size_full()
        .bg(theme.muted)
        .p(ui(12.))
        .font_family(theme.mono_font_family.clone())
        .text_size(ui(text_size));
    let block = |id: &'static str, text: SharedString, spans: Spans| {
        div()
            .id(id)
            .test_support()
            .child(StyledText::new(text).with_highlights(spans))
    };
    // Where the lines kept in view above the match begin. A match in a
    // file's first lines needs no scrolling to.
    let start = mark
        .map(|mark| context_start(&text, mark.start))
        .filter(|start| *start > 0);
    let Some(hit) = hit.filter(|_| start.is_some() || beyond) else {
        return body
            .overflow_y_scrollbar()
            // The wrapper's ID, so that each file has its own scroll position.
            .id(id)
            // The scrollbar wrapper hides its own element from UI tests, so
            // the text inside is the observed one.
            .child(block("preview-text", text, spans))
            .children(note)
            .into_any_element();
    };
    let mut blocks = match start {
        // In two blocks, so that the second can be scrolled to. A block
        // starts on a new line, as the line break between the two did.
        Some(start) => {
            let (first, second) = split_spans(spans, start);
            vec![
                block("preview-text", text[..start - 1].into(), first),
                block("preview-match", text[start..].into(), second),
            ]
        }
        // The match isn't here to scroll to, so the note that says so is.
        None => vec![block("preview-text", text, spans)],
    };
    blocks.extend(note);
    MatchScroll {
        id: id.into(),
        line: hit.line,
        body,
        blocks: blocks.into_iter().map(|b| b.into_any_element()).collect(),
    }
    .into_any_element()
}

/// The bytes of line `line` of `text`, counted from 1, without its line
/// break. `None` when the text has no such line.
fn line_range(text: &str, line: u64) -> Option<Range<usize>> {
    let mut start = 0;
    let mut number = 0;
    for piece in text.split_inclusive('\n') {
        number += 1;
        if number == line {
            return Some(start..start + piece.trim_end_matches(['\n', '\r']).len());
        }
        start += piece.len();
    }
    None
}

/// The bytes of the line a content search found, in `text`. `None` when the
/// text has no such line, or has other words there: the file was changed
/// after it was searched or read, and the mark would be on the wrong line.
fn hit_range(text: &str, hit: &ContentHit) -> Option<Range<usize>> {
    line_range(text, hit.line).filter(|line| {
        text[line.clone()]
            .trim_start()
            .starts_with(hit.text.as_str())
    })
}

/// Where a preview that opens at the line starting at `line_start` begins
/// what it shows: a few lines earlier, so the match isn't right under the
/// preview's edge, or at the line itself when those before it are long.
fn context_start(text: &str, line_start: usize) -> usize {
    let mut start = line_start;
    for _ in 0..MATCH_CONTEXT_LINES {
        // A line starts after the line break that ends the one before it.
        let Some(before) = text[..start].strip_suffix('\n') else {
            break;
        };
        let line_before = before.rfind('\n').map_or(0, |at| at + 1);
        if line_start - line_before > MATCH_CONTEXT_BYTES {
            break;
        }
        start = line_before;
    }
    start
}

/// `spans` with the text in `mark` drawn as a match: on `color`, and in the
/// preview's own text color whatever colored it before. A grammar picks its
/// colors for the preview's background, and on the mark some are hard to
/// read, gray comments in the dark theme most of all. Weight and slant stay.
fn with_mark(spans: Spans, mark: &Range<usize>, color: Hsla) -> Spans {
    let marked = |style: HighlightStyle| HighlightStyle {
        background_color: Some(color),
        color: None,
        ..style
    };
    let mut out = Vec::with_capacity(spans.len() + 3);
    // How far into the mark `out` has got.
    let mut at = mark.start;
    for (range, style) in spans {
        // The parts of the span before the mark, in it, and after it.
        let before = range.start..range.end.min(mark.start);
        let inside = range.start.max(at)..range.end.min(mark.end);
        let after = range.start.max(mark.end)..range.end;
        if !before.is_empty() {
            out.push((before, style));
        }
        if !inside.is_empty() {
            // Text between two spans has no style but the mark's.
            if at < inside.start {
                out.push((at..inside.start, marked(HighlightStyle::default())));
            }
            at = inside.end;
            out.push((inside, marked(style)));
        }
        if !after.is_empty() {
            if at < mark.end {
                out.push((at..mark.end, marked(HighlightStyle::default())));
                at = mark.end;
            }
            out.push((after, style));
        }
    }
    if at < mark.end {
        out.push((at..mark.end, marked(HighlightStyle::default())));
    }
    out
}

/// `spans` for a text drawn in two blocks, the second from the line that
/// starts at `start`. The first block's spans stay where they are, the
/// second's are counted from `start`, and a span over both is cut in two.
/// The line break before `start` is in neither block.
fn split_spans(spans: Spans, start: usize) -> (Spans, Spans) {
    let (mut first, mut second) = (Vec::new(), Vec::new());
    for (range, style) in spans {
        let before = range.start..range.end.min(start - 1);
        let after = range.start.max(start)..range.end;
        if !before.is_empty() {
            first.push((before, style));
        }
        if !after.is_empty() {
            second.push((after.start - start..after.end - start, style));
        }
    }
    (first, second)
}

/// A text preview that opens with a matching line in view. The kit's
/// scrollbar wrapper, which every other text preview scrolls in, keeps its
/// scroll handle to itself, so this is that wrapper built here around a
/// handle that can be told where to go. `body` scrolls, and of the `blocks`
/// in it the second is the one that goes to the top.
#[derive(IntoElement)]
struct MatchScroll {
    /// Made from the file's path, like the wrapper's, so that each file has
    /// its own scroll position.
    id: ElementId,
    /// The matching line. The preview goes to it once, and again only when
    /// another line takes its place.
    line: u64,
    body: Div,
    blocks: Vec<AnyElement>,
}

/// What a [`MatchScroll`] keeps for as long as it's drawn.
#[derive(Default)]
struct MatchScrollState {
    handle: ScrollHandle,
    /// The matching line the preview has gone to.
    at: Cell<Option<u64>>,
}

impl RenderOnce for MatchScroll {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut new = false;
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| {
            new = true;
            MatchScrollState::default()
        });
        let state = state.read(cx);
        let handle = state.handle.clone();
        // A handle goes to an item from where its area was last laid out,
        // which a new handle's never was: it would overshoot by the area's
        // distance from the top of the window. So the first frame draws the
        // area empty, and the text comes with the next, already in place.
        if new {
            window.request_animation_frame();
        } else if state.at.replace(Some(self.line)) != Some(self.line) {
            handle.scroll_to_top_of_item(1);
        }
        let area = self
            .body
            .id((self.id.clone(), "area"))
            .overflow_y_scroll()
            .restrict_scroll_to_axis()
            .track_scroll(&handle)
            .when(!new, |this| this.children(self.blocks));
        div()
            .id(self.id.clone())
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .overflow_y_hidden()
            // The wrapper's way with the wheel: the area's alone while it
            // can scroll, with a bounce at either end on a Mac.
            .child(ScrollBounce::new((self.id, "bounce"), &handle, area))
            .vertical_scrollbar(&handle)
    }
}

/// A file's icon, centered, with an optional line under it.
fn icon_with_caption(
    entry: &FileEntry,
    icon: Pixels,
    pal: &crate::theme::Palette,
    muted: Hsla,
    caption: Option<String>,
) -> AnyElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap(ui(10.))
        .child(file_icon(entry, icon, false, pal))
        .when_some(caption, |this, text| {
            this.child(div().text_size(ui(12.)).text_color(muted).child(text))
        })
        .into_any_element()
}

/// QuickLook's picture of `entry` filling the space (macOS), or `fallback`
/// while it loads and when there is none.
fn large_thumbnail(
    entry: &FileEntry,
    fallback: impl Fn() -> AnyElement + Clone + 'static,
) -> AnyElement {
    if !crate::thumbnails::wanted(entry) {
        return fallback();
    }
    img(crate::thumbnails::image(entry, 1024))
        .size_full()
        .object_fit(ObjectFit::Contain)
        .with_loading(fallback.clone())
        .with_fallback(fallback)
        .into_any_element()
}

fn table_cell(text: SharedString, width: f32) -> impl IntoElement {
    div()
        .w(ui(width))
        .flex_shrink_0()
        .px(ui(8.))
        .py(ui(3.))
        .truncate()
        .child(text)
}

#[cfg(test)]
mod tests {
    // Not `super::*`: GPUI's `test` attribute would replace the standard one.
    use super::{
        ContentHit, Preview, context_start, hit_range, line_range, load_preview, split_spans,
        with_mark,
    };
    use crate::nushell;
    use fffinder_core::FileEntry;
    use fffinder_core::ops::Progress;
    use gpui_kit::{FontWeight, HighlightStyle, Hsla, blue, red, yellow};

    /// Runs only where `nu` is installed.
    #[test]
    fn nushell_scripts_are_highlighted() {
        let Some(nu) = nushell::find_nu(None) else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("fff-highlight-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("demo.nu");
        std::fs::write(
            &script,
            "# Say hi\ndef greet [name: string] {\n  $\"hi ($name) #1\"\n}\n",
        )
        .unwrap();
        let entry = FileEntry::from_path(&script).unwrap();
        match load_preview(&entry, Some(&nu), false) {
            Preview::Code {
                text,
                spans,
                comments,
                ..
            } => {
                assert!(text.starts_with("# Say hi"), "{text}");
                assert_eq!(comments.len(), 1, "the comment is found");
                assert_eq!(&text[comments[0].clone()], "# Say hi");
                assert!(!text.contains('\u{1b}'));
                assert!(!spans.is_empty(), "nu-highlight colored something");
            }
            _ => panic!("expected highlighted code"),
        }
        // Structured files that are text stay text, colored; a binary one
        // is opened with Nushell.
        for (name, text) in [
            ("Cargo.toml", "[package]\nname = \"fff\"\n"),
            ("data.json", "{\"a\": [1, 2]}\n"),
            ("people.csv", "name,age\nana,31\n"),
        ] {
            std::fs::write(dir.join(name), text).unwrap();
            let entry = FileEntry::from_path(dir.join(name)).unwrap();
            assert!(
                matches!(load_preview(&entry, Some(&nu), false), Preview::Code { .. }),
                "{name} is highlighted text"
            );
        }
        let db = dir.join("shop.db");
        let made = std::process::Command::new(&nu)
            .args(["--no-config-file", "-c"])
            .arg("[[item qty]; [pen 3]] | into sqlite $env.DB --table-name stock")
            .env("DB", &db)
            .status()
            .unwrap();
        assert!(made.success());
        let entry = FileEntry::from_path(&db).unwrap();
        assert!(matches!(
            load_preview(&entry, Some(&nu), false),
            Preview::Table { .. }
        ));

        let plain = dir.join("notes.txt");
        std::fs::write(&plain, "def greet").unwrap();
        let entry = FileEntry::from_path(&plain).unwrap();
        assert!(matches!(
            load_preview(&entry, Some(&nu), false),
            Preview::Text { .. }
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_matching_line_is_found_by_its_number() {
        let text = "first\nsecond line\n\nlast, cut off";
        assert_eq!(line_range(text, 1), Some(0..5));
        assert_eq!(line_range(text, 2).map(|at| &text[at]), Some("second line"));
        assert_eq!(line_range(text, 3).map(|at| &text[at]), Some(""));
        assert_eq!(
            line_range(text, 4).map(|at| &text[at]),
            Some("last, cut off"),
            "where the preview stopped reading"
        );
        assert_eq!(line_range(text, 5), None, "past what was read");
        assert_eq!(line_range(text, 0), None, "lines count from 1");
        assert_eq!(line_range("only\n", 2), None, "a line break ends a line");
        assert_eq!(line_range("", 1), None);

        // Both characters of a Windows line break stay out of the line.
        let dos = "one\r\ntwo\r\nthree\r\n";
        assert_eq!(line_range(dos, 2).map(|at| &dos[at]), Some("two"));

        // Lines go by their line breaks, however many bytes a letter takes.
        let wide = "naïve café\n日本語のテキスト\nend\n";
        assert_eq!(
            line_range(wide, 2).map(|at| &wide[at]),
            Some("日本語のテキスト")
        );
        assert_eq!(line_range(wide, 3).map(|at| &wide[at]), Some("end"));
    }

    #[test]
    fn a_line_is_marked_only_while_it_reads_as_the_search_found_it() {
        let hit = |line, text: &str| ContentHit {
            line,
            text: text.to_string(),
        };
        let text = "one\n    let needle = 2;  \r\nthree\n";
        // The search hands a line on without the space at its ends.
        assert_eq!(
            hit_range(text, &hit(2, "let needle = 2;")).map(|at| &text[at]),
            Some("    let needle = 2;  ")
        );
        assert!(
            hit_range(text, &hit(2, "let needle")).is_some(),
            "a long line is cut short by the search, not by the preview"
        );

        // The file was changed after it was searched, or after it was read.
        assert_eq!(
            hit_range(text, &hit(3, "let needle = 2;")),
            None,
            "another line is there now"
        );
        assert_eq!(hit_range(text, &hit(9, "let needle = 2;")), None);
    }

    #[test]
    fn a_preview_opens_a_little_above_its_match() {
        let line_start = |text: &str, line| line_range(text, line).unwrap().start;
        let text = "one\ntwo\nthree\nfour\nfive\n";
        assert_eq!(
            context_start(text, line_start(text, 5)),
            line_start(text, 3),
            "two lines stay above the match"
        );
        assert_eq!(context_start(text, line_start(text, 3)), 0);
        assert_eq!(
            context_start(text, line_start(text, 2)),
            0,
            "one is all there is"
        );
        assert_eq!(context_start(text, 0), 0);

        // Long lines wrap, and enough of them would push the match out.
        let long = "x".repeat(200);
        let text = format!("short\n{long}\nabove\nmatch\n");
        assert_eq!(
            context_start(&text, line_start(&text, 4)),
            line_start(&text, 3),
            "only the short line above"
        );
        let text = format!("short\n{long}\nmatch\n");
        assert_eq!(
            context_start(&text, line_start(&text, 3)),
            line_start(&text, 3)
        );
    }

    fn colored(color: Hsla) -> HighlightStyle {
        HighlightStyle {
            color: Some(color),
            ..Default::default()
        }
    }

    #[test]
    fn a_marked_line_is_in_plain_text_color_on_the_mark() {
        let mark = yellow();
        let plain = HighlightStyle {
            background_color: Some(mark),
            ..Default::default()
        };
        let bold = |style: HighlightStyle| HighlightStyle {
            font_weight: Some(FontWeight::BOLD),
            ..style
        };
        assert_eq!(
            with_mark(Vec::new(), &(4..9), mark),
            [(4..9, plain)],
            "plain text has the mark alone"
        );
        // A span before the mark, one over its start, a gap, one over its
        // end, and one after it.
        let spans = vec![
            (0..2, colored(red())),
            (3..6, colored(blue())),
            (8..12, bold(colored(red()))),
            (14..16, colored(blue())),
        ];
        assert_eq!(
            with_mark(spans, &(4..10), mark),
            [
                (0..2, colored(red())),
                (3..4, colored(blue())),
                (4..6, plain),
                (6..8, plain),
                (8..10, bold(plain)),
                (10..12, bold(colored(red()))),
                (14..16, colored(blue())),
            ],
            "the colors go and the weight stays"
        );
        let on_red = HighlightStyle {
            background_color: Some(red()),
            ..colored(blue())
        };
        assert_eq!(
            with_mark(vec![(0..6, on_red)], &(2..4), mark),
            [(0..2, on_red), (2..4, plain), (4..6, on_red)],
            "the mark shows over a background the text had"
        );

        // Wherever the mark falls, the bytes in it change their color for
        // the mark, the others keep their style, and the spans stay in order
        // and apart, as drawing them needs.
        let style_at = |spans: &[(std::ops::Range<usize>, HighlightStyle)], at: usize| {
            let style = spans.iter().find(|(range, _)| range.contains(&at));
            style.map(|(_, style)| *style).unwrap_or_default()
        };
        let layouts = [
            vec![],
            vec![0..6, 6..12],
            vec![0..3, 3..7, 7..12],
            vec![2..4, 6..9],
            vec![1..2, 5..6, 10..11],
        ];
        for layout in layouts {
            let styles = [colored(red()), bold(colored(blue()))];
            let spans: Vec<_> = layout.into_iter().zip(styles.into_iter().cycle()).collect();
            for start in 0..12 {
                for end in start + 1..=12 {
                    let out = with_mark(spans.clone(), &(start..end), mark);
                    for at in 0..12 {
                        let mut expected = style_at(&spans, at);
                        if (start..end).contains(&at) {
                            expected.color = None;
                            expected.background_color = Some(mark);
                        }
                        assert_eq!(
                            style_at(&out, at),
                            expected,
                            "byte {at}, marking {start}..{end} of {spans:?}"
                        );
                    }
                    assert!(
                        out.iter().all(|(range, _)| !range.is_empty())
                            && out.windows(2).all(|two| two[0].0.end <= two[1].0.start),
                        "marking {start}..{end} of {spans:?} gave {out:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn spans_follow_a_text_cut_in_two_at_a_line() {
        // "one\ntwo\nthree", cut where "three" starts.
        let spans = vec![
            (0..3, colored(red())),
            (4..10, colored(blue())),
            (10..13, colored(red())),
        ];
        let (first, second) = split_spans(spans, 8);
        assert_eq!(
            first,
            [(0..3, colored(red())), (4..7, colored(blue()))],
            "up to the line break"
        );
        assert_eq!(
            second,
            [(0..2, colored(blue())), (2..5, colored(red()))],
            "counted from the cut"
        );

        let (first, second) = split_spans(vec![(7..8, colored(red()))], 8);
        assert!(
            first.is_empty() && second.is_empty(),
            "the line break is in neither part"
        );
    }

    #[test]
    fn previews_by_kind() {
        let dir = std::env::temp_dir().join(format!("fff-previews-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("pack")).unwrap();
        let write = |name: &str, text: &str| {
            std::fs::write(dir.join(name), text).unwrap();
            FileEntry::from_path(dir.join(name)).unwrap()
        };
        let load = |entry: &FileEntry| load_preview(entry, None, false);

        let code = write("main.py", "def main():\n    return 1\n");
        assert!(matches!(load(&code), Preview::Code { spans, .. } if spans.len() > 1));
        let notes = write("notes.md", "# Title\n\nSome *text*.\n");
        assert!(matches!(load(&notes), Preview::Markdown { .. }));
        assert!(matches!(
            load(&write("plain.txt", "words")),
            Preview::Text { .. }
        ));

        std::fs::write(dir.join("pack/a.txt"), "a").unwrap();
        let zip = dir.join("pack.zip");
        fffinder_core::archive::compress(&[dir.join("pack")], &zip, &Progress::default()).unwrap();
        match load(&FileEntry::from_path(&zip).unwrap()) {
            Preview::Table { rows, footer, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(footer.as_deref(), Some("2 items"));
            }
            _ => panic!("an archive lists its contents"),
        }

        // 1 s of 8 kHz, 8-bit mono WAV.
        let mut wav = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0".to_vec();
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(b"\x01\0\x08\0data");
        wav.extend_from_slice(&8000u32.to_le_bytes());
        std::fs::write(dir.join("tone.wav"), &wav).unwrap();
        let tone = FileEntry::from_path(dir.join("tone.wav")).unwrap();
        assert!(matches!(
            load(&tone),
            Preview::Media { duration: Some(d) } if d.as_secs() == 1
        ));

        let fonts = [
            "/System/Library/Fonts/Supplemental/Georgia.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];
        if let Some(font) = fonts.iter().find(|f| std::path::Path::new(f).is_file()) {
            match load(&FileEntry::from_path(font).unwrap()) {
                Preview::Font {
                    family,
                    weight,
                    italic,
                    data,
                } => {
                    assert!(!family.is_empty() && data.is_some());
                    assert_eq!(
                        (weight, italic),
                        (gpui_kit::FontWeight::NORMAL, false),
                        "a regular face"
                    );
                }
                _ => panic!("a font shows itself"),
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
