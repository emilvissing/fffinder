//! The everyday things people reach for without thinking: dragging a box
//! around items, the mouse's back and forward buttons, opening a folder in a
//! new tab, paging through a long list, and where the selection goes.

mod common;

use common::{Fixture, Open, Ui, cmd, item_id};
use fffinder::system::SystemCall;
use gpui_kit::{
    Modifiers, MouseButton, NavigationDirection, Pixels, Point, TestAppContext, point, px,
};

/// A folder with more items than fit in the window.
fn long_folder(name: &str) -> Fixture {
    let fx = Fixture::new(name);
    for ix in 0..200 {
        std::fs::write(fx.path(&format!("item {ix:03}.txt")), "x").unwrap();
    }
    fx
}

fn with_tabs<'a>(cx: &'a mut TestAppContext, dir: &'a std::path::Path) -> Ui<'a> {
    Ui::open_with(
        cx,
        dir,
        Open {
            workspace: true,
            ..Default::default()
        },
    )
}

/// How many tabs the window has, and the folder the last one shows.
fn tabs(ui: &Ui) -> (usize, Option<std::path::PathBuf>) {
    let tabs = ui
        .workspace
        .clone()
        .expect("opened with tabs")
        .read_with(ui.cx, |w, _| w.tabs().to_vec());
    let last = tabs
        .last()
        .and_then(|tab| tab.read_with(ui.cx, |f, _| f.current_folder().map(|p| p.to_path_buf())));
    (tabs.len(), last)
}

/// A point in the rows' empty space, under the last row of a short list.
fn empty_list_space(ui: &mut Ui, fx: &Fixture) -> Point<Pixels> {
    let row = ui.snapshot(item_id("row", &fx.path("Roadmap.md"))).bounds();
    point(row.center().x, row.bottom() + px(60.))
}

/// Cmd on macOS, Ctrl elsewhere.
fn ui_secondary() -> Modifiers {
    Modifiers {
        platform: cfg!(target_os = "macos"),
        control: !cfg!(target_os = "macos"),
        ..Modifiers::none()
    }
}

// ----- dragging a box ------------------------------------------------------------

#[gpui_kit::test]
fn a_reload_during_a_box_keeps_what_cmd_kept_selected(cx: &mut TestAppContext) {
    let fx = Fixture::new("box-reload");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.select("notes.txt");
    let empty = empty_list_space(&mut ui, &fx);
    let roadmap = ui.snapshot(item_id("row", &fx.path("Roadmap.md"))).bounds();
    ui.drag_and_hold_with(empty, roadmap.center(), ui_secondary());

    // A new first item moves every row down one.
    std::fs::write(fx.path("Aardvark.txt"), "x").unwrap();
    ui.update(|f, _, cx| f.reload_quietly(cx));
    ui.move_while_held(roadmap.center() + point(px(1.), px(0.)), ui_secondary());

    let selected = ui.selected();
    assert!(selected.contains(&"notes.txt".to_string()), "{selected:?}");
    assert!(
        !selected.contains(&"Budget.csv".to_string()),
        "{selected:?}"
    );
    ui.release(roadmap.center());
}

#[gpui_kit::test]
fn a_box_loads_previews_for_what_it_ends_on_not_what_it_crosses(cx: &mut TestAppContext) {
    let fx = Fixture::new("box-previews");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    let empty = empty_list_space(&mut ui, &fx);
    let notes = ui.snapshot(item_id("row", &fx.path("notes.txt"))).bounds();

    ui.drag_and_hold(empty, notes.center());
    let selected = ui.selected();
    assert!(selected.len() > 1, "{selected:?}");
    let loaded = |ui: &Ui, name: &str| ui.read(|f| f.has_preview_for_test(&ui.dir.join(name)));
    assert!(!loaded(&ui, "notes.txt"), "the box is still moving");
    assert!(!loaded(&ui, "Roadmap.md"));

    ui.release(notes.center());
    let last = ui
        .read(|f| f.primary_entry().map(|entry| entry.name))
        .unwrap();
    assert!(loaded(&ui, &last), "{last} is selected now");
}

#[gpui_kit::test]
fn dragging_in_empty_space_selects_the_rows_it_covers(cx: &mut TestAppContext) {
    let fx = Fixture::new("box-list");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let empty = empty_list_space(&mut ui, &fx);
    let notes = ui.snapshot(item_id("row", &fx.path("notes.txt"))).bounds();

    ui.drag_points(empty, notes.center(), Modifiers::none());

    assert_eq!(ui.selected(), ["notes.txt", "Projects", "Roadmap.md"]);
    assert!(ui.files_focused());

    // A press and let go in the same place clears, as a click there always has.
    ui.drag_points(empty, empty, Modifiers::none());
    assert!(ui.selected().is_empty());
}

#[gpui_kit::test]
fn dragging_from_a_rows_other_columns_draws_a_box(cx: &mut TestAppContext) {
    let fx = Fixture::new("box-row");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let budget = item_id("row", &fx.path("Budget.csv"));
    let date = ui.window(|window, _| {
        use gpui_kit::test::TestWindowExt as _;
        window.within(budget).find(("cell", 0usize)).bounds()
    });
    let notes = ui.snapshot(item_id("row", &fx.path("notes.txt"))).bounds();

    ui.drag_points(
        date.center(),
        point(date.center().x, notes.center().y),
        Modifiers::none(),
    );

    assert_eq!(ui.selected(), ["Budget.csv", "notes.txt"]);
    assert!(fx.path("Budget.csv").exists(), "nothing was moved");
    assert!(fx.path("notes.txt").exists());
}

#[gpui_kit::test]
fn cmd_or_shift_adds_a_box_to_the_selection(cx: &mut TestAppContext) {
    let fx = Fixture::new("box-add");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.select("Budget.csv");
    let empty = empty_list_space(&mut ui, &fx);
    let roadmap = ui.snapshot(item_id("row", &fx.path("Roadmap.md"))).bounds();

    ui.drag_points(
        empty,
        roadmap.center(),
        Modifiers {
            shift: true,
            ..Modifiers::none()
        },
    );

    assert_eq!(ui.selected(), ["Budget.csv", "Roadmap.md"]);
}

#[gpui_kit::test]
fn a_box_selects_tiles_in_the_icon_view(cx: &mut TestAppContext) {
    let fx = Fixture::new("box-icons");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-icons");
    let content = ui.snapshot("content").bounds();
    let budget = ui
        .snapshot(item_id("tile", &fx.path("Budget.csv")))
        .bounds();
    let notes = ui.snapshot(item_id("tile", &fx.path("notes.txt"))).bounds();

    // From under the tiles, up to the middle of the second one.
    let start = point(budget.left() + px(4.), content.bottom() - px(20.));
    ui.drag_points(start, notes.center(), Modifiers::none());

    assert_eq!(ui.selected(), ["Budget.csv", "notes.txt"]);

    // A box that stops in the blank margin beside an icon doesn't take it.
    let third = ui.read(|f| f.rows_for_test()[2].entry.path.clone());
    let next = ui.snapshot(item_id("tile", &third)).bounds();
    let into_margin = point(next.left() + px(6.), notes.center().y);
    ui.drag_points(start, into_margin, Modifiers::none());
    assert_eq!(ui.selected(), ["Budget.csv", "notes.txt"]);
}

#[gpui_kit::test]
fn a_box_past_the_edge_scrolls_the_list(cx: &mut TestAppContext) {
    let fx = long_folder("box-scroll");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let content = ui.snapshot("content").bounds();
    let first = ui.snapshot(item_id("row", &fx.path("Budget.csv"))).bounds();
    let date = point(first.right() - px(40.), first.center().y);
    let below = point(date.x, content.bottom() + px(50.));

    ui.drag_and_hold(date, below);
    let before = ui.selected().len();
    ui.cx
        .executor()
        .advance_clock(std::time::Duration::from_millis(500));
    ui.settle();

    let after = ui.selected().len();
    assert!(after > before, "{before} selected, then {after}");
}

#[gpui_kit::test]
fn dragging_a_name_still_moves_the_item(cx: &mut TestAppContext) {
    let fx = Fixture::new("box-name");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");

    ui.drag(
        item_id("name-cell", &fx.path("notes.txt")),
        item_id("row", &fx.path("Projects")),
        Modifiers::none(),
    );

    assert!(fx.path("Projects/notes.txt").exists());
}

// ----- the mouse's other buttons ---------------------------------------------------

#[gpui_kit::test]
fn the_mouse_back_and_forward_buttons_navigate(cx: &mut TestAppContext) {
    let fx = Fixture::new("mouse-nav");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));

    ui.click_with(MouseButton::Navigate(NavigationDirection::Back), "content");
    assert_eq!(ui.folder(), Some(fx.dir.clone()));

    ui.click_with(
        MouseButton::Navigate(NavigationDirection::Forward),
        "content",
    );
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
}

// ----- double-clicks -----------------------------------------------------------------

/// The name of the item, in the folder the window opened at, whose row or
/// tile of `kind` is drawn at `spot`.
fn item_at(ui: &mut Ui, kind: &'static str, spot: Point<Pixels>) -> Option<String> {
    use gpui_kit::test::TestWindowExt as _;
    let items: Vec<_> = ui
        .names()
        .into_iter()
        .map(|name| (item_id(kind, &ui.dir.join(&name)), name))
        .collect();
    ui.window(|window, cx| {
        window.render_frame(cx);
        items.into_iter().find_map(|(id, name)| {
            let item = window.try_find(id)?;
            item.bounds().contains(&spot).then_some(name)
        })
    })
}

#[gpui_kit::test]
fn a_double_click_opens_the_item_in_every_view(cx: &mut TestAppContext) {
    let fx = Fixture::new("double-click-opens");
    let mut ui = Ui::open(cx, &fx.dir);
    let notes = fx.path("notes.txt");
    for (view, kind) in [
        ("view-list", "row"),
        ("view-icons", "tile"),
        ("view-columns", "crow"),
        ("view-gallery", "strip"),
    ] {
        ui.click(view);
        let opened = ui.system_calls().len();

        ui.double_click(item_id(kind, &notes));
        let calls = ui.system_calls();
        assert_eq!(calls.len(), opened + 1, "{view}: {calls:?}");
        assert_eq!(calls.last(), Some(&SystemCall::Open(notes.clone())));
        assert_eq!(ui.selected(), ["notes.txt"], "{view}");

        ui.double_click(item_id(kind, &fx.path("Projects")));
        assert_eq!(ui.folder(), Some(fx.path("Projects")), "{view}");
        ui.click("back");
    }

    // One of several selected items opens the same way, and is the one left
    // selected.
    ui.click("view-list");
    ui.select("Budget.csv");
    ui.select_also("notes.txt");
    let opened = ui.system_calls().len();
    ui.double_click(item_id("row", &notes));
    assert_eq!(ui.system_calls().len(), opened + 1);
    assert_eq!(ui.selected(), ["notes.txt"]);
}

#[gpui_kit::test]
fn a_double_click_on_a_dialogs_button_does_nothing_to_the_item_under_it(cx: &mut TestAppContext) {
    let fx = long_folder("stray-done");
    let mut ui = Ui::open(cx, &fx.dir);
    for (view, kind) in [("view-list", "row"), ("view-icons", "tile")] {
        ui.click(view);
        ui.select("item 001.txt");
        ui.select_also("item 002.txt");
        ui.press(&cmd("/"));
        let done = ui.snapshot("help-done").bounds().center();
        let under = item_at(&mut ui, kind, done);
        assert!(under.is_some(), "{view}: Done is over an item");

        // The first click closes the dialog, and the second finds that item
        // where the button was.
        ui.double_click("help-done");
        assert!(!ui.dialog_open(), "{view}");
        let calls = ui.system_calls();
        assert!(calls.is_empty(), "{view}: {under:?} is under it: {calls:?}");
        assert_eq!(ui.selected(), ["item 001.txt", "item 002.txt"], "{view}");
    }

    // Nor when that item is one of several selected ones, where a click
    // narrows the selection to it once the mouse is let go.
    ui.click("view-list");
    ui.press(&cmd("a"));
    let all = ui.selected();
    assert_eq!(all.len(), ui.names().len());
    ui.press(&cmd("/"));
    ui.double_click("help-done");
    assert!(!ui.dialog_open());
    assert_eq!(ui.selected(), all);
    assert!(ui.system_calls().is_empty());
}

#[gpui_kit::test]
fn a_double_click_on_a_dialogs_button_holds_up_no_click_after_it(cx: &mut TestAppContext) {
    let fx = long_folder("stray-done-then");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("/"));
    let done = ui.snapshot("help-done").bounds().center();
    let under = item_at(&mut ui, "row", done).expect("Done is over an item");
    ui.double_click("help-done");
    assert!(ui.selected().is_empty());

    // Clicked without a pause or a move, the item where the button was is
    // clicked for the third and fourth time at that spot, as the system
    // counts. They are its first click and its second: no pause is needed
    // before a double-click opens it.
    ui.nth_click_point(done, 3);
    assert_eq!(ui.selected(), [under.as_str()]);
    assert!(ui.system_calls().is_empty());
    ui.nth_click_point(done, 4);
    assert_eq!(ui.system_calls(), [SystemCall::Open(fx.path(&under))]);

    // The same over empty space: the click after the double-click on the
    // button clears the selection, as a click there does.
    ui.click("search");
    ui.type_text("item 009");
    assert_eq!(ui.names(), ["item 009.txt"]);
    ui.select("item 009.txt");
    ui.click("search-help");
    assert_eq!(item_at(&mut ui, "row", done), None);
    ui.double_click("help-done");
    assert_eq!(ui.selected(), ["item 009.txt"]);
    ui.nth_click_point(done, 3);
    assert!(ui.selected().is_empty());
}

#[gpui_kit::test]
fn a_double_click_on_a_dialogs_button_leaves_the_keyboard_where_it_was(cx: &mut TestAppContext) {
    let fx = long_folder("stray-done-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.click("search");
    ui.type_text("item");
    ui.click("search-help");
    let done = ui.snapshot("help-done").bounds().center();
    assert!(
        item_at(&mut ui, "row", done).is_some(),
        "Done is over a result"
    );

    // Closing the dialog gives the keyboard back to the search field, and
    // the second click doesn't take it on to the files.
    ui.double_click("help-done");
    ui.type_text(" 00");
    assert_eq!(ui.read(|f| f.query().to_string()), "item 00");
    assert!(ui.system_calls().is_empty());

    // Nor does it where the results end above the button.
    ui.type_text("9");
    assert!(ui.names().len() < 5, "{:?}", ui.names());
    ui.click("search-help");
    assert_eq!(
        item_at(&mut ui, "row", done),
        None,
        "Done is over empty space"
    );
    ui.double_click("help-done");
    ui.type_text(".");
    assert_eq!(ui.read(|f| f.query().to_string()), "item 009.");
}

#[gpui_kit::test]
fn a_double_click_on_a_dialogs_button_keeps_the_selection_over_empty_space(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("stray-done-empty");
    let mut ui = Ui::open(cx, &fx.dir);
    for (view, kind) in [("view-list", "row"), ("view-icons", "tile")] {
        ui.click(view);
        ui.select("notes.txt");
        ui.select_also("Roadmap.md");
        ui.press(&cmd("/"));
        let done = ui.snapshot("help-done").bounds().center();
        // With so few items the button is over the view's empty space,
        // where a click clears the selection.
        assert!(ui.snapshot("content").bounds().contains(&done), "{view}");
        assert_eq!(item_at(&mut ui, kind, done), None, "{view}");

        ui.double_click("help-done");
        assert!(!ui.dialog_open(), "{view}");
        assert_eq!(ui.selected(), ["notes.txt", "Roadmap.md"], "{view}");
    }
}

#[gpui_kit::test]
fn a_double_click_on_cancel_or_the_close_button_does_nothing_to_the_files(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt as _;
    let fx = long_folder("stray-cancel");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.select("item 001.txt");

    ui.press(&cmd("shift-g"));
    let cancel = ui.snapshot("goto-cancel").bounds().center();
    assert!(item_at(&mut ui, "row", cancel).is_some());
    ui.double_click("goto-cancel");
    assert!(!ui.dialog_open());
    assert_eq!(ui.selected(), ["item 001.txt"], "after Cancel");

    ui.press(&cmd("/"));
    let close = ui.window(|window, _| window.within("dialog").find("close").bounds().center());
    assert!(item_at(&mut ui, "row", close).is_some());
    ui.window(|window, cx| window.within("dialog").double_click("close", cx));
    ui.settle();
    assert!(!ui.dialog_open());
    assert_eq!(ui.selected(), ["item 001.txt"], "after the close button");

    let calls = ui.system_calls();
    assert!(calls.is_empty(), "{calls:?}");
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
}

#[gpui_kit::test]
fn a_double_click_beside_a_dialog_does_nothing_to_the_item_there(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt as _;
    let fx = long_folder("stray-backdrop");
    let mut ui = Ui::open(cx, &fx.dir);
    // An item that the dialog leaves in view in each of the views.
    for (view, kind, name) in [
        ("view-list", "row", "item 030.txt"),
        ("view-icons", "tile", "item 030.txt"),
        ("view-columns", "crow", "item 030.txt"),
        ("view-gallery", "strip", "item 003.txt"),
    ] {
        ui.click(view);
        ui.select("item 001.txt");
        let selected = ui.selected();
        let item = item_id(kind, &fx.path(name));
        let spot = ui.snapshot(item.clone()).bounds().center();
        ui.press(&cmd("/"));
        let dialog = ui.window(|window, _| window.within("dialog").find("surface").bounds());
        assert!(
            !dialog.contains(&spot),
            "{view}: {name} is beside the dialog"
        );

        // A click beside the dialog closes it, and the second click of a
        // double-click there is on the item.
        ui.click_point(spot);
        assert!(!ui.dialog_open(), "{view}");
        ui.second_click(item);
        let calls = ui.system_calls();
        assert!(calls.is_empty(), "{view}: {calls:?}");
        assert_eq!(ui.selected(), selected, "{view}");
    }

    // The gallery's items leave a press to the file area around them, which
    // takes the keyboard for it. Not for this one: the search field, which
    // had the keyboard when the dialog opened, still has it.
    let item = item_id("strip", &fx.path("item 003.txt"));
    let spot = ui.snapshot(item.clone()).bounds().center();
    ui.click("search");
    ui.press(&cmd("/"));
    assert!(ui.dialog_open());
    ui.click_point(spot);
    ui.second_click(item);
    ui.type_text("budget");
    assert_eq!(ui.read(|f| f.query().to_string()), "budget");
    assert!(ui.system_calls().is_empty());
}

#[gpui_kit::test]
fn a_second_click_beside_a_dialog_draws_no_box_when_the_mouse_moves(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt as _;
    let fx = long_folder("stray-box");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.select("item 001.txt");
    // A row's other columns, where a press starts a box, beside the dialog.
    let row = item_id("row", &fx.path("item 030.txt"));
    let date = ui.window(|window, _| window.within(row).find(("cell", 0usize)).bounds().center());
    ui.press(&cmd("/"));
    let dialog = ui.window(|window, _| window.within("dialog").find("surface").bounds());
    assert!(
        !dialog.contains(&date),
        "the row's date is beside the dialog"
    );

    // The first click closes the dialog. The mouse moves while the second
    // is down, as it does a little in a hurried click.
    ui.click_point(date);
    assert!(!ui.dialog_open());
    ui.nth_drag_points(date, date + point(px(0.), px(30.)), 2);
    assert_eq!(ui.selected(), ["item 001.txt"]);
    assert!(ui.system_calls().is_empty());
}

#[gpui_kit::test]
fn a_double_click_on_a_menu_item_does_nothing_to_the_item_under_it(cx: &mut TestAppContext) {
    let fx = long_folder("stray-menu");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let copied = fx.path("item 001.txt");
    ui.right_click(item_id("row", &copied));
    let copy = "Copy “item 001.txt”";
    let spot = ui
        .snapshots()
        .into_iter()
        .find(|item| item.role() == Some(gpui_kit::Role::MenuItem) && item.label() == Some(copy))
        .expect("the menu has Copy")
        .bounds()
        .center();
    let under = item_at(&mut ui, "row", spot);
    assert!(under.is_some(), "the menu is over the rows");

    // The first click copies and closes the menu, and the second finds the
    // row that was under it.
    ui.click_menu_item(copy);
    assert!(!ui.menu_open());
    ui.nth_click_point(spot, 2);
    assert_eq!(ui.clipboard(), copied.display().to_string());
    let calls = ui.system_calls();
    assert!(calls.is_empty(), "{under:?} is under it: {calls:?}");
    assert_eq!(ui.selected(), ["item 001.txt"]);
}

#[gpui_kit::test]
fn a_double_click_that_closes_a_tab_does_nothing_to_the_tab_behind_it(cx: &mut TestAppContext) {
    let fx = long_folder("stray-tab");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.click("view-icons");
    ui.press(&cmd("t"));
    let close = ui.snapshot(("close-tab", 1usize)).bounds().center();
    ui.press(&cmd("w"));
    assert_eq!(tabs(&ui).0, 1);

    // With one tab there is no tab bar, and an icon is where the second
    // tab's close button was. It is the last thing clicked in the first tab.
    let under = item_at(&mut ui, "tile", close).expect("an icon is there");
    ui.click(item_id("tile", &fx.path(&under)));
    ui.press(&cmd("t"));
    assert_eq!(
        ui.snapshot(("close-tab", 1usize)).bounds().center(),
        close,
        "the close button is where it was"
    );

    ui.click(("close-tab", 1usize));
    assert_eq!(tabs(&ui).0, 1);
    ui.nth_click_point(close, 2);
    let calls = ui.system_calls();
    assert!(calls.is_empty(), "{under} is under it: {calls:?}");
    assert_eq!(ui.selected(), [under]);
}

#[gpui_kit::test]
fn a_double_click_that_closes_quick_look_does_nothing_to_the_file_behind_it(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("stray-quick-look");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let notes = fx.path("notes.txt");
    ui.click(item_id("row", &notes));
    ui.press("space");
    ui.in_quick_look(|ui| ui.click("ql-open"));
    assert!(ui.read(|f| f.quick_look_window()).is_none());

    // Quick Look's window was over the browser's, which only now gets the
    // mouse again. Where the system counts clicks across windows, the
    // second click of a double-click on Open comes to a file behind it.
    ui.second_click(item_id("row", &fx.path("Roadmap.md")));
    assert_eq!(ui.system_calls(), [SystemCall::Open(notes)]);
    assert_eq!(ui.selected(), ["notes.txt"]);
}

#[gpui_kit::test]
fn quick_double_clicks_at_one_spot_open_one_item_each(cx: &mut TestAppContext) {
    // A folder in a folder in a folder, each the only item, so that each is
    // drawn where the one before it was.
    let fx = Fixture::new("double-click-twice");
    let (first, second) = (fx.path("Projects/atlas"), fx.path("Projects/atlas/inner"));
    std::fs::remove_file(first.join("Project Atlas.txt")).unwrap();
    std::fs::create_dir_all(second.join("innermost")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.go_to(&fx.path("Projects"));
    let spot = ui.snapshot(item_id("row", &first)).bounds().center();

    ui.double_click(item_id("row", &first));
    assert_eq!(ui.folder(), Some(first.clone()));
    let inner = ui.snapshot(item_id("row", &second)).bounds();
    assert!(
        inner.contains(&spot),
        "the folder inside is at that spot now"
    );

    // Without a pause the system counts on, and the next double-click
    // starts with the third click at that spot.
    ui.nth_click_point(spot, 3);
    assert_eq!(ui.folder(), Some(first), "the third click only selects");
    assert_eq!(ui.selected(), ["inner"]);
    ui.nth_click_point(spot, 4);
    assert_eq!(ui.folder(), Some(second));
}

#[gpui_kit::test]
fn a_second_click_on_another_item_only_selects_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("second-click-elsewhere");
    let mut ui = Ui::open(cx, &fx.dir);
    let (notes, roadmap) = (fx.path("notes.txt"), fx.path("Roadmap.md"));
    for (view, kind) in [
        ("view-list", "row"),
        ("view-icons", "tile"),
        ("view-columns", "crow"),
        ("view-gallery", "strip"),
    ] {
        ui.click(view);
        // Two clicks close enough in time and place for the system to count
        // the second as a double-click, but on different items.
        ui.click(item_id(kind, &notes));
        ui.second_click(item_id(kind, &roadmap));
        assert_eq!(ui.selected(), ["Roadmap.md"], "{view}");
        let calls = ui.system_calls();
        assert!(calls.is_empty(), "{view}: {calls:?}");
    }

    // The same after a click in the empty space beside the items.
    ui.click("view-list");
    let empty = empty_list_space(&mut ui, &fx);
    ui.click_point(empty);
    assert!(ui.selected().is_empty());
    ui.second_click(item_id("row", &notes));
    assert_eq!(ui.selected(), ["notes.txt"]);
    assert!(ui.system_calls().is_empty());

    // A double-click that is all on the item opens it straight afterwards.
    ui.double_click(item_id("row", &notes));
    assert_eq!(ui.system_calls(), [SystemCall::Open(notes)]);
}

// ----- new tabs ---------------------------------------------------------------------

#[gpui_kit::test]
fn folders_open_in_new_tabs(cx: &mut TestAppContext) {
    let fx = Fixture::new("new-tab");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.click("view-list");
    let projects = fx.path("Projects");

    // From the item's menu.
    ui.right_click(item_id("row", &projects));
    ui.click_menu_item("Open in New Tab");
    assert_eq!(tabs(&ui), (2, Some(projects.clone())));
    assert_eq!(ui.folder(), Some(fx.dir.clone()), "this tab stays put");

    // With the middle button, and with Cmd+double-click.
    let workspace = ui.workspace.clone().unwrap();
    workspace.update(ui.cx, |w, _| assert_eq!(w.active_index(), 1));
    ui.window(|window, cx| {
        workspace.update(cx, |w, cx| w.select_tab(0, window, cx));
    });
    ui.settle();
    ui.click_with(MouseButton::Middle, item_id("row", &projects));
    assert_eq!(tabs(&ui).0, 3);
    ui.window(|window, cx| {
        workspace.update(cx, |w, cx| w.select_tab(0, window, cx));
    });
    ui.settle();
    let row = item_id("row", &projects);
    ui.double_click_with(row, ui_secondary());
    assert_eq!(tabs(&ui).0, 4);
}

#[gpui_kit::test]
fn files_have_no_open_in_new_tab(cx: &mut TestAppContext) {
    let fx = Fixture::new("new-tab-file");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.click("view-list");

    ui.right_click(item_id("row", &fx.path("notes.txt")));

    assert!(!ui.menu_labels().contains(&"Open in New Tab".to_string()));
}

/// Which tab shows, counted from the first.
fn showing(ui: &Ui) -> usize {
    ui.workspace
        .clone()
        .expect("opened with tabs")
        .read_with(ui.cx, |w, _| w.active_index())
}

/// Whether the tab at `ix` is whole inside the strip the tabs scroll in.
fn tab_in_view(ui: &mut Ui, ix: usize) -> bool {
    let strip = ui.snapshot("tabs").bounds();
    let tab = ui.snapshot(("tab", ix)).bounds();
    let close_enough = px(0.5);
    strip.left() - close_enough <= tab.left() && tab.right() <= strip.right() + close_enough
}

/// A window 850 points wide with ten tabs, the last one showing. Its tab
/// bar has 590 points for tabs that want 854.
fn more_tabs_than_fit<'a>(cx: &'a mut TestAppContext, dir: &'a std::path::Path) -> Ui<'a> {
    let mut ui = with_tabs(cx, dir);
    ui.resize(850., 600.);
    for _ in 0..9 {
        ui.press(&cmd("t"));
    }
    assert_eq!((tabs(&ui).0, showing(&ui)), (10, 9));
    ui
}

/// What Open in New Tab does for `count` folders at once: a tab at `dir` for
/// each, with no frame drawn until the last is open.
fn open_tabs_at_once(ui: &mut Ui, dir: &std::path::Path, count: usize) {
    let workspace = ui.workspace.clone().expect("opened with tabs");
    let folder = fffinder::model::Location::Folder(dir.to_path_buf());
    ui.window(|window, cx| {
        workspace.update(cx, |w, cx| {
            for _ in 0..count {
                w.new_tab(folder.clone(), window, cx);
            }
        })
    });
}

#[gpui_kit::test]
fn tabs_that_do_not_fit_scroll_and_the_new_tab_button_stays_in_reach(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-scroll");
    let mut ui = more_tabs_than_fit(cx, &fx.dir);

    let bar = ui.snapshot("tab-bar").bounds();
    let plus = ui.snapshot("new-tab").bounds();
    assert!(
        plus.right() <= bar.right(),
        "the + is at {plus:?}, and the bar ends at {:?}",
        bar.right()
    );
    assert!(tab_in_view(&mut ui, 9), "the tab just opened");
    assert!(
        !ui.snapshot(("tab", 0usize)).visible(),
        "the first is scrolled away"
    );

    // A tab switched to scrolls into view, at either end.
    ui.press("ctrl-tab");
    assert_eq!(showing(&ui), 0);
    assert!(tab_in_view(&mut ui, 0));
    ui.press("ctrl-shift-tab");
    assert_eq!(showing(&ui), 9);
    assert!(tab_in_view(&mut ui, 9));

    // The + and a close button still do their work at the bar's far end.
    ui.click("new-tab");
    assert_eq!((tabs(&ui).0, showing(&ui)), (11, 10));
    assert!(tab_in_view(&mut ui, 10));
    ui.click(("close-tab", 10usize));
    assert_eq!((tabs(&ui).0, showing(&ui)), (10, 9));
    assert!(tab_in_view(&mut ui, 9));
}

#[gpui_kit::test]
fn the_new_tab_button_follows_the_tabs_while_they_fit(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-fit");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.resize(1100., 700.);
    ui.press(&cmd("t"));

    let first = ui.snapshot(("tab", 0usize)).bounds();
    let last = ui.snapshot(("tab", 1usize)).bounds();
    let plus = ui.snapshot("new-tab").bounds();
    let bar = ui.snapshot("tab-bar").bounds();

    // Two tabs at their widest leave the bar room to spare, and the + is as
    // close to the last tab as the tabs are to each other.
    assert!(bar.right() - plus.right() > px(100.), "{plus:?} in {bar:?}");
    assert_eq!(plus.left() - last.right(), last.left() - first.right());
}

#[gpui_kit::test]
fn the_wheel_scrolls_tabs_that_do_not_fit(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-wheel");
    let mut ui = more_tabs_than_fit(cx, &fx.dir);
    // Back at the first tab the bar is at its start.
    ui.press("ctrl-tab");
    let first = |ui: &mut Ui| ui.snapshot(("tab", 0usize)).bounds().left();
    let start = first(&mut ui);

    // A mouse's wheel only turns up and down. Over the tabs that is sideways.
    ui.scroll("tabs", point(px(0.), px(-100.)));
    assert_eq!(first(&mut ui), start - px(100.));
    ui.scroll("tabs", point(px(-50.), px(0.)));
    assert_eq!(first(&mut ui), start - px(150.));

    // It stops with the last tab at the bar's end, and switches to nothing.
    ui.scroll("tabs", point(px(0.), px(-1000.)));
    assert!(tab_in_view(&mut ui, 9));
    assert_eq!(showing(&ui), 0);
}

#[gpui_kit::test]
fn closing_a_tab_moves_the_tab_bar_only_when_it_was_the_one_showing(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-close");
    let mut ui = more_tabs_than_fit(cx, &fx.dir);
    // The first tab shows, and the bar is scrolled to its far end.
    ui.press("ctrl-tab");
    ui.scroll("tabs", point(px(0.), px(-1000.)));
    assert!(tab_in_view(&mut ui, 9));

    // A tab closed there leaves the bar there, for the next one to close.
    ui.click(("close-tab", 9usize));
    assert_eq!((tabs(&ui).0, showing(&ui)), (9, 0));
    assert!(tab_in_view(&mut ui, 8));
    assert!(!ui.snapshot(("tab", 0usize)).visible());

    // Closing the tab that shows brings the bar to the one that takes over.
    ui.press(&cmd("w"));
    assert_eq!((tabs(&ui).0, showing(&ui)), (8, 0));
    assert!(tab_in_view(&mut ui, 0));
}

/// The browser of the tab that shows.
fn shown_tab(ui: &Ui) -> gpui_kit::Entity<fffinder::finder::Finder> {
    ui.workspace
        .clone()
        .expect("opened with tabs")
        .read_with(ui.cx, |w, _| w.active().clone())
}

/// Take the tab that shows to `dir`.
fn show_folder(ui: &mut Ui, dir: std::path::PathBuf) {
    let tab = shown_tab(ui);
    let folder = fffinder::model::Location::Folder(dir);
    ui.window(|window, cx| tab.update(cx, |f, cx| f.navigate(folder, window, cx)));
    ui.settle();
}

#[gpui_kit::test]
fn a_middle_click_closes_a_tab(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-middle-click");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.press(&cmd("t"));
    ui.press(&cmd("t"));
    let third = shown_tab(&ui);
    assert_eq!((tabs(&ui).0, showing(&ui)), (3, 2));

    // A tab that isn't showing goes, and the window stays on the one that is.
    ui.click_with(MouseButton::Middle, ("tab", 0usize));
    assert_eq!((tabs(&ui).0, showing(&ui)), (2, 1));
    assert_eq!(shown_tab(&ui).entity_id(), third.entity_id());

    // The tab that shows goes the same way, and one tab alone has no bar.
    ui.click_with(MouseButton::Middle, ("tab", 1usize));
    assert_eq!((tabs(&ui).0, showing(&ui)), (1, 0));
    assert!(!ui.exists("tab-bar"));
    let last = shown_tab(&ui);
    assert!(
        ui.window(|window, cx| last.read(cx).files_focused(window)),
        "its files have the keyboard"
    );
}

/// A folder whose first items are folders, so that one is where the tabs
/// were when the tab bar goes, and where the first tab comes when there is
/// a tab bar.
fn folders_first(name: &str) -> Fixture {
    let fx = Fixture::new(name);
    for folder in ["0a", "0b", "0c", "0d", "0e", "0f"] {
        std::fs::create_dir(fx.path(folder)).unwrap();
    }
    fx
}

#[gpui_kit::test]
fn quick_middle_clicks_at_one_spot_close_tabs_and_open_no_folder(cx: &mut TestAppContext) {
    let fx = folders_first("tabs-middle-quick");
    let mut ui = with_tabs(cx, &fx.dir);
    for (view, kind) in [("view-icons", "tile"), ("view-columns", "crow")] {
        // Three tabs at one folder in this view, the first one showing.
        ui.click(view);
        for _ in 0..2 {
            ui.press(&cmd("t"));
            ui.click(view);
        }
        ui.click(("tab", 0usize));
        let spot = ui.snapshot(("tab", 0usize)).bounds().center();

        // Each press closes the tab that the one before it brought under
        // the pointer.
        ui.nth_click_point_with(MouseButton::Middle, spot, 1);
        assert_eq!((tabs(&ui).0, showing(&ui)), (2, 0), "{view}");
        ui.nth_click_point_with(MouseButton::Middle, spot, 2);
        assert_eq!(tabs(&ui).0, 1, "{view}");

        // The tab bar went with the second tab. A folder is at that spot
        // now, and the next press in the row doesn't open it.
        let under = item_at(&mut ui, kind, spot).expect("an item is where the tab was");
        assert!(fx.path(&under).is_dir(), "{view}: {under}");
        ui.nth_click_point_with(MouseButton::Middle, spot, 3);
        assert_eq!(tabs(&ui).0, 1, "{view}: {under} is under it");

        // A press of its own opens the folder in a tab.
        ui.nth_click_point_with(MouseButton::Middle, spot, 1);
        assert_eq!(tabs(&ui), (2, Some(fx.path(&under))), "{view}");
        ui.press(&cmd("w"));
        assert_eq!(tabs(&ui).0, 1);
    }
}

#[gpui_kit::test]
fn a_double_middle_click_on_a_folder_opens_one_tab_and_closes_none(cx: &mut TestAppContext) {
    let fx = folders_first("tabs-middle-folder");
    let inner = fx.path("0a/inner");
    std::fs::create_dir_all(inner.join("innermost")).unwrap();
    let mut ui = with_tabs(cx, &fx.dir);
    ui.click("view-columns");
    // A tab closed with the middle button earlier has nothing to say here.
    ui.press(&cmd("t"));
    ui.click_with(MouseButton::Middle, ("tab", 1usize));
    assert_eq!(tabs(&ui).0, 1);
    // The top of the first row, where the first tab is once there are two.
    let row = ui.snapshot(item_id("crow", &fx.path("0a"))).bounds();
    let spot = point(row.center().x, row.top() + px(6.));

    ui.nth_click_point_with(MouseButton::Middle, spot, 1);
    assert_eq!((tabs(&ui), showing(&ui)), ((2, Some(fx.path("0a"))), 1));
    let tab = ui.snapshot(("tab", 0usize)).bounds();
    assert!(tab.contains(&spot), "the first tab is there now: {tab:?}");

    // The second press of a double press lands on that tab, the one the
    // folder was middle-clicked in. It stays.
    ui.nth_click_point_with(MouseButton::Middle, spot, 2);
    assert_eq!((tabs(&ui), showing(&ui)), ((2, Some(fx.path("0a"))), 1));

    // With a tab bar there already nothing moves, and the second press
    // lands on the folder inside, where the one that opened was. It opens
    // no tab of its own.
    ui.click("view-icons");
    let spot = ui.snapshot(item_id("tile", &inner)).bounds().center();
    ui.nth_click_point_with(MouseButton::Middle, spot, 1);
    assert_eq!(tabs(&ui), (3, Some(inner.clone())));
    let innermost = ui.snapshot(item_id("tile", &inner.join("innermost")));
    assert!(
        innermost.bounds().contains(&spot),
        "the folder inside is at that spot now"
    );
    ui.nth_click_point_with(MouseButton::Middle, spot, 2);
    assert_eq!(tabs(&ui), (3, Some(inner)));
}

#[gpui_kit::test]
fn close_other_tabs_keeps_the_one_clicked(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-close-others");
    let mut ui = with_tabs(cx, &fx.dir);
    // Three tabs, each at a folder of its own, the third one showing.
    ui.press(&cmd("t"));
    show_folder(&mut ui, fx.path("Projects"));
    let second = shown_tab(&ui);
    ui.press(&cmd("t"));
    show_folder(&mut ui, fx.path("Projects/atlas"));
    let third = shown_tab(&ui).downgrade();
    assert_eq!((tabs(&ui).0, showing(&ui)), (3, 2));

    ui.right_click(("tab", 1usize));
    ui.click_menu_item("Close Other Tabs");

    assert_eq!(tabs(&ui), (1, Some(fx.path("Projects"))));
    assert_eq!(shown_tab(&ui).entity_id(), second.entity_id());
    assert!(!ui.exists("tab-bar"));
    assert!(
        ui.window(|window, cx| second.read(cx).files_focused(window)),
        "its files have the keyboard"
    );
    assert!(
        third.upgrade().is_none(),
        "a closed tab's browser goes, and what it had running with it"
    );

    // The tab that shows can be the one kept, and goes on showing.
    ui.press(&cmd("t"));
    ui.press(&cmd("t"));
    ui.click(("tab", 1usize));
    let middle = shown_tab(&ui);
    ui.right_click(("tab", 1usize));
    ui.click_menu_item("Close Other Tabs");
    assert_eq!((tabs(&ui).0, showing(&ui)), (1, 0));
    assert_eq!(shown_tab(&ui).entity_id(), middle.entity_id());
    assert!(
        ui.window(|window, cx| middle.read(cx).files_focused(window)),
        "its files have the keyboard"
    );
}

#[gpui_kit::test]
fn a_dragged_tab_shows_its_name(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-drag-name");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.press(&cmd("t"));
    show_folder(&mut ui, fx.path("Projects"));
    let first = ui.snapshot(("tab", 0usize)).bounds().center();
    let second = ui.snapshot(("tab", 1usize)).bounds().center();

    ui.drag_and_hold(second, first);
    assert_eq!(ui.snapshot("drag-label").label(), Some("Projects"));

    // The first tab takes dropped items too. A tab let go over it still
    // goes there, and its name is gone with the drag.
    ui.release(first);
    assert!(!ui.exists("drag-label"));
    assert_eq!(showing(&ui), 0);
}

#[gpui_kit::test]
fn the_new_tab_button_stays_in_its_pane_in_split_view(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-split");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.resize(1100., 700.);
    for _ in 0..4 {
        ui.press(&cmd("t"));
    }

    // Half the window is too little for five tabs.
    ui.press(&cmd("\\"));

    let bar = ui.snapshot("tab-bar").bounds();
    let plus = ui.snapshot("new-tab").bounds();
    assert!(
        plus.right() <= bar.right(),
        "the + is at {plus:?}, and the bar ends at {:?}",
        bar.right()
    );
    // The other pane isn't in its way.
    ui.click("new-tab");
    assert_eq!(tabs(&ui).0, 6);
}

/// A tab bar scrolls to a tab from where it was last laid out, and a bar
/// that is new never was. So its first frame has to scroll in the next one,
/// and ask for that frame itself: nothing else may.
#[gpui_kit::test]
fn a_new_tab_bar_scrolls_to_the_tab_that_shows_in_its_second_frame(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt as _;
    let fx = Fixture::new("tabs-at-once");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.resize(1400., 600.);
    // The frames asked for so far, as by a toolbar that refits, aren't the
    // tab bar's.
    ui.window(|window, cx| window.simulate_next_frame(cx));

    // Fourteen folders at once, without the frame the harness draws after
    // every step.
    open_tabs_at_once(&mut ui, &fx.dir, 14);
    // What the platform does when the window's next frame is due.
    ui.window(|window, cx| window.simulate_next_frame(cx));

    let (strip, last) = ui.window(|window, _| {
        (
            window.find("tabs").bounds(),
            window.find(("tab", 14usize)).bounds(),
        )
    });
    assert!(
        strip.left() <= last.left() && last.right() <= strip.right(),
        "the tab that shows is at {last:?}, and the tabs have {strip:?}"
    );
}

#[gpui_kit::test]
fn the_tab_that_shows_stays_in_view_when_the_tab_bar_has_less_room(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-less-room");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.resize(1100., 700.);
    for _ in 0..7 {
        ui.press(&cmd("t"));
    }
    // Eight tabs fit a window this wide, with nothing scrolled.
    assert!(tab_in_view(&mut ui, 0) && tab_in_view(&mut ui, 7));

    // Split view leaves them half the window, which holds three.
    ui.press(&cmd("\\"));
    assert!(tab_in_view(&mut ui, 7), "in split view");
    assert!(!ui.snapshot(("tab", 0usize)).visible());
    ui.press(&cmd("\\"));
    assert!(tab_in_view(&mut ui, 0) && tab_in_view(&mut ui, 7));

    // A narrower window holds six.
    ui.resize(850., 700.);
    assert!(tab_in_view(&mut ui, 7), "in a narrower window");
    assert!(!ui.snapshot(("tab", 0usize)).visible());
}

/// A tab bar goes when its second-last tab closes, and the next one may
/// have less room than it had. A tab is scrolled to from where the bar is
/// now, not from where it was then.
#[gpui_kit::test]
fn a_tab_bar_that_comes_back_scrolls_to_the_tab_that_shows(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-back");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.resize(1100., 700.);
    ui.press(&cmd("t"));
    ui.press(&cmd("w"));
    assert_eq!(tabs(&ui).0, 1);

    // Split view leaves the next bar less room than those two tabs had.
    ui.press(&cmd("\\"));
    open_tabs_at_once(&mut ui, &fx.dir, 6);
    ui.settle();
    assert_eq!((tabs(&ui).0, showing(&ui)), (7, 6));
    assert!(tab_in_view(&mut ui, 6));
}

#[gpui_kit::test]
fn a_tabs_menu_closes_when_the_window_changes_size(cx: &mut TestAppContext) {
    let fx = Fixture::new("tabs-menu-resized");
    let mut ui = more_tabs_than_fit(cx, &fx.dir);
    let menu = ["Close Tab", "Close Other Tabs", "Move Tab to New Window"];
    ui.right_click(("tab", 7usize));
    assert_eq!(ui.menu_labels(), menu);

    // A narrower window scrolls the tabs to keep the one that shows in
    // view, so this tab is no longer where it was pressed, and that is where
    // its menu is drawn. Left open, the menu would be back when the tab
    // is, in a window as wide as it was, and without the keyboard.
    ui.resize(760., 600.);
    assert!(!ui.menu_open());
    ui.resize(850., 600.);
    assert!(!ui.menu_open(), "the menu is closed, not only out of sight");
    ui.press(&cmd("t"));
    assert_eq!(tabs(&ui).0, 11, "the keyboard is back with the files");

    // A menu closed by hand has nothing left to close: a dialog opened
    // since stays through the next change of size.
    ui.right_click(("tab", 7usize));
    ui.press("escape");
    ui.press(&cmd("shift-g"));
    ui.resize(760., 600.);
    assert!(ui.dialog_open());
    ui.close_dialog();

    // A window that comes to the front reports its bounds as well. It is no
    // bigger or smaller for that, and a menu stays open through it.
    ui.right_click(("tab", 7usize));
    ui.activate();
    assert_eq!(ui.menu_labels(), menu);
}

// ----- the sidebar ---------------------------------------------------------------------

#[gpui_kit::test]
fn favorites_drag_into_another_order(cx: &mut TestAppContext) {
    let fx = Fixture::new("fav-order");
    let mut ui = Ui::open(cx, &fx.dir);
    let (projects, atlas) = (fx.path("Projects"), fx.path("Projects/atlas"));
    ui.update(|f, _, cx| {
        f.add_favorite(projects.clone(), cx);
        f.add_favorite(atlas.clone(), cx);
    });
    let places = |ui: &Ui| {
        ui.read(|f| {
            f.favorites()
                .iter()
                .map(|place| place.path.clone())
                .collect::<Vec<_>>()
        })
    };
    let order = places(&ui);
    let last = order.len() - 1;
    assert_eq!(order[last], atlas);

    // The last one goes where the one before it is.
    ui.drag(("fav", last), ("fav", last - 1), Modifiers::none());

    let order = places(&ui);
    assert_eq!(order[last - 1], atlas);
    assert_eq!(order[last], projects);
    assert_eq!(
        ui.read(|f| f.config().favorites.clone()),
        order,
        "and it's saved"
    );

    // A click still opens it.
    ui.click(("fav", last));
    assert_eq!(ui.folder(), Some(projects));
}

// ----- holding a drag over a folder ------------------------------------------------------

#[gpui_kit::test]
fn holding_a_drag_over_a_folder_opens_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("spring");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let file = ui
        .snapshot(item_id("name-cell", &fx.path("notes.txt")))
        .bounds();
    let folder = ui.snapshot(item_id("row", &fx.path("Projects"))).bounds();

    ui.drag_and_hold(file.center(), folder.center());
    ui.settle();
    assert_eq!(ui.folder(), Some(fx.path("Projects")), "the folder opened");

    // The drag is still going, and it can go on into the next one down.
    let atlas = ui
        .snapshot(item_id("row", &fx.path("Projects/atlas")))
        .bounds();
    let moved = atlas.center();
    ui.window(|window, cx| {
        use gpui_kit::InputEvent as _;
        window.dispatch_event(
            gpui_kit::MouseMoveEvent {
                position: moved,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::none(),
            }
            .to_platform_input(),
            cx,
        );
    });
    ui.settle();
    ui.settle();
    assert_eq!(ui.folder(), Some(fx.path("Projects/atlas")));

    // Letting go puts the item in the folder that opened.
    let content = ui.snapshot("content").bounds();
    ui.release(content.center());
    assert!(fx.path("Projects/atlas/notes.txt").exists());
    assert!(!fx.path("notes.txt").exists());
}

#[gpui_kit::test]
fn holding_a_drag_over_a_folder_that_is_also_a_favorite_opens_it(cx: &mut TestAppContext) {
    // Its sidebar row hears about every move too, and says the drag isn't
    // over it. That can't cancel the wait the list row started.
    let fx = Fixture::new("spring-favorite");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let projects = fx.path("Projects");
    ui.update(|f, _, cx| f.add_favorite(projects.clone(), cx));
    let file = ui
        .snapshot(item_id("name-cell", &fx.path("notes.txt")))
        .bounds();
    let folder = ui.snapshot(item_id("row", &projects)).bounds();

    ui.drag_and_hold(file.center(), folder.center());
    // A person's hand keeps moving a little, and each move reaches both.
    let nudged = folder.center() + point(px(3.), px(0.));
    ui.window(|window, cx| {
        use gpui_kit::InputEvent as _;
        window.dispatch_event(
            gpui_kit::MouseMoveEvent {
                position: nudged,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::none(),
            }
            .to_platform_input(),
            cx,
        );
    });
    ui.settle();

    assert_eq!(ui.folder(), Some(projects));
    ui.release(nudged);
}

#[gpui_kit::test]
fn a_quick_drag_past_a_folder_leaves_it_closed(cx: &mut TestAppContext) {
    let fx = Fixture::new("spring-past");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let file = ui
        .snapshot(item_id("name-cell", &fx.path("notes.txt")))
        .bounds();
    let folder = ui.snapshot(item_id("row", &fx.path("Projects"))).bounds();

    // Letting go before the delay drops it on the folder, without opening it.
    ui.drag_points(file.center(), folder.center(), Modifiers::none());

    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert!(fx.path("Projects/notes.txt").exists());

    // And it stays closed, even if another drag starts before the delay ends.
    let budget = ui
        .snapshot(item_id("name-cell", &fx.path("Budget.csv")))
        .bounds();
    let empty = point(budget.center().x, budget.bottom() + px(200.));
    ui.drag_and_hold(budget.center(), empty);
    ui.settle();
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    ui.release(empty);
}

#[gpui_kit::test]
fn a_folder_of_the_place_the_window_left_takes_no_drag(cx: &mut TestAppContext) {
    use fffinder::model::Location;
    let fx = Fixture::new("spring-left");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let file = ui
        .snapshot(item_id("name-cell", &fx.path("notes.txt")))
        .bounds();
    let folder = ui.snapshot(item_id("row", &fx.path("Projects"))).bounds();
    let roadmap = ui.snapshot(item_id("row", &fx.path("Roadmap.md"))).bounds();
    // The next frame is drawn, and the mouse moves on with the button down.
    // Spawned work isn't let run, so a place gone to stays unread.
    let drag_on_to = |ui: &mut Ui, position: Point<Pixels>| {
        ui.window(|window, cx| {
            use gpui_kit::InputEvent as _;
            use gpui_kit::test::TestWindowExt as _;
            window.render_frame(cx);
            window.dispatch_event(
                gpui_kit::MouseMoveEvent {
                    position,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Modifiers::none(),
                }
                .to_platform_input(),
                cx,
            );
        });
    };

    // The window goes to Recents with a drag under way. Until Recents has
    // been read the folder's items still show, and a drop on the folder
    // among them is a drop on Recents, which takes none.
    ui.drag_and_hold(file.center(), folder.center());
    ui.press_without_settling(&cmd("shift-f"));
    let nudged = folder.center() + point(px(3.), px(0.));
    drag_on_to(&mut ui, nudged);
    ui.release(nudged);
    assert_eq!(ui.read(|f| f.location().clone()), Location::Recents);
    assert!(fx.path("notes.txt").exists());
    assert!(!fx.path("Projects/notes.txt").exists());

    // Nor does a drag that comes to rest on it wait for it to open, and
    // open it once the wait is over, when Recents takes long to read.
    ui.press(&cmd("["));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    ui.drag_and_hold(file.center(), roadmap.center());
    ui.press_without_settling(&cmd("shift-f"));
    ui.view.update(ui.cx, |f, _| f.stall_listing_for_test());
    drag_on_to(&mut ui, folder.center());
    ui.settle();
    ui.settle();
    assert_eq!(ui.read(|f| f.location().clone()), Location::Recents);
    ui.release(folder.center());
}

#[gpui_kit::test]
fn a_press_made_before_the_window_left_drags_nothing_away(cx: &mut TestAppContext) {
    use gpui_kit::InputEvent as _;
    let fx = Fixture::new("drag-left");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.go_to(&fx.path("Projects/atlas"));
    let inside = fx.path("Projects/atlas/Project Atlas.txt");
    let file = ui.snapshot(item_id("name-cell", &inside)).bounds();
    let below = file.center() + point(px(0.), px(120.));

    // The mouse goes down on a file and stays down while the window goes up
    // a folder, which isn't read yet.
    ui.window(|window, cx| {
        for event in [
            gpui_kit::MouseMoveEvent {
                position: file.center(),
                pressed_button: None,
                modifiers: Modifiers::none(),
            }
            .to_platform_input(),
            gpui_kit::MouseDownEvent {
                button: MouseButton::Left,
                position: file.center(),
                modifiers: Modifiers::none(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
        ] {
            window.dispatch_event(event, cx);
        }
    });
    ui.press_without_settling(&cmd("up"));
    assert_eq!(ui.folder(), Some(fx.path("Projects")));

    // The file still shows, and moving the mouse on doesn't pick it up: let
    // go of over the file area, it would be moved to the folder gone to.
    ui.window(|window, cx| {
        use gpui_kit::test::TestWindowExt as _;
        window.render_frame(cx);
        window.dispatch_event(
            gpui_kit::MouseMoveEvent {
                position: below,
                pressed_button: Some(MouseButton::Left),
                modifiers: Modifiers::none(),
            }
            .to_platform_input(),
            cx,
        );
    });
    assert!(ui.exists(item_id("name-cell", &inside)));
    assert!(!ui.window(|_, cx| cx.has_active_drag()));
    ui.release(below);
    assert!(inside.exists());
    assert!(!fx.path("Projects/Project Atlas.txt").exists());
    assert_eq!(ui.names(), ["atlas"]);
}

#[gpui_kit::test]
fn holding_a_drag_over_what_covers_a_folder_leaves_it_closed(cx: &mut TestAppContext) {
    let fx = Fixture::new("spring-covered");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    // Wide enough for every row to run on underneath the preview pane.
    ui.drag_by("resize-Name", 2000.);
    let file = ui
        .snapshot(item_id("name-cell", &fx.path("notes.txt")))
        .bounds();
    let folder = ui.snapshot(item_id("row", &fx.path("Projects"))).bounds();
    let pane = ui.snapshot("preview-pane").bounds();
    let over_the_pane = point(pane.center().x, folder.center().y);
    assert!(
        folder.contains(&over_the_pane),
        "the row goes under the pane"
    );
    // The name's own start, since the wide cell's middle is under the pane too.
    let grab = point(file.left() + px(40.), file.center().y);
    assert!(grab.x < pane.left());

    ui.drag_and_hold(grab, over_the_pane);
    ui.settle();
    ui.settle();
    assert_eq!(ui.folder(), Some(fx.dir.clone()));

    // Over the part of the row that shows, the same wait opens it.
    let shows = point(folder.left() + px(40.), folder.center().y);
    ui.move_while_held(shows, Modifiers::none());
    ui.settle();
    ui.settle();
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    ui.release(shows);
}

// ----- the path bar --------------------------------------------------------------------

#[gpui_kit::test]
fn items_drop_on_the_path_bar(cx: &mut TestAppContext) {
    let fx = Fixture::new("crumb-drop");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.go_to(&fx.path("Projects/atlas"));
    let crumbs = fffinder_core::places::breadcrumbs(&fx.path("Projects/atlas")).len();

    // The crumb before the last is the folder this one is in.
    ui.drag(
        item_id("name-cell", &fx.path("Projects/atlas/Project Atlas.txt")),
        ("crumb", crumbs - 2),
        Modifiers::none(),
    );

    assert!(fx.path("Projects/Project Atlas.txt").exists());
}

#[gpui_kit::test]
fn the_current_folder_stays_in_a_long_path_bar(cx: &mut TestAppContext) {
    let fx = Fixture::new("crumb-long");
    let deep = "a folder with quite a long name/".repeat(7) + "the folder shown";
    let deep = fx.path(&deep);
    std::fs::create_dir_all(&deep).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&deep);

    let crumbs = fffinder_core::places::breadcrumbs(&deep).len();
    let shown = ui.snapshot(("crumb", crumbs - 1)).bounds();
    let window = ui.snapshot("content").bounds();

    assert!(
        shown.right() <= window.right(),
        "the last crumb ends at {:?}, past the window at {:?}",
        shown.right(),
        window.right()
    );
}

// ----- the toolbar ------------------------------------------------------------------

#[gpui_kit::test]
fn the_toolbar_folds_what_does_not_fit(cx: &mut TestAppContext) {
    let fx = Fixture::new("toolbar-fold");
    let mut ui = Ui::open(cx, &fx.dir);
    let width = |ui: &mut Ui, points: f32| {
        let handle = ui.handle;
        gpui_kit::VisualTestContext::from_window(handle, ui.cx)
            .simulate_resize(gpui_kit::size(px(points), px(600.)));
        ui.settle();
        ui.frame();
    };

    width(&mut ui, 1400.);
    assert!(ui.exists("search-scope"));
    assert!(ui.exists("view-list"));
    assert!(ui.exists("sort"));
    assert!(ui.exists("nushell"));

    // The sidebar takes 212 of these, and the rest isn't quite enough.
    width(&mut ui, 850.);
    assert!(!ui.exists("search-scope"), "the first to go");
    assert!(ui.exists("view-list"));

    width(&mut ui, 800.);
    assert!(!ui.exists("view-list"), "the views become one menu");
    assert!(ui.exists("view-menu"));
    assert!(ui.exists("nushell"));

    width(&mut ui, 700.);
    assert!(!ui.exists("nushell"), "into the Actions menu");
    assert!(ui.exists("sort"));
    ui.click("actions");
    assert!(ui.menu_labels().contains(&"Nushell".to_string()));
    ui.press("escape");

    width(&mut ui, 640.);
    assert!(!ui.exists("sort"), "into the Actions menu too");
    ui.click("actions");
    assert!(
        ui.menu_labels()
            .contains(&"Sort and View Options".to_string())
    );
    ui.press("escape");

    // The search field stays inside the window all the way down.
    let search = ui.snapshot("search").bounds();
    assert!(search.right() <= px(640.), "{:?}", search.right());

    // And everything comes back.
    width(&mut ui, 1400.);
    assert!(ui.exists("search-scope"));
    assert!(ui.exists("view-list"));
}

#[gpui_kit::test]
fn the_toolbar_folds_just_when_the_drawn_controls_run_out_of_room(cx: &mut TestAppContext) {
    let fx = Fixture::new("toolbar-measured");
    let mut ui = Ui::open(cx, &fx.dir);
    let folds =
        |ui: &mut Ui| ["search-scope", "view-list", "nushell", "sort"].map(|id| ui.exists(id));
    let search_min = px(120.);
    let close_enough = px(1.5);
    let mut before = None;
    let mut seen = 0;

    // Down a point at a time. It takes two frames for a new width to be
    // measured and then used.
    for width in (640..=1000).rev() {
        let width = px(width as f32);
        gpui_kit::VisualTestContext::from_window(ui.handle, ui.cx)
            .simulate_resize(gpui_kit::size(width, px(600.)));
        ui.frame();
        ui.frame();
        let search = ui.snapshot("search").bounds();
        assert!(
            search.right() <= width,
            "at {width:?} the search field runs out of the window: {search:?}"
        );

        let now = folds(&mut ui);
        if let Some((folded_at, last_search)) = before
            && folded_at != now
        {
            seen += 1;
            // Measured against the sums in `toolbar.rs`: the last width with
            // this control showing left the search field at its narrowest.
            let spare: Pixels = last_search - search_min;
            assert!(
                spare.abs() <= close_enough,
                "{folded_at:?} became {now:?} at {width:?}, with {spare:?} to spare"
            );
        }
        before = Some((now, search.size.width));
    }
    assert_eq!(seen, 4, "each of the four folds happened");
}

#[gpui_kit::test]
fn the_scope_bar_wraps_rather_than_losing_its_controls(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-wrap");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("notes");
    ui.settle();
    let handle = ui.handle;
    gpui_kit::VisualTestContext::from_window(handle, ui.cx)
        .simulate_resize(gpui_kit::size(px(640.), px(600.)));
    ui.settle();

    let kind = ui.snapshot("kind").bounds();

    assert!(kind.right() <= px(640.), "{:?}", kind.right());
}

// ----- renaming in place ----------------------------------------------------------------

#[gpui_kit::test]
fn f2_renames_in_place(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-inline");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");

    // Only the part before the extension is selected, so typing replaces it.
    ui.select("Roadmap.md");
    ui.press("f2");
    assert!(ui.exists("rename-input"));
    assert!(!ui.dialog_open());
    ui.type_text("Plan B");
    ui.press("enter");
    assert!(!ui.exists("rename-input"));
    assert!(
        fx.path("Plan B.md").exists(),
        "a space is text, not Quick Look"
    );
    assert!(!ui.read(|f| f.quick_look_is_open()));
    assert_eq!(ui.selected(), ["Plan B.md"]);
    assert!(ui.files_focused(), "the keys go back to the files");

    // Letters are text too; they don't select other items.
    ui.select("Plan B.md");
    ui.press("f2");
    ui.type_text("n");
    assert_eq!(ui.selected(), ["Plan B.md"]);

    // Escape leaves the name alone.
    ui.press("escape");
    assert!(!ui.exists("rename-input"));
    assert!(fx.path("Plan B.md").exists());
    assert!(ui.files_focused());

    // Clicking somewhere else keeps what was typed.
    ui.press("f2");
    ui.type_text("Plan C");
    ui.click(item_id("row", &fx.path("notes.txt")));
    assert!(fx.path("Plan C.md").exists());
    assert!(!ui.exists("rename-input"));

    // Undo puts the old name back, as it does for any rename.
    ui.press(&cmd("z"));
    assert!(fx.path("Plan B.md").exists());
}

#[gpui_kit::test]
fn a_taken_name_keeps_editing(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-taken");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");

    ui.select("Roadmap.md");
    ui.press("f2");
    ui.press(&cmd("a"));
    ui.type_text("notes.txt");
    ui.press("enter");

    assert!(ui.exists("rename-input"), "still editing, to fix the name");
    assert!(fx.path("Roadmap.md").exists());
    ui.press("escape");
    assert!(fx.path("Roadmap.md").exists());
}

/// The names in `dir`, exactly as the disk has them.
fn names_on_disk(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|item| item.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Start renaming `name`, replacing all of it with `typed`.
fn retype_name(ui: &mut Ui, name: &str, typed: &str) {
    ui.select(name);
    ui.press("f2");
    ui.press(&cmd("a"));
    ui.type_text(typed);
}

#[gpui_kit::test]
fn a_rename_that_only_changes_case_goes_ahead(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-case");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");

    retype_name(&mut ui, "Roadmap.md", "ROADMAP.md");
    ui.press("enter");

    assert!(!ui.exists("rename-input"), "it wasn't refused as taken");
    let names = names_on_disk(&fx.dir);
    assert!(names.contains(&"ROADMAP.md".to_string()), "{names:?}");
    assert!(!names.contains(&"Roadmap.md".to_string()), "{names:?}");

    ui.press(&cmd("z"));
    assert!(names_on_disk(&fx.dir).contains(&"Roadmap.md".to_string()));
}

#[gpui_kit::test]
fn the_file_keys_come_back_when_the_item_being_renamed_goes(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-gone");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    retype_name(&mut ui, "Roadmap.md", "a/b");
    ui.press("enter");
    assert!(ui.exists("rename-input"), "refused, and still editing");

    std::fs::remove_file(fx.path("Roadmap.md")).unwrap();
    ui.update(|f, _, cx| f.reload_quietly(cx));

    assert!(!ui.exists("rename-input"));
    assert!(ui.files_focused());
    ui.select("Budget.csv");
    ui.press("down");
    assert_ne!(ui.selected(), ["Budget.csv"], "the arrow keys work again");
}

#[gpui_kit::test]
fn the_file_keys_come_back_when_a_refused_name_scrolls_away(cx: &mut TestAppContext) {
    let fx = long_folder("rename-scrolled");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    retype_name(&mut ui, "item 000.txt", "a/b");
    ui.press("enter");
    assert!(ui.exists("rename-input"));

    ui.scroll("list-columns", point(px(0.), px(-3000.)));

    assert!(!ui.exists("rename-input"));
    assert!(ui.files_focused());
    assert!(fx.path("item 000.txt").exists());
    ui.select("item 100.txt");
    ui.press("down");
    assert_eq!(ui.selected(), ["item 101.txt"]);
}

#[gpui_kit::test]
fn the_file_keys_come_back_when_a_key_leaves_the_folder_during_a_rename(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-left");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    ui.go_to(&fx.path("Projects"));
    ui.select("atlas");
    ui.press("f2");
    assert!(ui.exists("rename-input"));

    // The window's shortcuts work in the field too. Back shows another
    // folder, which has no row for the field.
    ui.press(&cmd("["));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert!(!ui.exists("rename-input"));
    assert!(fx.path("Projects/atlas").is_dir(), "nothing was renamed");
    assert!(ui.files_focused());
    assert_eq!(ui.selected(), ["Projects"]);
    ui.press("down");
    assert_eq!(ui.selected(), ["Roadmap.md"], "the arrow keys work again");
}

#[gpui_kit::test]
fn a_click_outside_the_files_ends_a_rename_and_gives_them_the_keyboard(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-click-out");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    let projects = fx.path("Projects");
    ui.update(|f, _, cx| f.add_favorite(projects.clone(), cx));
    let favorite = ui.read(|f| {
        f.favorites()
            .iter()
            .position(|p| p.path == projects)
            .unwrap()
    });
    ui.click("view-list");

    // The status bar takes no keys, and a press on it leaves the keyboard
    // where it is. That is in the files by then: a press outside the field
    // has ended the edit, and what was typed is kept.
    retype_name(&mut ui, "Roadmap.md", "Plan.md");
    ui.click("status-text");
    assert!(!ui.exists("rename-input"));
    assert!(fx.path("Plan.md").exists());
    assert!(ui.files_focused());
    assert_eq!(ui.selected(), ["Plan.md"]);
    ui.press("down");
    assert_eq!(ui.selected(), ["Projects"], "the arrow keys work again");

    // A row of the sidebar's list goes to its folder as well, and the name
    // is kept before the window leaves the item's folder.
    retype_name(&mut ui, "notes.txt", "todo.txt");
    ui.click(("fav", favorite));
    assert!(fx.path("todo.txt").exists());
    assert_eq!(ui.folder(), Some(projects));
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["atlas"]);

    // A name that was refused goes on being edited, as after any press
    // outside its field, and the field still has the keyboard: Escape
    // reaches it.
    ui.go_to(&fx.dir);
    retype_name(&mut ui, "Budget.csv", "a/b");
    ui.press("enter");
    ui.click("status-text");
    assert!(ui.exists("rename-input"), "still editing, to fix the name");
    ui.press("escape");
    assert!(!ui.exists("rename-input"));
    assert!(fx.path("Budget.csv").exists());
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn leaving_a_refused_name_says_why_once_and_keeps_the_selection(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-refused");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    retype_name(&mut ui, "Roadmap.md", "a/b");
    let before = ui.notification_count();

    ui.click(item_id("row", &fx.path("notes.txt")));

    assert_eq!(ui.notification_count(), before + 1, "said once");
    assert!(ui.exists("rename-input"), "still editing, to fix the name");
    assert_eq!(ui.selected(), ["Roadmap.md"], "the press only said why");

    // Pressing Return is asking again, and a different name is news.
    ui.press("enter");
    assert_eq!(ui.notification_count(), before + 2);
    ui.press(&cmd("a"));
    ui.type_text("c/d");
    ui.click(item_id("row", &fx.path("notes.txt")));
    assert_eq!(ui.notification_count(), before + 3);

    // A name that can be used finishes the same way as ever.
    ui.press(&cmd("a"));
    ui.type_text("Plan.md");
    ui.click(item_id("row", &fx.path("notes.txt")));
    assert!(!ui.exists("rename-input"));
    assert!(fx.path("Plan.md").exists());
}

#[gpui_kit::test]
fn a_press_on_another_item_that_ends_a_rename_leaves_that_item_selected(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-press-other");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");

    // The press keeps the name, and the folder is read again for it. The
    // item pressed is still the selected one after that, and the menu the
    // press opened acts on the item it names.
    ui.select("notes.txt");
    ui.press("f2");
    ui.type_text("plan");
    ui.right_click(item_id("row", &fx.path("Roadmap.md")));
    assert!(fx.path("plan.txt").exists());
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert!(ui.menu_labels().contains(&"Cut “Roadmap.md”".to_string()));
    ui.click_menu_item("Move to Trash");
    assert!(!fx.path("Roadmap.md").exists());
    assert!(fx.path("plan.txt").exists(), "not the item just renamed");

    // The same after a click, for the keys that follow it, among the icons.
    ui.click("view-icons");
    ui.select("plan.txt");
    ui.press("f2");
    ui.type_text("todo");
    ui.click(item_id("tile", &fx.path("Budget.csv")));
    assert!(fx.path("todo.txt").exists());
    assert_eq!(ui.selected(), ["Budget.csv"]);
    ui.press(&cmd("backspace"));
    assert!(!fx.path("Budget.csv").exists());
    assert!(fx.path("todo.txt").exists(), "not the item just renamed");

    // An arrow key that follows Return, before the folder has been read
    // again, moves on from the item as well.
    ui.click("view-list");
    ui.select("todo.txt");
    ui.press("f2");
    ui.type_text("zebra");
    ui.press_without_settling("enter");
    ui.press("up");
    assert!(fx.path("zebra.txt").exists());
    assert_eq!(ui.selected(), ["Projects"]);
}

#[gpui_kit::test]
fn a_dialog_opened_during_a_rename_gets_the_keyboard(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-dialog");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    retype_name(&mut ui, "Roadmap.md", "Plan.md");

    ui.press(&cmd("shift-g"));
    assert!(ui.dialog_open());
    assert!(fx.path("Plan.md").exists(), "the name was kept");
    ui.type_text("Projects");
    ui.press("enter");

    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert!(ui.files_focused(), "and the files get it back afterward");
}

#[gpui_kit::test]
fn moving_to_the_search_field_during_a_rename_leaves_the_keyboard_there(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-search");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    retype_name(&mut ui, "Roadmap.md", "Plan.md");

    ui.press(&cmd("f"));
    ui.type_text("notes");
    ui.settle();

    assert!(fx.path("Plan.md").exists(), "the name was kept");
    assert_eq!(ui.read(|f| f.query().to_string()), "notes");
}

#[gpui_kit::test]
fn return_ends_a_rename_in_its_own_window_with_quick_look_open(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-quick-look");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.select("Roadmap.md");
    ui.press("space");
    assert!(ui.read(|f| f.quick_look_window()).is_some());
    ui.press("f2");
    ui.type_text("Plan");

    // Quick Look's window reads this browser too, to draw the preview, and
    // it's the last of the two to have drawn when Return arrives.
    ui.press_after_quick_look_draws("enter");
    ui.settle();
    assert!(fx.path("Plan.md").exists());
    assert!(!ui.exists("rename-input"));
    assert!(
        ui.files_focused(),
        "the keys go back to the files in the window the name was edited in"
    );
    ui.press("down");
    assert_eq!(ui.selected(), ["Projects"], "the arrow keys work again");
}

#[gpui_kit::test]
fn switching_to_another_app_leaves_a_rename_as_it_is(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-switch");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    retype_name(&mut ui, "Roadmap.md", "Half typed");

    ui.deactivate();

    assert!(ui.exists("rename-input"));
    assert!(fx.path("Roadmap.md").exists(), "nothing was renamed");

    ui.activate();
    ui.type_text(" name");
    ui.press("enter");
    assert!(fx.path("Half typed name").exists());
}

#[gpui_kit::test]
fn a_rename_goes_on_when_its_row_comes_back_while_in_another_app(cx: &mut TestAppContext) {
    let fx = long_folder("rename-switch-scrolled");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    retype_name(&mut ui, "item 000.txt", "Half typed");

    // The list scrolls under the mouse while another app is in front. The
    // edit is left as it is, and nothing else takes the keyboard meanwhile.
    ui.deactivate();
    ui.scroll("list-columns", point(px(0.), px(-3000.)));
    assert!(!ui.exists("rename-input"));
    ui.scroll("list-columns", point(px(0.), px(3000.)));
    assert!(ui.exists("rename-input"));

    ui.activate();
    ui.type_text(" name");
    ui.press("enter");
    assert!(fx.path("Half typed name").exists());
}

#[gpui_kit::test]
fn the_file_keys_come_back_when_an_item_goes_while_in_another_app(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-away");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    retype_name(&mut ui, "Roadmap.md", "a/b");
    ui.press("enter");

    ui.deactivate();
    std::fs::remove_file(fx.path("Roadmap.md")).unwrap();
    ui.update(|f, _, cx| f.reload_quietly(cx));
    ui.activate();

    assert!(!ui.exists("rename-input"));
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn renaming_in_place_works_in_every_view_but_the_gallery(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-views");
    let mut ui = Ui::open(cx, &fx.dir);

    ui.click("view-icons");
    ui.select("notes.txt");
    ui.press("f2");
    ui.type_text("icons");
    ui.press("enter");
    assert!(fx.path("icons.txt").exists());

    ui.click("view-columns");
    ui.settle();
    ui.click(item_id("crow", &fx.path("icons.txt")));
    ui.press("f2");
    ui.type_text("columns");
    ui.press("enter");
    assert!(fx.path("columns.txt").exists());
    assert_eq!(ui.selected(), ["columns.txt"], "still the selected item");

    ui.click("view-gallery");
    ui.press("f2");
    assert!(ui.dialog_open());
    assert!(!ui.exists("rename-field"));
    ui.press("escape");
}

#[gpui_kit::test]
fn a_name_changed_in_a_further_column_shows_there_at_once(cx: &mut TestAppContext) {
    let fx = projects_with("rename-column", &["a.txt", "b.txt", "c.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));

    ui.press("f2");
    ui.type_text("zz");
    ui.press("enter");

    assert!(fx.path("Projects/zz.txt").exists());
    assert_eq!(columns(&ui)[1], ["a.txt", "atlas", "c.txt", "zz.txt"]);
    assert_eq!(ui.selected(), ["zz.txt"], "where it sorts now");
    assert!(ui.files_focused(), "the keys go back to the files");
    ui.look_for_changes(1);
    assert_eq!(
        ui.selected(),
        ["zz.txt"],
        "after the window has looked at the folder again too"
    );

    // A folder's own column follows it to its new name.
    ui.click(item_id("crow", &fx.path("Projects/atlas")));
    ui.press("f2");
    ui.type_text("maps");
    ui.press("enter");
    assert_eq!(ui.selected(), ["maps"]);
    assert_eq!(
        columns(&ui)[1..],
        [
            vec!["a.txt", "c.txt", "maps", "zz.txt"],
            vec!["Project Atlas.txt"]
        ]
    );
}

#[gpui_kit::test]
fn a_press_on_another_row_that_ends_a_rename_leaves_that_row_selected_in_its_column(
    cx: &mut TestAppContext,
) {
    let fx = projects_with("rename-press-column", &["a.txt", "b.txt", "c.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-columns");

    // The first column, which lists the window's folder.
    ui.click(item_id("crow", &fx.path("notes.txt")));
    ui.press("f2");
    ui.type_text("plan");
    ui.right_click(item_id("crow", &fx.path("Roadmap.md")));
    assert!(fx.path("plan.txt").exists());
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    ui.click_menu_item("Move to Trash");
    assert!(!fx.path("Roadmap.md").exists());
    assert!(fx.path("plan.txt").exists(), "not the item just renamed");

    // A further column, which shows the new name all the same.
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));
    ui.press("f2");
    ui.type_text("bb");
    ui.right_click(item_id("crow", &fx.path("Projects/c.txt")));
    assert_eq!(columns(&ui)[1], ["a.txt", "atlas", "bb.txt", "c.txt"]);
    assert_eq!(ui.selected(), ["c.txt"]);
    ui.click_menu_item("Move to Trash");
    assert!(!fx.path("Projects/c.txt").exists());
    assert!(
        fx.path("Projects/bb.txt").exists(),
        "not the item just renamed"
    );

    // A click on the folder's own row, a column back, makes that column the
    // one in use, and it stays so.
    ui.click(item_id("crow", &fx.path("Projects/a.txt")));
    ui.press("f2");
    ui.type_text("aa");
    ui.click(item_id("crow", &fx.path("Projects")));
    assert_eq!(columns(&ui)[1], ["aa.txt", "atlas", "bb.txt"]);
    assert_eq!(ui.selected(), ["Projects"]);
    ui.press("up");
    assert_eq!(
        ui.selected(),
        ["plan.txt"],
        "the keys go on from the folder"
    );

    // After Select All the other views' selection holds every item, and no
    // column shows it. What counts is the item the first column shows
    // selected: the one renamed after Return, and the one a press was on.
    ui.press(&cmd("a"));
    ui.press("f2");
    ui.type_text("todo");
    ui.press("enter");
    assert_eq!(ui.selected(), ["todo.txt"]);
    ui.press(&cmd("a"));
    ui.press("f2");
    ui.type_text("zebra");
    ui.right_click(item_id("crow", &fx.path("Budget.csv")));
    assert!(fx.path("zebra.txt").exists());
    assert_eq!(ui.selected(), ["Budget.csv"]);
}

#[gpui_kit::test]
fn a_press_inside_a_folder_that_ends_its_rename_leaves_the_folder_selected(
    cx: &mut TestAppContext,
) {
    let fx = projects_with("rename-press-inside", &["a.txt", "b.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");

    // An item inside a folder that is open in the list has its row under
    // the folder's old name only, and the folder is closed under its new
    // one. Nothing else would stay selected, so the folder is, as after
    // Return.
    ui.select("Projects");
    ui.press("right");
    ui.press("f2");
    ui.type_text("Work");
    ui.click(item_id("row", &fx.path("Projects/b.txt")));
    assert_eq!(
        ui.names(),
        ["Budget.csv", "notes.txt", "Roadmap.md", "Work"]
    );
    assert_eq!(ui.selected(), ["Work"]);

    // The same in a column after the first, where the folder's own column
    // is opened again for it.
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Work")));
    ui.click(item_id("crow", &fx.path("Work/atlas")));
    ui.press("f2");
    ui.type_text("maps");
    ui.click(item_id("crow", &fx.path("Work/atlas/Project Atlas.txt")));
    assert_eq!(ui.selected(), ["maps"]);
    assert_eq!(
        columns(&ui)[1..],
        [vec!["a.txt", "b.txt", "maps"], vec!["Project Atlas.txt"]]
    );
}

#[gpui_kit::test]
fn a_new_folder_is_named_in_place(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-new");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");

    ui.press(&cmd("shift-n"));

    assert!(!ui.dialog_open());
    assert_eq!(ui.selected(), ["untitled folder"]);
    ui.type_text("Archive");
    ui.press("enter");
    assert!(fx.path("Archive").is_dir());
    assert_eq!(ui.selected(), ["Archive"]);
}

#[gpui_kit::test]
fn a_new_folder_is_selected_in_the_column_view_too(cx: &mut TestAppContext) {
    let fx = Fixture::new("rename-new-column");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("notes.txt")));

    ui.press(&cmd("shift-n"));

    assert!(ui.exists("rename-input"));
    assert_eq!(ui.selected(), ["untitled folder"], "while it is named");
    ui.type_text("Archive");
    ui.press("enter");
    assert!(fx.path("Archive").is_dir());
    assert_eq!(ui.selected(), ["Archive"]);

    // With a further column in use the folder is made and named in the
    // first one all the same, which goes on showing the way to that column.
    // The other views have the folder selected under its name.
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/atlas")));
    ui.press(&cmd("shift-n"));
    ui.type_text("Later");
    ui.press("enter");
    assert!(fx.path("Later").is_dir());
    assert_eq!(ui.selected(), ["atlas"]);
    ui.click("view-list");
    assert_eq!(ui.selected(), ["Later"]);
}

/// The ID of the header of the group named `label`.
fn group_header(label: &str) -> gpui_kit::SharedString {
    gpui_kit::SharedString::from(format!("group-{label}"))
}

/// Collapse every group the view shows, and say which they were.
fn collapse_every_group(ui: &mut Ui) -> Vec<String> {
    let labels = ui.read(|f| f.group_headers());
    for label in &labels {
        ui.click(group_header(label));
    }
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    labels
}

#[gpui_kit::test]
fn a_new_folder_opens_its_collapsed_group_to_be_named(cx: &mut TestAppContext) {
    use fffinder_core::GroupKey;
    let fx = Fixture::new("rename-new-collapsed");
    std::fs::create_dir(fx.path("Empty")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));
    let labels = collapse_every_group(&mut ui);

    // The name is typed in the new folder's row, so the group it falls in
    // opens for it. The other groups stay as they were.
    ui.press(&cmd("shift-n"));
    assert_eq!(ui.names(), ["Empty", "Projects", "untitled folder"]);
    assert_eq!(ui.read(|f| f.group_headers()), labels);
    assert!(ui.exists("rename-input"), "the name's field is in its row");
    assert_eq!(ui.selected(), ["untitled folder"]);
    ui.type_text("Archive");
    ui.press("enter");
    assert!(fx.path("Archive").is_dir(), "the field had the keyboard");
    assert_eq!(ui.selected(), ["Archive"]);
    assert!(ui.files_focused(), "and the files have it afterward");
    ui.press("down");
    assert_eq!(ui.selected(), ["Empty"], "the arrow keys work");

    // The group stays open, as after a click on its header, and a tab's
    // groups are the same from folder to folder.
    ui.go_to(&fx.path("Projects"));
    assert_eq!(ui.names(), ["atlas"]);

    // Collapsed again, it leaves an empty folder with its message, whose
    // menu makes a folder too.
    ui.click(group_header("Folder"));
    ui.go_to(&fx.path("Empty"));
    ui.right_click("message-title");
    ui.click_menu_item("New Folder");
    assert_eq!(ui.names(), ["untitled folder"]);
    assert!(ui.exists("rename-input"));
    ui.type_text("Inner");
    ui.press("enter");
    assert!(fx.path("Empty/Inner").is_dir());
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn a_new_folder_from_a_menu_opens_its_collapsed_group_in_the_icon_view(cx: &mut TestAppContext) {
    use fffinder_core::GroupKey;
    let fx = Fixture::new("rename-new-collapsed-icons");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-icons");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));
    ui.click(group_header("Folder"));
    assert_eq!(ui.names(), ["Budget.csv", "Roadmap.md", "notes.txt"]);

    // From the empty-space menu, under the tiles.
    let area = ui.snapshot("content").bounds();
    ui.right_click_point(area.origin + point(px(300.), area.size.height - px(20.)));
    ui.click_menu_item("New Folder");
    assert!(ui.exists("rename-input"), "the name's field is in its tile");
    assert_eq!(ui.selected(), ["untitled folder"]);
    ui.type_text("Icons");
    ui.press("enter");
    assert!(fx.path("Icons").is_dir());
    assert_eq!(ui.selected(), ["Icons"]);
    assert!(ui.files_focused());

    // And from the Actions menu. Escape leaves the folder the name it has.
    ui.click(group_header("Folder"));
    ui.click("actions");
    ui.click_menu_item("New Folder");
    assert!(ui.exists("rename-input"));
    ui.press("escape");
    assert!(!ui.exists("rename-input"));
    assert!(fx.path("untitled folder").is_dir());
    assert_eq!(ui.selected(), ["untitled folder"]);
    assert!(ui.files_focused());
    ui.press("left");
    assert_eq!(ui.selected(), ["Projects"], "the arrow keys work");
}

#[gpui_kit::test]
fn a_new_folder_keeps_its_field_when_the_folder_is_read_again(cx: &mut TestAppContext) {
    use fffinder_core::GroupKey;
    let fx = Fixture::new("rename-new-last-opened");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    // When an item was last opened is read along with a folder's listing,
    // and New Folder shows its folder before the listing is read again.
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::LastOpened), cx));
    let labels = collapse_every_group(&mut ui);

    // The folder is in the same group before and after, so the group that
    // opened for it is the one it stays in.
    ui.press_without_settling(&cmd("shift-n"));
    assert_eq!(ui.read(|f| f.group_headers()), labels);
    assert!(ui.exists("rename-input"));
    ui.settle();
    assert_eq!(ui.read(|f| f.group_headers()), labels);
    assert!(ui.exists("rename-input"), "also once the folder is read");
    ui.type_text("Archive");
    ui.press("enter");
    assert!(fx.path("Archive").is_dir());
}

#[gpui_kit::test]
fn a_new_folder_opens_the_group_its_row_falls_in(cx: &mut TestAppContext) {
    use fffinder_core::GroupKey;
    let fx = Fixture::new("rename-new-sized");
    std::fs::create_dir(fx.path("untitled folder")).unwrap();
    std::fs::write(fx.path("untitled folder/old.txt"), "old").unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Size), cx));

    // A folder's size comes from the index, which here still counts the
    // file of a folder that has gone. A new folder that takes its name is
    // listed with that size, among the items of that size and not among
    // the folders without one.
    std::fs::remove_dir_all(fx.path("untitled folder")).unwrap();
    ui.update(|f, _, cx| f.reload_quietly(cx));
    assert_eq!(
        ui.names(),
        ["Budget.csv", "notes.txt", "Projects", "Roadmap.md"]
    );
    let labels = collapse_every_group(&mut ui);
    assert_eq!(labels, ["Tiny (under 100 KB)"]);

    ui.press(&cmd("shift-n"));
    assert_eq!(ui.read(|f| f.group_headers()), labels);
    assert!(ui.exists("rename-input"), "its group opened for it");
    assert_eq!(ui.selected(), ["untitled folder"]);
}

#[gpui_kit::test]
fn a_new_folder_in_a_view_without_groups_leaves_them_collapsed(cx: &mut TestAppContext) {
    use fffinder_core::GroupKey;
    let fx = Fixture::new("rename-new-no-groups");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));
    ui.click(group_header("Folder"));

    // The column view has no groups, so nothing hides the new folder's row
    // there, and the list's groups are left as they were.
    ui.click("view-columns");
    ui.press(&cmd("shift-n"));
    assert!(ui.exists("rename-input"));
    ui.type_text("Archive");
    ui.press("enter");
    assert!(fx.path("Archive").is_dir());
    ui.click("view-list");
    assert_eq!(ui.names(), ["Budget.csv", "Roadmap.md", "notes.txt"]);
}

#[gpui_kit::test]
fn a_new_folder_is_named_where_a_long_folder_lists_it(cx: &mut TestAppContext) {
    let fx = long_folder("rename-new-long");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");

    // "untitled folder" sorts after the 200 items, far below the rows on
    // screen, and its name's field only exists once its row is drawn.
    ui.press(&cmd("shift-n"));
    assert!(ui.exists("rename-input"), "the list scrolled to its row");
    ui.type_text("zz list");
    ui.press("enter");
    assert!(fx.path("zz list").is_dir());

    ui.click("view-columns");
    ui.press(&cmd("shift-n"));
    assert!(ui.exists("rename-input"), "and so did the column");
    ui.type_text("zz columns");
    ui.press("enter");
    assert!(fx.path("zz columns").is_dir());
}

#[gpui_kit::test]
fn the_keys_work_after_a_new_file_that_a_collapsed_group_hides(cx: &mut TestAppContext) {
    use fffinder_core::GroupKey;
    let fx = Fixture::new("new-file-collapsed");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.activate();
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));
    ui.click(group_header("Plain Text"));

    // A new file is named in a dialog before it is made, so no field waits
    // for a row to show in.
    ui.press(&cmd("alt-n"));
    assert!(ui.dialog_open());
    ui.type_text("fresh");
    ui.press("enter");
    assert!(fx.path("fresh.txt").exists());
    assert!(!ui.dialog_open());
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["Budget.csv"], "the arrow keys work");
}

// ----- the preview pane -----------------------------------------------------------------

#[gpui_kit::test]
fn the_preview_pane_follows_the_selection(cx: &mut TestAppContext) {
    let fx = Fixture::new("pane");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    assert!(!ui.exists("preview-pane"));

    ui.press(&cmd("shift-p"));
    assert!(ui.exists("preview-pane"));
    assert!(ui.exists("preview-pane-empty"), "nothing's selected yet");

    ui.select("Roadmap.md");
    assert!(!ui.exists("preview-pane-empty"));
    assert!(ui.read(|f| f.has_preview_for_test(&fx.path("Roadmap.md"))));

    ui.select_also("notes.txt");
    assert_eq!(ui.snapshot("preview-pane-many").label(), Some("2 items"));

    // It's for the views without a preview of their own.
    ui.click("view-gallery");
    assert!(!ui.exists("preview-pane"));
    ui.click("view-columns");
    assert!(!ui.exists("preview-pane"));
    ui.click("view-icons");
    assert!(ui.exists("preview-pane"));

    ui.press(&cmd("shift-p"));
    assert!(!ui.exists("preview-pane"));
}

#[gpui_kit::test]
fn a_preview_is_read_again_when_its_file_comes_back_changed(cx: &mut TestAppContext) {
    let fx = Fixture::new("pane-changed");
    let log = fx.path("log.txt");
    let lines =
        |what: &str| -> String { (1..=400).map(|ix| format!("line {ix} {what}\n")).collect() };
    std::fs::write(&log, lines("as written")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    let first_line = |ui: &Ui| {
        let text = ui.read(|f| f.preview_text_for_test(&log));
        text.and_then(|text| text.lines().next().map(str::to_string))
    };
    let text_top = |ui: &mut Ui| ui.snapshot("preview-text").bounds().origin;

    ui.select("log.txt");
    assert_eq!(first_line(&ui).as_deref(), Some("line 1 as written"));
    let top = text_top(&mut ui);
    ui.scroll_point(top + point(px(20.), px(20.)), point(px(0.), px(-300.)));
    let read_at = text_top(&mut ui);
    assert!(read_at.y < top.y - px(100.), "scrolled on");

    // One line longer, with the date it had: only its size says it changed.
    let written = std::fs::metadata(&log).unwrap().modified().unwrap();
    let dated = |when: std::time::SystemTime| {
        let file = std::fs::File::options().write(true).open(&log).unwrap();
        file.set_modified(when).unwrap();
    };
    std::fs::write(&log, lines("saved once") + "the end\n").unwrap();
    dated(written);
    ui.update(|f, _, cx| f.reload_quietly(cx));
    assert_eq!(first_line(&ui).as_deref(), Some("line 1 saved once"));
    assert_eq!(text_top(&mut ui), read_at, "still where it was being read");

    // As long as it was, a minute later: only its date says so. By now
    // more previews have been read than the window keeps, and the one
    // that's showing isn't let go with the others.
    let others: Vec<_> = (0..300)
        .map(|ix| fx.path(&format!("Projects/atlas/other {ix}.txt")))
        .collect();
    for other in &others {
        std::fs::write(other, "x").unwrap();
    }
    ui.update(|f, _, cx| {
        for other in &others {
            let other = fffinder_core::FileEntry::from_path(other).unwrap();
            f.ensure_preview(&other, cx);
        }
    });
    std::fs::write(&log, lines("SAVED ONCE") + "the end\n").unwrap();
    dated(written + std::time::Duration::from_secs(60));
    ui.update(|f, _, cx| f.reload_quietly(cx));
    assert_eq!(first_line(&ui).as_deref(), Some("line 1 SAVED ONCE"));
    assert_eq!(text_top(&mut ui), read_at, "and still there");

    // The gallery asks for its preview with every frame it draws, and has
    // it read once more for a file that changed, not once more a frame.
    ui.click("view-gallery");
    ui.select("log.txt");
    std::fs::write(&log, lines("saved twice")).unwrap();
    ui.update(|f, _, cx| f.reload_quietly(cx));
    assert_eq!(first_line(&ui).as_deref(), Some("line 1 saved twice"));
}

#[gpui_kit::test]
fn previews_show_what_other_programs_save(cx: &mut TestAppContext) {
    let fx = Fixture::new("outside-saves");
    let (notes, plan) = (fx.path("notes.txt"), fx.path("Projects/plan.txt"));
    let atlas = fx.path("Projects/atlas/Project Atlas.txt");
    std::fs::write(&plan, "the plan\n").unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    let text = |ui: &Ui, path: &std::path::Path| ui.read(|f| f.preview_text_for_test(path));
    // As most programs save: under another name, which then takes the
    // file's. That moves the folder's change time, so the next look for
    // changes reads the folder again, and no other folder.
    let save = |path: &std::path::Path, text: &str| {
        let draft = path.with_extension("draft");
        std::fs::write(&draft, text).unwrap();
        std::fs::rename(&draft, path).unwrap();
    };

    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    ui.select("notes.txt");
    assert_eq!(text(&ui, &notes).as_deref(), Some("plain notes\n"));
    save(&notes, "plain notes, and a few more\n");
    ui.look_for_changes(1);
    assert_eq!(
        text(&ui, &notes).as_deref(),
        Some("plain notes, and a few more\n"),
        "the window's own folder"
    );

    ui.select("Projects");
    ui.press("right");
    ui.select("plan.txt");
    assert_eq!(text(&ui, &plan).as_deref(), Some("the plan\n"));
    save(&plan, "the plan, as it is now\n");
    ui.look_for_changes(1);
    assert_eq!(
        text(&ui, &plan).as_deref(),
        Some("the plan, as it is now\n"),
        "a folder open in the list"
    );

    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/atlas")));
    ui.click(item_id("crow", &atlas));
    assert_eq!(text(&ui, &atlas).as_deref(), Some("atlas"));
    save(&atlas, "atlas, with its index");
    ui.look_for_changes(1);
    assert_eq!(
        text(&ui, &atlas).as_deref(),
        Some("atlas, with its index"),
        "a column pane"
    );
}

#[gpui_kit::test]
fn refresh_reads_the_shown_preview_again(cx: &mut TestAppContext) {
    let fx = Fixture::new("pane-refresh");
    let notes = fx.path("notes.txt");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    ui.select("notes.txt");
    assert!(ui.exists("preview-text"));

    // Written over where it is, and as long and as old as it was: nothing
    // about its row or its folder says that it changed.
    let written = std::fs::metadata(&notes).unwrap().modified().unwrap();
    std::fs::write(&notes, "other notes\n").unwrap();
    let file = std::fs::File::options().write(true).open(&notes).unwrap();
    file.set_modified(written).unwrap();
    ui.press(&cmd("r"));

    assert_eq!(
        ui.read(|f| f.preview_text_for_test(&notes)).as_deref(),
        Some("other notes\n")
    );
    assert!(ui.exists("preview-text"), "shown, not waited for");

    // The same for the column view's card, with the file in the column of
    // its own folder.
    let atlas = fx.path("Projects/atlas/Project Atlas.txt");
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/atlas")));
    ui.click(item_id("crow", &atlas));
    assert!(ui.exists("preview-text"));
    let written = std::fs::metadata(&atlas).unwrap().modified().unwrap();
    std::fs::write(&atlas, "ATLAS").unwrap();
    let file = std::fs::File::options().write(true).open(&atlas).unwrap();
    file.set_modified(written).unwrap();
    ui.press(&cmd("r"));

    assert_eq!(
        ui.read(|f| f.preview_text_for_test(&atlas)).as_deref(),
        Some("ATLAS")
    );
    assert!(ui.exists("preview-text"), "shown in the card");
}

#[gpui_kit::test]
fn a_preview_starts_at_its_first_line_however_far_the_last_one_was_scrolled(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("pane-scroll");
    for name in ["a.txt", "b.txt"] {
        let lines: String = (1..=400).map(|ix| format!("{name} line {ix}\n")).collect();
        std::fs::write(fx.path(name), lines).unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    let text_top = |ui: &mut Ui| ui.snapshot("preview-text").bounds().origin;

    ui.select("a.txt");
    let top = text_top(&mut ui);
    ui.select("b.txt");
    assert_eq!(text_top(&mut ui), top);

    // Over the first lines: the middle of the text is far below the pane.
    ui.scroll_point(top + point(px(20.), px(20.)), point(px(0.), px(-300.)));
    assert!(text_top(&mut ui).y < top.y - px(100.), "b.txt scrolled");

    ui.select("a.txt");
    assert_eq!(text_top(&mut ui), top, "a.txt is not where b.txt was left");
}

#[gpui_kit::test]
fn quick_look_shows_each_file_from_its_first_line(cx: &mut TestAppContext) {
    let fx = Fixture::new("quick-look-scroll");
    for name in ["a.rs", "b.rs"] {
        let lines: String = (1..=400)
            .map(|ix| format!("fn line_{ix}() {{}} // {name}\n"))
            .collect();
        std::fs::write(fx.path(name), lines).unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.select("a.rs");
    ui.press("space");
    let text_top = |ui: &mut Ui| ui.snapshot("preview-text").bounds().origin;

    let top = ui.in_quick_look(|ui| text_top(ui));
    ui.in_quick_look(|ui| ui.press("down"));
    assert_eq!(ui.selected(), ["b.rs"]);
    ui.in_quick_look(|ui| {
        ui.scroll_point(top + point(px(20.), px(20.)), point(px(0.), px(-300.)));
        let scrolled = text_top(ui);
        assert!(scrolled.y < top.y - px(100.), "b.rs scrolled");
        // Bigger text is still b.rs, so it stays where it was being read.
        ui.click("ql-text-bigger");
        assert_eq!(text_top(ui), scrolled, "b.rs kept its place");
        ui.press("up");
    });
    assert_eq!(ui.selected(), ["a.rs"]);
    let back = ui.in_quick_look(|ui| text_top(ui));
    assert_eq!(back, top, "a.rs is not where b.rs was left");
}

/// Files for a contents search for `needle-zebra`: it's far down `log.txt`
/// and `main.rs`, in the second line of `near.txt`, and past what a preview
/// reads of `long.txt`.
fn files_with_a_match(name: &str) -> Fixture {
    let fx = Fixture::new(name);
    let lines = |count: usize, at: usize, rest: &str| -> String {
        (1..=count)
            .map(|ix| match ix == at {
                true => format!("line {ix} needle-zebra\n"),
                false => format!("line {ix}{rest}\n"),
            })
            .collect()
    };
    std::fs::write(fx.path("log.txt"), lines(400, 300, "")).unwrap();
    std::fs::write(fx.path("near.txt"), lines(400, 2, "")).unwrap();
    let filler = " of a file that goes on for longer than a preview reads";
    std::fs::write(fx.path("long.txt"), lines(3000, 2500, filler)).unwrap();
    let code: String = (1..=400)
        .map(|ix| match ix {
            250 => "// needle-zebra\n".to_string(),
            _ => format!("fn line_{ix}() {{}}\n"),
        })
        .collect();
    std::fs::write(fx.path("main.rs"), code).unwrap();
    fx
}

/// The line marked in the selected item's preview.
fn marked_line(ui: &Ui) -> Option<String> {
    let path = ui.read(|f| f.selected_entries()[0].path.clone());
    ui.read(|f| f.preview_match_for_test(&path))
}

#[gpui_kit::test]
fn a_preview_opens_at_the_line_a_contents_search_found(cx: &mut TestAppContext) {
    let fx = files_with_a_match("pane-match");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    ui.click("search");
    ui.type_text("needle-zebra");
    ui.click("by-contents");
    assert_eq!(ui.names().len(), 4, "{:?}", ui.names());
    // How far below the top of the preview pane an element is.
    let below_top = |ui: &mut Ui, id: &'static str| {
        ui.snapshot(id).bounds().top() - ui.snapshot("preview-pane").bounds().top()
    };

    ui.select("log.txt");
    assert_eq!(marked_line(&ui).as_deref(), Some("line 300 needle-zebra"));
    let top = below_top(&mut ui, "preview-match");
    assert!(
        ui.snapshot("preview-match").visible() && top >= px(0.) && top < px(40.),
        "the lines around the match are at the top of the preview: {top:?}"
    );

    // A search that runs again leaves the preview where it's being read.
    let pane = ui.snapshot("preview-pane").bounds();
    let inside = point(pane.center().x, pane.top() + px(100.));
    ui.scroll_point(inside, point(px(0.), px(-150.)));
    let read_at = below_top(&mut ui, "preview-match");
    assert!(read_at < top - px(100.), "scrolled on from the match");
    ui.update(|f, _, cx| f.run_search(false, cx));
    assert_eq!(below_top(&mut ui, "preview-match"), read_at);

    // A match in a file's first lines shows without scrolling.
    ui.select("near.txt");
    assert_eq!(marked_line(&ui).as_deref(), Some("line 2 needle-zebra"));
    assert!(!ui.exists("preview-match"));
    let unscrolled = below_top(&mut ui, "preview-text");
    assert!(unscrolled > px(0.), "{unscrolled:?}");

    // Code opens at its match too, and so does a file gone back to.
    ui.select("main.rs");
    assert_eq!(marked_line(&ui).as_deref(), Some("// needle-zebra"));
    assert_eq!(below_top(&mut ui, "preview-match"), top);
    ui.select("log.txt");
    assert_eq!(
        below_top(&mut ui, "preview-match"),
        top,
        "at the match, not where it was left"
    );

    // A match past what the preview read can't be shown. The preview opens
    // at the note that says where it is.
    ui.select("long.txt");
    assert_eq!(marked_line(&ui), None);
    assert!(!ui.exists("preview-match"));
    let note = ui.snapshot("preview-note");
    assert_eq!(
        note.label(),
        Some("… (preview truncated; the match on line 2500 is further down)")
    );
    assert!(note.visible(), "the note is in view: {note:?}");

    // Without the search, the file opens at its top with nothing marked.
    ui.press("escape");
    ui.press("escape");
    assert!(!ui.read(|f| f.is_searching()));
    ui.select("log.txt");
    assert_eq!(marked_line(&ui), None);
    assert!(!ui.exists("preview-match"));
    assert_eq!(below_top(&mut ui, "preview-text"), unscrolled);
}

#[gpui_kit::test]
fn a_preview_goes_to_another_line_the_search_finds_in_the_same_file(cx: &mut TestAppContext) {
    let fx = Fixture::new("gallery-match");
    let lines: String = (1..=400)
        .map(|ix| match ix {
            100 => "line 100 needle-quokka\n".to_string(),
            300 => "line 300 needle-quokka-two\n".to_string(),
            _ => format!("line {ix}\n"),
        })
        .collect();
    std::fs::write(fx.path("log.txt"), lines).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-gallery");
    ui.click("search");
    ui.type_text("needle-quokka");
    ui.click("by-contents");
    // The gallery shows the one result without it being clicked.
    assert_eq!(ui.selected(), ["log.txt"]);
    assert_eq!(marked_line(&ui).as_deref(), Some("line 100 needle-quokka"));
    let top = ui.snapshot("preview-match").bounds().top();
    let above = ui.snapshot("preview-text").bounds().size.height;

    // Cmd+F selects the words, so these take their place.
    ui.press(&cmd("f"));
    ui.type_text("needle-quokka-two");
    assert_eq!(ui.selected(), ["log.txt"]);
    assert_eq!(
        marked_line(&ui).as_deref(),
        Some("line 300 needle-quokka-two")
    );
    assert_eq!(ui.snapshot("preview-match").bounds().top(), top);
    assert!(
        ui.snapshot("preview-text").bounds().size.height > above * 2.,
        "scrolled on to the other line"
    );
}

#[gpui_kit::test]
fn a_preview_marks_no_line_when_the_file_changed_after_the_search(cx: &mut TestAppContext) {
    let fx = files_with_a_match("changed-match");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    ui.click("search");
    ui.type_text("needle-zebra");
    ui.click("by-contents");
    let text_top = |ui: &mut Ui| ui.snapshot("preview-text").bounds().top();
    ui.select("near.txt");
    let unscrolled = text_top(&mut ui);

    // Ten lines more above the match, written once the search has read the
    // file. Its row goes on saying line 300, which is "line 290" now.
    let found = std::fs::read_to_string(fx.path("log.txt")).unwrap();
    let changed = format!("{}{found}", "one more\n".repeat(10));
    std::fs::write(fx.path("log.txt"), changed).unwrap();
    ui.select("log.txt");
    assert!(ui.read(|f| f.has_preview_for_test(&fx.path("log.txt"))));
    assert_eq!(marked_line(&ui), None, "line 300 is another line now");
    assert!(!ui.exists("preview-match"));
    assert_eq!(text_top(&mut ui), unscrolled, "the file opens at its top");
}

#[gpui_kit::test]
fn a_preview_follows_a_changed_file_once_its_search_has_run_again(cx: &mut TestAppContext) {
    let fx = files_with_a_match("rerun-match");
    let near = fx.path("near.txt");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    ui.click("search");
    ui.type_text("needle-zebra");
    ui.click("by-contents");
    ui.select("near.txt");
    assert_eq!(marked_line(&ui).as_deref(), Some("line 2 needle-zebra"));

    // A line more above the match, so the search finds it on line 3 next
    // time, where the preview as it was read before has another line.
    let found = std::fs::read_to_string(&near).unwrap();
    std::fs::write(&near, format!("one more\n{found}")).unwrap();
    ui.update(|f, _, cx| f.run_search(false, cx));

    assert_eq!(ui.selected(), ["near.txt"]);
    let text = ui.read(|f| f.preview_text_for_test(&near)).unwrap();
    assert!(text.starts_with("one more\nline 1\n"), "read again");
    assert_eq!(marked_line(&ui).as_deref(), Some("line 2 needle-zebra"));
}

#[gpui_kit::test]
fn quick_look_opens_at_the_line_a_contents_search_found(cx: &mut TestAppContext) {
    let fx = files_with_a_match("quick-look-match");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.click("search");
    ui.type_text("needle-zebra");
    ui.click("by-contents");

    // Opened on a result, and following the browser to another.
    ui.select("main.rs");
    ui.press("space");
    for name in ["main.rs", "log.txt"] {
        ui.select(name);
        let around = ui.in_quick_look(|ui| ui.snapshot("preview-match"));
        let top = around.bounds().top();
        assert!(
            around.visible() && top >= px(0.) && top < px(40.),
            "the lines around {name}'s match are at the top of Quick Look: {top:?}"
        );
    }

    // Bigger text is still the same file, so it stays where it's being read.
    ui.in_quick_look(|ui| {
        let text_top = |ui: &mut Ui| ui.snapshot("preview-text").bounds().top();
        ui.scroll_point(point(px(200.), px(100.)), point(px(0.), px(-150.)));
        let read_at = text_top(ui);
        ui.click("ql-text-bigger");
        assert_eq!(text_top(ui), read_at, "log.txt kept its place");
    });

    ui.select("long.txt");
    ui.in_quick_look(|ui| {
        assert!(!ui.exists("preview-match"));
        let note = ui.snapshot("preview-note");
        assert_eq!(
            note.label(),
            Some("… (preview truncated; the match on line 2500 is further down)")
        );
        assert!(note.visible(), "the note is in view: {note:?}");
    });
}

/// A preview that opens at a line draws its area empty for one frame, so it
/// has to ask for the next frame itself. In Quick Look's window nothing else
/// does, and the text would wait there for the next event.
#[gpui_kit::test]
fn a_preview_that_opens_at_a_line_shows_its_text_in_the_next_frame(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt as _;
    let fx = files_with_a_match("quick-look-frame");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.click("search");
    ui.type_text("needle-zebra");
    ui.click("by-contents");
    // Both files are shown once, so that log.txt's preview is read by the
    // time it's clicked again.
    ui.select("log.txt");
    ui.press("space");
    let top = ui.in_quick_look(|ui| ui.snapshot("preview-match").bounds().top());
    ui.select("near.txt");

    // The click `Ui::select` makes, without the frame the harness draws
    // after every step: Quick Look's window has drawn once, as the browser's
    // selection changed.
    let names = ui.names();
    let ix = names.iter().position(|name| name == "log.txt").unwrap();
    let view = ui.view.clone();
    ui.window(|window, cx| {
        view.update(cx, |f, cx| {
            f.click_row(ix, Default::default(), 1, window, cx)
        })
    });
    let quick_look = ui.read(|f| f.quick_look_window());
    let browser = std::mem::replace(&mut ui.handle, quick_look.expect("Quick Look is open"));
    let around = |ui: &mut Ui| {
        ui.window(|window, _| window.try_find("preview-match").map(|m| m.bounds().top()))
    };
    assert_eq!(around(&mut ui), None, "the first frame has no text yet");
    // What the platform does when the window's next frame is due.
    ui.window(|window, cx| window.simulate_next_frame(cx));
    assert_eq!(around(&mut ui), Some(top), "the text is at the match");
    ui.handle = browser;
}

#[gpui_kit::test]
fn a_table_preview_starts_at_its_left_edge_however_far_the_last_one_was_scrolled(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("pane-table-scroll");
    for name in ["a.zip", "b.zip"] {
        let progress = fffinder_core::ops::Progress::default();
        fffinder_core::archive::compress(&[fx.path("Projects")], &fx.path(name), &progress)
            .unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    // As narrow as it goes, so that a table is wider than the pane.
    ui.drag_by("resize-preview", 200.);
    let table_left = |ui: &mut Ui| ui.snapshot("preview-table").bounds().origin;

    ui.select("a.zip");
    let left = table_left(&mut ui);
    ui.select("b.zip");
    assert_eq!(table_left(&mut ui), left);

    ui.scroll_point(left + point(px(20.), px(20.)), point(px(-100.), px(0.)));
    assert!(table_left(&mut ui).x < left.x - px(50.), "b.zip scrolled");

    ui.select("a.zip");
    assert_eq!(
        table_left(&mut ui),
        left,
        "a.zip is not where b.zip was left"
    );
}

/// Zip archives of a folder of 150 files, each of them listed as a table of
/// more rows than a preview has room for.
fn long_archives(fx: &Fixture, names: &[&str]) {
    std::fs::create_dir_all(fx.path("Many")).unwrap();
    for ix in 0..150 {
        std::fs::write(fx.path(&format!("Many/file {ix:03}.txt")), "x").unwrap();
    }
    for name in names {
        let progress = fffinder_core::ops::Progress::default();
        fffinder_core::archive::compress(&[fx.path("Many")], &fx.path(name), &progress).unwrap();
    }
}

/// The room between the last row of the table preview and the line under
/// the table, the one with the count: none when the table is scrolled to
/// its end, and less than none while rows are out of view below.
fn room_under_the_rows(ui: &mut Ui) -> Pixels {
    let rows_end = ui.snapshot("preview-table").bounds().bottom();
    ui.snapshot("preview-table-footer").bounds().top() - rows_end
}

#[gpui_kit::test]
fn a_table_preview_scrolls_down_to_its_last_row(cx: &mut TestAppContext) {
    let fx = Fixture::new("pane-table-rows");
    long_archives(&fx, &["a.zip", "b.zip"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    let table_top = |ui: &mut Ui| ui.snapshot("preview-table").bounds().origin;

    ui.select("a.zip");
    let top = table_top(&mut ui);
    assert!(
        room_under_the_rows(&mut ui) < px(-1000.),
        "most rows don't fit"
    );
    ui.select("b.zip");
    assert_eq!(table_top(&mut ui), top);

    let over_rows = top + point(px(20.), px(40.));
    ui.scroll_point(over_rows, point(px(0.), px(-100.)));
    assert_eq!(
        table_top(&mut ui),
        top - point(px(0.), px(100.)),
        "b.zip scrolled down"
    );
    // Much further than it has rows: it stops with the last one in view.
    ui.scroll_point(over_rows, point(px(0.), px(-100_000.)));
    let room = room_under_the_rows(&mut ui);
    assert!(
        room.abs() < px(1.),
        "the last row ends {room:?} above the count"
    );

    ui.select("a.zip");
    assert_eq!(table_top(&mut ui), top, "a.zip is not where b.zip was left");
}

#[gpui_kit::test]
fn a_long_table_wider_than_the_pane_scrolls_both_ways(cx: &mut TestAppContext) {
    let fx = Fixture::new("pane-table-both-ways");
    long_archives(&fx, &["a.zip"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    // As narrow as it goes, so that the table is wider than the pane too.
    ui.drag_by("resize-preview", 200.);
    ui.select("a.zip");
    let table_top = |ui: &mut Ui| ui.snapshot("preview-table").bounds().origin;
    let top = table_top(&mut ui);
    let over_rows = top + point(px(20.), px(40.));

    ui.scroll_point(over_rows, point(px(0.), px(-100.)));
    ui.scroll_point(over_rows, point(px(-40.), px(0.)));
    assert_eq!(
        table_top(&mut ui),
        top - point(px(40.), px(100.)),
        "down, then sideways"
    );
    ui.scroll_point(over_rows, point(px(0.), px(-100.)));
    assert_eq!(
        table_top(&mut ui),
        top - point(px(40.), px(200.)),
        "and further down from there"
    );
}

#[gpui_kit::test]
fn a_table_scrolls_down_in_the_gallery_and_in_quick_look(cx: &mut TestAppContext) {
    let fx = Fixture::new("gallery-table-rows");
    long_archives(&fx, &["a.zip"]);
    let mut ui = Ui::open(cx, &fx.dir);
    let table_top = |ui: &mut Ui| ui.snapshot("preview-table").bounds().origin;
    // Turns the wheel over the table's first rows, and says where that left it.
    let scroll_down = |ui: &mut Ui, what: &str| {
        let top = table_top(ui);
        let over_rows = top + point(px(20.), px(40.));
        ui.scroll_point(over_rows, point(px(0.), px(-100.)));
        let scrolled = table_top(ui);
        assert_eq!(scrolled, top - point(px(0.), px(100.)), "{what} scrolled");
        (over_rows, scrolled)
    };
    let scroll_to_the_end = |ui: &mut Ui, over_rows, what: &str| {
        ui.scroll_point(over_rows, point(px(0.), px(-100_000.)));
        let room = room_under_the_rows(ui);
        assert!(
            room.abs() < px(1.),
            "{what}: the last row ends {room:?} above the count"
        );
    };

    ui.click("view-gallery");
    ui.select("a.zip");
    let (over_rows, _) = scroll_down(&mut ui, "the gallery");
    scroll_to_the_end(&mut ui, over_rows, "the gallery");

    ui.click("view-list");
    ui.select("a.zip");
    ui.press("space");
    ui.in_quick_look(|ui| {
        let (over_rows, scrolled) = scroll_down(ui, "Quick Look");
        // Bigger text is still a.zip, so it stays where it was being read.
        ui.click("ql-text-bigger");
        assert_eq!(table_top(ui), scrolled, "a.zip kept its place");
        scroll_to_the_end(ui, over_rows, "Quick Look");
    });
}

/// The column view scrolls sideways for a wheel that nothing in it takes, so
/// a table in its card takes the wheel while rows of it are out of view, at
/// its end too, and leaves it alone when all of them show.
#[gpui_kit::test]
fn a_table_in_the_column_views_card_takes_the_wheel_only_when_it_scrolls(cx: &mut TestAppContext) {
    let fx = Fixture::new("card-table-rows");
    long_archives(&fx, &["long.zip"]);
    let progress = fffinder_core::ops::Progress::default();
    fffinder_core::archive::compress(&[fx.path("Projects")], &fx.path("short.zip"), &progress)
        .unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    // Too narrow for the column and the card beside it.
    ui.resize(560., 600.);
    let table_top = |ui: &mut Ui| ui.snapshot("preview-table").bounds().origin;
    let column_left = |ui: &mut Ui| ui.snapshot(("column-pane", 0usize)).bounds().left();

    ui.click(item_id("crow", &fx.path("long.zip")));
    let (top, left) = (table_top(&mut ui), column_left(&mut ui));
    let over_rows = top + point(px(20.), px(40.));
    ui.scroll_point(over_rows, point(px(0.), px(-60.)));
    assert_eq!(
        table_top(&mut ui),
        top - point(px(0.), px(60.)),
        "the table scrolled down"
    );
    assert_eq!(column_left(&mut ui), left, "and the columns stayed");
    // Much further than it has rows: it stops with the last one in view, and
    // what is left of the wheel's turn isn't handed on to the columns.
    ui.scroll_point(over_rows, point(px(0.), px(-100_000.)));
    let room = room_under_the_rows(&mut ui);
    assert!(
        room.abs() < px(1.),
        "the last row ends {room:?} above the count"
    );
    assert_eq!(column_left(&mut ui), left, "the columns stayed there too");

    ui.click(item_id("crow", &fx.path("short.zip")));
    let top = table_top(&mut ui);
    assert!(room_under_the_rows(&mut ui) > px(0.), "every row shows");
    ui.scroll_point(top + point(px(20.), px(40.)), point(px(0.), px(-60.)));
    assert_eq!(table_top(&mut ui).y, top.y);
    assert!(column_left(&mut ui) < left, "the columns scrolled instead");
}

#[gpui_kit::test]
fn the_preview_pane_resizes_from_its_left_edge(cx: &mut TestAppContext) {
    let fx = Fixture::new("pane-width");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("shift-p"));
    let width = |ui: &Ui| ui.read(|f| f.config().preview_pane_width);
    let start = width(&ui);

    ui.drag_by("resize-preview", -40.);
    assert_eq!(width(&ui), start + 40., "dragging left widens it");

    ui.double_click("resize-preview");
    assert_eq!(width(&ui), start);
}

// ----- the status bar ---------------------------------------------------------------------

#[gpui_kit::test]
fn the_status_bar_adds_up_the_selection(cx: &mut TestAppContext) {
    let fx = Fixture::new("status-size");
    std::fs::write(fx.path("one.txt"), vec![b'x'; 2000]).unwrap();
    std::fs::write(fx.path("two.txt"), vec![b'x'; 3000]).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");

    ui.select("one.txt");
    ui.select_also("two.txt");

    let status = ui.snapshot("status-text");
    let text = status.label().unwrap();
    assert!(text.contains("2 of"), "{text}");
    assert!(text.contains("(5.0 KB)"), "{text}");
}

#[gpui_kit::test]
fn the_status_bar_keeps_up_with_the_selection_and_the_files(cx: &mut TestAppContext) {
    let fx = Fixture::new("status-fresh");
    std::fs::write(fx.path("one.txt"), vec![b'x'; 2000]).unwrap();
    std::fs::write(fx.path("two.txt"), vec![b'x'; 3000]).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let status = |ui: &mut Ui| ui.snapshot("status-text").label().unwrap().to_string();

    ui.select("one.txt");
    assert!(status(&mut ui).contains("(2.0 KB)"));
    ui.select("two.txt");
    assert!(status(&mut ui).contains("(3.0 KB)"), "another item");
    ui.select_also("one.txt");
    assert!(status(&mut ui).contains("(5.0 KB)"), "one more");

    std::fs::write(fx.path("one.txt"), vec![b'x'; 4000]).unwrap();
    ui.update(|f, _, cx| f.reload_quietly(cx));
    assert!(
        status(&mut ui).contains("(7.0 KB)"),
        "the same items, bigger"
    );
}

// ----- keys ----------------------------------------------------------------------------

#[gpui_kit::test]
fn home_and_end_reach_the_ends_of_a_column_with_nothing_selected(cx: &mut TestAppContext) {
    let fx = Fixture::new("column-ends");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.settle();
    let names = ui.names();
    assert!(ui.selected().is_empty());

    ui.press("end");
    assert_eq!(ui.selected(), [names[names.len() - 1].as_str()]);

    ui.press("home");
    assert_eq!(ui.selected(), [names[0].as_str()]);
}

#[gpui_kit::test]
fn up_with_nothing_selected_starts_from_the_end_of_a_column(cx: &mut TestAppContext) {
    let fx = Fixture::new("column-up");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.settle();
    let names = ui.names();

    ui.press("up");

    assert_eq!(ui.selected(), [names[names.len() - 1].as_str()]);
}

#[gpui_kit::test]
fn page_up_and_down_move_through_a_long_list(cx: &mut TestAppContext) {
    let fx = long_folder("paging");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let cursor = |ui: &Ui| ui.read(|f| f.selected_entries().first().map(|e| e.name.clone()));

    ui.press("home");
    let first = cursor(&ui);
    ui.press("pagedown");
    let paged = cursor(&ui);
    assert_ne!(paged, first);
    let row = ui.read(|f| {
        f.rows_for_test()
            .iter()
            .position(|r| Some(&r.entry.name) == paged.as_ref())
    });
    assert!(row.unwrap() > 5, "a page is more than a few rows: {row:?}");

    ui.press("pageup");
    assert_eq!(cursor(&ui), first);

    // Shift extends, and so it does for Home and End.
    ui.press("shift-pagedown");
    assert!(ui.selected().len() > 5);
    ui.press("home");
    ui.press("shift-end");
    assert_eq!(ui.selected().len(), ui.names().len());
    ui.press("end");
    assert_eq!(ui.selected().len(), 1);
    ui.press("shift-home");
    assert_eq!(ui.selected().len(), ui.names().len());
}

#[gpui_kit::test]
fn f5_refreshes_without_split_view(cx: &mut TestAppContext) {
    let fx = Fixture::new("f5");
    let mut ui = Ui::open(cx, &fx.dir);
    std::fs::write(fx.path("arrived.txt"), "x").unwrap();
    assert!(!ui.names().contains(&"arrived.txt".to_string()));

    ui.press("f5");

    assert!(ui.names().contains(&"arrived.txt".to_string()));
}

#[gpui_kit::test]
fn the_menu_still_warns_without_a_second_pane(cx: &mut TestAppContext) {
    let fx = Fixture::new("f5-menu");
    let mut ui = with_tabs(cx, &fx.dir);
    ui.select("notes.txt");
    std::fs::write(fx.path("arrived.txt"), "x").unwrap();

    ui.dispatch(fffinder::actions::CopyToOtherPane);

    assert!(
        !ui.names().contains(&"arrived.txt".to_string()),
        "the menu item doesn't refresh"
    );
}

#[gpui_kit::test]
fn f5_refreshes_outside_the_file_views_on_a_mac(cx: &mut TestAppContext) {
    // Outside the file views a Mac's menu bar would run Copy to Other Pane
    // for the key, so a Mac has it refresh anywhere in the window. On other
    // systems F5 is a key of the file views alone.
    if !cfg!(target_os = "macos") {
        return;
    }
    let fx = Fixture::new("f5-search-field");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    assert!(!ui.files_focused(), "the keyboard is in the search field");
    std::fs::write(fx.path("arrived.txt"), "x").unwrap();
    assert!(!ui.names().contains(&"arrived.txt".to_string()));

    ui.press("f5");

    assert!(ui.names().contains(&"arrived.txt".to_string()));
}

// ----- where a search starts, and where it leaves you ------------------------------------

/// The first item the window lists.
fn first_item(ui: &Ui) -> std::path::PathBuf {
    ui.read(|f| f.rows_for_test()[0].entry.path.clone())
}

/// The names of the items drawn as `kind`: `row` in the list, `tile` in the
/// icon view, `strip` in the gallery. Each draws only what's on screen.
fn on_screen(ui: &mut Ui, kind: &'static str) -> Vec<String> {
    let items: Vec<(String, std::path::PathBuf)> = ui.read(|f| {
        f.rows_for_test()
            .iter()
            .map(|r| (r.entry.name.clone(), r.entry.path.clone()))
            .collect()
    });
    ui.window(|window, cx| {
        use gpui_kit::test::TestWindowExt as _;
        window.render_frame(cx);
        items
            .into_iter()
            .filter(|(_, path)| window.try_find(item_id(kind, path)).is_some())
            .map(|(name, _)| name)
            .collect()
    })
}

#[gpui_kit::test]
fn new_results_start_at_the_top_of_the_list(cx: &mut TestAppContext) {
    let fx = long_folder("results-top-list");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press("end");
    assert!(
        !ui.exists(item_id("row", &fx.path("Budget.csv"))),
        "the folder is scrolled to its end"
    );

    ui.click("search");
    ui.type_text("item");
    assert!(ui.names().len() >= 200, "{}", ui.names().len());
    assert!(
        ui.exists(item_id("row", &first_item(&ui))),
        "the best match is on screen, not where the folder was scrolled to"
    );

    // And when the words change under results that were scrolled.
    ui.scroll("list-columns", point(px(0.), px(-3000.)));
    assert!(!ui.exists(item_id("row", &first_item(&ui))));
    ui.type_text("txt");
    assert_eq!(ui.read(|f| f.query().to_string()), "itemtxt");
    assert!(ui.names().len() >= 200, "{}", ui.names().len());
    assert!(ui.exists(item_id("row", &first_item(&ui))));
}

#[gpui_kit::test]
fn new_results_start_at_the_top_of_the_icon_view(cx: &mut TestAppContext) {
    let fx = long_folder("results-top-icons");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-icons");
    ui.press("end");
    assert!(
        !ui.exists(item_id("tile", &fx.path("Budget.csv"))),
        "the folder is scrolled to its end"
    );

    ui.click("search");
    ui.type_text("item");
    assert!(ui.names().len() >= 200, "{}", ui.names().len());
    assert!(ui.exists(item_id("tile", &first_item(&ui))));
}

#[gpui_kit::test]
fn new_results_start_at_the_top_in_column_view_too(cx: &mut TestAppContext) {
    let fx = long_folder("results-top-columns");
    std::fs::create_dir(fx.path("Zebra")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    // Going back selects the folder that was left, the last of this one.
    ui.go_to(&fx.path("Zebra"));
    ui.press(&cmd("["));
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));

    // Column view lists its results.
    ui.click("search");
    ui.type_text("item");
    assert!(ui.names().len() >= 200, "{}", ui.names().len());
    assert!(ui.exists(item_id("row", &first_item(&ui))));
}

#[gpui_kit::test]
fn other_words_let_go_of_the_result_that_was_selected(cx: &mut TestAppContext) {
    let fx = long_folder("results-selection");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.click("search");
    ui.type_text("item");
    ui.press("down");
    ui.press("end");
    let last = ui.selected();
    assert_eq!(last.len(), 1);
    assert_ne!(last[0], ui.names()[0]);

    // Every item still matches, the selected one among them.
    ui.click("search");
    ui.type_text("txt");
    assert!(ui.names().contains(&last[0]), "{:?}", ui.names());
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());

    // So Down Arrow goes to the best match, not on from one out of view.
    ui.press("down");
    assert!(ui.files_focused());
    assert_eq!(ui.selected(), [ui.names()[0].clone()]);
    assert!(ui.exists(item_id("row", &first_item(&ui))));
}

#[gpui_kit::test]
fn return_after_other_words_selects_their_first_result(cx: &mut TestAppContext) {
    let fx = long_folder("results-selection-return");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.click("search");
    ui.type_text("item");
    ui.press("down");
    ui.press("end");
    assert_ne!(ui.selected(), [ui.names()[0].clone()]);

    // Return comes before the results for the words as they are now.
    ui.click("search");
    ui.type_without_settling("txt");
    ui.press_without_settling("enter");
    ui.settle();

    assert!(ui.files_focused());
    assert_eq!(ui.selected(), [ui.names()[0].clone()]);
    assert!(ui.exists(item_id("row", &first_item(&ui))));
}

#[gpui_kit::test]
fn a_change_of_scope_kind_or_contents_starts_at_the_top_too(cx: &mut TestAppContext) {
    let fx = long_folder("results-top-controls");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    // Every item has an x in its name, and another inside it.
    ui.click("search");
    ui.type_text("x");
    let select_the_last = |ui: &mut Ui| {
        ui.click("search");
        ui.press("down");
        ui.press("end");
        assert_eq!(ui.selected().len(), 1);
        assert!(
            !ui.exists(item_id("row", &first_item(ui))),
            "the first result is out of view"
        );
    };
    let at_the_top = |ui: &mut Ui, changed: &str| {
        assert!(ui.names().len() >= 200, "{changed}: {}", ui.names().len());
        assert!(ui.selected().is_empty(), "{changed}: {:?}", ui.selected());
        assert!(ui.exists(item_id("row", &first_item(ui))), "{changed}");
    };

    select_the_last(&mut ui);
    ui.click("scope-folder");
    at_the_top(&mut ui, "the scope");

    select_the_last(&mut ui);
    ui.click("kind");
    ui.click_menu_item("Document");
    at_the_top(&mut ui, "the kind");

    select_the_last(&mut ui);
    ui.click("by-contents");
    at_the_top(&mut ui, "name or contents");
}

#[gpui_kit::test]
fn a_search_that_runs_again_keeps_its_place(cx: &mut TestAppContext) {
    let fx = long_folder("results-again");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.click("search");
    ui.type_text("item");
    ui.press("down");
    ui.press("end");
    let last = ui.selected();
    let shown = on_screen(&mut ui, "row");
    assert!(shown.contains(&last[0]), "{last:?} in {shown:?}");
    assert!(!ui.exists(item_id("row", &first_item(&ui))));

    // Refresh asks for the same results again.
    ui.press(&cmd("r"));

    assert_eq!(ui.selected(), last);
    assert_eq!(on_screen(&mut ui, "row"), shown);
}

#[gpui_kit::test]
fn ending_a_search_goes_back_to_the_folders_place(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-list");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    // The last item is selected, and the list is scrolled back up from it.
    ui.press("end");
    ui.scroll("list-columns", point(px(0.), px(1200.)));
    let shown = on_screen(&mut ui, "row");
    for end in ["Budget.csv", "Roadmap.md"] {
        assert!(!shown.contains(&end.to_string()), "{shown:?}");
    }

    ui.click("search");
    ui.type_text("item");
    assert!(ui.exists(item_id("row", &first_item(&ui))));
    ui.press("escape");

    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert_eq!(on_screen(&mut ui, "row"), shown);

    // And from a search that found nothing, so never drew a list.
    ui.type_text("qqzzqqzz");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    ui.press("escape");

    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert_eq!(on_screen(&mut ui, "row"), shown);
}

#[gpui_kit::test]
fn ending_a_search_goes_back_to_the_folders_place_in_the_icon_view(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-icons");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-icons");
    ui.press("end");
    ui.scroll("content", point(px(0.), px(700.)));
    let shown = on_screen(&mut ui, "tile");
    for end in ["Budget.csv", "Roadmap.md"] {
        assert!(!shown.contains(&end.to_string()), "{shown:?}");
    }

    ui.click("search");
    ui.type_text("item");
    assert!(ui.exists(item_id("tile", &first_item(&ui))));
    // From the results, Escape lets go of the selection, then ends the search.
    ui.press("down");
    assert!(ui.files_focused());
    ui.press("escape");
    ui.press("escape");

    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert_eq!(on_screen(&mut ui, "tile"), shown);
}

#[gpui_kit::test]
fn ending_a_search_in_the_gallery_shows_the_item_it_was_on(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-gallery");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-gallery");
    ui.press("end");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert!(ui.exists(item_id("strip", &fx.path("Roadmap.md"))));

    ui.click("search");
    ui.type_text("budget");
    assert_eq!(ui.selected(), ["Budget.csv"], "the gallery shows a result");
    ui.press("escape");

    assert_eq!(ui.selected(), ["Roadmap.md"]);
    let strip = on_screen(&mut ui, "strip");
    assert!(strip.contains(&"Roadmap.md".to_string()), "{strip:?}");
    assert!(!strip.contains(&"Budget.csv".to_string()), "{strip:?}");
}

#[gpui_kit::test]
fn a_gallery_chosen_during_a_search_still_shows_an_item(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-gallery-none");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    assert!(ui.selected().is_empty());
    ui.click("search");
    ui.type_text("item");

    // Nothing was selected in the folder, so there's nothing to select again.
    ui.click("view-gallery");
    ui.click("search");
    ui.press("escape");

    assert_eq!(ui.selected(), ["Budget.csv"], "the folder's first item");
}

#[gpui_kit::test]
fn ending_a_search_goes_back_to_where_recents_was_scrolled_to(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-recents");
    let mut ui = Ui::open(cx, &fx.dir);
    // Column view lists Recents, as it lists results.
    ui.click("view-columns");
    ui.click("recents");
    // The first item is selected, and scrolled out of view.
    let first = ui.names()[0].clone();
    ui.select(&first);
    ui.scroll("list-columns", point(px(0.), px(-1200.)));
    let shown = on_screen(&mut ui, "row");
    assert!(shown.len() > 5, "{shown:?}");
    assert!(!shown.contains(&first), "{shown:?}");

    ui.click("search");
    ui.type_text("item");
    assert!(ui.exists(item_id("row", &first_item(&ui))));
    ui.press("escape");

    // This is the view the search began in, so nothing scrolls to the item.
    assert_eq!(ui.selected(), [first]);
    assert_eq!(on_screen(&mut ui, "row"), shown);
}

#[gpui_kit::test]
fn a_view_chosen_during_a_search_shows_what_was_selected(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-other-view");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press("end");
    ui.click("search");
    ui.type_text("item");

    // The list's place means nothing to the icons.
    ui.click("view-icons");
    ui.click("search");
    ui.press("escape");

    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert!(ui.exists(item_id("tile", &fx.path("Roadmap.md"))));
}

#[gpui_kit::test]
fn a_view_chosen_during_a_search_isnt_left_where_the_results_were(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-other-view-top");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-icons");
    assert!(ui.selected().is_empty());
    ui.click("search");
    ui.type_text("item");

    // The list is first drawn for the results, and scrolled through them.
    ui.click("view-list");
    ui.scroll("list-columns", point(px(0.), px(-3000.)));
    assert!(!ui.exists(item_id("row", &first_item(&ui))));
    ui.click("search");
    ui.press("escape");

    // Nothing is selected to scroll to, and the folder's list was at its top.
    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
    assert!(ui.exists(item_id("row", &fx.path("Budget.csv"))));
}

#[gpui_kit::test]
fn a_search_in_the_gallery_leaves_the_list_where_it_was(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-gallery-list");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.scroll("list-columns", point(px(0.), px(-1200.)));
    let shown = on_screen(&mut ui, "row");
    let picked = shown[5].clone();
    ui.select(&picked);

    // Results start at the top of the list too, though it isn't showing.
    ui.click("view-gallery");
    ui.click("search");
    ui.type_text("budget");
    ui.press("escape");
    assert_eq!(ui.selected(), [picked]);

    ui.click("view-list");
    assert_eq!(on_screen(&mut ui, "row"), shown);
}

#[gpui_kit::test]
fn going_to_another_folder_from_a_search_starts_at_its_top(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-leave");
    for ix in 0..200 {
        std::fs::write(fx.path(&format!("Projects/page {ix:03}.txt")), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press("end");
    ui.click("search");
    ui.type_text("item");

    ui.go_to(&fx.path("Projects"));

    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
    assert!(ui.exists(item_id("row", &fx.path("Projects/atlas"))));
}

#[gpui_kit::test]
fn back_to_a_search_scrolls_to_the_folder_it_came_from(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-place-back");
    for ix in 0..120 {
        std::fs::create_dir(fx.path(&format!("box {ix:03}"))).unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.click("search");
    ui.type_text("box");
    assert!(ui.names().len() >= 120, "{}", ui.names().len());

    // The last result is a folder far below the ones on screen.
    ui.press("down");
    ui.press("end");
    let last = ui.selected();
    assert!(!ui.exists(item_id("row", &first_item(&ui))));
    ui.press("enter");
    assert_eq!(ui.folder(), Some(fx.path(&last[0])));

    // The results come back from their top, and then show that folder.
    ui.press(&cmd("["));
    assert_eq!(ui.read(|f| f.query().to_string()), "box");
    assert_eq!(ui.selected(), last);
    let shown = on_screen(&mut ui, "row");
    assert!(shown.contains(&last[0]), "{last:?} in {shown:?}");
}

#[gpui_kit::test]
fn a_folder_renamed_during_a_search_is_shown_from_its_top(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-renamed");
    for ix in 0..200 {
        std::fs::write(fx.path(&format!("Projects/page {ix:03}.txt")), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.click("view-list");
    ui.press("end");
    assert!(!ui.exists(item_id("row", &fx.path("Projects/atlas"))));

    // The folder the window is in is one of the results.
    ui.click("search");
    ui.type_text("projects");
    retype_name(&mut ui, "Projects", "Plans");
    ui.press("enter");
    assert_eq!(ui.folder(), Some(fx.path("Plans")));
    assert_eq!(ui.read(|f| f.query().to_string()), "projects");

    // Escape lets go of the selection first, if there is one.
    ui.press("escape");
    ui.press("escape");

    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
    assert!(ui.exists(item_id("row", &fx.path("Plans/atlas"))));
}

#[gpui_kit::test]
fn a_search_sent_along_with_a_folder_ends_at_that_folders_top(cx: &mut TestAppContext) {
    let fx = long_folder("search-place-sent");
    for ix in 0..200 {
        std::fs::write(fx.path(&format!("Projects/page {ix:03}.txt")), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press("end");

    // `fff gui` and a second launch can name a folder and words together.
    let projects = fx.path("Projects");
    ui.update(|f, window, cx| f.open_request(Some(&projects), Some("page"), false, window, cx));
    assert!(ui.names().len() >= 200, "{}", ui.names().len());
    ui.press("escape");

    assert_eq!(ui.folder(), Some(projects));
    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert!(ui.exists(item_id("row", &fx.path("Projects/atlas"))));
}

// ----- what is selected in the column view ---------------------------------------------

/// The fixture with `files` in Projects, beside its folder `atlas`.
fn projects_with(name: &str, files: &[&str]) -> Fixture {
    let fx = Fixture::new(name);
    for file in files {
        std::fs::write(fx.path("Projects").join(file), "x").unwrap();
    }
    fx
}

/// The names in each of the column view's columns, from the left.
fn columns(ui: &Ui) -> Vec<Vec<String>> {
    ui.read(|f| f.column_names_for_test())
}

#[gpui_kit::test]
fn a_column_read_again_keeps_its_selected_item(cx: &mut TestAppContext) {
    let fx = projects_with("column-reread", &[".x.txt", "b.txt", "c.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));
    assert_eq!(columns(&ui)[1], ["atlas", "b.txt", "c.txt"]);

    // Hidden files put one more item before it.
    ui.press(&cmd("shift-."));
    assert_eq!(columns(&ui)[1], [".x.txt", "atlas", "b.txt", "c.txt"]);
    assert_eq!(ui.selected(), ["b.txt"]);

    // So does a file another program makes, which the next look finds.
    std::fs::write(fx.path("Projects/a.txt"), "x").unwrap();
    ui.look_for_changes(1);
    assert_eq!(
        columns(&ui)[1],
        [".x.txt", "a.txt", "atlas", "b.txt", "c.txt"]
    );
    assert_eq!(ui.selected(), ["b.txt"]);

    // Once it has gone, no other item is selected in its place.
    std::fs::remove_file(fx.path("Projects/b.txt")).unwrap();
    ui.press(&cmd("r"));
    assert_eq!(columns(&ui)[1], [".x.txt", "a.txt", "atlas", "c.txt"]);
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
}

#[gpui_kit::test]
fn the_columns_after_a_folder_that_has_gone_close(cx: &mut TestAppContext) {
    let fx = Fixture::new("column-folder-gone");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/atlas")));
    assert_eq!(columns(&ui).len(), 3);

    // Another program renames the folder the last column shows.
    std::fs::rename(fx.path("Projects/atlas"), fx.path("Projects/maps")).unwrap();
    ui.press(&cmd("r"));
    assert_eq!(columns(&ui)[1..], [["maps"]]);
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
    // The arrow keys go on in the column that listed it.
    ui.press("down");
    assert_eq!(ui.selected(), ["maps"]);
    assert_eq!(columns(&ui)[1..], [vec!["maps"], vec!["Project Atlas.txt"]]);

    // And then the folder the second column shows.
    std::fs::rename(fx.path("Projects"), fx.path("Work")).unwrap();
    ui.press(&cmd("r"));
    assert_eq!(
        columns(&ui),
        [["Budget.csv", "notes.txt", "Roadmap.md", "Work"]]
    );
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
    ui.press("up");
    assert_eq!(ui.selected(), ["Work"], "the first column has the keys");
}

#[gpui_kit::test]
fn escape_lets_go_of_the_item_selected_in_a_column(cx: &mut TestAppContext) {
    let fx = projects_with("column-escape", &["b.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/atlas")));
    assert_eq!(columns(&ui).len(), 3);

    // In the column in use, and the column that showed what's in the item
    // goes with it.
    ui.press("escape");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
    assert_eq!(columns(&ui)[1..], [["atlas", "b.txt"]]);
    ui.press("left");
    assert_eq!(ui.selected(), ["Projects"], "the columns before it stay");

    // The first column's item is the one the other views show selected.
    ui.click(item_id("crow", &fx.path("notes.txt")));
    ui.click("view-list");
    assert_eq!(ui.selected(), ["notes.txt"]);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("notes.txt")));
    ui.press("escape");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
    ui.click("view-list");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());

    // They let go of an item no column shows selected too, one selected
    // before the columns were chosen: the status bar counts it.
    ui.select("Budget.csv");
    ui.click("view-columns");
    ui.press("escape");
    ui.click("view-list");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());

    // An item the window selected itself in a further column, the next one
    // after a move to the Trash, isn't one the other views have selected.
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));
    ui.press(&cmd("backspace"));
    assert_eq!(ui.selected(), ["atlas"]);
    ui.press("escape");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
}

#[gpui_kit::test]
fn the_columns_close_for_what_a_filter_leaves_out_once_it_has_answered(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = projects_with("column-filter", &["b.txt"]);
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    let open_projects = |ui: &mut Ui| {
        ui.click(item_id("crow", &fx.path("Projects")));
        ui.click(item_id("crow", &fx.path("Projects/b.txt")));
    };
    ui.click("view-columns");
    open_projects(&mut ui);

    // A filter that leaves Projects out takes its column along.
    ui.press(&cmd("alt-f"));
    ui.type_text("name =~ 'notes'");
    ui.press("enter");
    assert_eq!(columns(&ui), [["notes.txt"]]);
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());

    // One that is still running hasn't left anything out yet. When a search
    // ends, the filter runs again for the folder's items, and until it
    // answers they're held against what it kept of the results, which
    // Projects wasn't one of.
    ui.press(&cmd("a"));
    ui.type_text("name =~ 'o'");
    ui.press("enter");
    assert_eq!(columns(&ui), [["notes.txt", "Projects", "Roadmap.md"]]);
    open_projects(&mut ui);
    ui.press(&cmd("f"));
    ui.type_text("road");
    ui.press("escape");
    assert_eq!(columns(&ui)[1..], [["atlas", "b.txt"]]);
    assert_eq!(ui.selected(), ["b.txt"]);
}

#[gpui_kit::test]
fn a_sort_puts_every_column_in_its_order(cx: &mut TestAppContext) {
    let fx = Fixture::new("column-sort");
    std::fs::write(fx.path("Projects/a.txt"), "x".repeat(3000)).unwrap();
    std::fs::write(fx.path("Projects/b.txt"), "x".repeat(10)).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));
    assert_eq!(columns(&ui)[1], ["a.txt", "atlas", "b.txt"]);

    // The largest first.
    ui.click("sort");
    ui.click_menu_item("Size");
    assert_eq!(columns(&ui)[1], ["a.txt", "b.txt", "atlas"]);
    assert_eq!(ui.selected(), ["b.txt"], "the same item, where it is now");

    ui.click("sort");
    ui.click_menu_item("Ascending");
    assert_eq!(columns(&ui)[1], ["atlas", "b.txt", "a.txt"]);
    assert_eq!(ui.selected(), ["b.txt"]);
}

#[gpui_kit::test]
fn a_paste_into_the_windows_folder_leaves_a_further_column_as_it_is(cx: &mut TestAppContext) {
    let fx = projects_with("column-paste", &["b.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));

    // A paste goes into the folder the window shows, which is the first
    // column's, and what it brings is selected there in the other views.
    ui.press(&cmd("c"));
    ui.press(&cmd("v"));

    assert!(fx.path("b.txt").exists());
    assert_eq!(
        columns(&ui),
        [
            vec!["b.txt", "Budget.csv", "notes.txt", "Projects", "Roadmap.md"],
            vec!["atlas", "b.txt"]
        ]
    );
    let selected = ui.read(|f| f.selected_entries());
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].path, fx.path("Projects/b.txt"));
}

#[gpui_kit::test]
fn what_a_paste_brings_is_selected_in_the_first_column(cx: &mut TestAppContext) {
    let fx = Fixture::new("column-paste-first");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.select("Budget.csv");
    ui.select_also("notes.txt");
    ui.press(&cmd("c"));
    ui.go_to(&fx.path("Projects"));
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects/atlas")));

    ui.press(&cmd("v"));

    // A column has one item selected, so it's the first of them, and the
    // other views have them all.
    assert_eq!(columns(&ui), [["atlas", "Budget.csv", "notes.txt"]]);
    assert_eq!(ui.selected(), ["Budget.csv"]);
    ui.click("view-list");
    assert_eq!(ui.selected(), ["Budget.csv", "notes.txt"]);
}

#[gpui_kit::test]
fn what_undo_puts_back_in_an_earlier_column_leaves_the_one_in_use(cx: &mut TestAppContext) {
    let fx = projects_with("column-undo", &["b.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));
    ui.press(&cmd("backspace"));
    assert_eq!(ui.selected(), ["atlas"]);
    ui.press("right");
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);

    ui.press(&cmd("z"));

    assert_eq!(columns(&ui)[1], ["atlas", "b.txt"], "it's back");
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
    assert_eq!(columns(&ui).len(), 3);
}

#[gpui_kit::test]
fn a_result_sent_to_the_trash_leaves_the_columns_as_they_were(cx: &mut TestAppContext) {
    let fx = projects_with("column-results", &["b.txt", "zq one.txt", "zq two.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));

    // Results are a list in this view. The next one is selected there, and
    // it is in the folder of the column that was in use.
    ui.press(&cmd("f"));
    ui.type_text("zq");
    ui.press("enter");
    let first = ui.selected();
    ui.press(&cmd("backspace"));
    assert_eq!(ui.names().len(), 1, "one of the two is left");
    assert_eq!(ui.selected(), ui.names());
    assert_ne!(ui.selected(), first);

    ui.press("escape");
    ui.press("escape");
    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert_eq!(ui.selected(), ["b.txt"]);
}

// ----- the selection after a removal ---------------------------------------------------

#[gpui_kit::test]
fn what_could_not_go_to_the_trash_stays_selected(cx: &mut TestAppContext) {
    let fx = Fixture::new("trash-failed");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    // A Trash that's a file can't take anything.
    let trash = common::trash_dir(&fx.dir);
    std::fs::remove_dir_all(&trash).unwrap();
    std::fs::write(&trash, "").unwrap();

    ui.select("notes.txt");
    ui.press(&cmd("backspace"));

    assert!(fx.path("notes.txt").exists());
    assert_eq!(ui.selected(), ["notes.txt"], "to try again");
    std::fs::remove_file(trash).unwrap();
}

#[gpui_kit::test]
fn what_could_not_go_to_the_trash_keeps_its_column_as_it_was(cx: &mut TestAppContext) {
    let fx = Fixture::new("trash-failed-column");
    for ix in 0..100 {
        std::fs::write(fx.path(&format!("Projects/item {ix:03}.txt")), "x").unwrap();
        std::fs::write(fx.path(&format!("Projects/atlas/item {ix:03}.txt")), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    // A Trash that's a file can't take anything.
    let trash = common::trash_dir(&fx.dir);
    std::fs::remove_dir_all(&trash).unwrap();
    std::fs::write(&trash, "").unwrap();

    // A folder in the first column, whose items are in the second,
    // scrolled away from the top.
    let atlas = item_id("crow", &fx.path("Projects/atlas"));
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.scroll(atlas.clone(), point(px(0.), px(-1200.)));
    assert!(!ui.exists(atlas.clone()), "scrolled");
    ui.press(&cmd("backspace"));
    assert!(fx.path("Projects").exists());
    assert_eq!(ui.selected(), ["Projects"], "to try again");
    assert!(!ui.exists(atlas), "its column stayed as it was");

    // And one in the second column, which Right goes to the top of.
    let first = item_id("crow", &fx.path("Projects/atlas/item 000.txt"));
    ui.press("right");
    assert_eq!(ui.selected(), ["atlas"]);
    ui.scroll(first.clone(), point(px(0.), px(-1200.)));
    assert!(!ui.exists(first.clone()), "scrolled");
    ui.press(&cmd("backspace"));
    assert_eq!(ui.selected(), ["atlas"]);
    assert!(!ui.exists(first), "its column stayed as it was");
    std::fs::remove_file(trash).unwrap();
}

#[gpui_kit::test]
fn the_next_item_is_selected_after_moving_one_to_the_trash(cx: &mut TestAppContext) {
    let fx = Fixture::new("trash-next");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let names = ui.names();
    let at_notes = names.iter().position(|n| n == "notes.txt").unwrap();

    ui.select("notes.txt");
    ui.press(&cmd("backspace"));

    assert_eq!(ui.selected(), [names[at_notes + 1].as_str()]);

    // The last item has none after it, so the one before it is next.
    let last = ui.names().pop().unwrap();
    let before = ui.names()[ui.names().len() - 2].clone();
    ui.select(&last);
    ui.press(&cmd("backspace"));
    assert_eq!(ui.selected(), [before.as_str()]);
}

#[gpui_kit::test]
fn the_next_item_is_selected_in_the_column_one_left_for_the_trash(cx: &mut TestAppContext) {
    let fx = projects_with("trash-next-column", &["a.txt", "b.txt", "c.txt"]);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    ui.click(item_id("crow", &fx.path("Projects/b.txt")));

    ui.press(&cmd("backspace"));
    assert_eq!(columns(&ui)[1], ["a.txt", "atlas", "c.txt"]);
    assert_eq!(ui.selected(), ["c.txt"]);

    // The last item has none after it, so the one before it is next, and
    // being a folder it shows its items in a column of its own.
    ui.press(&cmd("backspace"));
    assert_eq!(ui.selected(), ["atlas"]);
    assert_eq!(
        columns(&ui)[1..],
        [vec!["a.txt", "atlas"], vec!["Project Atlas.txt"]]
    );

    // The first column lists the window's own folder.
    ui.click(item_id("crow", &fx.path("notes.txt")));
    ui.press(&cmd("backspace"));
    assert_eq!(ui.selected(), ["Projects"]);
    ui.press(&cmd("z"));
    assert_eq!(
        ui.selected(),
        ["notes.txt"],
        "Undo selects what it put back"
    );
}

// ----- the empty-space menu --------------------------------------------------------------

#[gpui_kit::test]
fn the_empty_space_menu_changes_the_view_and_sort(cx: &mut TestAppContext) {
    let fx = Fixture::new("space-menu");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let empty = empty_list_space(&mut ui, &fx);

    ui.right_click_point(empty);

    let labels = ui.menu_labels();
    for label in ["View", "Sort By", "Select All"] {
        assert!(labels.contains(&label.to_string()), "{label} in {labels:?}");
    }
    assert_eq!(
        labels.iter().filter(|l| *l == "Show Hidden Files").count(),
        1,
        "the submenus don't repeat it: {labels:?}"
    );
    ui.click_menu_item("Select All");
    assert_eq!(ui.selected().len(), ui.names().len());
}

#[gpui_kit::test]
fn folders_on_screen_show_what_other_programs_add(cx: &mut TestAppContext) {
    let fx = Fixture::new("outside-changes");
    let mut ui = Ui::open(cx, &fx.dir);
    // A little over the 2 seconds between looks.
    let next_look = |ui: &mut Ui| {
        ui.cx
            .executor()
            .advance_clock(std::time::Duration::from_millis(2100));
        ui.settle();
    };
    let shows = |ui: &Ui, name: &str| ui.names().iter().any(|n| n == name);

    ui.click("view-list");
    std::fs::write(fx.path("made outside.txt"), "x").unwrap();
    next_look(&mut ui);
    assert!(shows(&ui, "made outside.txt"), "the window's own folder");

    ui.select("Projects");
    ui.press("right");
    assert!(shows(&ui, "atlas"), "Projects is open in place");
    std::fs::write(fx.path("Projects/added inside.txt"), "x").unwrap();
    next_look(&mut ui);
    assert!(
        shows(&ui, "added inside.txt"),
        "a folder open in the list: {:?}",
        ui.names()
    );

    ui.click("view-columns");
    let projects = ui
        .read(|f| f.column_names_for_test())
        .first()
        .and_then(|names| names.iter().position(|n| n == "Projects"))
        .expect("Projects in the first pane");
    ui.update(|f, window, cx| f.select_in_column(0, projects, window, cx));
    std::fs::write(fx.path("Projects/added to the pane.txt"), "x").unwrap();
    next_look(&mut ui);
    let panes = ui.read(|f| f.column_names_for_test());
    assert!(
        panes
            .get(1)
            .is_some_and(|names| names.iter().any(|n| n == "added to the pane.txt")),
        "a column pane: {panes:?}"
    );
}
