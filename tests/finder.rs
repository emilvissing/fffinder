//! Browsing, searching, and every keyboard shortcut, driven through real key
//! presses in a headless window.

mod common;

use common::{Fixture, Open, Ui, cmd};
use fffinder::finder::QuickLookMode;
use fffinder::model::Location;
use fffinder_core::ViewMode;
use gpui_kit::component::{ActiveTheme as _, WindowExt as _};
use gpui_kit::{Keystroke, TestAppContext};

#[gpui_kit::test]
fn lists_navigates_and_goes_back(cx: &mut TestAppContext) {
    let fx = Fixture::new("nav");
    let mut ui = Ui::open(cx, &fx.dir);
    assert_eq!(
        ui.names(),
        ["Budget.csv", "notes.txt", "Projects", "Roadmap.md"]
    );

    ui.update(|f, window, cx| f.click_row(2, Default::default(), 2, window, cx));
    assert_eq!(ui.names(), ["atlas"]);

    ui.press(&cmd("["));
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(
        ui.selected(),
        ["Projects"],
        "back selects the folder we left"
    );
    ui.press(&cmd("]"));
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    ui.press("alt-left");
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    ui.press("alt-right");
    ui.press(&cmd("up"));
    assert_eq!(
        ui.folder().as_deref(),
        Some(fx.dir.as_path()),
        "Cmd-Up goes to the parent"
    );
    assert_eq!(ui.selected(), ["Projects"]);

    // Cmd-Down, Cmd-O, and Return open the selection.
    ui.press(&cmd("down"));
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    ui.press(&cmd("up"));
    ui.press(&cmd("o"));
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    ui.press(&cmd("up"));
    ui.press("enter");
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
}

#[gpui_kit::test]
fn go_shortcuts(cx: &mut TestAppContext) {
    let fx = Fixture::new("go-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-f"));
    assert_eq!(ui.read(|f| f.location().clone()), Location::Recents);
    ui.press(&cmd("shift-h"));
    assert_eq!(ui.folder(), dirs::home_dir());
}

#[gpui_kit::test]
fn searches_by_name_and_returns_to_browsing(cx: &mut TestAppContext) {
    let fx = Fixture::new("search");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    assert!(
        ui.window(|window, cx| window.has_focused_input(cx)),
        "Cmd-F focuses search"
    );
    ui.type_text("atlas");
    let found = ui.names();
    assert!(
        found.contains(&"Project Atlas.txt".to_string()),
        "{found:?}"
    );
    assert!(
        found.contains(&"atlas".to_string()),
        "folders match too: {found:?}"
    );

    // Return moves into the results with the first one selected.
    ui.press("enter");
    assert!(ui.files_focused());
    assert_eq!(ui.selected().len(), 1);

    // Escape clears the selection, then the search.
    ui.press("escape");
    assert!(ui.selected().is_empty());
    ui.press("escape");
    assert_eq!(
        ui.names(),
        ["Budget.csv", "notes.txt", "Projects", "Roadmap.md"]
    );
}

#[gpui_kit::test]
fn a_capital_in_a_search_word_doesnt_narrow_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("capitals");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("ATLAS");
    let found = ui.names();
    assert!(
        found.contains(&"Project Atlas.txt".to_string()),
        "{found:?}"
    );
    assert!(
        found.contains(&"atlas".to_string()),
        "folders match too: {found:?}"
    );
}

#[gpui_kit::test]
fn arrow_keys_select_in_each_view(cx: &mut TestAppContext) {
    let fx = Fixture::new("arrows");
    let mut ui = Ui::open(cx, &fx.dir);

    ui.press(&cmd("2"));
    assert_eq!(ui.read(|f| f.effective_view()), ViewMode::List);
    ui.press("down");
    ui.press("shift-down");
    assert_eq!(ui.selected(), ["Budget.csv", "notes.txt"]);
    ui.press("up");
    assert_eq!(ui.selected(), ["Budget.csv"]);
    ui.press("end");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    ui.press("home");
    assert_eq!(ui.selected(), ["Budget.csv"]);
    ui.press("shift-down");
    ui.press("shift-down");
    ui.press("shift-up");
    assert_eq!(ui.selected(), ["Budget.csv", "notes.txt"]);
    ui.press(&cmd("a"));
    assert_eq!(ui.selected().len(), 4, "Cmd-A selects everything");
    ui.press("escape");
    assert!(ui.selected().is_empty());

    // Icons and the gallery move with Left and Right too.
    ui.press(&cmd("1"));
    assert_eq!(ui.read(|f| f.effective_view()), ViewMode::Icons);
    ui.press("right");
    ui.press("right");
    assert_eq!(ui.selected(), ["notes.txt"]);
    ui.press("shift-left");
    assert_eq!(ui.selected(), ["Budget.csv", "notes.txt"]);
    ui.press("shift-right");
    assert_eq!(ui.selected(), ["notes.txt"]);

    ui.press(&cmd("4"));
    assert_eq!(ui.read(|f| f.effective_view()), ViewMode::Gallery);
    ui.press("right");
    assert_eq!(ui.selected().len(), 1);

    // Columns: Right enters a folder, Left comes back.
    ui.press(&cmd("3"));
    assert_eq!(ui.read(|f| f.effective_view()), ViewMode::Columns);
    ui.update(|f, window, cx| f.select_in_column(0, 2, window, cx));
    assert_eq!(ui.read(|f| f.column_names_for_test()).len(), 2);
    ui.press("right");
    assert_eq!(ui.selected(), ["atlas"]);
    ui.press("left");
    assert_eq!(ui.selected(), ["Projects"]);
}

#[gpui_kit::test]
fn clipboard_shortcuts(cx: &mut TestAppContext) {
    let fx = Fixture::new("clipboard");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("2"));
    ui.press("down");
    ui.press("shift-down");
    let both = format!(
        "{}\n{}",
        fx.path("Budget.csv").display(),
        fx.path("notes.txt").display()
    );
    ui.press(&cmd("c"));
    assert_eq!(ui.clipboard(), both);
    ui.cx
        .write_to_clipboard(gpui_kit::ClipboardItem::new_string(String::new()));
    ui.press(&cmd("alt-c"));
    assert_eq!(ui.clipboard(), both);
    ui.press(&cmd("shift-c"));
    assert!(
        ui.clipboard().starts_with('['),
        "NUON list: {}",
        ui.clipboard()
    );
    assert!(ui.clipboard().contains("notes.txt"));
}

#[gpui_kit::test]
fn window_shortcuts(cx: &mut TestAppContext) {
    let fx = Fixture::new("window-keys");
    let mut ui = Ui::open(cx, &fx.dir);

    ui.press(&cmd("shift-."));
    assert!(ui.read(|f| f.config().show_hidden));
    assert!(ui.names().contains(&".secret".to_string()));
    ui.press(&cmd("shift-."));
    assert!(!ui.names().contains(&".secret".to_string()));

    ui.press(&cmd("alt-s"));
    assert!(!ui.read(|f| f.config().show_sidebar));
    ui.press(&cmd("alt-s"));
    assert!(ui.read(|f| f.config().show_sidebar));

    let dark = ui.cx.update(|cx| cx.theme().is_dark());
    ui.press(&cmd("shift-d"));
    assert_ne!(ui.cx.update(|cx| cx.theme().is_dark()), dark);

    std::fs::write(fx.path("new.txt"), "x").unwrap();
    ui.press(&cmd("r"));
    assert!(
        ui.names().contains(&"new.txt".to_string()),
        "Cmd-R refreshes"
    );

    ui.press("ctrl-`");
    assert!(ui.read(|f| f.nu_panel_open()));
    ui.press("ctrl-`");
    assert!(!ui.read(|f| f.nu_panel_open()));

    ui.press(&cmd("/"));
    assert!(ui.dialog_open(), "Cmd-/ shows the search syntax");
    ui.press("escape");

    // Without nu this warns instead of opening a terminal.
    ui.press(&cmd("alt-t"));
    assert!(!ui.dialog_open());
}

#[gpui_kit::test]
fn file_shortcuts(cx: &mut TestAppContext) {
    let fx = Fixture::new("file-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-n"));
    assert!(fx.path("untitled folder").is_dir());
    assert!(ui.exists("rename-input"), "a new folder starts naming it");
    ui.type_text("Archive");
    ui.press("enter");
    assert!(!ui.exists("rename-input"));
    assert!(fx.path("Archive").is_dir());
    assert_eq!(ui.selected(), ["Archive"]);

    ui.select("notes.txt");
    ui.press("f2");
    assert!(ui.exists("rename-input"));
    ui.press("escape");
    assert!(!ui.exists("rename-input"));
    ui.press("space");
    assert!(ui.read(|f| f.quick_look_is_open()));
    ui.press(&cmd("y"));
    assert!(!ui.read(|f| f.quick_look_is_open()));

    ui.press(&cmd("w"));
    let open = ui.cx.update(|cx| cx.windows().len());
    assert_eq!(open, 0, "Cmd-W closes the window");
}

#[gpui_kit::test]
fn type_into_the_search_field_and_down_arrow(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-down");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("roadmap");
    ui.press("down");
    assert!(ui.files_focused());
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    // Keys typed in the field edit text instead of moving the selection.
    ui.click("search");
    ui.press("left");
    ui.press("backspace");
    assert_eq!(ui.read(|f| f.query().to_string()), "roadmp");
}

#[gpui_kit::test]
fn return_before_the_results_are_in_waits_for_them(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-early-return");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("roadmap");
    ui.press_without_settling("enter");
    assert!(
        !ui.files_focused(),
        "the rows are still the folder's, so the keyboard stays in the field"
    );
    assert!(ui.selected().is_empty());

    // Return doesn't sit out the pause a search waits for after a key, so
    // the answer comes without any time passing.
    ui.cx.run_until_parked();
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(ui.files_focused(), "the results came in");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn down_arrow_before_the_results_are_in_waits_for_them(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-early-down");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("atlas");
    ui.press_without_settling("down");
    assert!(!ui.files_focused());
    assert!(ui.selected().is_empty());

    ui.settle();
    let found = ui.names();
    assert!(found.len() > 1, "{found:?}");
    assert!(ui.files_focused());
    assert_eq!(ui.selected(), [found[0].clone()], "the first of {found:?}");
}

#[gpui_kit::test]
fn return_before_the_results_are_in_waits_in_an_empty_folder_too(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-early-return-empty");
    std::fs::create_dir(fx.path("Empty")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Empty"));
    assert!(ui.names().is_empty());
    ui.click("search");
    ui.type_without_settling("roadmap");
    ui.press_without_settling("enter");
    ui.settle();
    assert!(ui.files_focused(), "the folder had nothing to move into");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn return_after_a_switch_to_contents_waits_for_what_is_inside_files(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-contents-return");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("atlas");
    assert!(ui.names().len() > 1, "{:?}", ui.names());

    // The names found are gone, and the files haven't been looked into yet.
    ui.click_without_settling("by-contents");
    ui.press_without_settling(&cmd("f"));
    ui.press_without_settling("enter");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(!ui.files_focused());

    ui.settle();
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
    assert!(ui.files_focused(), "the results came in");
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
}

#[gpui_kit::test]
fn a_second_return_before_the_results_are_in_opens_nothing(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-twice");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("roadmap");
    ui.press_without_settling("enter");
    ui.press_without_settling("enter");
    ui.settle();
    assert!(
        ui.system_calls().is_empty(),
        "nothing from the folder was opened: {:?}",
        ui.system_calls()
    );
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert!(ui.files_focused());
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn return_on_a_search_that_finds_nothing_stays_in_the_field(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-nothing");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("zzqq-nothing");
    ui.press_without_settling("enter");
    ui.settle();
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(!ui.files_focused(), "there is nothing to move into");
    assert!(
        ui.window(|window, cx| window.has_focused_input(cx)),
        "the search field keeps the keyboard"
    );
}

#[gpui_kit::test]
fn typing_on_after_return_calls_off_the_move_into_the_results(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-then-type");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("road");
    ui.press_without_settling("enter");
    ui.type_without_settling("map");
    ui.settle();
    assert_eq!(ui.read(|f| f.query().to_string()), "roadmap");
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(!ui.files_focused(), "typing went on in the field");
    assert!(ui.selected().is_empty());

    // The same when the words and the caret end up as they were at Return.
    ui.press_without_settling("backspace");
    ui.press_without_settling("enter");
    ui.press_without_settling("backspace");
    ui.type_without_settling("a");
    ui.settle();
    assert_eq!(ui.read(|f| f.query().to_string()), "roadma");
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(!ui.files_focused(), "the words were edited in between");

    // And when a space is typed and deleted, which the words never show.
    ui.type_without_settling("p");
    ui.press_without_settling("enter");
    ui.type_without_settling(" ");
    ui.press_without_settling("backspace");
    ui.settle();
    assert_eq!(ui.read(|f| f.query().to_string()), "roadmap");
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(!ui.files_focused(), "a space came and went in between");
}

#[gpui_kit::test]
fn a_newline_the_search_field_drops_leaves_return_waiting(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-newline");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("roadmap");
    // Return from a system that also sends its newline as typed text. The
    // field has one line and drops it, and still says its text changed.
    ui.window(|window, cx| {
        window.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
    });
    ui.settle();
    assert_eq!(ui.read(|f| f.query().to_string()), "roadmap");
    assert!(ui.files_focused(), "nothing was edited, so Return stands");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn moving_the_caret_after_return_calls_off_the_move_into_the_results(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-then-left");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("roadmap");
    ui.press_without_settling("enter");
    // The words are the same, but they're being edited again.
    ui.press_without_settling("left");
    ui.settle();
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(!ui.files_focused(), "the keyboard stays where the caret is");
    assert!(ui.selected().is_empty());
}

#[gpui_kit::test]
fn other_words_from_a_second_launch_call_off_the_move_into_the_results(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-then-sent");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("road");
    ui.press_without_settling("enter");
    // `fff gui` and a second launch put their own words in the field, and
    // these leave the caret where it was.
    let view = ui.view.clone();
    ui.window(|window, cx| view.update(cx, |f, cx| f.open_request(None, Some("note"), window, cx)));
    ui.settle();
    let found = ui.names();
    assert!(found.contains(&"notes.txt".to_string()), "{found:?}");
    assert!(!ui.files_focused(), "Return was pressed for other words");
    assert!(ui.selected().is_empty());
}

#[gpui_kit::test]
fn results_that_come_in_leave_the_keyboard_where_it_has_gone_since(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-then-filter");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("roadmap");
    ui.press_without_settling("enter");
    // The filter bar opens with the keyboard in its field.
    ui.press_without_settling(&cmd("alt-f"));
    ui.settle();
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(!ui.files_focused());
    let typing_in = ui.window(|window, cx| window.focused_input(cx).map(|field| field.value(cx)));
    assert_eq!(
        typing_in.as_deref(),
        Some(""),
        "the filter field, which is empty, has the keyboard"
    );
}

#[gpui_kit::test]
fn return_before_the_results_are_in_waits_for_the_filter_too(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("search-early-return-filter");
    std::fs::write(fx.path("Projects/atlas/huge.log"), "x".repeat(20_000)).unwrap();
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    ui.press(&cmd("alt-f"));
    ui.type_text("size > 10kb");
    ui.press("enter");
    let kept = ui.names();
    assert!(!kept.contains(&"notes.txt".to_string()), "{kept:?}");

    // The search answers before the filter has looked at its results.
    ui.click("search");
    ui.type_without_settling("huge");
    ui.press_without_settling("enter");
    ui.settle();
    assert_eq!(ui.names(), ["huge.log"]);
    assert!(ui.files_focused());
    assert_eq!(ui.selected(), ["huge.log"]);
}

#[gpui_kit::test]
fn going_to_another_folder_calls_off_a_key_that_waits_for_the_filter(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("search-down-then-go-up");
    std::fs::write(fx.path("Projects/atlas/huge.log"), "x".repeat(20_000)).unwrap();
    std::fs::write(fx.path("Projects/big.log"), "x".repeat(20_000)).unwrap();
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    ui.go_to(&fx.path("Projects/atlas"));
    ui.press(&cmd("alt-f"));
    ui.type_without_settling("size > 10kb");
    ui.press_without_settling("enter");
    // The filter hasn't looked at the folder's items yet.
    ui.click_without_settling("search");
    ui.press_without_settling("down");
    assert!(!ui.files_focused());

    // Alt-Up goes to the enclosing folder, and the keyboard stays put.
    ui.press_without_settling("alt-up");
    ui.settle();
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert_eq!(ui.names(), ["big.log"]);
    assert!(
        !ui.files_focused(),
        "Down Arrow was pressed for the folder the window left"
    );
    assert!(
        ui.window(|window, cx| window.has_focused_input(cx)),
        "the search field keeps the keyboard"
    );
}

#[gpui_kit::test]
fn results_sent_from_nushell(cx: &mut TestAppContext) {
    let fx = Fixture::new("results");
    let mut ui = Ui::open(cx, &fx.dir);
    let sent = vec![
        fx.path("Roadmap.md"),
        fx.path("Projects/atlas/Project Atlas.txt"),
        fx.path("gone.txt"),
    ];
    let (shown, missing) =
        ui.update(|f, window, cx| f.show_results("Big files".into(), sent, window, cx));
    assert_eq!((shown, missing), (2, vec![fx.path("gone.txt")]));
    assert_eq!(
        ui.names(),
        ["Roadmap.md", "Project Atlas.txt"],
        "the order is kept"
    );
    assert_eq!(ui.read(|f| f.title()), "Big files");
    assert!(ui.read(|f| f.shows_parent_column()));

    ui.select("Project Atlas.txt");
    let report = ui.read(|f| f.selection_json(false));
    assert_eq!(report["location"], "results");
    assert_eq!(report["items"].as_array().unwrap().len(), 1);
    assert_eq!(report["items"][0]["name"], "Project Atlas.txt");
    assert_eq!(report["items"][0]["type"], "file");
    let everything = ui.read(|f| f.selection_json(true));
    assert_eq!(everything["items"].as_array().unwrap().len(), 2);

    ui.press(&cmd("["));
    assert_eq!(
        ui.folder().as_deref(),
        Some(fx.dir.as_path()),
        "Back leaves the results"
    );

    // `fff gui` and a second launch: a folder and a search.
    let projects = fx.path("Projects");
    ui.update(|f, window, cx| f.open_request(Some(&projects), Some("atlas"), window, cx));
    assert_eq!(ui.folder(), Some(projects));
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    let report = ui.read(|f| f.selection_json(true));
    assert_eq!(report["location"], "search");
    assert_eq!(report["query"], "atlas");

    // A file opens its folder with the file selected.
    let roadmap = fx.path("Roadmap.md");
    ui.update(|f, window, cx| f.open_request(Some(&roadmap), None, window, cx));
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn typing_jumps_to_matching_names(cx: &mut TestAppContext) {
    let fx = Fixture::new("type-select");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("2"));
    ui.type_text("r");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    // Letters typed within a second add to the prefix.
    ui.type_text("o");
    assert_eq!(ui.selected(), ["Roadmap.md"]);

    ui.cx
        .executor()
        .advance_clock(std::time::Duration::from_secs(2));
    ui.type_text("n");
    assert_eq!(ui.selected(), ["notes.txt"]);
    // No name starts with "np", so the next one after it is chosen.
    ui.type_text("p");
    assert_eq!(ui.selected(), ["Projects"]);

    ui.cx
        .executor()
        .advance_clock(std::time::Duration::from_secs(2));
    ui.type_text("zz");
    assert_eq!(
        ui.selected(),
        ["Roadmap.md"],
        "past the end picks the last item"
    );

    // Typing in the search field searches instead.
    ui.click("search");
    ui.type_text("b");
    assert_eq!(ui.read(|f| f.query().to_string()), "b");
    ui.press("escape");

    // The column view selects in the active column.
    ui.press(&cmd("3"));
    ui.update(|f, window, cx| f.focus_files(window, cx));
    ui.cx
        .executor()
        .advance_clock(std::time::Duration::from_secs(2));
    ui.type_text("p");
    assert_eq!(ui.selected(), ["Projects"]);
    assert_eq!(
        ui.read(|f| f.column_names_for_test()).len(),
        2,
        "the folder opens to the right"
    );
}

#[gpui_kit::test]
fn quick_look_footer_wraps_instead_of_cutting_the_caption(cx: &mut TestAppContext) {
    let fx = Fixture::new("ql-footer");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.select("notes.txt");
    ui.press("space");
    ui.in_quick_look(|ui| {
        ui.resize(1400., 500.);
        let caption = ui.snapshot("ql-caption").bounds();
        let controls = ui.snapshot("ql-controls").bounds();
        assert!(
            controls.top() < caption.bottom(),
            "a wide window has one footer line: {caption:?} {controls:?}"
        );

        ui.resize(420., 500.);
        let caption = ui.snapshot("ql-caption").bounds();
        let controls = ui.snapshot("ql-controls").bounds();
        assert!(
            controls.top() >= caption.bottom(),
            "a narrow one moves the controls under the caption: {caption:?} {controls:?}"
        );
    });
}

#[gpui_kit::test]
fn quick_look_explores_structured_files(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("explore");
    std::fs::write(
        fx.path("app.json"),
        r#"{"name": "fff", "deps": {"serde": {"version": "1"}}}"#,
    )
    .unwrap();
    std::fs::write(fx.path("broken.json"), r#"{"name": "#).unwrap();
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    let crumb = |depth: usize| ("explore-crumb", depth);
    ui.select("app.json");
    ui.press(&cmd("space"));
    assert!(ui.read(|f| f.quick_look_is_open()));
    assert_eq!(ui.read(|f| f.quick_look_mode()), QuickLookMode::Explore);
    ui.in_quick_look(|ui| {
        assert!(ui.exists(crumb(0)), "Cmd-Space explores the data");
        ui.press("down");
        ui.press("enter");
        assert!(ui.exists(crumb(1)), "Return goes into deps");
        ui.press("enter");
        assert!(ui.exists(crumb(2)), "and into serde");
        ui.click(crumb(0));
        assert!(!ui.exists(crumb(1)), "the path goes back to the top");
        ui.press("enter");
        assert!(ui.exists(crumb(1)), "the cursor stayed on deps");
        ui.press("escape");
        assert!(!ui.exists(crumb(1)), "Escape goes back out");
        ui.press("space");
    });
    assert!(
        ui.read(|f| f.quick_look_is_open()),
        "Space switches to text"
    );
    assert_eq!(ui.read(|f| f.quick_look_mode()), QuickLookMode::Text);
    ui.in_quick_look(|ui| {
        assert!(!ui.exists(crumb(0)));
        ui.press(&cmd("space"));
        assert!(ui.exists(crumb(0)), "and Cmd-Space back to explore");
        ui.press("escape");
    });
    assert!(
        !ui.read(|f| f.quick_look_is_open()),
        "Escape at the top closes Quick Look"
    );

    // Arrows still move through the items where there's no grid.
    ui.select("notes.txt");
    ui.press(&cmd("space"));
    ui.in_quick_look(|ui| ui.press("right"));
    assert_eq!(ui.selected(), ["Projects"]);

    // A file Nushell can't parse says why, and Space still shows its text.
    ui.select("broken.json");
    ui.in_quick_look(|ui| {
        assert!(
            ui.exists("explore-failure"),
            "a broken file explains itself"
        );
        ui.press("space");
        assert!(!ui.exists("explore-failure"), "Space shows the text");
    });
    assert!(ui.read(|f| f.quick_look_is_open()));
}

#[gpui_kit::test]
fn pdfs_scroll_and_zoom(cx: &mut TestAppContext) {
    use gpui_kit::{
        InputEvent as _, Modifiers, PinchEvent, ScrollDelta, ScrollEvent, ScrollSource, TouchPhase,
        point, px,
    };
    let fx = Fixture::new("pdf-pages");
    let pdf = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/crates/fffinder-core/testdata/pages.pdf"
    );
    std::fs::copy(pdf, fx.path("Manual.pdf")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    let shown = |ui: &Ui| ui.read(|f| f.shown_pdf_page());
    let zoom = |ui: &Ui| {
        ui.read(|f| f.shown_pdf_zoom())
            .expect("the pages are laid out")
    };

    ui.click("view-gallery");
    ui.select("Manual.pdf");
    assert_eq!(shown(&ui), Some((1, 2)), "a PDF opens at its first page");
    let fit_width = zoom(&ui);

    // The pages scroll in one column; the arrows and Page Up and Down go
    // from page to page.
    ui.click("pdf-next");
    assert_eq!(shown(&ui), Some((2, 2)));
    ui.click("pdf-next");
    assert_eq!(shown(&ui), Some((2, 2)), "the last page has no next");
    ui.press("pageup");
    assert_eq!(shown(&ui), Some((1, 2)), "Page Up in the gallery");

    // Zooming: the buttons, the keys, and the zoom menu.
    ui.click("pdf-zoom-in");
    let once = zoom(&ui);
    assert!(once > fit_width, "Zoom In makes the pages bigger");
    ui.press(&cmd("="));
    assert!(zoom(&ui) > once, "Cmd+= zooms in the gallery");
    ui.press(&cmd("0"));
    assert_eq!(zoom(&ui), fit_width, "Cmd+0 fits the width again");
    ui.click("pdf-zoom");
    ui.click_menu_item("Actual Size");
    assert_eq!(zoom(&ui), 100);
    ui.click("pdf-zoom");
    ui.click_menu_item("Fit Page");
    let whole = zoom(&ui);
    assert!(whole < fit_width, "a whole page is smaller than the width");

    // Pinching and Cmd+scrolling zoom too.
    let center = ui.snapshot("pdf-pages").bounds().center();
    ui.window(|window, cx| {
        let pinch = PinchEvent {
            position: center,
            delta: 0.5,
            modifiers: Modifiers::default(),
            phase: TouchPhase::Moved,
        };
        window.dispatch_event(pinch.to_platform_input(), cx);
    });
    ui.settle();
    let pinched = zoom(&ui);
    assert!(pinched > whole, "pinching out zooms in");
    ui.window(|window, cx| {
        let scroll = ScrollEvent {
            position: center,
            delta: ScrollDelta::Pixels(point(px(0.), px(-30.))),
            modifiers: Modifiers::secondary_key(cx),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Pointer,
        };
        window.dispatch_event(scroll.to_platform_input(), cx);
    });
    ui.settle();
    assert!(zoom(&ui) < pinched, "Cmd+scrolling zooms");
    ui.press(&cmd("0"));

    // Quick Look shows the PDF in its window, scrolled on its own.
    ui.click("pdf-next");
    ui.press("space");
    assert!(ui.read(|f| f.quick_look_is_open()));
    ui.in_quick_look(|ui| {
        assert_eq!(shown(ui), Some((1, 2)), "Quick Look starts at the top");
        ui.press("pagedown");
        assert_eq!(shown(ui), Some((2, 2)), "Page Down in Quick Look");
        ui.press("pageup");
        assert_eq!(shown(ui), Some((1, 2)), "Page Up in Quick Look");
        let fitted = zoom(ui);
        ui.press(&cmd("="));
        assert!(zoom(ui) > fitted, "Cmd+= zooms in Quick Look");
    });
}

#[gpui_kit::test]
fn long_pdfs_keep_only_the_pages_near_view(cx: &mut TestAppContext) {
    let fx = Fixture::new("pdf-long");
    let pdf = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/crates/fffinder-core/testdata/twelve.pdf"
    );
    std::fs::copy(pdf, fx.path("Long.pdf")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    let pages = |ui: &Ui| ui.read(|f| f.pdf_pages_for_test());

    ui.click("view-gallery");
    ui.select("Long.pdf");
    ui.settle();
    let (drawn, waiting) = pages(&ui);
    assert_eq!(
        waiting, 0,
        "the pages in view are drawn once, not over and over"
    );
    assert!(
        (1..=4).contains(&drawn),
        "only the pages in view and their neighbors are kept: {drawn}"
    );

    for _ in 0..11 {
        ui.press("pagedown");
    }
    assert_eq!(ui.read(|f| f.shown_pdf_page()), Some((12, 12)));
    ui.settle();
    let (drawn, waiting) = pages(&ui);
    assert_eq!(waiting, 0);
    assert!(
        (1..=4).contains(&drawn),
        "the first pages were let go: {drawn}"
    );
}
