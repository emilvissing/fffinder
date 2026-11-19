//! The everyday things people reach for without thinking: dragging a box
//! around items, the mouse's back and forward buttons, opening a folder in a
//! new tab, paging through a long list, and where the selection goes.

mod common;

use common::{Fixture, Open, Ui, cmd, item_id};
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

    ui.click("view-gallery");
    ui.press("f2");
    assert!(ui.dialog_open());
    assert!(!ui.exists("rename-field"));
    ui.press("escape");
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
