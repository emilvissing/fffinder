//! The sidebar's folder tree, driven with the mouse and keyboard the way
//! Explorer's navigation pane is, and the Full Disk Access prompt.

mod common;

use common::{Fixture, Open, Ui, cmd, item_id};
use fffinder::actions::{SidebarAsList, SidebarAsTree};
use fffinder::disk_access::{Access, Checked};
use fffinder::finder::{NodeKey, Section};
use fffinder::model::Location;
use fffinder::system::SystemCall;
use fffinder_core::{SidebarMode, SmartFolder};
use gpui_kit::{
    InputEvent as _, KeyDownEvent, Keystroke, Modifiers, MouseButton, TestAppContext, point, px,
};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// A window on the fixture with the fixture folder as a favorite and the
/// tree on, and that favorite's node. fffinder has Full Disk Access, so the
/// tree looks inside folders to tell which open, whatever the test runner
/// may read.
fn tree_window<'a>(cx: &'a mut TestAppContext, fx: &Fixture) -> (Ui<'a>, NodeKey) {
    let mut ui = Ui::open(cx, &fx.dir);
    ui.cx.update(|cx| cx.set_global(Checked(Access::Granted)));
    let dir = fx.dir.clone();
    ui.update(|f, _, cx| {
        f.add_favorite(dir, cx);
        f.set_sidebar_mode(SidebarMode::Tree, cx);
    });
    ui.settle();
    (ui, NodeKey::place(Section::Favorites, fx.dir.clone()))
}

fn selected(ui: &Ui) -> Option<NodeKey> {
    ui.read(|f| f.tree_selection_for_test())
}

/// Where the fixture folder is among Favorites.
fn favorite_ix(ui: &Ui, fx: &Fixture) -> usize {
    ui.read(|f| f.favorites().iter().position(|p| p.path == fx.dir))
        .expect("the fixture folder is a favorite")
}

#[gpui_kit::test]
fn a_favorite_opens_in_place_and_shows_only_folders(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-open");
    let (mut ui, home) = tree_window(cx, &fx);
    let projects = home.under(fx.path("Projects"));
    assert_eq!(
        selected(&ui),
        Some(home.clone()),
        "the folder shown is selected"
    );
    assert!(!ui.exists(projects.row_id()), "a place starts closed");

    ui.click(home.disclosure_id());
    assert!(ui.exists(projects.row_id()));
    assert!(!ui.exists(home.under(fx.path("notes.txt")).row_id()));
    assert_eq!(
        ui.folder(),
        Some(fx.dir.clone()),
        "opening doesn't move the window"
    );

    ui.click(projects.row_id());
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert_eq!(selected(&ui), Some(projects.clone()));

    // atlas holds a file and no folders, so nothing opens it.
    let atlas = projects.under(fx.path("Projects/atlas"));
    ui.click(projects.disclosure_id());
    assert!(ui.exists(atlas.row_id()));
    assert!(!ui.exists(atlas.disclosure_id()));

    ui.click(home.disclosure_id());
    assert!(!ui.exists(projects.row_id()), "closing hides its folders");
    assert_eq!(ui.folder(), Some(fx.path("Projects")));

    // Double-clicking a row opens and closes it too.
    ui.double_click(("fav", favorite_ix(&ui, &fx)));
    assert!(ui.exists(projects.row_id()));
}

#[gpui_kit::test]
fn the_tree_opens_down_to_the_folder_the_window_shows(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-follow");
    std::fs::create_dir_all(fx.path("Projects/atlas/docs")).unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    let docs = home.under(fx.path("Projects/atlas/docs"));
    ui.go_to(&fx.path("Projects/atlas/docs"));
    assert!(ui.exists(docs.row_id()));
    assert_eq!(selected(&ui), Some(docs));

    ui.press(&cmd("["));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(
        selected(&ui),
        Some(home.clone()),
        "Back goes back up the tree"
    );

    // Without following, a folder is selected only where it already shows.
    ui.update(|f, _, cx| f.set_expand_to_open_folder(false, cx));
    ui.click(home.disclosure_id());
    ui.go_to(&fx.path("Projects/atlas"));
    assert!(!ui.exists(home.under(fx.path("Projects")).row_id()));
    assert_eq!(selected(&ui), None);
}

#[gpui_kit::test]
fn arrows_move_through_the_tree_as_in_explorer(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-keys");
    std::fs::create_dir_all(fx.path("Archive")).unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    let (archive, projects) = (
        home.under(fx.path("Archive")),
        home.under(fx.path("Projects")),
    );
    let atlas = projects.under(fx.path("Projects/atlas"));
    ui.click(("fav", favorite_ix(&ui, &fx)));
    assert!(tree_focused(&mut ui), "a click gives the tree the keyboard");

    ui.press("right");
    assert!(ui.exists(archive.row_id()), "Right opens the place");
    ui.press("right");
    assert_eq!(selected(&ui), Some(archive.clone()), "then goes into it");
    assert_eq!(ui.folder(), Some(fx.path("Archive")));
    ui.press("down");
    assert_eq!(selected(&ui), Some(projects.clone()));
    ui.press("right");
    ui.press("right");
    assert_eq!(selected(&ui), Some(atlas.clone()));
    ui.press("left");
    assert_eq!(
        selected(&ui),
        Some(projects.clone()),
        "Left goes up from a closed folder"
    );
    ui.press("left");
    assert!(!ui.exists(atlas.row_id()), "and closes an open one");
    ui.press("up");
    assert_eq!(selected(&ui), Some(archive));
    ui.type_text("p");
    assert_eq!(selected(&ui), Some(projects), "typing goes to a name");
    ui.press("home");
    assert_eq!(ui.read(|f| f.location().clone()), Location::Recents);
    ui.press("end");
    assert_eq!(ui.read(|f| f.location().clone()), Location::Trash);

    // With no line active, Home still goes to the first line.
    ui.update(|f, window, cx| {
        let found = Location::Results {
            title: "Found".into(),
            paths: Default::default(),
        };
        f.navigate(found, window, cx)
    });
    ui.press("home");
    assert_eq!(ui.read(|f| f.location().clone()), Location::Recents);

    ui.click(("fav", favorite_ix(&ui, &fx)));
    ui.press("enter");
    assert!(ui.files_focused(), "Return takes the keyboard to the items");
}

/// Whether the browser's folder tree has the keyboard.
fn tree_focused(ui: &mut Ui) -> bool {
    ui.update(|f, window, _| f.tree_focused_for_test(window))
}

#[gpui_kit::test]
fn the_arrows_go_on_past_a_smart_folder_whose_folder_is_gone(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-smart-gone");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.cx.update(|cx| cx.set_global(Checked(Access::Granted)));
    // Two Smart Folders above the one place under Locations. The second
    // was saved for a folder that isn't there.
    let (first, gone) = (
        SmartFolder {
            name: "Atlas".into(),
            query: "atlas".into(),
            ..Default::default()
        },
        SmartFolder {
            name: "Roadmaps".into(),
            query: "roadmap".into(),
            scope: Some(fx.path("gone")),
            ..Default::default()
        },
    );
    let place = fffinder_core::places::favorite_for(&fx.dir);
    ui.update(|f, _, cx| {
        f.set_locations_for_test(vec![place]);
        f.add_smart_folder(first, cx);
        f.add_smart_folder(gone, cx);
        f.set_sidebar_mode(SidebarMode::Tree, cx);
    });
    ui.click(("loc", 0usize));
    assert!(tree_focused(&mut ui));

    // Up opens the Smart Folder above, which searches This Computer in place
    // of its folder. It's the line the keyboard is on all the same, so the
    // next Up goes on from it, not from the place below once more.
    ui.press("up");
    assert_eq!(ui.read(|f| f.query().to_string()), "roadmap");
    ui.press("up");
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
}

#[gpui_kit::test]
fn hidden_folders_show_with_hidden_files_or_on_the_way_down(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-hidden");
    std::fs::create_dir_all(fx.path(".cache/inner")).unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    let cache = home.under(fx.path(".cache"));
    ui.click(home.disclosure_id());
    assert!(!ui.exists(cache.row_id()));

    ui.update(|f, _, cx| f.set_show_hidden(true, cx));
    ui.settle();
    assert!(ui.exists(cache.row_id()));
    ui.update(|f, _, cx| f.set_show_hidden(false, cx));
    ui.settle();
    assert!(!ui.exists(cache.row_id()));

    ui.go_to(&fx.path(".cache/inner"));
    assert!(
        ui.exists(cache.row_id()),
        "a hidden folder shows on the way down"
    );
}

#[gpui_kit::test]
fn items_dropped_on_a_folder_in_the_tree_move_into_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-drop");
    std::fs::create_dir_all(fx.path("Archive")).unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    let (archive, projects) = (
        home.under(fx.path("Archive")),
        home.under(fx.path("Projects")),
    );
    ui.click("view-list");
    ui.click(home.disclosure_id());

    ui.drag(
        item_id("name-cell", &fx.path("notes.txt")),
        projects.row_id(),
        Modifiers::none(),
    );
    assert!(fx.path("Projects/notes.txt").exists());
    assert!(!fx.path("notes.txt").exists());

    // A folder in the tree drags like any item, and dropped on a folder it
    // goes in there, not into Favorites as well.
    let favorites = ui.read(|f| f.favorites().len());
    ui.drag(archive.row_id(), projects.row_id(), Modifiers::none());
    assert!(fx.path("Projects/Archive").is_dir());
    assert!(!ui.exists(archive.row_id()), "the tree shows the move");
    assert_eq!(ui.read(|f| f.favorites().len()), favorites);
}

#[gpui_kit::test]
fn holding_a_drag_over_a_closed_folder_opens_it_in_the_tree(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-spring");
    let (mut ui, home) = tree_window(cx, &fx);
    let projects = home.under(fx.path("Projects"));
    let atlas = projects.under(fx.path("Projects/atlas"));
    ui.click("view-list");
    ui.click(home.disclosure_id());
    let file = ui
        .snapshot(item_id("name-cell", &fx.path("notes.txt")))
        .bounds();
    let folder = ui.snapshot(projects.row_id()).bounds();

    ui.drag_and_hold(file.center(), folder.center());
    ui.settle();
    assert!(ui.exists(atlas.row_id()), "the node opened");
    assert_eq!(ui.folder(), Some(fx.dir.clone()), "the window stayed put");

    let into = ui.snapshot(atlas.row_id()).bounds().center();
    ui.move_while_held(into, Modifiers::none());
    ui.release(into);
    assert!(fx.path("Projects/atlas/notes.txt").exists());
}

#[gpui_kit::test]
fn new_folder_and_rename_happen_in_the_tree(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-name");
    let (mut ui, home) = tree_window(cx, &fx);
    let projects = home.under(fx.path("Projects"));
    ui.click(home.disclosure_id());

    ui.right_click(projects.row_id());
    assert!(
        ui.menu_labels()
            .contains(&"Paste Into “Projects”".to_string())
    );
    ui.click_menu_item("New Folder");
    assert!(fx.path("Projects/untitled folder").is_dir());
    assert!(ui.exists("rename-input"), "it's named in its row");
    ui.type_text("Gamma");
    ui.press("enter");
    assert!(fx.path("Projects/Gamma").is_dir());
    let gamma = projects.under(fx.path("Projects/Gamma"));
    assert!(ui.exists(gamma.row_id()));
    assert!(tree_focused(&mut ui), "the keyboard is back in the tree");

    // Renaming the folder the window shows takes the window along, and the
    // tree keeps what was open inside it.
    std::fs::create_dir(fx.path("Projects/Gamma/inner")).unwrap();
    ui.click(gamma.row_id());
    assert_eq!(ui.folder(), Some(fx.path("Projects/Gamma")));
    ui.click(gamma.disclosure_id());
    assert!(ui.exists(gamma.under(fx.path("Projects/Gamma/inner")).row_id()));
    ui.click(gamma.row_id());
    ui.press("f2");
    ui.type_text("Delta");
    ui.press("enter");
    assert_eq!(ui.folder(), Some(fx.path("Projects/Delta")));
    let delta = projects.under(fx.path("Projects/Delta"));
    assert_eq!(selected(&ui), Some(delta.clone()));
    assert!(ui.exists(delta.row_id()));
    assert!(!ui.exists(gamma.row_id()));
    assert!(ui.exists(delta.under(fx.path("Projects/Delta/inner")).row_id()));
    ui.press(&cmd("["));
    assert_eq!(
        ui.folder(),
        Some(fx.dir.clone()),
        "Back goes where the window was before, not to the old name"
    );
}

#[gpui_kit::test]
fn a_rename_in_the_tree_leaves_a_columns_selection_alone(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-name-columns");
    std::fs::write(fx.path("Projects/b.txt"), "x").unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    let projects = home.under(fx.path("Projects"));
    ui.click(home.disclosure_id());
    ui.click(projects.disclosure_id());
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));

    // The folder beside the selected item, in the second column.
    ui.right_click(projects.under(fx.path("Projects/atlas")).row_id());
    ui.click_menu_item("Rename…");
    ui.type_text("maps");
    ui.press("enter");

    assert!(fx.path("Projects/maps").is_dir());
    let columns = ui.read(|f| f.column_names_for_test());
    assert_eq!(columns[1], ["b.txt", "maps"], "the column has the name");
    assert_eq!(ui.selected(), ["b.txt"]);
}

#[gpui_kit::test]
fn a_press_on_a_file_that_ends_a_rename_in_the_tree_leaves_the_file_selected(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("tree-name-press");
    let (mut ui, home) = tree_window(cx, &fx);
    ui.activate();
    let projects = home.under(fx.path("Projects"));
    ui.click(home.disclosure_id());
    ui.click("view-list");

    // The folder is listed among the files as well. A right-click on a file
    // there keeps the name, and its menu acts on the file.
    ui.right_click(projects.row_id());
    ui.click_menu_item("Rename…");
    ui.type_text("Work");
    ui.right_click(item_id("row", &fx.path("notes.txt")));
    assert!(fx.path("Work").is_dir());
    assert_eq!(ui.selected(), ["notes.txt"]);
    ui.click_menu_item("Move to Trash");
    assert!(!fx.path("notes.txt").exists());
    assert!(fx.path("Work").is_dir(), "not the folder just renamed");

    // The same on a file that was selected before the name was edited,
    // where the press selects nothing new.
    ui.select("Budget.csv");
    ui.right_click(home.under(fx.path("Work")).row_id());
    ui.click_menu_item("Rename…");
    ui.type_text("Jobs");
    ui.right_click(item_id("row", &fx.path("Budget.csv")));
    assert!(fx.path("Jobs").is_dir());
    assert_eq!(ui.selected(), ["Budget.csv"]);
    ui.click_menu_item("Move to Trash");
    assert!(!fx.path("Budget.csv").exists());
    assert!(fx.path("Jobs").is_dir(), "not the folder just renamed");

    // Such a file stays selected after Return as well. The folder is
    // selected among the files only when nothing else is.
    ui.select("Roadmap.md");
    ui.right_click(home.under(fx.path("Jobs")).row_id());
    ui.click_menu_item("Rename…");
    ui.type_text("Plans");
    ui.press("enter");
    assert!(fx.path("Plans").is_dir());
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn a_folder_named_in_the_tree_is_held_to_the_filter_in_the_files(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("tree-name-filtered");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    ui.cx.update(|cx| cx.set_global(Checked(Access::Granted)));
    let dir = fx.dir.clone();
    ui.update(|f, _, cx| {
        f.add_favorite(dir, cx);
        f.set_sidebar_mode(SidebarMode::Tree, cx);
        f.set_filter("name =~ 'notes'".into(), cx);
    });
    ui.activate();
    ui.click("view-list");
    assert_eq!(ui.names(), ["notes.txt"]);

    // The folder is made in the folder the window shows, and named in the
    // tree. The files have no field of their own to keep a row for, so they
    // list what the filter keeps and no more.
    ui.right_click(("fav", favorite_ix(&ui, &fx)));
    ui.click_menu_item("New Folder");
    assert!(fx.path("untitled folder").is_dir());
    assert!(
        ui.exists("rename-input"),
        "it's named in its row of the tree"
    );
    assert_eq!(ui.names(), ["notes.txt"]);
    ui.type_text("Gamma");
    ui.press("enter");
    assert!(fx.path("Gamma").is_dir());
    assert_eq!(ui.names(), ["notes.txt"]);
    assert!(tree_focused(&mut ui), "the keyboard is back in the tree");
}

#[gpui_kit::test]
fn moving_the_folder_shown_takes_the_window_along(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-move-shown");
    std::fs::create_dir_all(fx.path("Archive/inner")).unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    let (archive, projects) = (
        home.under(fx.path("Archive")),
        home.under(fx.path("Projects")),
    );
    ui.click(home.disclosure_id());
    ui.click(archive.row_id());
    ui.click(archive.disclosure_id());
    assert_eq!(ui.folder(), Some(fx.path("Archive")));

    ui.drag(archive.row_id(), projects.row_id(), Modifiers::none());
    assert!(fx.path("Projects/Archive/inner").is_dir());
    assert_eq!(ui.folder(), Some(fx.path("Projects/Archive")));
    let moved = home.under(fx.path("Projects/Archive"));
    assert_eq!(selected(&ui), Some(moved.clone()));
    assert!(ui.exists(moved.row_id()), "the tree opens down to it");
    assert!(
        ui.exists(moved.under(fx.path("Projects/Archive/inner")).row_id()),
        "and keeps it open"
    );

    // Undo moves it back, and the window with it.
    ui.press(&cmd("z"));
    assert!(fx.path("Archive/inner").is_dir());
    assert_eq!(ui.folder(), Some(fx.path("Archive")));
    assert_eq!(selected(&ui), Some(archive.clone()));
    assert!(ui.exists(archive.row_id()));
}

#[gpui_kit::test]
fn a_tree_folder_let_go_off_a_folder_stays_out_of_favorites(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-drop-off");
    std::fs::create_dir_all(fx.path("Archive")).unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    let archive = home.under(fx.path("Archive"));
    ui.click(home.disclosure_id());
    let favorites = ui.read(|f| f.favorites().len());

    ui.drag(archive.row_id(), "recents", Modifiers::none());
    assert_eq!(ui.read(|f| f.favorites().len()), favorites);
    assert!(fx.path("Archive").is_dir(), "nothing moved");

    // A folder from the items, let go there, is added as in Finder.
    ui.click("view-list");
    ui.drag(
        item_id("name-cell", &fx.path("Archive")),
        "recents",
        Modifiers::none(),
    );
    assert_eq!(ui.read(|f| f.favorites().len()), favorites + 1);
}

#[gpui_kit::test]
fn trashing_the_folder_shown_from_the_tree_goes_to_its_parent(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-trash");
    let (mut ui, home) = tree_window(cx, &fx);
    let atlas = home.under(fx.path("Projects/atlas"));
    ui.go_to(&fx.path("Projects/atlas"));
    assert!(ui.exists(atlas.row_id()));

    ui.right_click(atlas.row_id());
    ui.click_menu_item("Move to Trash");
    assert!(!fx.path("Projects/atlas").exists());
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert!(!ui.exists(atlas.row_id()));

    // A place stays put: the tree's Delete leaves favorites and disks alone,
    // and so does Copy, so a paste can't copy a whole disk.
    ui.click(home.under(fx.path("Projects")).row_id());
    ui.press(&cmd("c"));
    let copied = ui.clipboard();
    assert_eq!(copied, fx.path("Projects").display().to_string());
    ui.click(("fav", favorite_ix(&ui, &fx)));
    ui.press("delete");
    assert!(fx.dir.exists());
    ui.press(&cmd("c"));
    assert_eq!(ui.clipboard(), copied);
}

#[gpui_kit::test]
fn folders_made_elsewhere_show_up_in_the_tree(cx: &mut TestAppContext) {
    // In an open folder the window isn't showing, so only the tree's own
    // look for changes can find it.
    let fx = Fixture::new("tree-live");
    let (mut ui, home) = tree_window(cx, &fx);
    let projects = home.under(fx.path("Projects"));
    ui.click(home.disclosure_id());
    ui.click(projects.disclosure_id());
    let later = projects.under(fx.path("Projects/Later"));
    assert!(!ui.exists(later.row_id()));

    std::fs::create_dir(fx.path("Projects/Later")).unwrap();
    ui.cx.executor().advance_clock(Duration::from_secs(3));
    ui.settle();
    assert!(ui.exists(later.row_id()));

    std::fs::remove_dir(fx.path("Projects/Later")).unwrap();
    ui.cx.executor().advance_clock(Duration::from_secs(3));
    ui.settle();
    assert!(!ui.exists(later.row_id()));

    // atlas holds no folders, so it can't open, until one is made in it.
    let atlas = projects.under(fx.path("Projects/atlas"));
    assert!(ui.exists(atlas.row_id()));
    assert!(!ui.exists(atlas.disclosure_id()));
    std::fs::create_dir(fx.path("Projects/atlas/maps")).unwrap();
    ui.cx.executor().advance_clock(Duration::from_secs(3));
    ui.settle();
    assert!(ui.exists(atlas.disclosure_id()), "now it opens");
}

#[cfg(unix)]
#[gpui_kit::test]
fn a_folder_that_cant_be_read_shows_a_lock(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt as _;
    let fx = Fixture::new("tree-locked");
    let locked = fx.path("Locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    let row = home.under(locked.clone());
    ui.click(home.disclosure_id());
    let (shown, lock, opens) = (
        ui.exists(row.row_id()),
        ui.exists("unreadable"),
        ui.exists(row.disclosure_id()),
    );
    // Readable again before anything can fail, so the fixture can go.
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(shown);
    assert!(lock, "a lock says it can't be read");
    assert!(!opens, "and it has no triangle to open it");
}

#[gpui_kit::test]
fn the_sidebar_switches_between_list_and_tree(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-mode");
    let (mut ui, home) = tree_window(cx, &fx);
    assert!(ui.exists(home.disclosure_id()));
    ui.dispatch(SidebarAsList);
    assert_eq!(ui.read(|f| f.config().sidebar_mode), SidebarMode::List);
    assert!(!ui.exists(home.disclosure_id()));
    // The list works as Finder's sidebar does.
    ui.click(("fav", favorite_ix(&ui, &fx)));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert!(!tree_focused(&mut ui));
    ui.dispatch(SidebarAsTree);
    assert_eq!(ui.read(|f| f.config().sidebar_mode), SidebarMode::Tree);
    assert!(ui.exists(home.disclosure_id()));

    // Settings has the same choice.
    ui.press(&cmd(","));
    ui.scroll("use-nu-env", point(px(0.), px(-2000.)));
    ui.click("sidebar-list");
    assert_eq!(ui.read(|f| f.config().sidebar_mode), SidebarMode::List);
    ui.click("sidebar-tree");
    assert_eq!(ui.read(|f| f.config().sidebar_mode), SidebarMode::Tree);
}

#[gpui_kit::test]
fn the_files_get_the_keyboard_when_the_tree_goes(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-gone");
    let (mut ui, _) = tree_window(cx, &fx);
    let shown = |ui: &Ui| ui.read(|f| f.config().show_sidebar);
    let click_in_the_tree = |ui: &mut Ui| {
        ui.click(("fav", favorite_ix(ui, &fx)));
        assert!(tree_focused(ui), "a click gives the tree the keyboard");
    };

    // The keyboard can't stay in a tree that isn't drawn. No key would do
    // anything there, not even the one that hid the sidebar, to show it
    // again.
    click_in_the_tree(&mut ui);
    ui.press(&cmd("alt-s"));
    assert!(!shown(&ui));
    assert!(ui.files_focused());
    ui.press(&cmd("alt-s"));
    assert!(shown(&ui), "the key that hid the sidebar shows it again");

    // The button that hides it leaves the keyboard where it is, as buttons
    // do, and that is in the tree.
    click_in_the_tree(&mut ui);
    ui.click("hide-sidebar");
    assert!(!shown(&ui));
    assert!(ui.files_focused());
    ui.press(&cmd("alt-s"));
    assert!(shown(&ui));

    // As a list, the sidebar has no tree to have the keyboard.
    click_in_the_tree(&mut ui);
    ui.dispatch(SidebarAsList);
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(
        ui.selected(),
        ["Budget.csv"],
        "the arrow keys are the files'"
    );
}

#[gpui_kit::test]
fn the_files_get_the_keyboard_when_another_tab_turns_the_tree_off(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-gone-elsewhere");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            workspace: true,
            ..Default::default()
        },
    );
    let dir = fx.dir.clone();
    ui.update(|f, _, cx| {
        f.add_favorite(dir, cx);
        f.set_sidebar_mode(SidebarMode::Tree, cx);
    });
    // A second tab, then the first again, with the keyboard in its tree.
    ui.press(&cmd("t"));
    ui.press("ctrl-shift-tab");
    ui.click(("fav", favorite_ix(&ui, &fx)));
    assert!(tree_focused(&mut ui));
    let other = ui
        .workspace
        .clone()
        .expect("opened with tabs")
        .read_with(ui.cx, |w, _| w.tabs()[1].clone());

    // The sidebar's mode is one setting for every tab and window, so the
    // tree goes here when it is turned off anywhere.
    other.update(ui.cx, |f, cx| f.set_sidebar_mode(SidebarMode::List, cx));
    ui.settle();
    assert_eq!(ui.read(|f| f.config().sidebar_mode), SidebarMode::List);
    assert!(ui.files_focused(), "the files of the tab that had the tree");
    let in_the_other = ui.window(|window, cx| other.read(cx).files_focused(window));
    assert!(!in_the_other, "not those of the tab that isn't showing");
    ui.press("down");
    assert_eq!(ui.selected(), ["Budget.csv"]);
}

#[gpui_kit::test]
fn the_files_of_the_pane_whose_tree_went_get_the_keyboard(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-gone-split");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            workspace: true,
            ..Default::default()
        },
    );
    let dir = fx.dir.clone();
    ui.update(|f, _, cx| {
        f.add_favorite(dir, cx);
        f.set_sidebar_mode(SidebarMode::Tree, cx);
    });
    ui.press(&cmd("\\"));
    let pane = ui
        .workspace
        .clone()
        .expect("opened with tabs")
        .read_with(ui.cx, |w, _| w.split_pane().cloned())
        .expect("split view is on");
    let files_of = |ui: &mut Ui, of: &gpui_kit::Entity<fffinder::finder::Finder>| {
        ui.window(|window, cx| of.read(cx).files_focused(window))
    };
    let selected_in = |ui: &Ui, of: &gpui_kit::Entity<fffinder::finder::Finder>| -> Vec<String> {
        of.read_with(ui.cx, |f, _| {
            f.selected_entries().into_iter().map(|e| e.name).collect()
        })
    };
    let favorite = favorite_ix(&ui, &fx);

    // Both panes are drawn, and each is told that the keyboard is in
    // something that went. The other pane starts without a sidebar, so the
    // only tree is the first pane's.
    ui.click(("fav", favorite));
    assert!(tree_focused(&mut ui));
    ui.press(&cmd("alt-s"));
    assert!(!ui.read(|f| f.config().show_sidebar));
    assert!(
        ui.files_focused(),
        "the files of the pane that had the tree"
    );
    assert!(!files_of(&mut ui, &pane), "not those of the other pane");
    ui.press("down");
    assert_eq!(ui.selected(), ["Budget.csv"]);
    assert!(selected_in(&ui, &pane).is_empty(), "which got no key");

    // The same the other way around: the other pane's sidebar, shown by the
    // key while that pane has the keyboard, and hidden with it in its tree.
    ui.click_in("split-pane", "status-text");
    ui.press(&cmd("alt-s"));
    assert!(pane.read_with(ui.cx, |f, _| f.config().show_sidebar));
    ui.click_in("split-pane", ("fav", favorite));
    let in_its_tree = ui.window(|window, cx| pane.read(cx).tree_focused_for_test(window));
    assert!(
        in_its_tree,
        "a click gives the other pane's tree the keyboard"
    );
    ui.press(&cmd("alt-s"));
    assert!(!pane.read_with(ui.cx, |f, _| f.config().show_sidebar));
    assert!(files_of(&mut ui, &pane));
    assert!(!ui.files_focused());
    ui.press("down");
    assert_eq!(selected_in(&ui, &pane), ["Budget.csv"]);
    assert_eq!(ui.selected(), ["Budget.csv"], "the first pane got no key");
}

#[gpui_kit::test]
fn the_files_are_drawn_with_the_keyboard_once_the_tree_has_gone(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-gone-drawn");
    let (mut ui, _) = tree_window(cx, &fx);
    // A frame says what got the keyboard only in an active window.
    ui.activate();
    ui.click(("fav", favorite_ix(&ui, &fx)));
    assert!(tree_focused(&mut ui));
    let drawn = Rc::new(Cell::new(false));
    ui.update({
        let drawn = drawn.clone();
        move |f, window, cx| {
            let files = f.files_focus_handle();
            cx.on_focus(&files, window, move |_, _, _| drawn.set(true))
                .detach();
        }
    });

    // The key the way the system sends it. `press` draws a frame after the
    // key whether or not the window asked for one, and here the window has
    // to ask: the frame that finds the tree gone is drawn before the files
    // get the keyboard, and shows their selection as inactive.
    let hide = ui
        .cx
        .update(|cx| Keystroke::parse_with_os(&cmd("alt-s"), cx.platform_info().os))
        .expect("a key");
    let hide = KeyDownEvent {
        keystroke: hide,
        is_held: false,
        prefer_character_input: false,
    };
    ui.window(|window, cx| {
        window.dispatch_event(hide.to_platform_input(), cx);
    });
    assert!(!ui.read(|f| f.config().show_sidebar));
    assert!(drawn.get(), "a frame found the keyboard in the files");
}

#[gpui_kit::test]
fn a_rename_in_the_tree_ends_with_the_keyboard_in_the_files_when_the_tree_goes(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("tree-gone-rename");
    let (mut ui, home) = tree_window(cx, &fx);
    // A field says that it lost the keyboard only in an active window.
    ui.activate();
    let projects = home.under(fx.path("Projects"));
    ui.click(home.disclosure_id());
    ui.click(projects.row_id());
    ui.press("f2");
    assert!(ui.exists("rename-input"));
    ui.type_text("Plans");

    // The field goes with the sidebar. What was typed is kept, as when the
    // field goes any other way, and the keyboard doesn't go back to a tree
    // that isn't there.
    ui.press(&cmd("alt-s"));
    assert!(fx.path("Plans").is_dir());
    assert_eq!(ui.folder(), Some(fx.path("Plans")));
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["atlas"]);

    // The same when the tree is put away for the list.
    ui.press(&cmd("alt-s"));
    ui.click(home.under(fx.path("Plans")).row_id());
    ui.press("f2");
    ui.type_text("Notes");
    ui.dispatch(SidebarAsList);
    assert!(fx.path("Notes").is_dir());
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["atlas"]);
}

#[gpui_kit::test]
fn a_rename_in_the_tree_goes_on_when_the_tree_comes_back_while_in_another_app(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("tree-back-rename");
    let (mut ui, home) = tree_window(cx, &fx);
    ui.activate();
    let projects = home.under(fx.path("Projects"));
    ui.click(home.disclosure_id());
    ui.click(projects.row_id());
    ui.press("f2");
    ui.type_text("Half");

    // The sidebar is hidden and shown again from another window, which has
    // the same setting, while this one is behind it. The edit is left as it
    // is, and nothing else takes the keyboard meanwhile.
    ui.deactivate();
    ui.update(|f, _, cx| f.set_sidebar_visible(false, cx));
    assert!(!ui.exists("rename-input"));
    ui.update(|f, _, cx| f.set_sidebar_visible(true, cx));
    assert!(ui.exists("rename-input"));

    ui.activate();
    ui.type_text(" typed");
    ui.press("enter");
    assert!(fx.path("Half typed").is_dir());
    assert!(tree_focused(&mut ui), "the keyboard is back in the tree");
}

#[gpui_kit::test]
fn a_rename_in_the_tree_ends_in_the_files_when_the_window_is_back_and_the_tree_is_not(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("tree-not-back-rename");
    let (mut ui, home) = tree_window(cx, &fx);
    ui.activate();
    let projects = home.under(fx.path("Projects"));
    ui.click(home.disclosure_id());
    ui.click(projects.row_id());
    ui.press("f2");
    ui.type_text("Plans");

    // The sidebar is hidden from another window while this one is behind
    // it, and stays hidden.
    ui.deactivate();
    ui.update(|f, _, cx| f.set_sidebar_visible(false, cx));
    assert!(
        fx.path("Projects").is_dir(),
        "the edit waits for the window"
    );

    ui.activate();
    assert!(fx.path("Plans").is_dir(), "what was typed is kept");
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["atlas"]);
}

#[gpui_kit::test]
fn a_click_on_the_status_bar_leaves_the_keyboard_in_the_tree(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-status-bar");
    let (mut ui, home) = tree_window(cx, &fx);
    ui.click(("fav", favorite_ix(&ui, &fx)));
    assert!(tree_focused(&mut ui));

    ui.click("status-text");
    assert!(tree_focused(&mut ui));
    ui.press("right");
    assert!(
        ui.exists(home.under(fx.path("Projects")).row_id()),
        "Right opens the place, as before the click"
    );
}

#[gpui_kit::test]
fn a_middle_click_opens_a_tree_folder_in_a_new_tab(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-tab");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        common::Open {
            workspace: true,
            ..Default::default()
        },
    );
    let dir = fx.dir.clone();
    ui.update(|f, _, cx| {
        f.add_favorite(dir, cx);
        f.set_sidebar_mode(SidebarMode::Tree, cx);
    });
    ui.settle();
    let home = NodeKey::place(Section::Favorites, fx.dir.clone());
    ui.click(home.disclosure_id());
    ui.click_with(
        MouseButton::Middle,
        home.under(fx.path("Projects")).row_id(),
    );
    let tabs = ui
        .workspace
        .clone()
        .expect("opened with tabs")
        .read_with(ui.cx, |w, _| w.tabs().to_vec());
    assert_eq!(tabs.len(), 2);
    let last = tabs[1].read_with(ui.cx, |f, _| f.current_folder().map(|p| p.to_path_buf()));
    assert_eq!(last, Some(fx.path("Projects")));
    assert_eq!(ui.folder(), Some(fx.dir.clone()), "this tab stays put");
}

#[gpui_kit::test]
fn missing_full_disk_access_opens_system_settings(cx: &mut TestAppContext) {
    let fx = Fixture::new("disk-access");
    let mut ui = Ui::open(cx, &fx.dir);
    let opened = SystemCall::OpenUrl(fffinder::disk_access::SETTINGS_URL.into());
    ui.update(|f, window, cx| f.ask_for_full_disk_access(window, cx));
    ui.settle();
    assert!(ui.dialog_open());
    assert!(
        ui.system_calls().is_empty(),
        "System Settings doesn't come up in front of the dialog by itself"
    );

    ui.click("disk-access-open");
    assert_eq!(ui.system_calls(), std::slice::from_ref(&opened));
    assert!(ui.dialog_open(), "opening Settings leaves the dialog up");
    // The click that brings the window back from System Settings lands
    // outside the dialog, and mustn't close it.
    ui.click_point(point(px(10.), px(300.)));
    assert!(ui.dialog_open());

    ui.click("disk-access-never");
    assert!(!ui.dialog_open());
    assert!(!ui.read(|f| f.config().ask_for_full_disk_access));
}

#[gpui_kit::test]
fn indexing_waits_for_full_disk_access(cx: &mut TestAppContext) {
    let fx = Fixture::new("disk-access-held");
    let mut ui = Ui::open(cx, &fx.dir);
    let roots = |ui: &mut Ui| {
        ui.cx
            .read(|cx| cx.global::<fffinder::finder::AppState>().engine.roots())
    };
    let held = |ui: &mut Ui| ui.cx.read(fffinder::disk_access::index_held);
    // As startup leaves things when access is off and asking is on.
    ui.cx.update(|cx| {
        cx.global::<fffinder::finder::AppState>()
            .engine
            .set_roots(&[]);
        cx.set_global(fffinder::disk_access::IndexHeld(true));
    });
    ui.update(|f, _, cx| f.set_search_roots(vec![fx.dir.clone()], cx));
    assert!(roots(&mut ui).is_empty(), "new folders to index wait too");
    assert!(ui.exists("index-status"));

    ui.click("index-status");
    assert!(ui.dialog_open(), "the sidebar says why, and asks again");
    assert!(ui.system_calls().is_empty());
    ui.click("disk-access-index");
    assert!(!ui.dialog_open());
    assert!(!held(&mut ui));
    assert_eq!(roots(&mut ui).len(), 1, "indexing starts");

    // Turning asking off starts it as well.
    ui.cx.update(|cx| {
        cx.global::<fffinder::finder::AppState>()
            .engine
            .set_roots(&[]);
        cx.set_global(fffinder::disk_access::IndexHeld(true));
    });
    ui.update(|f, _, cx| f.set_ask_for_full_disk_access(false, cx));
    assert!(!held(&mut ui));
    assert_eq!(roots(&mut ui).len(), 1);
}

#[gpui_kit::test]
fn a_deep_tree_scrolls_sideways_and_down(cx: &mut TestAppContext) {
    let fx = Fixture::new("tree-sideways");
    let deep = "Projects/a folder with a rather long name/and another one inside it/\
                and a third level down/with a fourth one under that";
    std::fs::create_dir_all(fx.path(deep)).unwrap();
    let (mut ui, home) = tree_window(cx, &fx);
    ui.go_to(&fx.path(deep));
    let row = home.under(fx.path(deep));
    assert!(ui.exists(row.row_id()), "the tree opens down to the folder");
    assert!(ui.exists("sidebar-sideways"), "and is too wide to fit");
    let before = ui.snapshot(row.row_id()).bounds().origin.x;
    ui.scroll("sidebar-sideways", point(px(-200.), px(0.)));
    let after = ui.snapshot(row.row_id()).bounds().origin.x;
    assert!(
        after < before,
        "a sideways swipe moves the rows: {before:?} -> {after:?}"
    );

    // Up and down still scroll the rows, now inside the sideways scroller.
    for n in 0..40 {
        std::fs::create_dir(fx.path(&format!("Folder {n:02}"))).unwrap();
    }
    ui.go_to(&fx.dir);
    ui.press(&cmd("r"));
    let last = home.under(fx.path("Folder 39"));
    assert!(
        !ui.exists(last.row_id()),
        "the last folder is below the window"
    );
    for _ in 0..20 {
        ui.scroll("sidebar-sideways", point(px(0.), px(-400.)));
    }
    assert!(ui.exists(last.row_id()), "scrolling down shows it");

    // The list of places cuts long names short instead, as Finder's does.
    ui.update(|f, _, cx| f.set_sidebar_mode(SidebarMode::List, cx));
    assert!(!ui.exists("sidebar-sideways"));
}
