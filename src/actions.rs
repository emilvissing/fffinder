//! Actions, key bindings, and the application menu bar.

use gpui_kit::component::input;
use gpui_kit::{
    Action, App, KeyBinding, KeyBindingContextPredicate, Menu, MenuItem, OsAction, actions,
};
use std::rc::Rc;

actions!(
    finder,
    [
        Quit,
        NewWindow,
        NewTab,
        ShowNextTab,
        ShowPreviousTab,
        MoveTabToNewWindow,
        ToggleSplitView,
        CopyToOtherPane,
        MoveToOtherPane,
        CloseWindow,
        ShowSettings,
        FocusSearch,
        GoBack,
        GoForward,
        GoUp,
        GoHome,
        GoRecents,
        GoToFolder,
        ConnectToServer,
        OpenSelected,
        QuickLook,
        QuickLookExplore,
        ExploreInto,
        ExploreBack,
        GetInfo,
        ToggleSidebar,
        SidebarAsList,
        SidebarAsTree,
        TogglePreviewPane,
        ViewIcons,
        ViewList,
        ViewColumns,
        ViewGallery,
        ToggleHidden,
        ToggleFoldersFirst,
        ToggleGroups,
        Refresh,
        SelectAll,
        Escape,
        MoveUp,
        MoveDown,
        MoveLeft,
        MoveRight,
        ExtendUp,
        ExtendDown,
        ExtendLeft,
        ExtendRight,
        MoveHome,
        MoveEnd,
        ExtendHome,
        ExtendEnd,
        ExtendPageUp,
        ExtendPageDown,
        PageUp,
        PageDown,
        CopyPath,
        CopyAsNuon,
        MoveItemsHere,
        ShowInEnclosingFolder,
        RevealInFileManager,
        MoveToTrash,
        PutBack,
        DeleteImmediately,
        EmptyTrash,
        GoTrash,
        NewFolder,
        NewFile,
        Duplicate,
        MakeAlias,
        MakeSymlink,
        Compress,
        Extract,
        Rename,
        OpenInNushell,
        ToggleNuPanel,
        SearchHelp,
        Rescan,
        Choose,
        ToggleAppearance,
        ToggleFilter,
        SaveSearch,
        OpenInEditor,
        PreviewTextBigger,
        PreviewTextSmaller,
        PreviewTextReset,
        NextPdfPage,
        PreviousPdfPage,
    ]
);

/// Key context of the browser window.
pub const FINDER: &str = "Finder";
/// Key context of the file views; arrows, Space and Return only apply here
/// so they keep working normally in text fields.
pub const FILES: &str = "FinderFiles";
/// [`FILES`], in a window with no second pane, where F5 refreshes as it does
/// in other file managers instead of copying across.
pub const FILES_ONE_PANE: &str = "FinderFiles OnePane";
/// The file views while a name is being edited in them.
pub const FILES_RENAMING: &str = "FinderFilesRenaming";
/// Key context of the Quick Look preview, so Space closes it and the arrows
/// move through the items, or through explore mode's grid.
pub const QUICK_LOOK: &str = "QuickLook";
/// Key context of the sidebar's folder tree, once a row is clicked: the
/// arrows move through it and the file keys act on its folder.
pub const TREE: &str = "FolderTree";
/// The folder tree while a name is being edited in it.
pub const TREE_RENAMING: &str = "FolderTreeRenaming";

/// The key bindings, in the order they're bound. Where an action has more
/// than one key, the order decides which is shown beside it: the macOS menu
/// bar shows the first, and the window's own menus and tooltips the last.
/// `mac` is whether the keys are for macOS.
fn key_table(mac: bool, cx: &App) -> Vec<KeyBinding> {
    let finder = context(FINDER);
    let files = context(FILES);
    let quick_look = context(QUICK_LOOK);
    let tree = context(TREE);
    let one_pane = context("FinderFiles && OnePane");
    let mut bindings = Vec::new();
    // The first key the table has for Refresh, Open, and Quick Look is F5,
    // Return, and Space, and a Mac's menu bar would show those. Finder's shows
    // Cmd-O and Cmd-Y, and Cmd-R is how a Mac refreshes, so a Mac binds these
    // here too, ahead of the table, whose order decides what the window's
    // menus show on every system. They still act once; see the end of the
    // table.
    //
    // A key the window doesn't take is the menu bar's to try, and with Cmd-R
    // on Refresh the one item there with F5 is Copy to Other Pane. So that F5
    // outside the file views can't copy, a Mac has it refresh anywhere in the
    // window. In the file views the table's own F5 bindings come first, bound
    // as they are to less of the window.
    if mac {
        bindings.extend([
            key("secondary-r", Refresh, &finder, cx),
            key("f5", Refresh, &finder, cx),
            key("secondary-o", OpenSelected, &files, cx),
            key("secondary-y", QuickLook, &files, cx),
        ]);
    }
    bindings.extend([
        key("secondary-q", Quit, &None, cx),
        key("secondary-,", ShowSettings, &None, cx),
        key("secondary-n", NewWindow, &None, cx),
        key("secondary-w", CloseWindow, &finder, cx),
        key("secondary-t", NewTab, &finder, cx),
        key("ctrl-tab", ShowNextTab, &finder, cx),
        key("ctrl-shift-tab", ShowPreviousTab, &finder, cx),
        key("secondary-shift-]", ShowNextTab, &finder, cx),
        key("secondary-shift-[", ShowPreviousTab, &finder, cx),
        key("secondary-\\", ToggleSplitView, &finder, cx),
        key("secondary-f", FocusSearch, &finder, cx),
        // Finder's key for each of these goes first, for the menu bar.
        key("secondary-[", GoBack, &finder, cx),
        key("secondary-]", GoForward, &finder, cx),
        key("alt-left", GoBack, &finder, cx),
        key("alt-right", GoForward, &finder, cx),
        key("secondary-up", GoUp, &finder, cx),
        key("alt-up", GoUp, &finder, cx),
        key("secondary-shift-h", GoHome, &finder, cx),
        key("secondary-shift-f", GoRecents, &finder, cx),
        key("secondary-shift-g", GoToFolder, &finder, cx),
        key("secondary-k", ConnectToServer, &finder, cx),
        key("secondary-1", ViewIcons, &finder, cx),
        key("secondary-2", ViewList, &finder, cx),
        key("secondary-3", ViewColumns, &finder, cx),
        key("secondary-4", ViewGallery, &finder, cx),
        key("secondary-shift-.", ToggleHidden, &finder, cx),
        key("f5", CopyToOtherPane, &files, cx),
        key("f5", Refresh, &one_pane, cx),
        key("secondary-r", Refresh, &finder, cx),
        key("secondary-alt-s", ToggleSidebar, &finder, cx),
        key("secondary-shift-p", TogglePreviewPane, &finder, cx),
        key("secondary-shift-n", NewFolder, &finder, cx),
        key("secondary-i", GetInfo, &finder, cx),
        key("secondary-alt-n", NewFile, &finder, cx),
        key("secondary-shift-backspace", EmptyTrash, &finder, cx),
        key("secondary-alt-c", CopyPath, &finder, cx),
        key("secondary-shift-c", CopyAsNuon, &finder, cx),
        key("secondary-alt-t", OpenInNushell, &finder, cx),
        key("ctrl-`", ToggleNuPanel, &finder, cx),
        key("secondary-shift-d", ToggleAppearance, &finder, cx),
        key("secondary-/", SearchHelp, &finder, cx),
        key("secondary-alt-f", ToggleFilter, &finder, cx),
        key("secondary-s", SaveSearch, &finder, cx),
        // File views.
        key("up", MoveUp, &files, cx),
        key("down", MoveDown, &files, cx),
        key("left", MoveLeft, &files, cx),
        key("right", MoveRight, &files, cx),
        key("shift-up", ExtendUp, &files, cx),
        key("shift-down", ExtendDown, &files, cx),
        key("shift-left", ExtendLeft, &files, cx),
        key("shift-right", ExtendRight, &files, cx),
        key("home", MoveHome, &files, cx),
        key("end", MoveEnd, &files, cx),
        key("shift-home", ExtendHome, &files, cx),
        key("shift-end", ExtendEnd, &files, cx),
        key("shift-pageup", ExtendPageUp, &files, cx),
        key("shift-pagedown", ExtendPageDown, &files, cx),
        key("enter", OpenSelected, &files, cx),
        key("secondary-down", OpenSelected, &files, cx),
        key("secondary-o", OpenSelected, &files, cx),
        key("secondary-enter", ShowInEnclosingFolder, &files, cx),
        key("space", QuickLook, &files, cx),
        key("secondary-y", QuickLook, &files, cx),
        key("secondary-space", QuickLookExplore, &files, cx),
        key("space", QuickLook, &quick_look, cx),
        key("secondary-y", QuickLook, &quick_look, cx),
        key("secondary-space", QuickLookExplore, &quick_look, cx),
        key("escape", ExploreBack, &quick_look, cx),
        key("enter", ExploreInto, &quick_look, cx),
        key("up", MoveUp, &quick_look, cx),
        key("down", MoveDown, &quick_look, cx),
        key("left", MoveLeft, &quick_look, cx),
        key("right", MoveRight, &quick_look, cx),
        key("secondary-=", PreviewTextBigger, &quick_look, cx),
        key("secondary-+", PreviewTextBigger, &quick_look, cx),
        key("secondary--", PreviewTextSmaller, &quick_look, cx),
        key("secondary-0", PreviewTextReset, &quick_look, cx),
        key("pagedown", NextPdfPage, &quick_look, cx),
        key("pageup", PreviousPdfPage, &quick_look, cx),
        // The same keys for the gallery's preview, from the files.
        key("secondary-=", PreviewTextBigger, &files, cx),
        key("secondary-+", PreviewTextBigger, &files, cx),
        key("secondary--", PreviewTextSmaller, &files, cx),
        key("secondary-0", PreviewTextReset, &files, cx),
        key("pagedown", NextPdfPage, &files, cx),
        key("pageup", PreviousPdfPage, &files, cx),
        key("escape", Escape, &files, cx),
        key("secondary-a", SelectAll, &files, cx),
        // Edit › Cut, Copy, and Paste act on files when the files have focus.
        key("secondary-x", input::Cut, &files, cx),
        key("secondary-c", input::Copy, &files, cx),
        key("secondary-v", input::Paste, &files, cx),
        key("secondary-alt-v", MoveItemsHere, &files, cx),
        key("f2", Rename, &files, cx),
        key("secondary-d", Duplicate, &files, cx),
        key("f6", MoveToOtherPane, &files, cx),
        key("secondary-backspace", MoveToTrash, &files, cx),
        key("secondary-alt-backspace", DeleteImmediately, &files, cx),
        key("secondary-z", input::Undo, &files, cx),
        key("secondary-shift-z", input::Redo, &files, cx),
        key("delete", MoveToTrash, &files, cx),
        // The folder tree, as Explorer's navigation pane has it.
        key("up", MoveUp, &tree, cx),
        key("down", MoveDown, &tree, cx),
        key("left", MoveLeft, &tree, cx),
        key("right", MoveRight, &tree, cx),
        key("home", MoveHome, &tree, cx),
        key("end", MoveEnd, &tree, cx),
        key("pageup", PageUp, &tree, cx),
        key("pagedown", PageDown, &tree, cx),
        key("enter", OpenSelected, &tree, cx),
        key("f2", Rename, &tree, cx),
        key("secondary-x", input::Cut, &tree, cx),
        key("secondary-c", input::Copy, &tree, cx),
        key("secondary-v", input::Paste, &tree, cx),
        key("secondary-alt-v", MoveItemsHere, &tree, cx),
        key("secondary-backspace", MoveToTrash, &tree, cx),
        key("delete", MoveToTrash, &tree, cx),
        key("secondary-z", input::Undo, &tree, cx),
        key("secondary-shift-z", input::Redo, &tree, cx),
    ]);
    // The last key bound to Back, Forward, Enclosing Folder, Move to Trash,
    // and Open is the one Explorer and the Linux file managers use, which
    // suits their menus and tooltips. A Mac's should show Finder's, so those
    // are bound again here, last. A key bound twice still acts once, as long
    // as its handler doesn't pass it on with `cx.propagate()`.
    if mac {
        bindings.extend([
            key("secondary-[", GoBack, &finder, cx),
            key("secondary-]", GoForward, &finder, cx),
            key("secondary-up", GoUp, &finder, cx),
            key("secondary-backspace", MoveToTrash, &files, cx),
            key("secondary-backspace", MoveToTrash, &tree, cx),
            key("secondary-o", OpenSelected, &files, cx),
        ]);
    }
    bindings
}

pub fn bind_keys(cx: &mut App) {
    let bindings = key_table(cfg!(target_os = "macos"), cx);
    // Terminals keep most Cmd shortcuts for themselves, so in one every Cmd
    // binding also works with Ctrl. Added last, menus and tooltips show the
    // Ctrl form.
    let aliases: Vec<KeyBinding> = if crate::term::active() {
        bindings
            .iter()
            .filter_map(|binding| ctrl_alias(binding, cx))
            .collect()
    } else {
        Vec::new()
    };
    cx.bind_keys(bindings);
    cx.bind_keys(aliases);
    // What Explorer and the Linux file managers use for these.
    if !cfg!(target_os = "macos") {
        let finder = context(FINDER);
        let files = context(FILES);
        cx.bind_keys([
            key("ctrl-l", GoToFolder, &finder, cx),
            key("alt-enter", GetInfo, &finder, cx),
            key("shift-delete", DeleteImmediately, &files, cx),
        ]);
    }
}

/// A key context's predicate, parsed once for all of its bindings.
fn context(name: &str) -> Option<Rc<KeyBindingContextPredicate>> {
    let predicate = KeyBindingContextPredicate::parse(name).expect("key contexts are plain names");
    Some(Rc::new(predicate))
}

/// `action` bound to `keys` in `context`. `secondary` in `keys` is Cmd on
/// macOS and Ctrl on Linux and Windows, which only the running app knows.
fn key(
    keys: &str,
    action: impl Action,
    context: &Option<Rc<KeyBindingContextPredicate>>,
    cx: &App,
) -> KeyBinding {
    KeyBinding::load(keys, Box::new(action), context.clone(), false, None, cx)
        .expect("the key table's keystrokes are valid")
}

/// The binding with Cmd swapped for Ctrl, if it uses Cmd at all.
fn ctrl_alias(binding: &KeyBinding, cx: &App) -> Option<KeyBinding> {
    if !binding
        .keystrokes()
        .iter()
        .any(|k| k.inner().modifiers.platform)
    {
        return None;
    }
    let os = cx.platform_info().os;
    let keystrokes: Vec<String> = binding
        .keystrokes()
        .iter()
        .map(|keystroke| {
            let mut keystroke = keystroke.inner().clone();
            if keystroke.modifiers.platform {
                keystroke.modifiers.platform = false;
                keystroke.modifiers.control = true;
            }
            keystroke.unparse(os)
        })
        .collect();
    KeyBinding::load(
        &keystrokes.join(" "),
        binding.action().boxed_clone(),
        binding.predicate(),
        false,
        None,
        cx,
    )
    .ok()
}

/// What Make Alias is called: Finder's "Make Alias", Explorer's "Create
/// Shortcut", or nothing on Linux, where links are the aliases.
pub fn alias_label() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        Some("Make Alias")
    } else if cfg!(windows) {
        Some("Create Shortcut")
    } else {
        None
    }
}

pub fn set_menus(cx: &mut App) {
    // Finder aliases are macOS only; symbolic links work everywhere.
    let mut file = vec![
        MenuItem::action("New Window", NewWindow),
        MenuItem::action("New Tab", NewTab),
        MenuItem::action("New Folder", NewFolder),
        MenuItem::action("New File…", NewFile),
        MenuItem::separator(),
        MenuItem::action("Open", OpenSelected),
        MenuItem::action("Open in Editor", OpenInEditor),
        MenuItem::action("Quick Look", QuickLook),
        MenuItem::action("Explore Data", QuickLookExplore),
        MenuItem::action("Get Info", GetInfo),
        MenuItem::action("Rename…", Rename),
        MenuItem::action("Duplicate", Duplicate),
    ];
    // Finder aliases on macOS, shortcuts on Windows; Linux has symbolic links.
    if let Some(label) = alias_label() {
        file.push(MenuItem::action(label, MakeAlias));
    }
    file.extend([
        MenuItem::action("Make Symbolic Link", MakeSymlink),
        MenuItem::action("Compress", Compress),
        MenuItem::action("Extract", Extract),
        MenuItem::separator(),
        MenuItem::action("Move to Trash", MoveToTrash),
        MenuItem::action("Put Back", PutBack),
        MenuItem::action("Delete Immediately…", DeleteImmediately),
        MenuItem::separator(),
        MenuItem::action("Close Window", CloseWindow),
    ]);
    cx.set_menus([
        Menu::new("fffinder").items([
            MenuItem::action("Settings…", ShowSettings),
            MenuItem::action("Rescan Index", Rescan),
            MenuItem::action("Empty Trash…", EmptyTrash),
            MenuItem::separator(),
            MenuItem::action("Quit fffinder", Quit),
        ]),
        Menu::new("File").items(file),
        Menu::new("Edit").items([
            MenuItem::os_action("Undo", input::Undo, OsAction::Undo),
            MenuItem::os_action("Redo", input::Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("Cut", input::Cut, OsAction::Cut),
            MenuItem::os_action("Copy", input::Copy, OsAction::Copy),
            MenuItem::os_action("Paste", input::Paste, OsAction::Paste),
            MenuItem::action("Move Items Here", MoveItemsHere),
            MenuItem::separator(),
            MenuItem::action("Copy Path", CopyPath),
            MenuItem::action("Copy as Nushell List", CopyAsNuon),
            MenuItem::action("Select All Items", SelectAll),
            MenuItem::separator(),
            MenuItem::action("Find", FocusSearch),
            MenuItem::action("Save Search…", SaveSearch),
            MenuItem::action("Filter with Nushell", ToggleFilter),
            MenuItem::action("Search Syntax…", SearchHelp),
        ]),
        Menu::new("View").items([
            MenuItem::action("as Icons", ViewIcons),
            MenuItem::action("as List", ViewList),
            MenuItem::action("as Columns", ViewColumns),
            MenuItem::action("as Gallery", ViewGallery),
            MenuItem::separator(),
            MenuItem::action("Toggle Sidebar", ToggleSidebar),
            MenuItem::action("Sidebar as List", SidebarAsList),
            MenuItem::action("Sidebar as Tree", SidebarAsTree),
            MenuItem::action("Toggle Preview Pane", TogglePreviewPane),
            MenuItem::action("Show/Hide Hidden Files", ToggleHidden),
            MenuItem::action("Keep Folders on Top", ToggleFoldersFirst),
            MenuItem::action("Use Groups", ToggleGroups),
            MenuItem::action("Toggle Light/Dark", ToggleAppearance),
            MenuItem::action("Refresh", Refresh),
        ]),
        Menu::new("Go").items([
            MenuItem::action("Back", GoBack),
            MenuItem::action("Forward", GoForward),
            MenuItem::action("Enclosing Folder", GoUp),
            MenuItem::separator(),
            MenuItem::action("Recents", GoRecents),
            MenuItem::action("Home", GoHome),
            MenuItem::action("Trash", GoTrash),
            MenuItem::action("Go to Folder…", GoToFolder),
            MenuItem::action("Connect to Server…", ConnectToServer),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Show Previous Tab", ShowPreviousTab),
            MenuItem::action("Show Next Tab", ShowNextTab),
            MenuItem::action("Move Tab to New Window", MoveTabToNewWindow),
            MenuItem::separator(),
            MenuItem::action("Split View", ToggleSplitView),
            MenuItem::action("Copy to Other Pane", CopyToOtherPane),
            MenuItem::action("Move to Other Pane", MoveToOtherPane),
        ]),
        Menu::new("Nushell").items([
            MenuItem::action("Pipeline Panel", ToggleNuPanel),
            MenuItem::action("Open in Nushell", OpenInNushell),
            MenuItem::action("Copy as Nushell List", CopyAsNuon),
        ]),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Modifiers, TestAppContext};

    #[gpui_kit::test]
    fn secondary_is_the_systems_shortcut_key(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let binding = key("secondary-n", NewWindow, &context(FINDER), cx);
            let keystroke = binding.keystrokes()[0].inner();
            assert_eq!(keystroke.modifiers, Modifiers::secondary_key(cx));
            assert_eq!(keystroke.key, "n");
            assert_eq!(binding.predicate(), context(FINDER));
        });
    }

    /// The actions with a key from Finder and one from the other file
    /// managers: the action, its context, Finder's key, and the others'.
    const TWO_KEY_ACTIONS: [(&dyn Action, &str, &str, &str); 5] = [
        (&GoBack, FINDER, "secondary-[", "alt-left"),
        (&GoForward, FINDER, "secondary-]", "alt-right"),
        (&GoUp, FINDER, "secondary-up", "alt-up"),
        (&MoveToTrash, FILES, "secondary-backspace", "delete"),
        (&MoveToTrash, TREE, "secondary-backspace", "delete"),
    ];

    /// The keys `table` has for `action` in `context`, in the order it binds them.
    fn keys_for(table: &[KeyBinding], action: &dyn Action, context: &str, cx: &App) -> Vec<String> {
        let (context, os) = (super::context(context), cx.platform_info().os);
        table
            .iter()
            .filter(|binding| binding.action().partial_eq(action) && binding.predicate() == context)
            .map(|binding| binding.keystrokes()[0].inner().unparse(os))
            .collect()
    }

    /// `keys` as [`keys_for`] spells them: `secondary` is Cmd or Ctrl here.
    fn spelled(keys: &str, cx: &App) -> String {
        let (binding, os) = (key(keys, Quit, &None, cx), cx.platform_info().os);
        binding.keystrokes()[0].inner().unparse(os)
    }

    #[gpui_kit::test]
    fn finders_keys_are_bound_first_and_last_on_a_mac(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let table = key_table(true, cx);
            for (action, context, finders, others) in TWO_KEY_ACTIONS {
                assert_eq!(
                    keys_for(&table, action, context, cx),
                    [finders, others, finders].map(|keys| spelled(keys, cx)),
                    "{} in {context}",
                    action.name()
                );
            }
        });
    }

    #[gpui_kit::test]
    fn the_other_file_managers_keys_are_bound_last_off_a_mac(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let table = key_table(false, cx);
            for (action, context, finders, others) in TWO_KEY_ACTIONS {
                assert_eq!(
                    keys_for(&table, action, context, cx),
                    [finders, others].map(|keys| spelled(keys, cx)),
                    "{} in {context}",
                    action.name()
                );
            }
        });
    }

    /// Refresh, Open, and Quick Look have a key without Cmd, bound first, and
    /// one with it: the action, and all of its keys in the order they're
    /// bound, whatever their context, on a Mac and off one. A Mac shows the
    /// first in its menu bar and the last in the window's menus.
    const CMD_KEY_ACTIONS: [(&dyn Action, &[&str], &[&str]); 3] = [
        (
            &Refresh,
            &["secondary-r", "f5", "f5", "secondary-r"],
            &["f5", "secondary-r"],
        ),
        (
            &OpenSelected,
            &[
                "secondary-o",
                "enter",
                "secondary-down",
                "secondary-o",
                "enter",
                "secondary-o",
            ],
            &["enter", "secondary-down", "secondary-o", "enter"],
        ),
        (
            &QuickLook,
            &[
                "secondary-y",
                "space",
                "secondary-y",
                "space",
                "secondary-y",
            ],
            &["space", "secondary-y", "space", "secondary-y"],
        ),
    ];

    /// Every key `table` has for `action`, in the order it binds them.
    fn all_keys_for(table: &[KeyBinding], action: &dyn Action, cx: &App) -> Vec<String> {
        let os = cx.platform_info().os;
        table
            .iter()
            .filter(|binding| binding.action().partial_eq(action))
            .map(|binding| binding.keystrokes()[0].inner().unparse(os))
            .collect()
    }

    #[gpui_kit::test]
    fn refresh_open_and_quick_look_have_their_cmd_key_first_and_last_on_a_mac(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            let table = key_table(true, cx);
            for (action, on_a_mac, _) in CMD_KEY_ACTIONS {
                let bound: Vec<String> = on_a_mac.iter().map(|keys| spelled(keys, cx)).collect();
                assert_eq!(all_keys_for(&table, action, cx), bound, "{}", action.name());
            }
            // The first of the two F5s is the Mac's own, for the whole window.
            assert_eq!(
                keys_for(&table, &Refresh, FINDER, cx),
                ["secondary-r", "f5", "secondary-r"].map(|keys| spelled(keys, cx))
            );
        });
    }

    #[gpui_kit::test]
    fn refresh_open_and_quick_look_have_only_the_tables_keys_off_a_mac(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let table = key_table(false, cx);
            for (action, _, off_a_mac) in CMD_KEY_ACTIONS {
                let bound: Vec<String> = off_a_mac.iter().map(|keys| spelled(keys, cx)).collect();
                assert_eq!(all_keys_for(&table, action, cx), bound, "{}", action.name());
            }
            assert_eq!(
                keys_for(&table, &Refresh, FINDER, cx),
                [spelled("secondary-r", cx)],
                "F5 is a key of the file views"
            );
        });
    }

    #[gpui_kit::test]
    fn ctrl_alias_swaps_cmd_for_ctrl(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let binding = key("cmd-shift-n", NewFolder, &context(FINDER), cx);
            let alias = ctrl_alias(&binding, cx).expect("a Cmd binding has an alias");
            let os = cx.platform_info().os;
            assert_eq!(alias.keystrokes()[0].inner().unparse(os), "ctrl-shift-n");
            assert!(alias.action().partial_eq(&NewFolder));
            assert_eq!(alias.predicate(), binding.predicate());
            assert!(ctrl_alias(&key("alt-left", GoBack, &None, cx), cx).is_none());
        });
    }
}
