//! Browsing, searching, and every keyboard shortcut, driven through real key
//! presses in a headless window.

mod common;

use common::{Fixture, Open, Ui, cmd, item_id};
use fffinder::finder::{PickerOptions, QuickLookMode, SearchScope};
use fffinder::model::Location;
use fffinder::system::SystemCall;
use fffinder_core::{KindFilter, SmartFolder, ViewMode};
use gpui_kit::component::{ActiveTheme as _, WindowExt as _};
use gpui_kit::{Keystroke, TestAppContext};
use std::path::Path;

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
fn alt_up_goes_to_the_enclosing_folder_too(cx: &mut TestAppContext) {
    let fx = Fixture::new("nav-alt-up");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.press("alt-up");
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(ui.selected(), ["Projects"], "as Cmd-Up does");
}

#[gpui_kit::test]
fn back_and_cmd_up_select_the_folder_left_in_the_column_view_too(cx: &mut TestAppContext) {
    let fx = Fixture::new("nav-columns");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("3"));
    ui.go_to(&fx.path("Projects"));
    assert_eq!(ui.read(|f| f.effective_view()), ViewMode::Columns);

    ui.press(&cmd("["));
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(ui.selected(), ["Projects"]);
    assert_eq!(
        ui.read(|f| f.column_names_for_test()),
        [
            vec!["Budget.csv", "notes.txt", "Projects", "Roadmap.md"],
            vec!["atlas"]
        ],
        "with its items in the next column, as after a click on it"
    );
    ui.press("right");
    assert_eq!(ui.selected(), ["atlas"], "the arrow keys go on from it");

    ui.press(&cmd("]"));
    ui.press(&cmd("up"));
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(ui.selected(), ["Projects"]);
}

#[gpui_kit::test]
fn a_key_bound_twice_acts_once(cx: &mut TestAppContext) {
    let fx = Fixture::new("nav-keys-twice");
    let mut ui = Ui::open(cx, &fx.dir);
    // A Mac has Finder's keys for Back, Forward, Enclosing Folder, and Move
    // to Trash bound twice. Binding the keys again does that to all of them.
    ui.cx.update(fffinder::actions::bind_keys);

    ui.select("notes.txt");
    ui.press(&cmd("backspace"));
    assert_eq!(ui.names(), ["Budget.csv", "Projects", "Roadmap.md"]);
    assert_eq!(ui.notification_count(), 0, "nothing was trashed twice");

    ui.go_to(&fx.path("Projects"));
    ui.go_to(&fx.path("Projects/atlas"));
    ui.press(&cmd("["));
    assert_eq!(ui.folder(), Some(fx.path("Projects")), "one step back");
    ui.press(&cmd("["));
    ui.press(&cmd("]"));
    assert_eq!(ui.folder(), Some(fx.path("Projects")), "one step forward");
    ui.press(&cmd("]"));
    ui.press(&cmd("up"));
    assert_eq!(ui.folder(), Some(fx.path("Projects")), "one folder up");
}

#[gpui_kit::test]
fn open_quick_look_and_refresh_act_once_with_their_keys_bound_twice(cx: &mut TestAppContext) {
    let fx = Fixture::new("open-keys-twice");
    let mut ui = Ui::open(cx, &fx.dir);
    // A Mac has the Cmd keys for Open, Quick Look, and Refresh bound more
    // than once too. Binding the keys again does that to all of their keys.
    ui.cx.update(fffinder::actions::bind_keys);

    ui.select("notes.txt");
    let opened = SystemCall::Open(fx.path("notes.txt"));
    for (before, keys) in ["enter".to_string(), cmd("down"), cmd("o")]
        .iter()
        .enumerate()
    {
        ui.press(keys);
        assert_eq!(
            ui.system_calls(),
            vec![opened.clone(); before + 1],
            "{keys} opens the file once"
        );
    }

    ui.press(&cmd("y"));
    assert!(
        ui.read(|f| f.quick_look_is_open()),
        "opened, not closed again"
    );
    ui.press("space");
    assert!(
        !ui.read(|f| f.quick_look_is_open()),
        "closed, not opened again"
    );
    ui.press("space");
    assert!(ui.read(|f| f.quick_look_is_open()));
    ui.press(&cmd("y"));
    assert!(!ui.read(|f| f.quick_look_is_open()));

    std::fs::write(fx.path("first.txt"), "x").unwrap();
    ui.press("f5");
    assert!(
        ui.names().contains(&"first.txt".to_string()),
        "F5 refreshes"
    );
    std::fs::write(fx.path("second.txt"), "x").unwrap();
    ui.press(&cmd("r"));
    assert!(ui.names().contains(&"second.txt".to_string()));
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
fn back_from_a_folder_opened_from_the_results_returns_to_the_search(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-history");
    let mut ui = Ui::open(cx, &fx.dir);
    let (projects, atlas) = (fx.path("Projects"), fx.path("Projects/atlas"));
    let shown = |ui: &Ui| (ui.folder(), ui.read(|f| f.query().to_string()));
    let at = |folder: &Path, query: &str| (Some(folder.to_path_buf()), query.to_string());
    ui.go_to(&projects);
    ui.press(&cmd("f"));
    ui.type_text("atlas");
    ui.double_click(item_id("tile", &atlas));
    assert_eq!(
        shown(&ui),
        at(&atlas, ""),
        "opening a folder ends the search"
    );

    // Back leads to the results first, with the folder it left selected
    // among them, as it is in a folder.
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), at(&projects, "atlas"));
    let found = ui.names();
    assert!(
        found.contains(&"Project Atlas.txt".to_string()),
        "{found:?}"
    );
    assert_eq!(ui.selected(), ["atlas"]);
    assert!(ui.files_focused(), "the arrow keys go on from there");
    assert!(ui.exists("scope-computer"), "the scope bar is back too");

    // Then to the folder the search was made in, and then to the one before
    // it. No press leaves the window as it was.
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), at(&projects, ""));
    assert_eq!(ui.names(), ["atlas"]);
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), at(&fx.dir, ""));

    // Forward goes the same way again.
    ui.press(&cmd("]"));
    assert_eq!(shown(&ui), at(&projects, ""));
    ui.press(&cmd("]"));
    assert_eq!(shown(&ui), at(&projects, "atlas"));
    let found = ui.names();
    assert!(
        found.contains(&"Project Atlas.txt".to_string()),
        "{found:?}"
    );
    ui.press(&cmd("]"));
    assert_eq!(shown(&ui), at(&atlas, ""));
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
}

#[gpui_kit::test]
fn other_words_typed_before_a_search_is_back_have_nothing_selected(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-history-typed");
    let mut ui = Ui::open(cx, &fx.dir);
    let atlas = fx.path("Projects/atlas");
    ui.click("search");
    ui.type_text("atlas");
    ui.double_click(item_id("tile", &atlas));
    assert_eq!(ui.folder(), Some(atlas));

    // Back, and a letter is gone from the field before the search it led to
    // has answered. The folder Back came from is found for these words too,
    // but they're another search, and that starts with nothing selected.
    ui.press_without_settling(&cmd("["));
    ui.click_without_settling("search");
    ui.press_without_settling("backspace");
    ui.settle();
    assert_eq!(ui.read(|f| f.query().to_string()), "atla");
    let found = ui.names();
    assert!(found.contains(&"atlas".to_string()), "{found:?}");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
}

#[gpui_kit::test]
fn back_during_a_search_ends_it_and_stays_in_the_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-history-ends");
    let mut ui = Ui::open(cx, &fx.dir);
    let projects = fx.path("Projects");
    let query = |ui: &Ui| ui.read(|f| f.query().to_string());
    let in_a_field = |ui: &mut Ui| ui.window(|window, cx| window.has_focused_input(cx));
    ui.go_to(&projects);
    ui.press(&cmd("f"));
    ui.type_text("road");
    assert_eq!(ui.names(), ["Roadmap.md"]);

    // The keyboard is in the search field, where Cmd+[ is Back too.
    ui.press(&cmd("["));
    assert_eq!(query(&ui), "");
    assert_eq!(ui.folder(), Some(projects.clone()), "not the folder before");
    assert_eq!(ui.names(), ["atlas"]);
    assert!(!ui.exists("scope-computer"));

    // Forward is the search again, and with nothing after it Forward leaves
    // it alone.
    ui.press(&cmd("]"));
    assert_eq!(query(&ui), "road");
    assert_eq!(ui.folder(), Some(projects.clone()));
    assert_eq!(ui.names(), ["Roadmap.md"]);
    ui.press(&cmd("]"));
    assert_eq!(query(&ui), "road");
    assert_eq!(ui.names(), ["Roadmap.md"]);

    // The words are in the field again: Escape there empties a field with
    // words in it, and leaves one that has none.
    ui.press("escape");
    assert_eq!(query(&ui), "");
    assert!(in_a_field(&mut ui), "the field had the words to empty");

    // A search ended any other way isn't one to go back or forward to.
    ui.press(&cmd("]"));
    assert_eq!(query(&ui), "");
    ui.press(&cmd("["));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    ui.press(&cmd("]"));
    assert_eq!(ui.folder(), Some(projects));
    assert_eq!(query(&ui), "");
}

#[gpui_kit::test]
fn a_search_comes_back_where_it_looked_and_for_what(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-history-scope");
    let mut ui = Ui::open(cx, &fx.dir);
    let projects = fx.path("Projects");
    let search = |ui: &Ui| {
        ui.read(|f| {
            (
                f.query().to_string(),
                f.search_scope(),
                f.searches_contents(),
            )
        })
    };
    ui.go_to(&projects);
    ui.click("search");
    ui.type_text("atlas");
    ui.click("scope-folder");
    ui.click("by-contents");
    assert_eq!(ui.names(), ["Project Atlas.txt"]);

    // The next search, in another folder, is everywhere and by name.
    ui.go_to(&fx.dir);
    ui.click("search");
    ui.type_text("notes");
    ui.click("scope-computer");
    ui.click("by-name");
    assert_eq!(ui.names(), ["notes.txt"]);

    ui.press(&cmd("["));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(search(&ui).0, "");
    ui.press(&cmd("["));
    assert_eq!(ui.folder(), Some(projects.clone()));
    assert_eq!(
        search(&ui),
        ("atlas".to_string(), SearchScope::Folder, true),
        "in Projects only, and inside files"
    );
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());

    ui.press(&cmd("]"));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(search(&ui).0, "");
    ui.press(&cmd("]"));
    assert_eq!(
        search(&ui),
        ("notes".to_string(), SearchScope::Computer, false)
    );
    assert_eq!(ui.names(), ["notes.txt"]);
}

#[gpui_kit::test]
fn a_search_comes_back_in_place_of_one_for_the_same_words(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-history-same-words");
    let mut ui = Ui::open(cx, &fx.dir);
    let search = |ui: &Ui| ui.read(|f| (f.query().to_string(), f.search_scope()));
    ui.go_to(&fx.path("Projects"));
    ui.click("search");
    ui.type_text("road");
    assert_eq!(ui.names(), ["Roadmap.md"]);

    // Back ends the search. The same words typed again look only in the
    // folder this time, which doesn't have the file.
    ui.press(&cmd("["));
    ui.type_text("road");
    ui.click("scope-folder");
    assert_eq!(search(&ui), ("road".to_string(), SearchScope::Folder));
    let found = ui.names();
    assert!(!found.contains(&"Roadmap.md".to_string()), "{found:?}");

    // Forward is the search Back ended. Its words are in the field already,
    // but it looked in This Computer, and it runs there again.
    ui.press(&cmd("]"));
    assert_eq!(search(&ui), ("road".to_string(), SearchScope::Computer));
    assert_eq!(ui.names(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn back_returns_to_a_search_left_without_leaving_its_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-history-reveal");
    let mut ui = Ui::open(cx, &fx.dir);
    let atlas = fx.path("Projects/atlas");
    let shown = |ui: &Ui| (ui.folder(), ui.read(|f| f.query().to_string()));
    let here = |query: &str| (Some(atlas.clone()), query.to_string());
    ui.go_to(&atlas);
    ui.click("search");
    ui.type_text("atlas");
    assert!(ui.names().len() > 1, "{:?}", ui.names());

    // The result is in the folder the window is in already.
    ui.right_click(item_id("tile", &atlas.join("Project Atlas.txt")));
    ui.click_menu_item("Show in Enclosing Folder");
    assert_eq!(shown(&ui), here(""));
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);

    // The folder is one of its own results here, but not one that Back came
    // from, so it isn't selected.
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), here("atlas"));
    let found = ui.names();
    assert!(found.contains(&"atlas".to_string()), "{found:?}");
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
    // Forward, by its button here, is the folder without the search again.
    ui.click("forward");
    assert_eq!(shown(&ui), here(""));
    assert_eq!(ui.names(), ["Project Atlas.txt"]);

    // The same for going to the folder itself, as by its row in the sidebar.
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), here("atlas"));
    ui.go_to(&atlas);
    assert_eq!(shown(&ui), here(""));
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), here("atlas"));

    // Cmd-Up from a search goes somewhere new, and Back from there is the
    // search again.
    ui.press(&cmd("up"));
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert_eq!(ui.selected(), ["atlas"]);
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), here("atlas"));
    assert!(ui.names().len() > 1, "{:?}", ui.names());
}

#[gpui_kit::test]
fn cmd_return_shows_the_selected_result_in_its_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("reveal-key");
    let mut ui = Ui::open(cx, &fx.dir);
    let projects = fx.path("Projects");
    let shown = |ui: &Ui| (ui.folder(), ui.read(|f| f.query().to_string()));
    ui.go_to(&projects);

    // The keyboard does all of it: the words, Return into the results, and
    // Cmd-Return on the one that is selected there.
    ui.press(&cmd("f"));
    ui.type_text("road");
    ui.press("enter");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    ui.press(&cmd("enter"));
    assert_eq!(
        shown(&ui),
        (Some(fx.dir.clone()), String::new()),
        "the file's folder, and no search"
    );
    assert_eq!(
        ui.names(),
        ["Budget.csv", "notes.txt", "Projects", "Roadmap.md"]
    );
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert!(ui.files_focused(), "the arrow keys go on from the file");
    assert!(ui.system_calls().is_empty(), "the file wasn't opened");

    // Back is the search again, as it is after the menu's item.
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), (Some(projects), "road".to_string()));
    assert_eq!(ui.names(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn cmd_return_selects_the_result_in_its_folders_column(cx: &mut TestAppContext) {
    let fx = Fixture::new("reveal-key-columns");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("3"));
    ui.go_to(&fx.path("Projects"));

    // The results are a list in this view too, and the folder the key leads
    // to is in columns again.
    ui.press(&cmd("f"));
    ui.type_text("road");
    ui.press("enter");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    ui.press(&cmd("enter"));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(ui.read(|f| f.effective_view()), ViewMode::Columns);
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert!(
        ui.read(|f| f.has_preview_for_test(&fx.path("Roadmap.md"))),
        "with its preview beside the column"
    );
}

#[gpui_kit::test]
fn cmd_return_shows_an_item_of_recents_or_of_sent_results_in_its_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("reveal-key-lists");
    let mut ui = Ui::open(cx, &fx.dir);
    let atlas = fx.path("Projects/atlas");

    ui.press(&cmd("shift-f"));
    ui.select("Project Atlas.txt");
    ui.press(&cmd("enter"));
    assert_eq!(ui.folder(), Some(atlas.clone()));
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
    ui.press(&cmd("["));
    assert_eq!(ui.read(|f| f.location().clone()), Location::Recents);

    let sent = vec![fx.path("Roadmap.md"), atlas.join("Project Atlas.txt")];
    ui.update(|f, window, cx| f.show_results("Sent".into(), sent, window, cx));
    ui.select("Project Atlas.txt");
    ui.press(&cmd("enter"));
    assert_eq!(ui.folder(), Some(atlas));
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
}

#[gpui_kit::test]
fn cmd_return_does_nothing_in_a_folder_or_with_two_results_selected(cx: &mut TestAppContext) {
    let fx = Fixture::new("reveal-key-nothing");
    let mut ui = Ui::open(cx, &fx.dir);

    // An item in a folder is shown in its folder already. In the columns
    // that goes for an item further in too, whose folder is a column there.
    ui.select("Roadmap.md");
    ui.press(&cmd("enter"));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert!(ui.system_calls().is_empty(), "the key doesn't open it");
    ui.press(&cmd("3"));
    ui.update(|f, window, cx| f.select_in_column(0, 2, window, cx));
    ui.press("right");
    assert_eq!(ui.selected(), ["atlas"]);
    ui.press(&cmd("enter"));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(ui.selected(), ["atlas"]);

    // Two results are in two folders.
    ui.press(&cmd("f"));
    ui.type_text("atlas");
    ui.select("Project Atlas.txt");
    ui.select_also("atlas");
    ui.press(&cmd("enter"));
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(ui.selected().len(), 2);
}

#[gpui_kit::test]
fn cmd_return_in_the_trash_acts_only_on_a_result_of_a_search(cx: &mut TestAppContext) {
    let fx = Fixture::new("reveal-key-trash");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.select("notes.txt");
    ui.press(&cmd("backspace"));
    ui.update(|f, window, cx| f.navigate(Location::Trash, window, cx));

    // The Trash keeps its items in a folder that isn't one to browse.
    ui.select("notes.txt");
    ui.press(&cmd("enter"));
    assert_eq!(ui.read(|f| f.location().clone()), Location::Trash);
    assert_eq!(ui.selected(), ["notes.txt"]);

    // A search made there finds items where they are, like any other.
    ui.press(&cmd("f"));
    ui.type_text("road");
    ui.press("enter");
    ui.press(&cmd("enter"));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn cmd_return_leaves_an_item_of_the_trash_alone_while_the_next_place_is_read(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("reveal-key-stale");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.select("notes.txt");
    ui.press(&cmd("backspace"));
    ui.update(|f, window, cx| f.navigate(Location::Trash, window, cx));

    // A place's items stay on screen until the place gone to next has read
    // its own, which Recents does only once the index is ready. Recents is
    // a list the key works in, and what's selected is still the Trash's.
    ui.press_without_settling(&cmd("shift-f"));
    ui.press_without_settling("down");
    assert_eq!(
        ui.selected(),
        ["notes.txt"],
        "the Trash's item, still listed"
    );
    ui.press(&cmd("enter"));
    assert_eq!(ui.read(|f| f.location().clone()), Location::Recents);
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());
}

#[gpui_kit::test]
fn cmd_return_in_a_file_chooser_shows_a_result_and_chooses_nothing(cx: &mut TestAppContext) {
    let fx = Fixture::new("reveal-key-chooser");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            picker: Some(PickerOptions { multiple: false }),
            ..Default::default()
        },
    );
    ui.go_to(&fx.path("Projects"));
    ui.click("search");
    ui.type_text("road");
    ui.press("down");
    assert_eq!(ui.selected(), ["Roadmap.md"]);

    // Return here would choose the file, which ends the chooser and the
    // process with it.
    ui.press(&cmd("enter"));
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert!(ui.exists("picker-choose"), "still choosing");
}

#[gpui_kit::test]
fn a_smart_folder_opened_twice_is_one_step_back_from_its_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-history-smart");
    let mut ui = Ui::open(cx, &fx.dir);
    let projects = fx.path("Projects");
    let shown = |ui: &Ui| (ui.folder(), ui.read(|f| f.query().to_string()));
    let saved = SmartFolder {
        name: "Atlas".into(),
        query: "atlas".into(),
        kind: KindFilter::Any,
        scope: Some(projects.clone()),
        contents: false,
        filter: None,
    };
    ui.update(|f, _, cx| f.add_smart_folder(saved, cx));
    ui.click(("smart", 0usize));
    assert_eq!(shown(&ui), (Some(projects.clone()), "atlas".into()));

    // It's the same search in the same folder again, not one more to go
    // back through.
    ui.click(("smart", 0usize));
    assert_eq!(shown(&ui), (Some(projects.clone()), "atlas".into()));
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), (Some(projects), String::new()));
    ui.press(&cmd("["));
    assert_eq!(shown(&ui), (Some(fx.dir.clone()), String::new()));
}

#[gpui_kit::test]
fn results_show_what_the_index_finds_changed(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-index");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("atlas");
    ui.select("Project Atlas.txt");

    std::fs::write(fx.path("atlas-two.txt"), "x").unwrap();
    ui.rescan();
    // One look sees that the index changed, and the next that it has
    // stopped changing.
    ui.look_for_changes(2);
    let found = ui.names();
    assert!(found.contains(&"atlas-two.txt".to_string()), "{found:?}");
    assert_eq!(
        ui.selected(),
        ["Project Atlas.txt"],
        "what was selected still is"
    );

    std::fs::remove_file(fx.path("atlas-two.txt")).unwrap();
    ui.rescan();
    ui.look_for_changes(2);
    let found = ui.names();
    assert!(!found.contains(&"atlas-two.txt".to_string()), "{found:?}");
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
}

/// Whether the window lists an item named `name`.
fn lists(ui: &Ui, name: &str) -> bool {
    ui.names().iter().any(|listed| listed == name)
}

#[gpui_kit::test]
fn results_follow_the_files_the_index_sees_come_and_go(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-files");
    std::fs::write(fx.path("Projects/atlas/Atlas Notes.txt"), "x").unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("atlas");
    ui.select("Atlas Notes.txt");

    // A file the search would list is new, and what the index reports are
    // other files, which the search can't list.
    std::fs::write(fx.path("atlas-two.txt"), "x").unwrap();
    ui.reindex();
    ui.report_change(None, Some(&fx.path("Budget 2.csv")));
    ui.look_for_changes(2);
    ui.report_change(None, Some(&fx.path("Budget 3.csv")));
    ui.look_for_changes(2);
    assert!(
        !lists(&ui, "atlas-two.txt"),
        "the search doesn't run again for a file that is none of its business"
    );

    // Had it run again for those two, it would be held back for three looks.
    ui.report_change(None, Some(&fx.path("atlas-two.txt")));
    ui.look_for_changes(1);
    assert!(
        !lists(&ui, "atlas-two.txt"),
        "it waits for a look at which no more files have come"
    );
    ui.look_for_changes(1);
    assert!(lists(&ui, "atlas-two.txt"), "{:?}", ui.names());
    assert_eq!(ui.selected(), ["Atlas Notes.txt"]);

    // A result's file is renamed by another program.
    let (was, now) = (fx.path("atlas-two.txt"), fx.path("atlas-2.txt"));
    std::fs::rename(&was, &now).unwrap();
    ui.reindex();
    ui.report_change(Some(&was), Some(&now));
    ui.look_for_changes(2);
    assert!(lists(&ui, "atlas-2.txt"), "{:?}", ui.names());
    assert!(!lists(&ui, "atlas-two.txt"));

    // And one is removed. The search runs again for it because it lists
    // the file, whatever the file's name.
    let gone = fx.path("Projects/atlas/Project Atlas.txt");
    std::fs::remove_file(&gone).unwrap();
    ui.reindex();
    ui.report_change(Some(&gone), None);
    ui.look_for_changes(2);
    assert_eq!(
        {
            let mut names = ui.names();
            names.sort();
            names
        },
        ["Atlas Notes.txt", "atlas", "atlas-2.txt"]
    );
    assert_eq!(ui.selected(), ["Atlas Notes.txt"]);
}

#[gpui_kit::test]
fn a_result_renamed_here_isnt_searched_for_again(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-own-rename");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.press(&cmd("f"));
    ui.type_text("atlas");

    // A result is renamed in the window, twice, and the window lists it
    // under its new name at once. A moment later the index reports it.
    let folder = fx.path("Projects/atlas");
    let names = ["Project Atlas.txt", "Atlas Plan.txt", "Atlas Plans.txt"];
    for renamed in names.windows(2) {
        let (was, now) = (folder.join(renamed[0]), folder.join(renamed[1]));
        ui.click(item_id("row", &was));
        ui.press("f2");
        ui.type_text(renamed[1].trim_end_matches(".txt"));
        ui.press("enter");
        assert!(lists(&ui, renamed[1]), "{:?}", ui.names());
        ui.reindex();
        ui.report_change(Some(&was), Some(&now));
        ui.look_for_changes(2);
    }

    // Had the search run again for each of the two and come out as it was,
    // it would be held back for three looks.
    std::fs::write(fx.path("atlas-two.txt"), "x").unwrap();
    ui.reindex();
    ui.report_change(None, Some(&fx.path("atlas-two.txt")));
    ui.look_for_changes(2);
    assert!(lists(&ui, "atlas-two.txt"), "{:?}", ui.names());
    assert!(lists(&ui, "Atlas Plans.txt"));
}

#[gpui_kit::test]
fn results_stay_when_what_changed_isnt_among_them(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-none");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("atlas");
    let before = ui.names();

    // The index reports a file with the search's words in its name, so the
    // search runs again. The file is gone by then. Another match that the
    // answer has, which no report was of, stands for the difference between
    // two answers to one search that many equal matches make: it isn't shown.
    std::fs::write(fx.path("atlas-two.txt"), "x").unwrap();
    ui.reindex();
    ui.report_change(None, Some(&fx.path("atlas-gone.txt")));
    ui.look_for_changes(2);
    assert_eq!(ui.names(), before);

    // Changes that leave the results as they were are followed less closely
    // each time: after the second, the search isn't run again for 3 looks.
    ui.report_change(None, Some(&fx.path("atlas-gone.txt")));
    ui.look_for_changes(2);
    assert_eq!(ui.names(), before);
    ui.report_change(None, Some(&fx.path("atlas-two.txt")));
    ui.look_for_changes(3);
    assert_eq!(ui.names(), before, "not yet");
    ui.look_for_changes(1);
    assert!(lists(&ui, "atlas-two.txt"), "{:?}", ui.names());

    // Results that changed are followed closely again.
    std::fs::write(fx.path("atlas-three.txt"), "x").unwrap();
    ui.reindex();
    ui.report_change(None, Some(&fx.path("atlas-three.txt")));
    ui.look_for_changes(2);
    assert!(lists(&ui, "atlas-three.txt"), "{:?}", ui.names());
}

#[gpui_kit::test]
fn a_search_held_back_is_followed_closely_again_after_a_quiet_minute(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-after-quiet");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("atlas");
    let before = ui.names();

    // Four times over, the index reports a file with the search's words
    // that is gone by the time the search runs again. The search comes out
    // as it was each time, and is held back for 1, 3, 7 and then 15 looks.
    std::fs::write(fx.path("atlas-two.txt"), "x").unwrap();
    ui.reindex();
    let gone = fx.path("atlas-gone.txt");
    for looks in [2, 2, 4, 8] {
        ui.report_change(None, Some(&gone));
        ui.look_for_changes(looks);
    }
    assert_eq!(ui.names(), before);

    // A minute passes with nothing that is the search's business. The next
    // such file holds the search back for one look again, not for 31.
    ui.look_for_changes(30);
    ui.report_change(None, Some(&gone));
    ui.look_for_changes(2);
    assert_eq!(ui.names(), before);
    ui.report_change(None, Some(&fx.path("atlas-two.txt")));
    ui.look_for_changes(2);
    assert!(lists(&ui, "atlas-two.txt"), "{:?}", ui.names());
}

#[gpui_kit::test]
fn results_wait_half_a_minute_at_most_for_files_to_stop_coming(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-stream");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("atlas");

    // Files the search would list come one at every look, as they do while
    // a folder is being copied. The index has them; its reports follow.
    let coming: Vec<_> = (1..=16)
        .map(|n| fx.path(&format!("atlas {n}.txt")))
        .collect();
    for file in &coming {
        std::fs::write(file, "x").unwrap();
    }
    ui.reindex();
    for file in &coming[..15] {
        ui.report_change(None, Some(file));
        ui.look_for_changes(1);
    }
    assert!(
        !lists(&ui, "atlas 1.txt"),
        "it waits for a look at which no more files have come"
    );
    ui.report_change(None, Some(&coming[15]));
    ui.look_for_changes(1);
    assert!(lists(&ui, "atlas 1.txt"), "{:?}", ui.names());
    assert!(lists(&ui, "atlas 16.txt"), "{:?}", ui.names());

    // The files that come after that are waited for from the start.
    std::fs::write(fx.path("atlas 17.txt"), "x").unwrap();
    ui.reindex();
    ui.report_change(None, Some(&fx.path("atlas 17.txt")));
    ui.look_for_changes(1);
    assert!(!lists(&ui, "atlas 17.txt"), "{:?}", ui.names());
    ui.look_for_changes(1);
    assert!(lists(&ui, "atlas 17.txt"), "{:?}", ui.names());
}

#[gpui_kit::test]
fn a_search_catches_up_a_minute_after_files_stop_coming(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-after-stream");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("atlas");

    // For some minutes, a file that the search could list is reported at
    // every look, and is gone by the time the search runs again. The search
    // comes out as it was each time, so it runs less and less often: after
    // the sixth time, not for 63 looks.
    std::fs::write(fx.path("atlas-two.txt"), "x").unwrap();
    ui.reindex();
    let gone = fx.path("atlas-gone.txt");
    for _ in 0..112 {
        ui.report_change(None, Some(&gone));
        ui.look_for_changes(1);
    }
    assert!(!lists(&ui, "atlas-two.txt"), "{:?}", ui.names());

    // The last file to come is one that is there. A minute after it, the
    // search has caught up.
    ui.report_change(None, Some(&fx.path("atlas-two.txt")));
    ui.look_for_changes(31);
    assert!(lists(&ui, "atlas-two.txt"), "{:?}", ui.names());
}

#[gpui_kit::test]
fn a_search_of_one_folder_follows_that_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-scope");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.update(|f, _, cx| f.set_scope(SearchScope::Folder, cx));
    ui.press(&cmd("f"));
    ui.type_text("atlas");

    std::fs::write(fx.path("Projects/atlas-inside.txt"), "x").unwrap();
    std::fs::write(fx.path("atlas-outside.txt"), "x").unwrap();
    ui.reindex();
    ui.report_change(None, Some(&fx.path("atlas-outside.txt")));
    ui.look_for_changes(3);
    assert!(
        !lists(&ui, "atlas-inside.txt"),
        "a file outside the folder is none of the search's business"
    );
    ui.report_change(None, Some(&fx.path("Projects/atlas-inside.txt")));
    ui.look_for_changes(2);
    assert!(lists(&ui, "atlas-inside.txt"), "{:?}", ui.names());
    assert!(!lists(&ui, "atlas-outside.txt"));
}

#[gpui_kit::test]
fn a_search_run_again_for_the_index_shows_no_sign_of_running(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-quietly");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("atlas");

    std::fs::write(fx.path("atlas-two.txt"), "x").unwrap();
    ui.rescan();
    ui.look_for_changes(1);
    // A window that comes to the front looks at once, and at this look the
    // search runs again. It is followed a step at a time until its answer
    // shows, with no time passing.
    ui.window(|window, _| window.activate_window());
    let mut steps = 0;
    while !lists(&ui, "atlas-two.txt") {
        assert!(ui.cx.executor().tick(), "the search didn't run again");
        assert!(
            !ui.read(|f| f.search_in_flight_for_test()),
            "no spinner, and Return wouldn't wait"
        );
        steps += 1;
    }
    assert!(steps > 1, "the answer took its time: {steps} steps");
}

#[gpui_kit::test]
fn results_follow_what_a_watched_index_reports(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-watcher");
    let options = Open {
        watch: true,
        ..Default::default()
    };
    let mut ui = Ui::open_with(cx, &fx.dir, options);
    let engine = ui.engine();
    // The watcher answers in its own time, as it does for the app, so each
    // step here is tried until it has: up to ten seconds.
    let pause = || std::thread::sleep(std::time::Duration::from_millis(50));

    // The index starts watching a moment after it's finished, and the window
    // asks it to report from then on. A file made after that is counted.
    // The window asks every 400 ms for half a minute, so the clock goes no
    // faster than lets it keep asking through all of the ten seconds.
    let counted = (0..200).any(|n| {
        std::fs::write(fx.path(&format!("made {n}.txt")), "x").unwrap();
        pause();
        ui.cx
            .executor()
            .advance_clock(std::time::Duration::from_millis(100));
        ui.cx.run_until_parked();
        engine.changes().0 > 0
    });
    assert!(
        counted,
        "the window didn't have the index report its changes"
    );

    ui.press(&cmd("f"));
    ui.type_text("atlas");
    assert!(!lists(&ui, "atlas-two.txt"));
    std::fs::write(fx.path("atlas-two.txt"), "x").unwrap();
    let listed = (0..200).any(|_| {
        pause();
        ui.look_for_changes(1);
        lists(&ui, "atlas-two.txt")
    });
    assert!(listed, "{:?}", ui.names());

    std::fs::remove_file(fx.path("atlas-two.txt")).unwrap();
    let gone = (0..200).any(|_| {
        pause();
        ui.look_for_changes(1);
        !lists(&ui, "atlas-two.txt")
    });
    assert!(gone, "{:?}", ui.names());

    // Recents hears of a new file the same way.
    ui.update(|f, window, cx| f.navigate(Location::Recents, window, cx));
    assert!(!lists(&ui, "just made.txt"));
    std::fs::write(fx.path("just made.txt"), "x").unwrap();
    let listed = (0..200).any(|_| {
        pause();
        ui.look_for_changes(1);
        lists(&ui, "just made.txt")
    });
    assert!(listed, "{:?}", ui.names());
}

#[gpui_kit::test]
fn a_contents_search_is_left_to_refresh(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-follows-contents");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("f"));
    ui.type_text("atlas");
    ui.click("by-contents");
    assert_eq!(ui.names(), ["Project Atlas.txt"]);

    std::fs::write(fx.path("more.txt"), "atlas again").unwrap();
    ui.rescan();
    ui.look_for_changes(3);
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
    ui.press(&cmd("r"));
    assert!(lists(&ui, "more.txt"), "{:?}", ui.names());
}

#[gpui_kit::test]
fn recents_shows_what_the_index_finds_changed(cx: &mut TestAppContext) {
    let fx = Fixture::new("recents-follows-index");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-f"));
    assert!(lists(&ui, "notes.txt"), "{:?}", ui.names());
    ui.select("notes.txt");

    std::fs::write(fx.path("just made.txt"), "x").unwrap();
    ui.rescan();
    ui.look_for_changes(2);
    assert!(lists(&ui, "just made.txt"), "{:?}", ui.names());
    assert_eq!(ui.selected(), ["notes.txt"]);

    // In a folder, what the index counts is nobody's business.
    ui.go_to(&fx.path("Projects"));
    ui.rescan();
    ui.look_for_changes(3);
    assert_eq!(ui.names(), ["atlas"]);
}

/// Have the fixture's file `name` last written to `minutes` ago.
fn written_ago(fx: &Fixture, name: &str, minutes: u64) {
    let ago = std::time::Duration::from_secs(minutes * 60);
    let file = std::fs::File::options()
        .write(true)
        .open(fx.path(name))
        .unwrap();
    file.set_modified(std::time::SystemTime::now() - ago)
        .unwrap();
}

#[gpui_kit::test]
fn recents_follows_the_files_that_are_written_to(cx: &mut TestAppContext) {
    let fx = Fixture::new("recents-follows-writes");
    // Each file was last written to so many minutes ago.
    let written = |name: &str, minutes: u64| written_ago(&fx, name, minutes);
    written("notes.txt", 10);
    written("Roadmap.md", 20);
    written("Budget.csv", 30);
    written("Projects/atlas/Project Atlas.txt", 40);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-f"));
    let newest_first = ["notes.txt", "Roadmap.md", "Budget.csv", "Project Atlas.txt"];
    assert_eq!(ui.names(), newest_first);
    let (budget, roadmap) = (fx.path("Budget.csv"), fx.path("Roadmap.md"));

    // A report of a file needn't be of a write: macOS notes with a file
    // when it was last opened, and that is reported like one. Recents is
    // read again and comes out as it was.
    ui.report_change(Some(&budget), Some(&budget));
    ui.look_for_changes(1);
    assert_eq!(ui.names(), newest_first);

    // Then the file is written to, with nothing else reported in between.
    written("Budget.csv", 5);
    ui.reindex();
    ui.report_change(Some(&budget), Some(&budget));
    ui.look_for_changes(2);
    assert_eq!(ui.names()[0], "Budget.csv", "{:?}", ui.names());

    // Written to again it stays first, so Recents isn't read again. A file
    // that no report was of stands for what reading it would show.
    std::fs::write(fx.path("unseen.txt"), "x").unwrap();
    written("unseen.txt", 50);
    ui.reindex();
    ui.report_change(Some(&budget), Some(&budget));
    ui.look_for_changes(3);
    assert!(!lists(&ui, "unseen.txt"), "{:?}", ui.names());
    ui.report_change(None, Some(&fx.path("unseen.txt")));
    ui.look_for_changes(2);
    assert!(lists(&ui, "unseen.txt"), "{:?}", ui.names());

    // A file that is being written to isn't waited for, as files that come
    // one after the other are: a download can take long.
    let notes = fx.path("notes.txt");
    written("notes.txt", 2);
    ui.reindex();
    ui.report_change(Some(&notes), Some(&notes));
    ui.look_for_changes(1);
    assert_eq!(ui.names()[0], "notes.txt", "{:?}", ui.names());

    // A report that leaves Recents as it was holds it back for a look. Once
    // that look has passed with nothing to catch up with, the next such
    // report holds it back for one look again, not for three.
    ui.report_change(Some(&roadmap), Some(&roadmap));
    ui.look_for_changes(3);
    ui.report_change(Some(&roadmap), Some(&roadmap));
    ui.look_for_changes(1);
    written("Roadmap.md", 1);
    ui.reindex();
    ui.report_change(Some(&roadmap), Some(&roadmap));
    ui.look_for_changes(1);
    assert_eq!(ui.names()[0], "notes.txt", "held back for a look");
    ui.look_for_changes(1);
    assert_eq!(ui.names()[0], "Roadmap.md", "{:?}", ui.names());
}

#[gpui_kit::test]
fn recents_previews_a_file_as_it_was_last_written(cx: &mut TestAppContext) {
    let fx = Fixture::new("recents-preview");
    let written = |name: &str, minutes: u64| written_ago(&fx, name, minutes);
    written("notes.txt", 10);
    written("Roadmap.md", 20);
    written("Budget.csv", 30);
    written("Projects/atlas/Project Atlas.txt", 40);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-f"));
    ui.press(&cmd("shift-p"));
    let roadmap = fx.path("Roadmap.md");
    let text = |ui: &Ui| ui.read(|f| f.preview_text_for_test(&roadmap));
    ui.select("Roadmap.md");
    assert_eq!(text(&ui).as_deref(), Some("# Roadmap\n"));

    // Written to, which puts it first once the index reports it.
    std::fs::write(&roadmap, "# Roadmap\n\nShip it.\n").unwrap();
    written("Roadmap.md", 5);
    ui.reindex();
    ui.report_change(Some(&roadmap), Some(&roadmap));
    ui.look_for_changes(2);
    assert_eq!(ui.names()[0], "Roadmap.md", "{:?}", ui.names());
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert_eq!(text(&ui).as_deref(), Some("# Roadmap\n\nShip it.\n"));
}

#[gpui_kit::test]
fn recents_waits_half_a_minute_at_most_for_writing_to_stop(cx: &mut TestAppContext) {
    let fx = Fixture::new("recents-follows-two");
    let written = |name: &str, minutes: u64| written_ago(&fx, name, minutes);
    written("notes.txt", 10);
    written("Roadmap.md", 20);
    written("Budget.csv", 30);
    written("Projects/atlas/Project Atlas.txt", 40);
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-f"));
    let newest_first = ["notes.txt", "Roadmap.md", "Budget.csv", "Project Atlas.txt"];
    assert_eq!(ui.names(), newest_first);

    // Two files are written to in turn, as two downloads are, so every look
    // brings word of both. The index has them as its newest; reports follow.
    let (budget, roadmap) = (fx.path("Budget.csv"), fx.path("Roadmap.md"));
    written("Budget.csv", 2);
    written("Roadmap.md", 1);
    ui.reindex();
    let both_are_written_to = |ui: &mut Ui| {
        ui.report_change(Some(&budget), Some(&budget));
        ui.report_change(Some(&roadmap), Some(&roadmap));
        ui.look_for_changes(1);
    };
    for _ in 0..15 {
        both_are_written_to(&mut ui);
    }
    assert_eq!(
        ui.names(),
        newest_first,
        "it waits for a look at which the writing has stopped"
    );
    both_are_written_to(&mut ui);
    assert_eq!(
        ui.names(),
        ["Roadmap.md", "Budget.csv", "notes.txt", "Project Atlas.txt"]
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
    ui.press_without_settling("enter");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(!ui.files_focused());

    ui.settle();
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
    assert!(ui.files_focused(), "the results came in");
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
}

#[gpui_kit::test]
fn return_goes_on_waiting_through_a_switch_to_contents(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-then-contents");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_without_settling("plain");
    ui.press_without_settling("enter");
    // The click leaves the keyboard in the field, where Return still waits,
    // now for the files that have the word in them.
    ui.click_without_settling("by-contents");
    assert!(!ui.files_focused());
    assert!(ui.selected().is_empty());

    ui.settle();
    assert_eq!(ui.names(), ["notes.txt"]);
    assert!(ui.files_focused(), "the results came in");
    assert_eq!(ui.selected(), ["notes.txt"]);
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
    ui.window(|window, cx| {
        view.update(cx, |f, cx| {
            f.open_request(None, Some("note"), false, window, cx)
        })
    });
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
fn return_moves_into_the_results_of_its_own_window_with_quick_look_open(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-return-quick-look");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.select("notes.txt");
    ui.press("space");
    assert!(ui.read(|f| f.quick_look_window()).is_some());
    ui.click("search");
    ui.type_text("roadmap");
    assert_eq!(ui.names(), ["Roadmap.md"]);

    // Quick Look's window reads this browser too, to draw the preview, and
    // it's the last of the two to have drawn when Return arrives.
    ui.press_after_quick_look_draws("enter");
    ui.settle();
    assert!(
        ui.files_focused(),
        "the keyboard moved in the window Return was pressed in"
    );
    assert_eq!(ui.selected(), ["Roadmap.md"]);

    // Quick Look's window kept its own keyboard, so Escape still closes it.
    ui.in_quick_look(|ui| ui.press("escape"));
    assert!(!ui.read(|f| f.quick_look_is_open()));
}

#[gpui_kit::test]
fn return_waits_for_the_results_in_its_own_window_with_quick_look_open(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-early-return-quick-look");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.select("notes.txt");
    ui.press("space");
    assert!(ui.read(|f| f.quick_look_window()).is_some());
    ui.click("search");
    ui.type_without_settling("roadmap");
    ui.press_after_quick_look_draws("enter");
    assert!(!ui.files_focused(), "the results aren't in yet");

    ui.settle();
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(
        ui.files_focused(),
        "the results came in, and the keyboard moved in the window Return was pressed in"
    );
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn return_goes_on_waiting_for_the_results_when_quick_look_closes(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-early-return-quick-look-gone");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.select("notes.txt");
    ui.press("space");
    ui.click("search");
    ui.type_without_settling("roadmap");
    ui.press_after_quick_look_draws("enter");

    // The wait is for the browser's window, so Quick Look's can go.
    ui.close_quick_look();
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(ui.files_focused(), "the results came in");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
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
    ui.update(|f, window, cx| f.open_request(Some(&projects), Some("atlas"), false, window, cx));
    assert_eq!(ui.folder(), Some(projects));
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    let report = ui.read(|f| f.selection_json(true));
    assert_eq!(report["location"], "search");
    assert_eq!(report["query"], "atlas");

    // A file opens its folder with the file selected.
    let roadmap = fx.path("Roadmap.md");
    ui.update(|f, window, cx| f.open_request(Some(&roadmap), None, false, window, cx));
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(ui.selected(), ["Roadmap.md"]);
}

#[gpui_kit::test]
fn a_search_sent_with_here_stays_in_the_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("open-here");
    let mut ui = Ui::open(cx, &fx.dir);
    let scope = |ui: &Ui| ui.read(|f| f.search_scope());
    let projects = fx.path("Projects");

    // Without it, the folder is only where the window goes.
    ui.update(|f, window, cx| f.open_request(Some(&projects), Some("budget"), false, window, cx));
    assert_eq!(scope(&ui), SearchScope::Computer);
    assert_eq!(ui.names(), ["Budget.csv"]);

    ui.update(|f, window, cx| f.open_request(Some(&projects), Some("budget"), true, window, cx));
    assert_eq!(ui.folder(), Some(projects.clone()));
    assert_eq!(scope(&ui), SearchScope::Folder);
    assert!(ui.names().is_empty(), "Budget.csv is outside Projects");
    assert!(ui.exists("scope-folder"));

    ui.update(|f, window, cx| f.open_request(Some(&projects), Some("atlas"), true, window, cx));
    let names = ui.names();
    assert!(names.iter().any(|n| n == "Project Atlas.txt"), "{names:?}");

    // It stays on, as when the button is clicked, until it's clicked again.
    ui.update(|f, window, cx| f.open_request(Some(&projects), Some("budget"), false, window, cx));
    assert_eq!(scope(&ui), SearchScope::Folder);
    assert!(ui.names().is_empty());
    ui.click("search-scope");
    assert_eq!(ui.names(), ["Budget.csv"]);
}

#[gpui_kit::test]
fn here_narrows_a_search_that_is_sent_again_with_the_same_words(cx: &mut TestAppContext) {
    let fx = Fixture::new("open-here-same-words");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.click("search");
    ui.type_text("budget");
    assert_eq!(ui.names(), ["Budget.csv"]);

    ui.update(|f, window, cx| f.open_request(None, Some("budget"), true, window, cx));
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Folder);
    assert!(
        ui.names().is_empty(),
        "the same words, now only in Projects"
    );
}

#[gpui_kit::test]
fn here_is_the_folder_the_window_goes_to(cx: &mut TestAppContext) {
    let fx = Fixture::new("open-here-recents");
    let mut ui = Ui::open(cx, &fx.dir);
    let scope = |ui: &Ui| ui.read(|f| f.search_scope());

    // Recents isn't a folder, so there is nothing to search only in.
    ui.update(|f, window, cx| f.navigate(Location::Recents, window, cx));
    ui.update(|f, window, cx| f.open_request(None, Some("budget"), true, window, cx));
    assert_eq!(scope(&ui), SearchScope::Computer);
    assert_eq!(ui.names(), ["Budget.csv"]);

    let projects = fx.path("Projects");
    ui.update(|f, window, cx| f.open_request(Some(&projects), Some("budget"), true, window, cx));
    assert_eq!(scope(&ui), SearchScope::Folder);
    assert!(ui.names().is_empty(), "Budget.csv is outside Projects");

    // A file opens its folder, and that is the folder searched.
    ui.click("search-scope");
    assert_eq!(ui.names(), ["Budget.csv"], "the button turns it off");
    let file = fx.path("Projects/atlas/Project Atlas.txt");
    ui.update(|f, window, cx| f.open_request(Some(&file), Some("budget"), true, window, cx));
    assert_eq!(ui.folder(), Some(fx.path("Projects/atlas")));
    assert_eq!(scope(&ui), SearchScope::Folder);
    assert!(ui.names().is_empty(), "Budget.csv is outside atlas");
}

#[gpui_kit::test]
fn a_window_opened_with_here_shows_nothing_from_outside_its_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("window-here");
    let mut ui = Ui::open(cx, &fx.dir);
    // A launch with `--here` opens the window searching, and has it search
    // only in its folder before that first search has answered.
    let open = |ui: &mut Ui, words: &str| -> Vec<String> {
        let start = Location::Folder(ui.dir.join("Projects"));
        let finder = ui.cx.update(|cx| {
            let (_, finder) =
                fffinder::open_window(start, Some(words.into()), None, cx).expect("a window");
            finder.update(cx, |f, cx| f.search_here(cx));
            finder
        });
        ui.settle();
        finder.read_with(ui.cx, |f, _| {
            assert_eq!(f.search_scope(), SearchScope::Folder);
            let rows = f.rows_for_test().iter();
            rows.map(|r| r.entry.name.clone()).collect()
        })
    };

    let found = open(&mut ui, "atlas");
    assert!(
        found.contains(&"Project Atlas.txt".to_string()),
        "{found:?}"
    );
    let found = open(&mut ui, "budget");
    assert!(
        found.is_empty(),
        "Budget.csv is outside Projects: {found:?}"
    );
}

#[gpui_kit::test]
fn a_file_chooser_opened_with_here_searches_only_its_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("chooser-here");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            picker: Some(PickerOptions { multiple: false }),
            ..Default::default()
        },
    );
    ui.go_to(&fx.path("Projects"));
    // As after `fff pick --here`, with the words typed in the chooser.
    ui.update(|f, _, cx| f.search_here(cx));
    ui.click("search");
    ui.type_text("budget");
    assert!(ui.names().is_empty(), "Budget.csv is outside Projects");
    ui.click("search-scope");
    assert_eq!(ui.names(), ["Budget.csv"], "the button turns it off");
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
fn quick_look_explores_with_a_nu_chosen_while_it_is_open(cx: &mut TestAppContext) {
    let fx = Fixture::new("explore-chosen");
    // An empty file can't be run, which Quick Look then says.
    let fake = fx.path("fake-nu");
    std::fs::write(&fake, "").unwrap();
    std::fs::write(fx.path("app.json"), r#"{"deps": {"serde": "1"}}"#).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    let crumb = |depth: usize| ("explore-crumb", depth);

    // Without a `nu` there's no data to explore, only the file's text.
    ui.select("app.json");
    ui.press(&cmd("space"));
    ui.in_quick_look(|ui| {
        assert!(!ui.exists(crumb(0)));
        assert!(!ui.exists("explore-failure"));
    });

    // One chosen in Settings is asked for the data at once.
    ui.press(&cmd(","));
    ui.click("choose-nu");
    ui.cx
        .simulate_path_prompt_response(|_| Some(vec![fake.clone()]));
    ui.settle();
    ui.in_quick_look(|ui| assert!(ui.exists("explore-failure")));

    // And so is a real one, where there is one, in place of the one that
    // couldn't open the file.
    ui.click("nu-auto");
    if fffinder::nushell::find_nu(None).is_some() {
        ui.in_quick_look(|ui| {
            assert!(!ui.exists("explore-failure"));
            ui.press("enter");
            assert!(ui.exists(crumb(1)), "Return goes into deps");
        });
    }
}

#[gpui_kit::test]
fn quick_look_explores_a_file_again_once_it_has_changed(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("explore-changed");
    // Half a file, as one still being written is. It's ten minutes old, so
    // that the rest of it comes with another date.
    std::fs::write(fx.path("app.json"), r#"{"deps": "#).unwrap();
    written_ago(&fx, "app.json", 10);
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
    ui.in_quick_look(|ui| assert!(ui.exists("explore-failure")));

    std::fs::write(fx.path("app.json"), r#"{"deps": {"serde": "1"}}"#).unwrap();
    ui.update(|f, _, cx| f.reload_quietly(cx));
    ui.in_quick_look(|ui| {
        assert!(!ui.exists("explore-failure"), "the whole file is data");
        ui.press("enter");
        assert!(ui.exists(crumb(1)), "Return goes into deps");
    });
}

#[gpui_kit::test]
fn quick_look_opens_a_pdf_again_once_it_has_changed(cx: &mut TestAppContext) {
    let fx = Fixture::new("pdf-changed");
    let testdata = |name: &str| {
        let core = concat!(env!("CARGO_MANIFEST_DIR"), "/crates/fffinder-core");
        format!("{core}/testdata/{name}")
    };
    // Ten minutes old, so that the document that takes its place comes
    // with another date.
    std::fs::copy(testdata("pages.pdf"), fx.path("Manual.pdf")).unwrap();
    written_ago(&fx, "Manual.pdf", 10);
    let mut ui = Ui::open(cx, &fx.dir);
    let shown = |ui: &Ui| ui.read(|f| f.shown_pdf_page());
    ui.click("view-list");
    ui.select("Manual.pdf");
    ui.press("space");
    ui.in_quick_look(|ui| {
        ui.press("pagedown");
        assert_eq!(shown(ui), Some((2, 2)));
    });

    // The folder read again with the document as it was: the page stays.
    ui.update(|f, _, cx| f.reload_quietly(cx));
    assert_eq!(shown(&ui), Some((2, 2)));

    std::fs::copy(testdata("twelve.pdf"), fx.path("Manual.pdf")).unwrap();
    ui.update(|f, _, cx| f.reload_quietly(cx));
    assert_eq!(shown(&ui), Some((1, 12)), "the new one, at its first page");
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
