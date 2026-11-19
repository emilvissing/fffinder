//! Every control in the window, driven with real clicks and key presses, with
//! an observable effect checked for each. A step that hangs fails the run
//! (see `common::HANG_LIMIT`).

mod common;

use common::{Fixture, Open, Ui, at, cmd, item_id};
use fffinder::actions::*;
use fffinder::finder::{AppState, PickerOptions, Preview, SearchScope, tildify};
use fffinder::model::Location;
use fffinder::system::SystemCall;
use fffinder_core::terminals::{Launch, Terminal};
use fffinder_core::{Appearance, GroupKey, KindFilter, SmartFolder, SortKey, ViewMode};
use gpui_kit::Action as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::{Focusable as _, KeyContext, TestAppContext};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

// ----- toolbar -------------------------------------------------------------------

#[gpui_kit::test]
fn view_buttons_switch_views(cx: &mut TestAppContext) {
    let fx = Fixture::new("view-buttons");
    let mut ui = Ui::open(cx, &fx.dir);
    for (id, mode) in [
        ("view-list", ViewMode::List),
        ("view-columns", ViewMode::Columns),
        ("view-gallery", ViewMode::Gallery),
        ("view-icons", ViewMode::Icons),
    ] {
        ui.click(id);
        assert_eq!(ui.read(|f| f.effective_view()), mode, "after clicking {id}");
    }
}

#[gpui_kit::test]
fn toolbar_buttons_that_would_do_nothing_are_disabled(cx: &mut TestAppContext) {
    let fx = Fixture::new("disabled-buttons");
    let mut ui = Ui::open(cx, &fx.dir);
    // A disabled button is drawn dimmed, so where these three have nothing
    // to do they have to be disabled, and not only do nothing. A button
    // tells a test when it is disabled, and nothing when it isn't.
    let disabled = |ui: &mut Ui| {
        ["back", "forward", "search-scope"].map(|id| ui.snapshot(id).disabled() == Some(true))
    };
    assert_eq!(disabled(&mut ui), [true, true, false], "in a new window");

    ui.double_click(item_id("tile", &fx.path("Projects")));
    assert_eq!(disabled(&mut ui), [false, true, false], "a folder further");
    ui.click("back");
    assert_eq!(disabled(&mut ui), [true, false, false], "and back again");

    // Recents isn't a folder, so there is none to search only in.
    ui.click("recents");
    assert_eq!(disabled(&mut ui), [false, true, true], "in Recents");
}

#[gpui_kit::test]
fn back_and_forward_buttons(cx: &mut TestAppContext) {
    let fx = Fixture::new("back-forward");
    let mut ui = Ui::open(cx, &fx.dir);
    // With nowhere to go, neither button does anything.
    ui.click("back");
    ui.click("forward");
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));

    ui.double_click(item_id("tile", &fx.path("Projects")));
    assert_eq!(ui.names(), ["atlas"]);
    assert!(ui.read(|f| f.can_go_back()));

    ui.click("back");
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(
        ui.selected(),
        ["Projects"],
        "back selects the folder we left"
    );
    ui.click("forward");
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
}

#[gpui_kit::test]
fn the_back_button_ends_a_search_where_there_is_no_folder_to_go_back_to(cx: &mut TestAppContext) {
    let fx = Fixture::new("back-ends-search");
    let mut ui = Ui::open(cx, &fx.dir);
    let query = |ui: &Ui| ui.read(|f| f.query().to_string());
    ui.click("search");
    ui.type_text("atlas");

    // A disabled button does nothing when it's clicked, so these two say
    // that Back is enabled during the search, and Forward after it.
    ui.click("back");
    assert_eq!(query(&ui), "");
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(ui.names().len(), 4);
    assert!(!ui.read(|f| f.can_go_back()), "the window opened here");
    ui.click("forward");
    assert_eq!(query(&ui), "atlas");
    assert!(
        ui.names().contains(&"Project Atlas.txt".to_string()),
        "{:?}",
        ui.names()
    );
    assert!(ui.read(|f| f.can_go_back()), "back to the folder's items");

    // Forward has nowhere to go from the search, and leaves it alone.
    ui.click("forward");
    assert_eq!(query(&ui), "atlas");
    ui.right_click("forward");
    assert!(!ui.menu_open());
}

#[gpui_kit::test]
fn back_and_forward_buttons_list_their_history(cx: &mut TestAppContext) {
    let fx = Fixture::new("history-menus");
    let mut ui = Ui::open(cx, &fx.dir);
    // With nowhere to go there is nothing to list, and a menu with nothing
    // in it must not take the keyboard from the files.
    for button in ["back", "forward"] {
        ui.right_click(button);
        assert!(!ui.menu_open(), "{button} has no history yet");
        assert!(ui.files_focused(), "after a right-click on {button}");
    }

    ui.go_to(&fx.path("Projects"));
    ui.go_to(&fx.path("Projects/atlas"));
    let start = Location::Folder(fx.dir.clone()).title();
    ui.right_click("back");
    assert_eq!(
        ui.menu_labels(),
        ["Projects", start.as_str()],
        "nearest first"
    );

    ui.click_menu_item(&start);
    assert_eq!(ui.folder(), Some(fx.dir.clone()));
    assert_eq!(
        ui.selected(),
        ["Projects"],
        "selects the folder gone into from here"
    );
    assert!(!ui.read(|f| f.can_go_back()));
    assert!(!ui.menu_open(), "choosing a place closes the menu");
    assert!(ui.files_focused(), "the keyboard is back with the files");

    ui.right_click("forward");
    assert_eq!(ui.menu_labels(), ["Projects", "atlas"]);
    ui.press("escape");
    assert!(!ui.menu_open());
    assert!(ui.files_focused(), "after closing the menu with Escape");
    assert_eq!(ui.folder(), Some(fx.dir.clone()), "Escape goes nowhere");

    ui.right_click("forward");
    ui.click_menu_item("atlas");
    assert_eq!(ui.folder(), Some(fx.path("Projects/atlas")));
    ui.right_click("forward");
    assert!(!ui.menu_open(), "forward is used up");
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn a_history_menu_lists_a_search_as_a_place_of_its_own(cx: &mut TestAppContext) {
    let fx = Fixture::new("history-search");
    let mut ui = Ui::open(cx, &fx.dir);
    let (projects, atlas) = (fx.path("Projects"), fx.path("Projects/atlas"));
    let start = Location::Folder(fx.dir.clone()).title();
    let shown = |ui: &Ui| (ui.folder(), ui.read(|f| f.query().to_string()));
    ui.go_to(&projects);
    ui.click("search");
    ui.type_text("atlas");
    let searching = ui.read(|f| f.title());

    // From a search, the first step back is the folder it was made in.
    ui.right_click("back");
    assert_eq!(ui.menu_labels(), ["Projects", start.as_str()]);
    ui.press("escape");
    assert_eq!(shown(&ui), (Some(projects.clone()), "atlas".into()));

    // A search the window left is a place of its own, named as the window
    // was while it showed.
    ui.double_click(item_id("tile", &atlas));
    ui.right_click("back");
    assert_eq!(
        ui.menu_labels(),
        [searching.as_str(), "Projects", start.as_str()]
    );
    ui.click_menu_item(&searching);
    assert_eq!(shown(&ui), (Some(projects.clone()), "atlas".into()));
    assert_eq!(ui.selected(), ["atlas"], "the folder gone into from here");
    ui.right_click("forward");
    assert_eq!(ui.menu_labels(), ["atlas"]);
    ui.press("escape");

    // Two steps at once pass through the folder without the search, and
    // Forward leads back through both.
    ui.right_click("back");
    assert_eq!(ui.menu_labels(), ["Projects", start.as_str()]);
    ui.click_menu_item(&start);
    assert_eq!(shown(&ui), (Some(fx.dir.clone()), String::new()));
    assert_eq!(ui.selected(), ["Projects"]);
    ui.right_click("forward");
    assert_eq!(ui.menu_labels(), ["Projects", searching.as_str(), "atlas"]);
    ui.click_menu_item(&searching);
    assert_eq!(shown(&ui), (Some(projects.clone()), "atlas".into()));
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());

    // The search's own folder is two steps back from the folder it led to.
    ui.click("forward");
    assert_eq!(shown(&ui), (Some(atlas), String::new()));
    ui.right_click("back");
    ui.click_menu_item("Projects");
    assert_eq!(shown(&ui), (Some(projects), String::new()));
    assert_eq!(ui.names(), ["atlas"]);
}

#[gpui_kit::test]
fn a_history_menu_lists_the_twelve_nearest_places(cx: &mut TestAppContext) {
    let fx = Fixture::new("history-long");
    let mut ui = Ui::open(cx, &fx.dir);
    let stops: Vec<String> = (1..=13).map(|n| format!("stop {n:02}")).collect();
    for stop in &stops {
        std::fs::create_dir(fx.path(stop)).unwrap();
        ui.go_to(&fx.path(stop));
    }

    // Thirteen places lie behind stop 13, the folder the window opened at
    // being the farthest.
    ui.right_click("back");
    let nearest: Vec<&str> = stops[..12].iter().rev().map(String::as_str).collect();
    assert_eq!(ui.menu_labels(), nearest);

    ui.click_menu_item("stop 01");
    assert_eq!(ui.folder(), Some(fx.path("stop 01")), "twelve steps back");
    assert!(ui.read(|f| f.can_go_back()), "one more is left");

    ui.right_click("forward");
    assert_eq!(ui.menu_labels(), stops[1..]);
    ui.click_menu_item("stop 13");
    assert_eq!(ui.folder(), Some(fx.path("stop 13")));
}

#[gpui_kit::test]
fn a_history_menu_goes_only_where_it_says(cx: &mut TestAppContext) {
    let fx = Fixture::new("history-moved-on");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.go_to(&fx.path("Projects/atlas"));
    ui.right_click("back");
    assert_eq!(ui.menu_labels().len(), 2);

    // Keys still reach the window while a menu is open. This one goes back
    // to Projects, which the menu still lists as one step away.
    ui.press("alt-left");
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert!(ui.menu_open(), "the menu is still up");

    ui.click_menu_item("Projects");
    assert_eq!(
        ui.folder(),
        Some(fx.path("Projects")),
        "not a step further back, to a place the item didn't name"
    );
    assert!(!ui.menu_open());
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn a_history_menu_goes_to_a_search_only_while_it_is_that_far_back(cx: &mut TestAppContext) {
    let fx = Fixture::new("history-search-moved-on");
    let mut ui = Ui::open(cx, &fx.dir);
    let shown = |ui: &Ui| (ui.folder(), ui.read(|f| f.query().to_string()));
    ui.click("search");
    ui.type_text("atlas");
    let searching = ui.read(|f| f.title());
    ui.double_click(item_id("tile", &fx.path("Projects/atlas")));
    ui.right_click("back");
    assert_eq!(ui.menu_labels()[0], searching);

    // A key goes back to the search under the open menu. One step back is
    // now its folder: the same place, but not the search the item names.
    ui.press("alt-left");
    assert_eq!(shown(&ui), (Some(fx.dir.clone()), "atlas".into()));
    ui.click_menu_item(&searching);
    assert_eq!(
        shown(&ui),
        (Some(fx.dir.clone()), "atlas".into()),
        "the item didn't end the search it names"
    );
    assert!(!ui.menu_open());
}

#[gpui_kit::test]
fn a_history_menu_closes_when_the_window_changes_size(cx: &mut TestAppContext) {
    let fx = Fixture::new("history-resized");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.right_click("back");
    assert!(ui.menu_open());

    // The buttons keep to the window's right edge, so in a narrower window
    // Back is no longer where it was pressed, and that is where its menu is
    // drawn. Left open, the menu would hold the keyboard without showing.
    let size = ui.window(|window, _| window.viewport_size());
    ui.resize(f32::from(size.width) - 120., f32::from(size.height));
    assert!(!ui.menu_open());
    assert!(ui.files_focused(), "the keyboard is back with the files");
    ui.press("alt-left");
    assert_eq!(ui.folder(), Some(fx.dir.clone()), "and their keys work");

    // A window that comes to the front reports its bounds as well. It is no
    // bigger or smaller for that, and a menu stays open through it.
    ui.right_click("forward");
    ui.activate();
    assert_eq!(ui.menu_labels(), ["Projects"]);
}

#[gpui_kit::test]
fn the_keys_work_on_when_a_history_menu_is_left_behind_by_its_button(cx: &mut TestAppContext) {
    let fx = Fixture::new("history-left-behind");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.resize(900., 600.);
    ui.go_to(&fx.path("Projects"));
    ui.right_click("back");
    assert!(ui.menu_open());

    // Keys still reach the window while a menu is open. Hiding the sidebar
    // leaves the window its size and gives the toolbar another width, so the
    // menu isn't closed, and isn't drawn either: Back is no longer where it
    // was pressed. The menu had the keyboard.
    ui.press(&cmd("alt-s"));
    assert!(!ui.read(|f| f.config().show_sidebar));
    assert!(
        !ui.menu_open(),
        "in a window this wide, hiding the sidebar moves Back"
    );
    assert!(ui.files_focused());
    ui.press(&cmd("alt-s"));
    assert!(
        ui.read(|f| f.config().show_sidebar),
        "the key that hid the sidebar shows it again"
    );
}

#[gpui_kit::test]
fn sidebar_hide_and_show_buttons(cx: &mut TestAppContext) {
    let fx = Fixture::new("sidebar-toggle");
    let mut ui = Ui::open(cx, &fx.dir);
    assert!(!ui.exists("show-sidebar"));
    ui.click("hide-sidebar");
    assert!(!ui.read(|f| f.config().show_sidebar));
    assert!(!ui.exists("hide-sidebar"));
    ui.click("show-sidebar");
    assert!(ui.read(|f| f.config().show_sidebar));
}

#[gpui_kit::test]
fn every_command_a_tooltip_names_has_keys_to_show(cx: &mut TestAppContext) {
    let fx = Fixture::new("tooltip-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    let named: Vec<Box<dyn gpui_kit::Action>> = vec![
        GoBack.boxed_clone(),
        GoForward.boxed_clone(),
        ViewIcons.boxed_clone(),
        ViewList.boxed_clone(),
        ViewColumns.boxed_clone(),
        ViewGallery.boxed_clone(),
        ToggleSidebar.boxed_clone(),
        NewTab.boxed_clone(),
        CloseWindow.boxed_clone(),
        ToggleFilter.boxed_clone(),
        SaveSearch.boxed_clone(),
        SearchHelp.boxed_clone(),
        ShowSettings.boxed_clone(),
        ToggleNuPanel.boxed_clone(),
    ];
    // A tooltip asks for its keys this way, and shows none when there are none.
    let without: Vec<&str> = ui.window(|window, _| {
        named
            .iter()
            .filter(|action| {
                Kbd::binding_for_action(action.as_ref(), Some(FINDER), window).is_none()
            })
            .map(|action| action.name())
            .collect()
    });
    assert!(without.is_empty(), "no keys for {without:?}");
}

#[gpui_kit::test]
fn the_keys_shown_for_going_back_forward_and_up_are_this_systems(cx: &mut TestAppContext) {
    let fx = Fixture::new("usual-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    // Each has Finder's keys and the other file managers'. Both work, and a
    // tooltip or a menu shows the ones bound last.
    let two_keys: [(Box<dyn gpui_kit::Action>, &str, &str); 3] = [
        (GoBack.boxed_clone(), "cmd-[", "alt-left"),
        (GoForward.boxed_clone(), "cmd-]", "alt-right"),
        (GoUp.boxed_clone(), "cmd-up", "alt-up"),
    ];
    for (action, finders, others) in two_keys {
        let shown = ui.window(|window, cx| {
            let finder = KeyContext::parse(FINDER).unwrap();
            let binding = window
                .highest_precedence_binding_for_action_in_context(action.as_ref(), finder)
                .unwrap_or_else(|| panic!("no keys for {}", action.name()));
            let os = cx.platform_info().os;
            binding.keystrokes()[0].inner().unparse(os)
        });
        let usual = if cfg!(target_os = "macos") {
            finders
        } else {
            others
        };
        assert_eq!(shown, usual, "{}", action.name());
    }
}

#[gpui_kit::test]
fn tab_bar_tooltips_name_their_shortcuts(cx: &mut TestAppContext) {
    let fx = Fixture::new("tab-tips");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            workspace: true,
            ..Default::default()
        },
    );
    ui.press(&cmd("t"));

    ui.hover("new-tab");
    assert_eq!(ui.snapshot("shortcut-tip").label(), Some("New Tab"));

    // Cmd-W closes the tab that shows, so only its close button names keys.
    ui.hover_point(at(0., 0.));
    ui.hover(("close-tab", 1usize));
    assert_eq!(ui.snapshot("shortcut-tip").label(), Some("Close Tab"));
    ui.hover_point(at(0., 0.));
    ui.hover(("close-tab", 0usize));
    assert!(!ui.exists("shortcut-tip"));
}

#[gpui_kit::test]
fn sort_menu_items(cx: &mut TestAppContext) {
    let fx = Fixture::new("sort-menu");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("sort");
    assert_eq!(
        ui.menu_labels(),
        [
            "Sort By",
            "Name",
            "Date Modified",
            "Date Created",
            "Size",
            "Kind",
            "Ascending",
            "Descending",
            "Keep Folders on Top",
            "Show Hidden Files",
            "Group By"
        ]
    );
    ui.click_menu_item("Size");
    assert!(!ui.menu_open(), "choosing an item closes the menu");
    assert_eq!(ui.read(|f| f.current_sort().key), SortKey::Size);

    ui.click("sort");
    ui.click_menu_item("Ascending");
    assert!(ui.read(|f| f.current_sort().ascending));
    ui.click("sort");
    ui.click_menu_item("Descending");
    assert!(!ui.read(|f| f.current_sort().ascending));

    assert!(!ui.names().contains(&".secret".to_string()));
    ui.click("sort");
    ui.click_menu_item("Show Hidden Files");
    assert!(ui.read(|f| f.config().show_hidden));
    assert!(ui.names().contains(&".secret".to_string()));

    // Sorted by name, descending: Projects is in the middle until folders go on top.
    ui.click("sort");
    ui.click_menu_item("Name");
    assert_ne!(ui.names()[0], "Projects");
    ui.click("sort");
    ui.click_menu_item("Keep Folders on Top");
    assert_eq!(ui.names()[0], "Projects");
}

#[gpui_kit::test]
fn nushell_menu_items(cx: &mut TestAppContext) {
    let fx = Fixture::new("nushell-menu");
    let mut ui = Ui::open(cx, &fx.dir);
    // Disabled without a selection: clicking it copies nothing.
    ui.click("nushell");
    ui.click_menu_item("Copy as Nushell List");
    assert_eq!(ui.clipboard(), "");
    ui.press("escape");

    ui.click("nushell");
    ui.click_menu_item("Run a Pipeline…");
    assert!(ui.read(|f| f.nu_panel_open()));
    ui.click("nushell");
    ui.click_menu_item("Hide Pipeline Panel");
    assert!(!ui.read(|f| f.nu_panel_open()));

    ui.select("Roadmap.md");
    ui.click("nushell");
    ui.click_menu_item("Copy as Nushell List");
    assert!(ui.clipboard().contains("Roadmap.md"), "{}", ui.clipboard());

    // Without nu this reports a problem instead of opening a terminal.
    ui.click("nushell");
    ui.click_menu_item("Open Nushell Here");
    assert!(!ui.menu_open());
}

#[gpui_kit::test]
fn actions_menu_items(cx: &mut TestAppContext) {
    let fx = Fixture::new("actions-menu");
    let mut ui = Ui::open(cx, &fx.dir);
    // These need a selection, so they're disabled and do nothing yet.
    for needs_selection in ["Quick Look", "Rename…", "Move to Trash"] {
        ui.click("actions");
        ui.click_menu_item(needs_selection);
        assert!(!ui.dialog_open(), "{needs_selection} without a selection");
        ui.press("escape");
    }

    ui.click("actions");
    ui.click_menu_item("Add Folder to Sidebar");
    assert!(ui.read(|f| f.favorites().iter().any(|p| p.path == fx.dir)));

    ui.select("Roadmap.md");
    ui.click("actions");
    ui.click_menu_item("Copy Path");
    assert_eq!(ui.clipboard(), fx.path("Roadmap.md").display().to_string());

    ui.click("actions");
    ui.click_menu_item(fffinder::finder::reveal_label());
    assert_eq!(
        ui.system_calls(),
        [SystemCall::Reveal(fx.path("Roadmap.md"))]
    );

    ui.click("actions");
    ui.click_menu_item("Quick Look");
    assert!(
        ui.read(|f| f.quick_look_window()).is_some(),
        "Quick Look opens its window"
    );
    ui.close_quick_look();
    assert!(ui.files_focused(), "focus stays with the files");

    for (item, close) in [
        ("Go to Folder…", "goto-cancel"),
        ("Search Syntax…", "help-done"),
        ("Settings…", "settings-done"),
    ] {
        ui.click("actions");
        ui.click_menu_item(item);
        assert!(ui.dialog_open(), "{item} opens a dialog");
        ui.click(close);
        assert!(!ui.dialog_open(), "{close} closes the {item} dialog");
        assert!(
            ui.files_focused(),
            "focus returns to the files after {item}"
        );
    }
    assert!(fx.path("Roadmap.md").exists(), "cancel kept the file");

    ui.click("actions");
    ui.click_menu_item("Move to Trash");
    assert!(!ui.dialog_open(), "the Trash doesn't ask");
    assert!(!fx.path("Roadmap.md").exists());

    ui.click("actions");
    ui.click_menu_item("New Folder");
    assert!(fx.path("untitled folder").is_dir());
    assert!(ui.exists("rename-input"), "a new folder starts naming it");
    ui.press("escape");
    assert!(ui.names().contains(&"untitled folder".to_string()));

    ui.select("notes.txt");
    ui.click("actions");
    ui.click_menu_item("Rename…");
    assert!(ui.exists("rename-input"));
    ui.press("escape");
}

#[gpui_kit::test]
fn the_actions_menu_shows_a_result_in_its_enclosing_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("actions-menu-enclosing");
    let mut ui = Ui::open(cx, &fx.dir);
    let item = "Show in Enclosing Folder";

    // In a folder every item is in the folder shown, so the menu leaves it out.
    ui.select("Roadmap.md");
    ui.click("actions");
    assert!(!ui.menu_labels().contains(&item.to_string()));
    ui.press("escape");

    // Among results it's there, disabled until one of them is selected.
    ui.click("search");
    ui.type_text("atlas");
    ui.click("actions");
    ui.click_menu_item(item);
    assert!(ui.menu_open(), "a disabled item doesn't close its menu");
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    ui.press("escape");

    // The keyboard is in the search field when the item is chosen. The search
    // ends, and the field is left for the files, where the result is selected.
    ui.select("Project Atlas.txt");
    ui.click("search");
    ui.click("actions");
    ui.click_menu_item(item);
    assert_eq!(ui.folder(), Some(fx.path("Projects/atlas")));
    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
    assert!(ui.files_focused(), "not left in the emptied search field");
}

#[gpui_kit::test]
fn every_menu_bar_action_has_a_handler(cx: &mut TestAppContext) {
    let fx = Fixture::new("menu-bar");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.select("Roadmap.md");
    let actions: Vec<Box<dyn gpui_kit::Action>> = vec![
        ShowSettings.boxed_clone(),
        Rescan.boxed_clone(),
        NewFolder.boxed_clone(),
        OpenSelected.boxed_clone(),
        QuickLook.boxed_clone(),
        Rename.boxed_clone(),
        MoveToTrash.boxed_clone(),
        CloseWindow.boxed_clone(),
        CopyPath.boxed_clone(),
        CopyAsNuon.boxed_clone(),
        SelectAll.boxed_clone(),
        FocusSearch.boxed_clone(),
        SearchHelp.boxed_clone(),
        ViewIcons.boxed_clone(),
        ViewList.boxed_clone(),
        ViewColumns.boxed_clone(),
        ViewGallery.boxed_clone(),
        ToggleSidebar.boxed_clone(),
        ToggleHidden.boxed_clone(),
        ToggleAppearance.boxed_clone(),
        Refresh.boxed_clone(),
        GoBack.boxed_clone(),
        GoForward.boxed_clone(),
        GoUp.boxed_clone(),
        GoRecents.boxed_clone(),
        GoHome.boxed_clone(),
        GoToFolder.boxed_clone(),
        ToggleNuPanel.boxed_clone(),
        OpenInNushell.boxed_clone(),
        ToggleFilter.boxed_clone(),
        SaveSearch.boxed_clone(),
        OpenInEditor.boxed_clone(),
        MoveItemsHere.boxed_clone(),
        PutBack.boxed_clone(),
        DeleteImmediately.boxed_clone(),
        EmptyTrash.boxed_clone(),
        GoTrash.boxed_clone(),
        GetInfo.boxed_clone(),
        ToggleFoldersFirst.boxed_clone(),
        ConnectToServer.boxed_clone(),
        NewTab.boxed_clone(),
        ShowNextTab.boxed_clone(),
        ShowPreviousTab.boxed_clone(),
        MoveTabToNewWindow.boxed_clone(),
        ToggleSplitView.boxed_clone(),
        CopyToOtherPane.boxed_clone(),
        MoveToOtherPane.boxed_clone(),
        NewFile.boxed_clone(),
        Duplicate.boxed_clone(),
        MakeAlias.boxed_clone(),
        MakeSymlink.boxed_clone(),
        Compress.boxed_clone(),
        Extract.boxed_clone(),
        gpui_kit::component::input::Undo.boxed_clone(),
    ];
    // Menus send actions to whatever has focus: the files, or a text field.
    for focus in ["files", "search"] {
        if focus == "search" {
            ui.click("search");
        }
        let missing: Vec<&str> = ui.window(|window, cx| {
            actions
                .iter()
                .filter(|a| !window.is_action_available(a.as_ref(), cx))
                .map(|a| a.name())
                .collect()
        });
        assert!(
            missing.is_empty(),
            "with focus in {focus}, no handler for {missing:?}"
        );
    }
}

// ----- search ----------------------------------------------------------------------

#[gpui_kit::test]
fn search_field_typing_escape_and_down_arrow(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-field");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("atlas");
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    assert!(
        ui.names().contains(&"Project Atlas.txt".to_string()),
        "{:?}",
        ui.names()
    );
    assert!(ui.exists("scope-computer"), "the scope bar appears");

    ui.press("down");
    assert!(ui.files_focused(), "Down moves into the results");
    assert_eq!(ui.selected().len(), 1);

    ui.click("search");
    ui.press("escape");
    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert!(!ui.exists("scope-computer"));
    assert_eq!(ui.names().len(), 4);
}

#[gpui_kit::test]
fn escape_in_an_empty_search_field_gives_the_files_the_keyboard(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt as _;
    let fx = Fixture::new("search-escape");
    std::fs::create_dir(fx.path("Empty")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    let in_a_field = |ui: &mut Ui| ui.window(|window, cx| window.has_focused_input(cx));
    let query = |ui: &Ui| ui.read(|f| f.query().to_string());
    ui.click("search");
    ui.type_text("atlas");

    // With words in it, Escape empties the field and stays there, so other
    // words can be typed.
    ui.press("escape");
    assert_eq!(query(&ui), "");
    assert!(in_a_field(&mut ui), "the first Escape stays in the field");
    ui.type_text("atlas");
    assert_eq!(query(&ui), "atlas");

    ui.press("escape");
    assert_eq!(query(&ui), "");
    assert!(!ui.exists("scope-computer"));
    ui.press("escape");
    assert!(ui.files_focused(), "the second Escape leaves the field");
    assert_eq!(ui.names().len(), 5, "the folder's items are showing");
    // With the keyboard in the files, a letter selects an item. In the
    // field it would search.
    ui.type_text("r");
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    assert_eq!(query(&ui), "");

    // The same from a field that was empty all along, and in a folder with
    // no items for the keyboard to move into.
    ui.go_to(&fx.path("Empty"));
    ui.press(&cmd("f"));
    assert!(in_a_field(&mut ui));
    ui.press("escape");
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn search_scope_button(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-button");
    let mut ui = Ui::open(cx, &fx.dir);
    let scope = |ui: &Ui| ui.read(|f| f.search_scope());
    ui.go_to(&fx.path("Projects"));
    ui.click("search-scope");
    assert_eq!(scope(&ui), SearchScope::Folder, "set before searching");
    ui.update(|f, _, cx| f.set_query("atlas".into(), true, cx));
    let names = ui.names();
    assert!(names.iter().any(|n| n == "Project Atlas.txt"), "{names:?}");
    ui.update(|f, _, cx| f.set_query("budget".into(), true, cx));
    assert!(ui.names().is_empty(), "Budget.csv is outside Projects");
    assert!(ui.exists("scope-folder"));

    ui.click("search-scope");
    assert_eq!(scope(&ui), SearchScope::Computer);
    assert_eq!(ui.names(), ["Budget.csv"], "the search runs again");
    ui.click("scope-folder");
    ui.click("search-scope");
    assert_eq!(
        scope(&ui),
        SearchScope::Computer,
        "it follows the scope bar"
    );

    // Recents isn't a folder, so the button does nothing there.
    ui.update(|f, window, cx| f.navigate(Location::Recents, window, cx));
    ui.click("search-scope");
    assert_eq!(scope(&ui), SearchScope::Computer);
}

#[gpui_kit::test]
fn a_contents_search_of_one_folder_finds_its_files_whatever_its_name(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-names");
    // More files have the word than a search may list, and newer files are
    // looked inside first. The two folders' own are an hour older.
    for n in 0..40 {
        std::fs::write(fx.path(&format!("hay-{n:02}.txt")), "zebraneedle\n").unwrap();
    }
    let hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    for folder in ["Tax Returns", "drafts[old]"] {
        std::fs::create_dir(fx.path(folder)).unwrap();
        for name in ["one.txt", "two.txt"] {
            let path = fx.path(folder).join(name);
            std::fs::write(&path, "zebraneedle\n").unwrap();
            let file = std::fs::File::options().write(true).open(&path).unwrap();
            file.set_modified(hour_ago).unwrap();
        }
    }
    let mut ui = Ui::open(cx, &fx.dir);
    let status = |ui: &mut Ui| ui.snapshot("status-text").label().unwrap().to_string();
    let names = |ui: &Ui| {
        let mut names = ui.names();
        names.sort();
        names
    };
    ui.update(|f, _, cx| f.set_result_limit(5, cx));

    ui.go_to(&fx.path("Tax Returns"));
    ui.click("search-scope");
    ui.update(|f, _, cx| f.set_query("zebraneedle".into(), true, cx));
    ui.click("by-contents");
    assert_eq!(names(&ui), ["one.txt", "two.txt"], "a space in its name");
    assert_eq!(status(&mut ui), "2 items", "and no more of them to find");

    // Going to another folder ends the search. The next one is of that
    // folder, and of contents again.
    ui.go_to(&fx.path("drafts[old]"));
    ui.update(|f, _, cx| f.set_query("zebraneedle".into(), true, cx));
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Folder);
    assert!(ui.read(|f| f.searches_contents()));
    assert_eq!(names(&ui), ["one.txt", "two.txt"], "brackets in its name");
    assert_eq!(status(&mut ui), "2 items");
}

#[gpui_kit::test]
fn typing_on_stops_the_contents_search_for_what_was_typed_before(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-stop");
    let mut ui = Ui::open(cx, &fx.dir);
    let abort = |ui: &Ui| ui.read(|f| f.search_abort_for_test());
    ui.click("search");
    ui.type_text("plain");
    ui.click("by-contents");
    assert_eq!(ui.names(), ["notes.txt"], "the file with the word in it");

    // More is typed before the search for the first letter has answered.
    ui.press(&cmd("f"));
    ui.type_without_settling("a");
    assert_eq!(ui.read(|f| f.query().to_string()), "a");
    let first = abort(&ui);
    ui.type_text("tlas");
    assert!(
        first.load(Ordering::Relaxed),
        "the search for `a` was told to stop"
    );
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
    let shown = abort(&ui);
    assert!(
        !shown.load(Ordering::Relaxed),
        "the search on screen ran to its end"
    );

    // Emptying the field stops a search too.
    ui.type_without_settling("x");
    let last = abort(&ui);
    ui.press("escape");
    assert_eq!(ui.read(|f| f.query().to_string()), "");
    assert!(
        last.load(Ordering::Relaxed),
        "the search for `atlasx` was told to stop"
    );
}

#[gpui_kit::test]
fn closing_a_tab_stops_its_contents_search(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-stop-tab");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            workspace: true,
            ..Default::default()
        },
    );
    let workspace = ui.workspace.clone().expect("opened with tabs");
    let tabs = |ui: &Ui| workspace.read_with(ui.cx, |w, _| w.tabs().len());
    ui.press(&cmd("t"));
    assert_eq!(tabs(&ui), 2);
    ui.click("search");
    ui.type_text("plain");
    ui.click("by-contents");

    // The new tab closes before its next search has answered. Only the
    // search's flag is kept: a handle to the tab would keep the tab alive.
    ui.press(&cmd("f"));
    ui.type_without_settling("atlas");
    let search = workspace.read_with(ui.cx, |w, cx| {
        let tab = w.active().read(cx);
        assert_eq!(tab.query(), "atlas");
        assert!(tab.searches_contents());
        tab.search_abort_for_test()
    });
    assert!(!search.load(Ordering::Relaxed));
    ui.press(&cmd("w"));
    assert_eq!(tabs(&ui), 1);
    assert!(
        search.load(Ordering::Relaxed),
        "the closed tab's search was told to stop"
    );
}

#[gpui_kit::test]
fn a_contents_search_lists_every_file_up_to_the_result_limit(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-contents-limit");
    // The word is on twenty lines of each file, and a row is a file.
    for n in 0..30 {
        let lines = "zebraneedle here\n".repeat(20);
        std::fs::write(fx.path(&format!("hay-{n:02}.txt")), lines).unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    let status = |ui: &mut Ui| ui.snapshot("status-text").label().unwrap().to_string();
    ui.update(|f, _, cx| {
        f.set_result_limit(60, cx);
        f.set_search_contents(true, cx);
        f.set_query("zebraneedle".into(), true, cx);
    });
    assert_eq!(ui.names().len(), 30, "every file with the word in it");
    assert_eq!(status(&mut ui), "30 items", "and that's all of them");

    ui.update(|f, _, cx| f.set_result_limit(10, cx));
    assert_eq!(ui.names().len(), 10);
    assert_eq!(status(&mut ui), "10+ items", "twenty more have it");

    // Thirty fit a limit of thirty exactly: nothing was left out. A limit
    // of one fewer leaves one out.
    ui.update(|f, _, cx| f.set_result_limit(30, cx));
    assert_eq!(ui.names().len(), 30);
    assert_eq!(status(&mut ui), "30 items");
    ui.update(|f, _, cx| f.set_result_limit(29, cx));
    assert_eq!(ui.names().len(), 29);
    assert_eq!(status(&mut ui), "29+ items");
}

#[gpui_kit::test]
fn a_search_is_marked_only_when_the_result_limit_left_items_out(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-name-limit");
    for n in 1..=5 {
        std::fs::write(fx.path(&format!("zlog{n}.txt")), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    let status = |ui: &mut Ui| ui.snapshot("status-text").label().unwrap().to_string();
    ui.update(|f, _, cx| {
        f.set_result_limit(3, cx);
        f.set_query("zlog".into(), true, cx);
    });
    assert_eq!(ui.names().len(), 3);
    assert_eq!(status(&mut ui), "3+ items");

    // Resting the mouse on the count says what the mark means.
    ui.hover("status-text");
    assert_eq!(
        ui.snapshot("status-more").label(),
        Some(
            "More items match. Narrow the search, or raise Results shown in Settings › Search \
             engine."
        )
    );
    ui.hover_point(at(0., 0.));

    // One item of several is no "1 item".
    ui.update(|f, _, cx| f.set_result_limit(1, cx));
    assert_eq!(status(&mut ui), "1+ items");

    // Five results fit a limit of five: nothing was left out.
    ui.update(|f, _, cx| f.set_result_limit(5, cx));
    assert_eq!(ui.names().len(), 5);
    assert_eq!(status(&mut ui), "5 items");
    ui.hover("status-text");
    assert!(!ui.exists("status-more"));
    ui.hover_point(at(0., 0.));

    // A selection's count takes the place of the one with the mark, which
    // leaves nothing to explain.
    ui.update(|f, _, cx| f.set_result_limit(3, cx));
    let first = ui.names()[0].clone();
    ui.select(&first);
    let selected = status(&mut ui);
    assert!(selected.starts_with("1 of 3 selected"), "{selected}");
    ui.hover("status-text");
    assert!(!ui.exists("status-more"));
}

#[gpui_kit::test]
fn a_sort_chooses_among_every_match_not_among_the_ones_listed(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-sort-limit");
    for (name, size) in [
        ("a.log", 10),
        ("b.log", 3000),
        ("c.log", 200),
        ("d.log", 50),
    ] {
        std::fs::write(fx.path(name), vec![b'x'; size]).unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    let status = |ui: &mut Ui| ui.snapshot("status-text").label().unwrap().to_string();
    ui.click("view-list");
    ui.update(|f, _, cx| {
        f.set_result_limit(3, cx);
        f.set_query("*.log".into(), true, cx);
    });
    assert_eq!(ui.names().len(), 3, "three of the four fit");

    // The three largest of the four, then the three smallest.
    ui.click("Size");
    assert_eq!(ui.names(), ["b.log", "c.log", "d.log"]);
    assert_eq!(status(&mut ui), "3+ items");
    ui.select("c.log");
    ui.click("Size");
    assert_eq!(ui.names(), ["a.log", "d.log", "c.log"]);
    assert_eq!(
        ui.selected(),
        ["c.log"],
        "the search that ran again isn't a new one"
    );

    // The Sort menu's directions search again as well.
    ui.click("sort");
    ui.click_menu_item("Descending");
    assert_eq!(ui.names(), ["b.log", "c.log", "d.log"]);
    // The direction the list already has is nothing to search again for.
    let view = ui.view.clone();
    ui.window(|_, cx| {
        view.update(cx, |f, cx| {
            f.set_ascending(false, cx);
            assert!(!f.search_in_flight_for_test());
        })
    });
    ui.click("Name");
    assert_eq!(ui.names(), ["a.log", "b.log", "c.log"]);

    // The sort stays on for the searches that follow.
    ui.update(|f, _, cx| f.set_result_limit(2, cx));
    assert_eq!(ui.names(), ["a.log", "b.log"]);
    ui.update(|f, _, cx| f.set_query("[b-d].log".into(), true, cx));
    assert_eq!(ui.names(), ["b.log", "c.log"]);
}

#[gpui_kit::test]
fn a_sort_by_a_word_stays_with_the_names_that_have_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-sort-word");
    std::fs::create_dir_all(fx.path("ledger")).unwrap();
    for (name, size) in [
        ("ledger.txt", 5),
        ("my-ledger-notes.txt", 3000),
        ("xledgerx.txt", 200),
        // Only its folder has the word, and it is the largest of them all.
        ("ledger/huge.bin", 9000),
    ] {
        std::fs::write(fx.path(name), vec![b'x'; size]).unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    let listed = |ui: &Ui| {
        let mut names = ui.names();
        names.sort();
        names
    };
    ui.click("view-list");
    ui.update(|f, _, cx| {
        f.set_result_limit(2, cx);
        f.set_query("ledger".into(), true, cx);
    });
    let best = listed(&ui);
    assert_eq!(
        best,
        ["ledger", "ledger.txt"],
        "the names that are the word"
    );

    // Four names have the word, so the largest two of those are listed, and
    // not the file that is only in a folder named for it.
    ui.click("Size");
    assert_eq!(ui.names(), ["my-ledger-notes.txt", "xledgerx.txt"]);

    // Back in the order a search starts with, the best matches are listed again.
    ui.click("sort");
    ui.click_menu_item("Relevance");
    assert_eq!(listed(&ui), best);

    // A list with every match in it is only put in order: the file in the
    // folder stays, and nothing is searched again.
    ui.update(|f, _, cx| f.set_result_limit(5, cx));
    ui.click_without_settling("Size");
    assert!(!ui.read(|f| f.search_in_flight_for_test()));
    ui.settle();
    assert_eq!(
        ui.names(),
        [
            "huge.bin",
            "my-ledger-notes.txt",
            "xledgerx.txt",
            "ledger.txt",
            "ledger"
        ]
    );

    // A sort changed while another search is on its way goes with that
    // search, though the list on screen has every match of its own.
    ui.update(|f, _, cx| {
        f.set_result_limit(2, cx);
        f.set_query("huge".into(), true, cx);
    });
    assert_eq!(ui.names(), ["huge.bin"]);
    let view = ui.view.clone();
    ui.window(|_, cx| {
        view.update(cx, |f, cx| {
            f.set_query("ledger".into(), true, cx);
            f.set_ascending(true, cx);
        })
    });
    ui.settle();
    assert_eq!(ui.names(), ["ledger", "ledger.txt"], "the smallest two");
}

#[gpui_kit::test]
fn a_list_with_loose_matches_in_it_keeps_its_items_under_a_sort(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-sort-loose");
    std::fs::create_dir_all(fx.path("ledger")).unwrap();
    // A file and the folder have the word in their names. The files in the
    // folder match it by their path alone.
    for (name, size) in [
        ("ledger.txt", 5),
        ("ledger/a.bin", 300),
        ("ledger/b.bin", 200),
        ("ledger/c.bin", 100),
    ] {
        std::fs::write(fx.path(name), vec![b'x'; size]).unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    let status = |ui: &mut Ui| ui.snapshot("status-text").label().unwrap().to_string();
    ui.click("view-list");
    ui.update(|f, _, cx| {
        f.set_result_limit(3, cx);
        f.set_query("ledger".into(), true, cx);
    });
    let best = ui.names();
    assert!(best[..2].contains(&"ledger.txt".to_string()), "{best:?}");
    assert!(best[..2].contains(&"ledger".to_string()), "{best:?}");
    let loose = best[2].clone();
    assert!(loose.ends_with(".bin"), "a file in the folder: {loose}");
    assert_eq!(status(&mut ui), "3+ items");

    // Two names don't fill three places, so a sort has nothing to choose
    // among: the rows are put in order, and nothing is searched again.
    ui.select(&loose);
    ui.click_without_settling("Size");
    assert!(!ui.read(|f| f.search_in_flight_for_test()));
    ui.settle();
    assert_eq!(ui.names(), [loose.as_str(), "ledger.txt", "ledger"]);
    assert_eq!(ui.selected(), [loose.as_str()]);
    ui.click_without_settling("Name");
    assert!(!ui.read(|f| f.search_in_flight_for_test()));
    ui.settle();
    assert_eq!(ui.names(), [loose.as_str(), "ledger", "ledger.txt"]);
}

#[gpui_kit::test]
fn results_searched_again_under_a_sort_go_back_to_best_first(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-sort-back");
    for name in ["zebra.txt", "zebra-notes.txt", "my-zebra.txt"] {
        std::fs::write(fx.path(name), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_query("zebra".into(), true, cx));
    let best = ui.names();
    assert_eq!(best[0], "zebra.txt", "the name that is the word");

    let by_name = ["my-zebra.txt", "zebra-notes.txt", "zebra.txt"];
    ui.click("Name");
    assert_eq!(ui.names(), by_name);
    // The search runs again with the sort on, as Refresh has it do.
    ui.press(&cmd("r"));
    assert_eq!(ui.names(), by_name);

    // Every match is listed, and they're as good a match as they were.
    ui.click("sort");
    ui.click_menu_item("Relevance");
    assert_eq!(ui.names(), best);
}

#[gpui_kit::test]
fn results_have_what_the_columns_and_groups_show_of_them(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-columns");
    let file = fx.path("Budget.csv");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    let row = |ui: &Ui| ui.read(|f| f.rows_for_test()[0].clone());
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_query("roadmap".into(), true, cx));
    assert_eq!(ui.names(), ["Roadmap.md"]);
    // The index holds neither, and nothing shows them yet.
    assert_eq!(row(&ui).entry.mode, None);
    assert_eq!(row(&ui).entry.last_opened, None);

    // Groups by a date the index doesn't hold have the date to go by.
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::LastOpened), cx));
    assert_ne!(ui.read(|f| f.group_headers()), ["No Date"]);
    ui.update(|f, _, cx| f.set_group_by(None, cx));

    // A column added during a search is filled in.
    ui.update(|f, _, cx| f.set_query("budget".into(), true, cx));
    assert_eq!(ui.names(), ["Budget.csv"]);
    assert_eq!(row(&ui).entry.last_opened, None, "nothing shows it now");
    ui.right_click("Size");
    ui.click_menu_item("Date Last Opened");
    assert!(!row(&ui).opened().is_empty());
    assert_eq!(row(&ui).entry.mode, None, "the date is all that was read");
    ui.right_click("Size");
    ui.click_menu_item("Permissions");
    if cfg!(unix) {
        assert_eq!(row(&ui).permissions().to_string(), "rw-r-----");
    }
    ui.right_click("Size");
    ui.click_menu_item("Date Created");
    // Not every file system records when a file was made.
    if std::fs::metadata(&file).is_ok_and(|meta| meta.created().is_ok()) {
        assert!(!row(&ui).created().is_empty());
    }

    // And so are the results of the next search.
    ui.update(|f, _, cx| f.set_query("roadmap".into(), true, cx));
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert!(!row(&ui).opened().is_empty());
    assert_eq!(row(&ui).entry.mode.is_some(), cfg!(unix));
}

#[gpui_kit::test]
fn results_sorted_by_a_date_the_index_lacks_are_read_for_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-sort-date");
    let mut ui = Ui::open(cx, &fx.dir);
    let entry = |ui: &Ui| ui.read(|f| f.rows_for_test()[0].entry.clone());
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_query("roadmap".into(), true, cx));
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert_eq!((entry(&ui).created, entry(&ui).mode), (None, None));

    // No column shows the date, and the one result is every match.
    ui.click("sort");
    ui.click_menu_item("Date Created");
    assert_eq!(ui.read(|f| f.current_sort().key), SortKey::Created);
    // Not every file system records when a file was made.
    let made = std::fs::metadata(fx.path("Roadmap.md")).unwrap().created();
    assert_eq!(entry(&ui).created, made.ok());
    assert_eq!(entry(&ui).mode.is_some(), cfg!(unix), "read along with it");
}

#[gpui_kit::test]
fn a_search_from_recents_can_be_sorted_by_name(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-sort-recents");
    for name in ["zebra.txt", "a-zebra.txt", "m-zebra.txt"] {
        std::fs::write(fx.path(name), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.update(|f, window, cx| f.navigate(Location::Recents, window, cx));
    ui.update(|f, _, cx| f.set_query("zebra".into(), true, cx));
    assert_eq!(ui.names()[0], "zebra.txt", "the name that is the word");

    ui.click("Name");
    assert_eq!(ui.names(), ["a-zebra.txt", "m-zebra.txt", "zebra.txt"]);
}

#[gpui_kit::test]
fn scope_bar_controls(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-bar");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.update(|f, _, cx| f.set_query("atlas".into(), true, cx));

    ui.click("scope-folder");
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Folder);
    ui.click("scope-computer");
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Computer);

    ui.click("by-contents");
    assert!(ui.read(|f| f.searches_contents()));
    ui.click("kind");
    assert!(!ui.menu_open(), "Kind is off for a contents search");
    ui.click("by-name");
    assert!(!ui.read(|f| f.searches_contents()));

    ui.click("kind");
    let labels: Vec<String> = KindFilter::ALL
        .iter()
        .map(|k| k.label().to_string())
        .collect();
    assert_eq!(ui.menu_labels(), labels);
    ui.click_menu_item("Folder");
    assert_eq!(ui.read(|f| f.config().kind_filter), KindFilter::Folder);
    assert_eq!(ui.names(), ["atlas"], "only folders match");
    ui.click("kind");
    ui.click_menu_item("Any Kind");
    assert_eq!(ui.read(|f| f.config().kind_filter), KindFilter::Any);

    ui.click("search-help");
    assert!(ui.dialog_open());
    ui.click("help-done");
    assert!(!ui.dialog_open());
}

#[gpui_kit::test]
fn scope_bar_tooltips_name_their_shortcuts(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-tips");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.update(|f, _, cx| f.set_query("atlas".into(), true, cx));

    ui.hover("filter-toggle");
    assert_eq!(
        ui.snapshot("shortcut-tip").label(),
        Some("Filter with Nushell")
    );
    ui.hover_point(at(0., 0.));
    ui.hover("search-help");
    assert_eq!(ui.snapshot("shortcut-tip").label(), Some("Search Syntax"));
}

#[gpui_kit::test]
fn typing_goes_on_in_the_search_field_after_a_click_in_the_scope_bar(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-bar-typing");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("a");
    // A letter after each click spells the word out.
    for (control, letter, typed) in [
        ("by-contents", "t", "at"),
        ("by-name", "l", "atl"),
        ("scope-folder", "a", "atla"),
        ("scope-computer", "s", "atlas"),
    ] {
        ui.click(control);
        ui.type_text(letter);
        assert_eq!(
            ui.read(|f| f.query().to_string()),
            typed,
            "typed after a click on {control}"
        );
    }

    // Down Arrow moves from the field into what the click found.
    ui.click("by-contents");
    ui.press("down");
    assert!(ui.files_focused());
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
}

#[gpui_kit::test]
fn a_click_in_the_scope_bar_leaves_the_keyboard_in_the_files(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-bar-files");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("atlas");
    ui.press("down");
    assert!(ui.files_focused());
    for control in ["scope-folder", "scope-computer", "by-name", "by-contents"] {
        ui.click(control);
        assert!(ui.files_focused(), "after a click on {control}");
    }

    // The arrow keys still move through what the last click found.
    assert!(ui.selected().is_empty());
    ui.press("down");
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
}

#[gpui_kit::test]
fn a_click_in_the_scope_bar_leaves_the_keyboard_in_the_filter_field(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt as _;
    let fx = Fixture::new("scope-bar-filter");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("atlas");
    // The funnel opens the filter bar with the keyboard in its field.
    ui.click("filter-toggle");
    ui.click("by-contents");
    assert!(ui.read(|f| f.searches_contents()));
    let typing_in = ui.window(|window, cx| window.focused_input(cx).map(|field| field.value(cx)));
    assert_eq!(
        typing_in.as_deref(),
        Some(""),
        "the filter field, which is empty, has the keyboard"
    );
}

#[gpui_kit::test]
fn a_click_in_the_scope_bar_gives_the_files_a_keyboard_that_was_nowhere(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-bar-nowhere");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("atlas");
    // With the browser itself focused, the keyboard is with no part of it:
    // the search field has lost it, and the files don't have it. No click or
    // key does that, but code that focuses the view does.
    ui.update(|f, window, cx| f.focus_handle(cx).focus(window, cx));
    ui.type_text("x");
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    assert!(!ui.files_focused());

    ui.click("by-contents");
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
}

#[gpui_kit::test]
fn a_click_in_the_scope_bar_ends_a_rename_with_the_keyboard_in_the_files(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-bar-rename");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("roadmap");
    ui.press("down");
    ui.press("f2");
    assert!(ui.exists("rename-input"));
    ui.type_text("Plan");

    // The keyboard is in the name's field, which doesn't outlive the click:
    // what was typed is kept, as after a click anywhere else, and the files
    // have the keyboard again.
    ui.click("scope-folder");
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Folder);
    assert!(!ui.exists("rename-input"));
    assert!(fx.path("Plan.md").exists());
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn the_kind_menu_gives_the_keyboard_back_to_the_search_field(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-bar-kind");
    let mut ui = Ui::open(cx, &fx.dir);
    let query = |ui: &Ui| ui.read(|f| f.query().to_string());
    ui.click("search");
    ui.type_text("at");

    ui.click("kind");
    ui.click_menu_item("Folder");
    ui.type_text("l");
    assert_eq!(query(&ui), "atl", "typed after choosing a kind");

    ui.click("kind");
    assert!(ui.menu_open());
    ui.press("escape");
    assert!(!ui.menu_open());
    ui.type_text("a");
    assert_eq!(query(&ui), "atla", "typed after closing the menu");

    // A click elsewhere in the bar closes the menu and does its own work.
    ui.click("kind");
    assert!(ui.menu_open());
    ui.click("scope-folder");
    assert!(!ui.menu_open());
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Folder);
    ui.type_text("s");
    assert_eq!(
        query(&ui),
        "atlas",
        "typed after a click with the menu open"
    );
}

#[gpui_kit::test]
fn closing_search_syntax_gives_the_keyboard_back_to_the_search_field(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-bar-help");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("atla");
    ui.click("search-help");
    assert!(ui.dialog_open());
    ui.click("help-done");
    assert!(!ui.dialog_open());
    ui.type_text("s");
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
}

#[gpui_kit::test]
fn a_click_on_a_search_syntax_example_searches_for_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("syntax-example");
    let mut ui = Ui::open(cx, &fx.dir);
    let query = |ui: &Ui| ui.read(|f| f.query().to_string());
    // The row of `*.pdf  *.md`, and the Kind menu's, which is no example.
    let (extensions, kind_menu) = (("syntax-row", 2usize), ("syntax-row", 9usize));
    ui.press(&cmd("/"));
    assert!(ui.dialog_open());
    assert_eq!(ui.snapshot(extensions).selected(), Some(false));

    ui.click(extensions);
    assert!(ui.dialog_open(), "the table stays, for the next example");
    assert_eq!(query(&ui), "*.pdf *.md", "with one space, as it's typed");
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert_eq!(
        ui.snapshot(extensions).selected(),
        Some(true),
        "the row is marked as the one searched for"
    );

    // The second click of a double-click finds the same row, not a result
    // to open.
    ui.double_click(extensions);
    assert_eq!(query(&ui), "*.pdf *.md");
    assert!(ui.system_calls().is_empty());
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));

    ui.click(kind_menu);
    assert_eq!(query(&ui), "*.pdf *.md");
    assert_eq!(ui.snapshot(kind_menu).selected(), None);

    // The keyboard goes back to the files, where it was, and they are now
    // the results.
    ui.click("help-done");
    assert!(!ui.dialog_open());
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["Roadmap.md"]);

    // The mark is for what's being searched for, so the table has it the
    // next time it opens.
    ui.press(&cmd("/"));
    assert_eq!(ui.snapshot(extensions).selected(), Some(true));
}

#[gpui_kit::test]
fn a_double_click_on_what_opens_search_syntax_tries_no_example(cx: &mut TestAppContext) {
    let fx = Fixture::new("syntax-example-opener");
    let mut ui = Ui::open(cx, &fx.dir);
    let query = |ui: &Ui| ui.read(|f| f.query().to_string());
    let extensions = ("syntax-row", 2usize);
    ui.click("search");
    ui.type_text("atlas");

    // In a small window the table opens over the (i) that was clicked for
    // it, and the second click of a double-click there finds a row under
    // it. That's no click on the example: the words in the field stay.
    ui.click("search-help");
    ui.second_click(extensions);
    assert!(ui.dialog_open());
    assert_eq!(query(&ui), "atlas");
    assert_eq!(ui.snapshot(extensions).selected(), Some(false));

    // A click of its own is one.
    ui.click(extensions);
    assert_eq!(query(&ui), "*.pdf *.md");
}

#[gpui_kit::test]
fn a_search_syntax_example_takes_the_place_of_a_contents_search(cx: &mut TestAppContext) {
    let fx = Fixture::new("syntax-example-contents");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("plain");
    ui.click("by-contents");
    assert_eq!(ui.names(), ["notes.txt"], "the file with the word in it");

    // The table is about names, so that's what an example searches. It
    // doesn't sit out the pause a search waits for after a key: the answer
    // comes without any time passing.
    ui.click("search-help");
    ui.click_without_settling(("syntax-row", 2usize));
    ui.cx.run_until_parked();
    assert!(!ui.read(|f| f.searches_contents()));
    assert_eq!(ui.names(), ["Roadmap.md"]);

    // The search field has the example, to go on typing from.
    ui.click("help-done");
    ui.type_text(" *.csv");
    assert_eq!(ui.read(|f| f.query().to_string()), "*.pdf *.md *.csv");
    let mut names = ui.names();
    names.sort();
    assert_eq!(names, ["Budget.csv", "Roadmap.md"]);
}

#[gpui_kit::test]
fn search_syntax_marks_no_example_during_a_contents_search(cx: &mut TestAppContext) {
    let fx = Fixture::new("syntax-example-in-files");
    let mut ui = Ui::open(cx, &fx.dir);
    let extensions = ("syntax-row", 2usize);
    ui.click("search");
    ui.type_text("*.pdf *.md");
    ui.click("by-contents");
    assert!(ui.names().is_empty(), "no file has these words in it");

    // Searched for in files, the words are text to find, not the example.
    ui.click("search-help");
    assert_eq!(ui.snapshot(extensions).selected(), Some(false));

    // A click searches for the same words by name, as the example.
    ui.click(extensions);
    assert!(!ui.read(|f| f.searches_contents()));
    assert_eq!(ui.names(), ["Roadmap.md"]);
    assert_eq!(ui.snapshot(extensions).selected(), Some(true));
}

#[gpui_kit::test]
fn undo_in_the_search_field_brings_back_what_an_example_replaced(cx: &mut TestAppContext) {
    let fx = Fixture::new("syntax-example-undo");
    let mut ui = Ui::open(cx, &fx.dir);
    let query = |ui: &Ui| ui.read(|f| f.query().to_string());
    let (extensions, exclusions) = (("syntax-row", 2usize), ("syntax-row", 7usize));
    ui.click("search");
    ui.type_text("atlas");
    ui.click("search-help");
    ui.click(extensions);
    ui.click(exclusions);
    // A click on the example that's already there changes nothing, so
    // there's no step of its own to undo.
    ui.click(exclusions);
    assert_eq!(query(&ui), "!*.log !test");
    ui.click("help-done");

    ui.press(&cmd("z"));
    assert_eq!(query(&ui), "*.pdf *.md", "the example before");
    assert_eq!(ui.names(), ["Roadmap.md"]);
    ui.press(&cmd("z"));
    assert_eq!(query(&ui), "atlas", "what was typed");
    let mut names = ui.names();
    names.sort();
    assert_eq!(names, ["Project Atlas.txt", "atlas"]);
}

#[gpui_kit::test]
fn search_syntax_says_whether_typos_are_allowed(cx: &mut TestAppContext) {
    let fx = Fixture::new("syntax-typos");
    let mut ui = Ui::open(cx, &fx.dir);
    let fuzzy_words = |ui: &mut Ui| {
        let row = ui.snapshot(("syntax-row", 0usize));
        row.label().expect("the row says what it shows").to_string()
    };
    ui.press(&cmd("/"));
    assert_eq!(
        fuzzy_words(&mut ui),
        "budget report: Fuzzy words; every word must match (typos allowed)"
    );
    ui.click("help-done");

    ui.update(|f, _, cx| f.update_search_settings(|s| s.typos = false, cx));
    ui.press(&cmd("/"));
    assert_eq!(
        fuzzy_words(&mut ui),
        "budget report: Fuzzy words; every word must match (typos off in Settings)"
    );
}

#[gpui_kit::test]
fn a_click_in_the_other_panes_scope_bar_takes_the_keyboard_to_its_files(cx: &mut TestAppContext) {
    let fx = Fixture::new("scope-bar-split");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            workspace: true,
            ..Default::default()
        },
    );
    ui.press(&cmd("\\"));
    let pane = ui
        .workspace
        .clone()
        .expect("opened with tabs")
        .read_with(ui.cx, |w, _| w.split_pane().cloned())
        .expect("split view is on");
    // The new pane has the keyboard, so this is its search.
    ui.press(&cmd("f"));
    ui.type_text("atlas");
    assert_eq!(pane.read_with(ui.cx, |f, _| f.query().to_string()), "atlas");
    // Back in the first pane, in its search field.
    let first = ui.view.clone();
    ui.window(|window, cx| first.read(cx).files_focus_handle().focus(window, cx));
    ui.press(&cmd("f"));
    ui.type_text("road");
    assert_eq!(ui.read(|f| f.query().to_string()), "road");

    ui.click_in("split-pane", "by-contents");
    assert!(pane.read_with(ui.cx, |f, _| f.searches_contents()));
    let in_its_files = ui.window(|window, cx| pane.read(cx).files_focused(window));
    assert!(in_its_files, "the pane that was clicked has the keyboard");
    ui.press("down");
    let selected: Vec<String> = pane.read_with(ui.cx, |f, _| {
        f.selected_entries().into_iter().map(|e| e.name).collect()
    });
    assert_eq!(selected, ["Project Atlas.txt"]);
    assert!(ui.selected().is_empty(), "the first pane got no key");
}

#[gpui_kit::test]
fn typing_on_after_no_results_keeps_the_message(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-nothing-typing");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("qqzzqqzz");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));

    // The next letter's search hasn't answered yet.
    ui.type_without_settling("q");
    assert!(ui.read(|f| f.search_in_flight_for_test()));
    assert!(
        !ui.exists("skeleton"),
        "placeholders would flash between two No Results"
    );
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));

    ui.settle();
    assert!(!ui.read(|f| f.search_in_flight_for_test()));
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));
}

#[gpui_kit::test]
fn the_first_letter_of_a_search_takes_the_folders_items_away(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-first-letter");
    let mut ui = Ui::open(cx, &fx.dir);
    assert_eq!(ui.names().len(), 4);
    ui.click("search");
    ui.type_without_settling("a");
    assert!(ui.read(|f| f.search_in_flight_for_test()));
    assert!(
        ui.names().is_empty(),
        "the folder's items aren't results: {:?}",
        ui.names()
    );
    assert!(ui.exists("skeleton"), "placeholders until the results come");

    ui.settle();
    let found = ui.names();
    assert!(found.contains(&"atlas".to_string()), "{found:?}");
    assert!(!ui.exists("skeleton"));
}

#[gpui_kit::test]
fn switching_between_name_and_contents_takes_the_other_results_away(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-name-contents");
    let mut ui = Ui::open(cx, &fx.dir);
    let details = |ui: &Ui| -> Vec<String> {
        ui.read(|f| {
            f.rows_for_test()
                .iter()
                .map(|r| r.detail.to_string())
                .collect()
        })
    };
    ui.click("search");
    ui.type_text("atlas");
    assert!(ui.names().len() > 1, "{:?}", ui.names());

    // Names found show their folder, which isn't a matching line.
    ui.click_without_settling("by-contents");
    assert!(ui.read(|f| f.search_in_flight_for_test()));
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(ui.exists("skeleton"));
    ui.settle();
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
    assert_eq!(details(&ui), ["1: atlas"]);

    // And the other way around.
    ui.click_without_settling("by-name");
    assert!(ui.read(|f| f.search_in_flight_for_test()));
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(ui.exists("skeleton"));
    ui.settle();
    assert!(ui.names().len() > 1, "{:?}", ui.names());
    assert!(
        details(&ui).iter().all(|d| !d.starts_with("1: ")),
        "{:?}",
        details(&ui)
    );
}

#[gpui_kit::test]
fn a_contents_search_that_found_nothing_says_it_is_searching_again(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-contents-again");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("qqzzqqzz");
    ui.click("by-contents");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));

    // Looking inside files can take seconds, and until it has answered
    // nothing says there are no results for the words as they are now.
    ui.click("search");
    ui.type_without_settling("q");
    assert_eq!(ui.read(|f| f.query().to_string()), "qqzzqqzzq");
    assert!(ui.read(|f| f.search_in_flight_for_test()));
    assert!(!ui.exists("skeleton"));
    assert_eq!(ui.snapshot("message-title").label(), Some("Searching…"));

    ui.settle();
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));
}

/// The line under the title of the file area's message.
fn message_detail(ui: &mut Ui) -> String {
    ui.snapshot("message-detail").label().unwrap().to_string()
}

#[gpui_kit::test]
fn no_results_in_one_folder_offers_to_search_this_computer(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-nothing-scope");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.click("search-scope");
    ui.click("search");
    ui.type_text("budge");
    assert!(ui.names().is_empty(), "Budget.csv is outside Projects");
    assert!(ui.exists("empty-widen-scope"));
    assert!(!ui.exists("empty-any-kind"), "the kind is Any Kind");
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));
    assert_eq!(
        message_detail(&mut ui),
        "The search looked only in “Projects”."
    );

    // The button does what This Computer in the scope bar does, and like
    // that one it leaves the keyboard in the search field.
    ui.click("empty-widen-scope");
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Computer);
    assert_eq!(ui.names(), ["Budget.csv"]);
    ui.type_text("t");
    assert_eq!(ui.read(|f| f.query().to_string()), "budget");
}

#[gpui_kit::test]
fn no_results_for_one_kind_offers_any_kind(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-nothing-kind");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("roadmap");
    ui.click("kind");
    ui.click_menu_item("Folder");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(ui.exists("empty-any-kind"));
    assert!(
        !ui.exists("empty-widen-scope"),
        "the search is of This Computer"
    );
    assert_eq!(
        message_detail(&mut ui),
        "The search looked only for items of the kind Folder."
    );

    ui.click("empty-any-kind");
    assert_eq!(ui.read(|f| f.config().kind_filter), KindFilter::Any);
    assert_eq!(ui.names(), ["Roadmap.md"]);

    // A contents search looks inside files of every kind, whatever the menu
    // was left at, so its message has nothing to say about the kind.
    ui.click("kind");
    ui.click_menu_item("Folder");
    ui.type_text("qqzz");
    ui.click("by-contents");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));
    let detail = message_detail(&mut ui);
    assert!(!detail.contains("kind"), "{detail}");
    assert!(!ui.exists("empty-any-kind"));
}

#[gpui_kit::test]
fn a_button_under_no_results_takes_no_keyboard_away_with_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-nothing-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.click("search-scope");
    ui.click("search");
    ui.type_text("budget");
    assert!(ui.names().is_empty());
    let on_the_button = |ui: &mut Ui| ui.snapshot("empty-widen-scope").focused() == Some(true);

    // In the file area Return and Space are the files' keys, so they
    // wouldn't press the button, and Tab passes it by.
    for _ in 0..40 {
        ui.press("tab");
        assert!(!on_the_button(&mut ui));
    }

    // A right click gives a button the keyboard all the same. This one is
    // gone once the search finds something, and the keyboard would be
    // nowhere.
    ui.right_click("empty-widen-scope");
    assert!(on_the_button(&mut ui));
    ui.click("empty-widen-scope");
    assert_eq!(ui.names(), ["Budget.csv"]);
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["Budget.csv"]);

    // With the browser itself focused, the keyboard is with no part of it.
    // No click or key does that, but code that focuses the view does. The
    // button gives the keyboard to the files then, as the scope bar does.
    ui.click("search-scope");
    assert!(ui.names().is_empty());
    ui.update(|f, window, cx| f.focus_handle(cx).focus(window, cx));
    assert!(!ui.files_focused());
    ui.click("empty-widen-scope");
    assert_eq!(ui.names(), ["Budget.csv"]);
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn a_button_under_no_results_that_goes_by_itself_leaves_the_files_the_keyboard(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("search-nothing-gone");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.click("search-scope");
    ui.click("search");
    ui.type_text("budget");
    assert!(ui.names().is_empty());

    // A right click gives the button the keyboard. The toolbar's button
    // then widens the search without it, and leaves the keyboard where it
    // is, as buttons do: on a button that goes once something is found.
    ui.right_click("empty-widen-scope");
    assert_eq!(ui.snapshot("empty-widen-scope").focused(), Some(true));
    ui.click("search-scope");
    assert_eq!(ui.names(), ["Budget.csv"]);
    assert!(!ui.exists("empty-widen-scope"));
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), ["Budget.csv"]);
}

#[gpui_kit::test]
fn a_double_click_on_a_button_under_no_results_opens_nothing(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-nothing-double");
    // Enough files that one of them is listed where the button was.
    for n in 0..60 {
        std::fs::write(fx.path(&format!("quokka {n:02}.txt")), "").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.go_to(&fx.path("Projects"));
    ui.click("search-scope");
    ui.click("search");
    ui.type_text("quokka");
    assert!(ui.names().is_empty(), "{:?}", ui.names());

    let button = ui.snapshot("empty-widen-scope").bounds().center();
    ui.click("empty-widen-scope");
    assert_eq!(ui.names().len(), 60);
    let found: Vec<PathBuf> = ui.read(|f| {
        f.rows_for_test()
            .iter()
            .map(|r| r.entry.path.clone())
            .collect()
    });
    let under = found
        .iter()
        .map(|path| item_id("row", path))
        .find(|row| ui.exists(row.clone()) && ui.snapshot(row.clone()).bounds().contains(&button))
        .expect("a result is listed where the button was");

    // The second click of a double-click on the button lands on that
    // result, and isn't a double-click on it, or a click at all.
    ui.second_click(under.clone());
    assert_eq!(ui.system_calls(), []);
    assert!(ui.selected().is_empty(), "{:?}", ui.selected());

    // A double-click of its own is one.
    ui.double_click(under);
    assert!(
        matches!(ui.system_calls().as_slice(), [SystemCall::Open(_)]),
        "{:?}",
        ui.system_calls()
    );
}

#[gpui_kit::test]
fn a_double_click_on_a_button_under_no_results_doesnt_press_the_next_one(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-nothing-next");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.click("search-scope");
    ui.click("search");
    ui.type_text("budget");
    ui.click("kind");
    ui.click_menu_item("Folder");
    assert!(ui.names().is_empty(), "{:?}", ui.names());

    // Budget.csv is no folder, so This Computer has nothing either. The one
    // button left is then in the middle, where part of the first one was.
    let first = ui.snapshot("empty-widen-scope").bounds();
    ui.click("empty-widen-scope");
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Computer);
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    let next = ui.snapshot("empty-any-kind").bounds().center();
    assert!(first.contains(&next), "{first:?} doesn't hold {next:?}");

    // The second click of a double-click there isn't a click on that one.
    ui.second_click("empty-any-kind");
    assert_eq!(ui.read(|f| f.config().kind_filter), KindFilter::Folder);
    assert!(ui.names().is_empty(), "{:?}", ui.names());

    // A click of its own is one.
    ui.click("empty-any-kind");
    assert_eq!(ui.read(|f| f.config().kind_filter), KindFilter::Any);
    assert_eq!(ui.names(), ["Budget.csv"]);
}

#[gpui_kit::test]
fn a_message_and_its_buttons_fit_a_narrow_pane(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-nothing-narrow");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.click("search-scope");
    ui.click("search");
    ui.type_text("budget");
    ui.click("kind");
    ui.click_menu_item("Folder");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    let (widen, any_kind) = ("empty-widen-scope", "empty-any-kind");
    let side_by_side = ui.snapshot(widen).bounds().top() == ui.snapshot(any_kind).bounds().top();
    assert!(side_by_side, "there is room for both on one line");

    // Too narrow for either line of text, or for the two buttons abreast.
    ui.resize(360., 600.);
    let area = ui.snapshot("content").bounds();
    let inside = |ui: &mut Ui, id: &'static str| {
        let part = ui.snapshot(id).bounds();
        assert!(
            part.left() > area.left() && part.right() < area.right(),
            "{id} at {part:?} isn't clear of the edges of {area:?}"
        );
        part
    };
    inside(&mut ui, "message-title");
    inside(&mut ui, "message-detail");
    let (first, second) = (inside(&mut ui, widen), inside(&mut ui, any_kind));
    assert!(
        second.top() >= first.bottom(),
        "{second:?} is under {first:?}"
    );
}

#[gpui_kit::test]
fn an_empty_folder_says_so_and_offers_nothing(cx: &mut TestAppContext) {
    let fx = Fixture::new("empty-folder");
    std::fs::create_dir(fx.path("Empty")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Empty"));
    assert_eq!(ui.snapshot("message-title").label(), Some("No Items"));
    assert_eq!(message_detail(&mut ui), "This folder is empty.");
    for button in ["empty-remove-filter", "empty-widen-scope", "empty-any-kind"] {
        assert!(!ui.exists(button), "{button}");
    }
}

#[gpui_kit::test]
fn sent_results_with_no_item_left_arent_called_an_empty_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("empty-results");
    let mut ui = Ui::open(cx, &fx.dir);
    let sent = vec![fx.path("notes.txt")];
    ui.update(|f, window, cx| f.show_results("Sent".into(), sent, window, cx));
    assert_eq!(ui.names(), ["notes.txt"]);

    // In the Trash, the one item is no longer where the list has it.
    ui.select("notes.txt");
    ui.press(&cmd("backspace"));
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert_eq!(ui.snapshot("message-title").label(), Some("No Items"));
    assert_eq!(
        message_detail(&mut ui),
        "The items listed here can no longer be found."
    );
}

#[gpui_kit::test]
fn a_folder_whose_index_is_still_being_built_doesnt_say_no_results(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-outside");
    // A folder outside the search locations gets an index of its own the
    // first time it's searched. This one holds enough that the search's
    // first answer nearly always comes before that index is built.
    let outside = Fixture::new("search-outside-folder");
    for i in 0..300 {
        std::fs::write(outside.path(&format!("Projects/file {i}.txt")), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    let title = |ui: &mut Ui| {
        ui.exists("message-title")
            .then(|| ui.snapshot("message-title").label().map(str::to_string))
            .flatten()
    };
    ui.go_to(&outside.dir);
    ui.click("search-scope");
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Folder);
    ui.click("search");
    ui.type_without_settling("atlas");

    // The search answers when the typing has paused. Unless the index was
    // built in no time, that answer comes from a part of it.
    ui.cx
        .executor()
        .advance_clock(std::time::Duration::from_millis(50));
    ui.cx.run_until_parked();
    if ui.names().is_empty() {
        assert_eq!(title(&mut ui).as_deref(), Some("Indexing…"));
    }

    // With the index built, the search runs again by itself.
    let engine = ui.cx.read(|cx| cx.global::<AppState>().engine.clone());
    assert!(engine.prepare(
        Some(outside.dir.as_path()),
        std::time::Duration::from_secs(20)
    ));
    for _ in 0..5 {
        ui.settle();
        assert_ne!(title(&mut ui).as_deref(), Some("No Results"));
    }
    let found = ui.names();
    assert!(
        found.contains(&"Project Atlas.txt".to_string()),
        "{found:?}"
    );
}

#[gpui_kit::test]
fn indexing_stays_while_the_search_is_tried_again(cx: &mut TestAppContext) {
    let fx = Fixture::new("search-outside-again");
    // As in the test before this one, a folder whose index is nearly always
    // still being built when the search's first answer comes.
    let outside = Fixture::new("search-outside-again-folder");
    for i in 0..300 {
        std::fs::write(outside.path(&format!("Projects/file {i}.txt")), "x").unwrap();
    }
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&outside.dir);
    ui.click("search-scope");
    ui.click("search");
    ui.type_without_settling("atlas");
    ui.cx
        .executor()
        .advance_clock(std::time::Duration::from_millis(50));
    ui.cx.run_until_parked();
    if ui.names().is_empty() {
        assert_eq!(ui.snapshot("message-title").label(), Some("Indexing…"));

        // Until the index is built the search is tried again every so often.
        // Refresh tries it now, and that search hasn't answered yet.
        ui.press_without_settling(&cmd("r"));
        assert!(ui.read(|f| f.search_in_flight_for_test()));
        assert!(
            !ui.exists("skeleton"),
            "placeholders would flash at every try"
        );
        assert_eq!(ui.snapshot("message-title").label(), Some("Indexing…"));
    }
    ui.settle();
}

// ----- sidebar, path bar, status bar ---------------------------------------------

#[gpui_kit::test]
fn sidebar_rows_and_menus(cx: &mut TestAppContext) {
    let fx = Fixture::new("sidebar");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("recents");
    assert_eq!(ui.read(|f| f.location().clone()), Location::Recents);

    let projects = fx.path("Projects");
    ui.update(|f, _, cx| f.add_favorite(projects.clone(), cx));
    let ix = ui.read(|f| {
        f.favorites()
            .iter()
            .position(|p| p.path == projects)
            .unwrap()
    });
    ui.click(("fav", ix));
    assert_eq!(ui.folder(), Some(projects.clone()));

    if ui.exists(("loc", 0usize)) {
        ui.click(("loc", 0usize));
        assert!(ui.folder().is_some(), "a location opens a folder");
    }

    ui.right_click(("fav", ix));
    assert!(ui.menu_open());
    assert!(ui.menu_labels().contains(&"Open Nushell Here".to_string()));
    ui.click_menu_item("Remove from Sidebar");
    assert!(!ui.read(|f| f.favorites().iter().any(|p| p.path == projects)));

    ui.click("index-settings");
    assert!(ui.dialog_open(), "the gear opens Settings");
}

#[gpui_kit::test]
fn a_click_in_the_sidebars_list_leaves_the_keyboard_where_it_was(cx: &mut TestAppContext) {
    let fx = Fixture::new("sidebar-keyboard");
    let mut ui = Ui::open(cx, &fx.dir);
    let query = |ui: &Ui| ui.read(|f| f.query().to_string());
    let projects = fx.path("Projects");
    let atlas = SmartFolder {
        name: "Atlas".into(),
        query: "atlas".into(),
        kind: KindFilter::Any,
        scope: None,
        contents: false,
        filter: None,
    };
    ui.update(|f, _, cx| {
        f.add_favorite(projects.clone(), cx);
        f.add_smart_folder(atlas, cx);
    });
    let favorite = ui.read(|f| {
        f.favorites()
            .iter()
            .position(|p| p.path == projects)
            .unwrap()
    });
    ui.click("view-list");
    assert!(ui.files_focused());

    // In the files, where the arrow keys then move through what the row
    // shows: a favorite's folder, and what a Smart Folder finds.
    ui.click(("fav", favorite));
    assert_eq!(ui.folder(), Some(projects.clone()));
    assert!(ui.files_focused(), "after a click on a favorite");
    ui.press("down");
    assert_eq!(ui.selected(), ["atlas"]);
    ui.click(("smart", 0usize));
    assert_eq!(query(&ui), "atlas");
    assert!(ui.files_focused(), "after a click on a Smart Folder");
    let found = ui.names();
    ui.press("down");
    assert_eq!(ui.selected(), found[..1], "the first of {found:?}");

    // In the search field, where the row ends the search and the next
    // letters start another in the folder it shows.
    ui.click("search");
    ui.click(("fav", favorite));
    assert_eq!(query(&ui), "");
    ui.type_text("atl");
    assert_eq!(query(&ui), "atl");
}

#[gpui_kit::test]
fn a_click_on_the_status_bar_or_the_path_bar_leaves_the_keyboard_in_the_files(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("bars-keyboard");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.select("Budget.csv");

    ui.click("status-text");
    assert!(ui.files_focused(), "after a click on the status bar");
    ui.press("down");
    assert_eq!(ui.selected(), ["notes.txt"], "the arrow keys still move");
    ui.type_text("ro");
    assert_eq!(ui.selected(), ["Roadmap.md"], "and typing still selects");

    // The path bar, to the right of its last crumb.
    let last = fffinder_core::places::breadcrumbs(&fx.dir).len() - 1;
    let crumb = ui.snapshot(("crumb", last)).bounds();
    let width = ui.window(|window, _| window.viewport_size().width);
    let blank = gpui_kit::point((crumb.right() + width) / 2., crumb.center().y);
    ui.click_point(blank);
    assert!(ui.files_focused(), "after a click on the path bar");
    ui.press("up");
    assert_eq!(ui.selected(), ["Projects"]);

    // The right button and the middle one take it no more than the left.
    ui.right_click("status-text");
    assert!(!ui.menu_open(), "the status bar has no menu");
    assert!(ui.files_focused(), "after a right-click");
    ui.click_with(gpui_kit::MouseButton::Middle, "status-text");
    assert!(ui.files_focused(), "after a middle-click");
    ui.press("up");
    assert_eq!(ui.selected(), ["notes.txt"]);
}

#[gpui_kit::test]
fn a_click_on_the_other_panes_status_bar_takes_the_keyboard_to_its_files(cx: &mut TestAppContext) {
    let fx = Fixture::new("bars-split");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            workspace: true,
            ..Default::default()
        },
    );
    ui.press(&cmd("\\"));
    let pane = ui
        .workspace
        .clone()
        .expect("opened with tabs")
        .read_with(ui.cx, |w, _| w.split_pane().cloned())
        .expect("split view is on");
    // The new pane has the keyboard, so this is its Nushell panel. Then
    // back to the first pane's files.
    ui.press("ctrl-`");
    assert!(pane.read_with(ui.cx, |f, _| f.nu_panel_open()));
    let first = ui.view.clone();
    ui.window(|window, cx| first.read(cx).files_focus_handle().focus(window, cx));

    // A button leaves the keyboard where it is, which is in the other pane.
    // Run has nothing to run.
    ui.click_in("split-pane", "nu-run");
    assert!(ui.files_focused(), "the first pane still has the keyboard");

    // The status bar takes no keys, and the keyboard isn't in its pane to
    // stay where it is.
    ui.click_in("split-pane", "status-text");
    let in_its_files = ui.window(|window, cx| pane.read(cx).files_focused(window));
    assert!(in_its_files, "the pane that was clicked has the keyboard");
    ui.press("down");
    let selected: Vec<String> = pane.read_with(ui.cx, |f, _| {
        f.selected_entries().into_iter().map(|e| e.name).collect()
    });
    assert_eq!(selected, ["Budget.csv"]);
    assert!(ui.selected().is_empty(), "the first pane got no key");
}

#[gpui_kit::test]
fn typing_goes_on_in_the_search_field_after_a_click_on_the_status_bar(cx: &mut TestAppContext) {
    let fx = Fixture::new("bars-typing");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("at");
    ui.click("status-text");
    ui.type_text("las");
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
}

#[gpui_kit::test]
fn the_settings_tooltip_names_its_shortcut(cx: &mut TestAppContext) {
    let fx = Fixture::new("settings-tip");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.hover("index-settings");
    assert_eq!(ui.snapshot("shortcut-tip").label(), Some("Settings"));
}

#[gpui_kit::test]
fn the_index_status_is_read_in_the_background(cx: &mut TestAppContext) {
    let fx = Fixture::new("index-status");
    let mut ui = Ui::open(cx, &fx.dir);
    let files = ui.read(|f| f.indexed_files());
    assert!(files > 0, "the sidebar has the count once the window is up");

    // Reading the status takes the index's locks, and can wait seconds for
    // them behind a scan or a content search. So asking for it, as Rescan
    // does, leaves what the window shows as it is, and the answer comes in
    // afterwards. Here the answer is another one: the index has lost its
    // folders in between.
    ui.cx.update(|cx| {
        cx.global::<fffinder::finder::AppState>()
            .engine
            .set_roots(&[]);
    });
    ui.window(|window, cx| window.dispatch_action(Rescan.boxed_clone(), cx));
    assert_eq!(ui.read(|f| f.indexed_files()), files);
    ui.settle();
    assert_eq!(ui.read(|f| f.indexed_files()), 0);
}

#[gpui_kit::test]
fn path_bar_crumbs_navigate(cx: &mut TestAppContext) {
    let fx = Fixture::new("crumbs");
    let mut ui = Ui::open(cx, &fx.dir);
    let atlas = fx.path("Projects/atlas");
    ui.go_to(&atlas);
    let crumbs = fffinder_core::places::breadcrumbs(&atlas).len();
    ui.click(("crumb", crumbs - 2));
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
}

#[gpui_kit::test]
fn status_bar_slider_resizes_icons(cx: &mut TestAppContext) {
    let fx = Fixture::new("slider");
    let mut ui = Ui::open(cx, &fx.dir);
    let before = ui.read(|f| f.icon_size_for_test());
    let bar = ui.snapshot("slider-bar-container").bounds();
    ui.click_at(
        "slider-bar-container",
        at(
            f32::from(bar.size.width) - 1.,
            f32::from(bar.size.height) / 2.,
        ),
    );
    let after = ui.read(|f| f.icon_size_for_test());
    assert!(after > before, "icon size {before} -> {after}");
    ui.click("view-list");
    assert!(
        !ui.exists("slider-bar-container"),
        "the slider is only for icons"
    );
}

#[gpui_kit::test]
fn status_bar_has_no_fps_by_default(cx: &mut TestAppContext) {
    let fx = Fixture::new("fps-off");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.cx
        .executor()
        .advance_clock(std::time::Duration::from_secs(2));
    ui.settle();
    assert!(!ui.exists("fps"));
}

#[gpui_kit::test]
fn status_bar_shows_fps_when_asked(cx: &mut TestAppContext) {
    let fx = Fixture::new("fps-on");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            fps: true,
            ..Default::default()
        },
    );
    ui.cx
        .executor()
        .advance_clock(std::time::Duration::from_secs(2));
    ui.settle();
    assert!(ui.exists("fps"), "shown once the first second has passed");
}

// ----- views -----------------------------------------------------------------------

#[gpui_kit::test]
fn list_headers_sort(cx: &mut TestAppContext) {
    let fx = Fixture::new("headers");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    for (header, key) in [
        ("Date Modified", SortKey::Modified),
        ("Size", SortKey::Size),
        ("Kind", SortKey::Kind),
        ("Name", SortKey::Name),
    ] {
        ui.click(header);
        assert_eq!(ui.read(|f| f.current_sort().key), key, "{header}");
    }
    let ascending = ui.read(|f| f.current_sort().ascending);
    ui.click("Name");
    assert_ne!(
        ui.read(|f| f.current_sort().ascending),
        ascending,
        "a second click flips"
    );
}

#[gpui_kit::test]
fn items_select_and_open_in_every_view(cx: &mut TestAppContext) {
    let fx = Fixture::new("items");
    let mut ui = Ui::open(cx, &fx.dir);
    let roadmap = fx.path("Roadmap.md");

    ui.click(item_id("tile", &roadmap));
    assert_eq!(ui.selected(), ["Roadmap.md"], "icons");

    ui.click("view-list");
    ui.click(item_id("row", &fx.path("notes.txt")));
    assert_eq!(ui.selected(), ["notes.txt"], "list");

    ui.click("view-gallery");
    ui.click(item_id("strip", &roadmap));
    assert_eq!(ui.selected(), ["Roadmap.md"], "gallery");
    assert!(
        ui.read(|f| f.has_preview_for_test(&roadmap)),
        "gallery loads the preview"
    );

    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("Projects")));
    assert_eq!(
        ui.read(|f| f.column_names_for_test()),
        vec![
            vec!["Budget.csv", "notes.txt", "Projects", "Roadmap.md"],
            vec!["atlas"]
        ]
    );
    ui.click(item_id("crow", &fx.path("Projects/atlas")));
    assert_eq!(ui.read(|f| f.column_names_for_test()).len(), 3);

    ui.click("view-list");
    ui.double_click(item_id("row", &fx.path("Projects")));
    assert_eq!(
        ui.folder(),
        Some(fx.path("Projects")),
        "double-click opens a folder"
    );
}

#[gpui_kit::test]
fn gallery_previews_an_item_with_nothing_selected(cx: &mut TestAppContext) {
    let fx = Fixture::new("gallery");
    let mut ui = Ui::open(cx, &fx.dir);
    assert!(ui.selected().is_empty());
    ui.click("view-gallery");
    assert_eq!(
        ui.selected(),
        ["Budget.csv"],
        "the gallery selects the first item"
    );
    assert!(
        ui.read(|f| f.has_preview_for_test(&fx.path("Budget.csv"))),
        "the preview finished loading instead of spinning"
    );
    // Escape clears the selection; the shown item still gets its preview.
    ui.press("escape");
    assert!(ui.read(|f| f.has_preview_for_test(&fx.path("Budget.csv"))));
}

#[gpui_kit::test]
fn context_menus_on_items_and_empty_space(cx: &mut TestAppContext) {
    let fx = Fixture::new("context-menus");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.right_click(item_id("row", &fx.path("notes.txt")));
    assert_eq!(ui.selected(), ["notes.txt"]);
    assert!(
        ui.menu_labels()
            .contains(&"Copy as Nushell List".to_string())
    );
    ui.click_menu_item("Copy Path");
    assert_eq!(ui.clipboard(), fx.path("notes.txt").display().to_string());

    ui.right_click(item_id("row", &fx.path("notes.txt")));
    ui.click_menu_item("Run Nushell Pipeline…");
    assert!(ui.read(|f| f.nu_panel_open()));
    ui.dispatch(ToggleNuPanel);

    // Empty space below the rows.
    let content = ui.snapshot("content").bounds();
    let below = at(40., f32::from(content.size.height) - 30.);
    ui.right_click_at("content", below);
    assert!(ui.menu_open(), "the background menu opens");
    assert!(!ui.menu_labels().contains(&"Copy Path".to_string()));
    ui.click_menu_item("Show Hidden Files");
    assert!(ui.read(|f| f.config().show_hidden));

    ui.right_click_at("content", below);
    ui.click_menu_item("Refresh");
    ui.right_click_at("content", below);
    ui.click_menu_item("New Folder");
    assert!(fx.path("untitled folder").is_dir());
    ui.press("escape");
}

#[gpui_kit::test]
fn an_empty_folder_has_the_empty_space_menu(cx: &mut TestAppContext) {
    let fx = Fixture::new("context-menu-empty");
    std::fs::create_dir(fx.path("Empty")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.select("notes.txt");
    ui.press(&cmd("c"));
    ui.go_to(&fx.path("Empty"));
    assert_eq!(ui.snapshot("message-title").label(), Some("No Items"));

    // All of an empty folder is empty space: where it is blank, and on the
    // message too.
    ui.right_click_at("content", at(120., 120.));
    assert!(ui.menu_open(), "the background menu opens");
    let labels = ui.menu_labels();
    for item in ["New Folder", "Paste Item", "Show Hidden Files"] {
        assert!(labels.contains(&item.to_string()), "{item} in {labels:?}");
    }
    ui.press("escape");
    assert!(!ui.menu_open());

    ui.right_click("message-title");
    ui.click_menu_item("New Folder");
    assert!(fx.path("Empty/untitled folder").is_dir());
    ui.press("escape");
    assert_eq!(ui.names(), ["untitled folder"]);
}

#[gpui_kit::test]
fn the_empty_space_menu_opens_where_an_items_menu_did_before(cx: &mut TestAppContext) {
    let fx = Fixture::new("context-menu-same-spot");
    std::fs::create_dir(fx.path("Empty")).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let item_menu = |ui: &mut Ui| ui.menu_labels().contains(&"Copy Path".to_string());

    // The list's last row goes to the Trash by its key, with the pointer
    // still where the row's menu was opened. That point is empty space now.
    let spot = ui
        .snapshot(item_id("row", &fx.path("Roadmap.md")))
        .bounds()
        .center();
    ui.right_click_point(spot);
    assert!(item_menu(&mut ui));
    ui.press("escape");
    ui.press(&cmd("backspace"));
    assert!(!ui.names().contains(&"Roadmap.md".to_string()));
    ui.right_click_point(spot);
    assert!(ui.menu_open(), "this press is on no item");
    assert!(!item_menu(&mut ui));
    ui.press("escape");
    assert!(ui.files_focused());

    // A folder opened by its key leaves the pointer where its row was, over
    // the message of the empty folder.
    let spot = ui
        .snapshot(item_id("row", &fx.path("Empty")))
        .bounds()
        .center();
    ui.right_click_point(spot);
    assert!(item_menu(&mut ui));
    ui.press("escape");
    ui.press("enter");
    assert_eq!(ui.snapshot("message-title").label(), Some("No Items"));
    ui.right_click_point(spot);
    ui.click_menu_item("New Folder");
    assert!(fx.path("Empty/untitled folder").is_dir());

    // The new folder is named where it shows, and the keys are the files'
    // again after it.
    ui.type_text("Reports");
    ui.press("enter");
    assert!(fx.path("Empty/Reports").is_dir());
    assert_eq!(ui.selected(), ["Reports"]);
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn no_results_has_the_empty_space_menu_except_on_its_buttons(cx: &mut TestAppContext) {
    let fx = Fixture::new("context-menu-no-results");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));
    ui.click("search-scope");
    ui.click("search");
    ui.type_text("budget");
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));

    // The menu takes the keyboard while it is open, and gives it back to
    // the search field.
    ui.right_click("message-detail");
    assert!(ui.menu_open());
    ui.press("escape");
    assert!(!ui.menu_open());
    ui.type_text("s");
    assert_eq!(ui.read(|f| f.query().to_string()), "budgets");

    // A button is no empty space.
    ui.right_click("empty-widen-scope");
    assert!(!ui.menu_open());
}

#[gpui_kit::test]
fn the_item_menu_shows_a_result_in_its_enclosing_folder(cx: &mut TestAppContext) {
    let fx = Fixture::new("item-menu-enclosing");
    let mut ui = Ui::open(cx, &fx.dir);
    let item = "Show in Enclosing Folder".to_string();
    ui.click("view-list");

    // An item in a folder is shown in its folder already.
    ui.right_click(item_id("row", &fx.path("notes.txt")));
    assert!(!ui.menu_labels().contains(&item));
    ui.press("escape");

    let hit = fx.path("Projects/atlas/Project Atlas.txt");
    ui.click("search");
    ui.type_text("atlas");
    ui.right_click(item_id("row", &hit));
    assert!(ui.menu_labels().contains(&item));
    // The kit says which key hints it drew only in a debug build.
    if cfg!(debug_assertions) {
        let keys = if cfg!(target_os = "macos") {
            "kbd:cmd-enter"
        } else {
            "kbd:ctrl-enter"
        };
        let mut window = gpui_kit::VisualTestContext::from_window(ui.handle, ui.cx);
        assert!(
            window.debug_bounds(keys).is_some(),
            "no {keys} beside the item"
        );
    }

    ui.click_menu_item(&item);
    assert_eq!(ui.folder().as_deref(), hit.parent());
    assert_eq!(ui.selected(), ["Project Atlas.txt"]);
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn the_keys_work_on_when_an_items_menu_loses_its_item(cx: &mut TestAppContext) {
    let fx = Fixture::new("context-menu-item-gone");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.right_click(item_id("row", &fx.path("notes.txt")));
    assert!(ui.menu_open());

    // Keys still reach the window while a menu is open. The other view has
    // no row for the menu to be drawn at, and the menu had the keyboard.
    ui.press(&cmd("1"));
    assert_eq!(ui.read(|f| f.effective_view()), ViewMode::Icons);
    assert!(!ui.menu_open());
    assert!(ui.files_focused());
    ui.press(&cmd("2"));
    assert_eq!(
        ui.read(|f| f.effective_view()),
        ViewMode::List,
        "the next key does its work"
    );
    ui.press("down");
    assert_eq!(ui.selected(), ["Projects"], "and so do the arrow keys");
}

#[gpui_kit::test]
fn the_keys_work_on_when_the_messages_menu_loses_its_message(cx: &mut TestAppContext) {
    let fx = Fixture::new("context-menu-message-gone");
    std::fs::create_dir(fx.path("Empty")).unwrap();
    std::fs::write(fx.path("Empty/.draft"), "").unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.go_to(&fx.path("Empty"));
    ui.right_click("message-title");
    assert!(ui.menu_open());

    // An item that shows up while the menu is open takes the message away,
    // and with it the menu, which had the keyboard.
    ui.update(|f, _, cx| f.set_show_hidden(true, cx));
    assert_eq!(ui.names(), [".draft"]);
    assert!(!ui.menu_open());
    assert!(ui.files_focused());
    ui.press("down");
    assert_eq!(ui.selected(), [".draft"]);
}

#[gpui_kit::test]
fn the_item_menu_shows_this_systems_keys_for_move_to_trash(cx: &mut TestAppContext) {
    // The kit says which key hints it drew only in a debug build.
    if !cfg!(debug_assertions) {
        return;
    }
    let fx = Fixture::new("trash-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.right_click(item_id("row", &fx.path("notes.txt")));
    assert!(ui.menu_labels().contains(&"Move to Trash".to_string()));

    // Both keys move items to the Trash everywhere; a menu shows one of them.
    let (usual, other) = if cfg!(target_os = "macos") {
        ("kbd:cmd-backspace", "kbd:delete")
    } else {
        ("kbd:delete", "kbd:ctrl-backspace")
    };
    let mut window = gpui_kit::VisualTestContext::from_window(ui.handle, ui.cx);
    assert!(
        window.debug_bounds(usual).is_some(),
        "no {usual} in the menu"
    );
    assert!(window.debug_bounds(other).is_none(), "{other} in the menu");
}

#[gpui_kit::test]
fn menus_in_the_window_show_this_systems_keys_for_open_quick_look_and_refresh(
    cx: &mut TestAppContext,
) {
    // The kit says which key hints it drew only in a debug build.
    if !cfg!(debug_assertions) {
        return;
    }
    let fx = Fixture::new("open-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");

    // Each has a key with Cmd and one without, and a menu shows one of them:
    // Finder's Cmd-O for Open on a Mac and Return elsewhere, and the key with
    // Cmd for the other two everywhere.
    let (shown, others) = if cfg!(target_os = "macos") {
        (["kbd:cmd-o", "kbd:cmd-y"], ["kbd:enter", "kbd:space"])
    } else {
        (["kbd:enter", "kbd:ctrl-y"], ["kbd:ctrl-o", "kbd:space"])
    };
    ui.right_click(item_id("row", &fx.path("notes.txt")));
    assert!(ui.menu_labels().contains(&"Quick Look".to_string()));
    let mut window = gpui_kit::VisualTestContext::from_window(ui.handle, ui.cx);
    for keys in shown {
        assert!(
            window.debug_bounds(keys).is_some(),
            "no {keys} in the item menu"
        );
    }
    for keys in others {
        assert!(
            window.debug_bounds(keys).is_none(),
            "{keys} in the item menu"
        );
    }
    ui.press("escape");

    let refresh = if cfg!(target_os = "macos") {
        "kbd:cmd-r"
    } else {
        "kbd:ctrl-r"
    };
    let content = ui.snapshot("content").bounds();
    ui.right_click_at("content", at(40., f32::from(content.size.height) - 30.));
    assert!(ui.menu_labels().contains(&"Refresh".to_string()));
    assert!(
        window.debug_bounds(refresh).is_some(),
        "no {refresh} in the background menu"
    );
    assert!(
        window.debug_bounds("kbd:f5").is_none(),
        "kbd:f5 in the background menu"
    );
}

// ----- dialogs ---------------------------------------------------------------------

#[gpui_kit::test]
fn settings_dialog_controls(cx: &mut TestAppContext) {
    let fx = Fixture::new("settings");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd(","));
    assert!(ui.dialog_open());

    // The switch used to dispatch an action the dialog's focus could not
    // deliver, so clicking it did nothing.
    assert_eq!(ui.snapshot("show-hidden").checked(), Some(false));
    ui.click("show-hidden");
    assert!(ui.read(|f| f.config().show_hidden));
    assert_eq!(ui.snapshot("show-hidden").checked(), Some(true));
    assert!(ui.names().contains(&".secret".to_string()));
    ui.click("show-hidden");
    assert!(!ui.read(|f| f.config().show_hidden));

    for (id, appearance, dark) in [
        ("ap-dark", Appearance::Dark, true),
        ("ap-light", Appearance::Light, false),
        ("ap-system", Appearance::System, false),
    ] {
        ui.click(id);
        assert_eq!(ui.read(|f| f.config().appearance), appearance, "{id}");
        if appearance != Appearance::System {
            assert_eq!(ui.cx.update(|cx| cx.theme().is_dark()), dark, "{id}");
        }
    }

    ui.click("rescan");
    assert!(ui.dialog_open(), "Rescan keeps Settings open");

    // Add Folder… asks for folders with the system picker.
    ui.click("add-root");
    assert!(ui.cx.did_prompt_for_paths());
    let projects = fx.path("Projects");
    ui.cx
        .simulate_path_prompt_response(|_| Some(vec![projects.clone()]));
    ui.settle();
    assert_eq!(
        ui.read(|f| f.config().search_roots.clone()),
        vec![fx.dir.clone(), projects.clone()]
    );
    ui.click(("remove-root", 1usize));
    assert_eq!(
        ui.read(|f| f.config().search_roots.clone()),
        vec![fx.dir.clone()]
    );

    // These would index the whole disk or the home folder; check the
    // setting, then put the fixture back before the engine starts on it.
    let disk = if cfg!(windows) { "C:\\" } else { "/" };
    ui.click_without_settling("add-disk");
    assert_eq!(
        ui.read(|f| f.config().search_roots.clone()),
        vec![std::path::PathBuf::from(disk)]
    );
    let root = fx.dir.clone();
    ui.update(|f, _, cx| f.set_search_roots(vec![root.clone()], cx));
    ui.click_without_settling("reset-roots");
    assert_eq!(
        ui.read(|f| f.config().search_roots.clone()),
        fffinder_core::config::default_search_roots()
    );
    ui.update(|f, _, cx| f.set_search_roots(vec![root.clone()], cx));

    ui.click("settings-done");
    assert!(!ui.dialog_open());
    assert!(ui.files_focused(), "Done returns focus to the files");

    ui.press(&cmd(","));
    ui.press("escape");
    assert!(!ui.dialog_open(), "Escape closes Settings");
    ui.press(&cmd(","));
    ui.close_dialog();
    assert!(!ui.dialog_open(), "the close button closes Settings");
}

#[gpui_kit::test]
fn settings_text_sizes(cx: &mut TestAppContext) {
    let fx = Fixture::new("text-sizes");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let row = item_id("row", &fx.path("notes.txt"));
    let row_height = |ui: &mut Ui| ui.snapshot(row.clone()).bounds().size.height;
    let font = |ui: &mut Ui| ui.window(|_, cx| cx.theme().font_size);
    let (height, size) = (row_height(&mut ui), font(&mut ui));
    ui.press(&cmd(","));

    // Text size scales the window's own layout and the kit's controls.
    assert_eq!(ui.snapshot("ui-text-reset").label(), Some("100%"));
    ui.click("ui-text-bigger");
    ui.click("ui-text-bigger");
    assert_eq!(ui.read(|f| f.config().ui_scale), 1.2);
    assert_eq!(ui.snapshot("ui-text-reset").label(), Some("120%"));
    assert_eq!(font(&mut ui), size * 1.2);
    // Switching light and dark reloads the theme; the scale applies once.
    ui.close_dialog();
    ui.press(&cmd("shift-d"));
    ui.press(&cmd("shift-d"));
    assert_eq!(font(&mut ui), size * 1.2, "not scaled twice");
    ui.press(&cmd(","));
    ui.click("ui-text-smaller");
    assert_eq!(ui.read(|f| f.config().ui_scale), 1.1);
    ui.close_dialog();
    // Layout snaps to half pixels.
    let grown = f32::from(row_height(&mut ui)) / f32::from(height);
    assert!((grown - 1.1).abs() < 0.03, "rows grow with it: {grown}");

    // Nushell text size has its own row; both reset from their labels.
    ui.press(&cmd(","));
    ui.click("nu-text-bigger");
    assert_eq!(ui.read(|f| f.config().nu_font_size), 13.);
    assert_eq!(ui.snapshot("nu-text-reset").label(), Some("13 pt"));
    ui.click("nu-text-reset");
    ui.click("ui-text-reset");
    assert_eq!(ui.read(|f| f.config().nu_font_size), 12.);
    assert_eq!(ui.read(|f| f.config().ui_scale), 1.0);
    assert_eq!(font(&mut ui), size, "the theme's font size comes back");
    ui.close_dialog();
    assert_eq!(row_height(&mut ui), height);
}

#[gpui_kit::test]
fn settings_search_engine(cx: &mut TestAppContext) {
    use fffinder::finder::AppState;
    use fffinder_core::SearchCase;
    let fx = Fixture::new("search-engine");
    let mut ui = Ui::open(cx, &fx.dir);
    let engine = ui.window(|_, cx| cx.global::<AppState>().engine.clone());
    let search = |ui: &Ui| ui.read(|f| f.config().search.clone());
    ui.press(&cmd(","));

    // A switch that shapes the index rebuilds it; searching still works.
    ui.click("follow-symlinks");
    assert!(search(&ui).follow_symlinks);
    assert!(engine.wait_ready(std::time::Duration::from_secs(20)));
    ui.settle();
    assert!(
        engine.search_settings().follow_symlinks,
        "the engine has it"
    );
    ui.click("git-recency");
    assert!(!search(&ui).git_recency);
    ui.click("typos");
    assert!(!search(&ui).typos);

    ui.click("case-sensitive");
    assert_eq!(search(&ui).case, SearchCase::Sensitive);
    ui.click("time-limit-less");
    assert_eq!(search(&ui).time_limit_secs, 2);
    assert_eq!(ui.snapshot("time-limit-value").label(), Some("2 s"));
    ui.click("threads-more");
    assert_eq!(search(&ui).threads, 1);
    ui.click("max-file-size-more");
    assert_eq!(search(&ui).max_file_size_mb, 20);
    ui.click("result-limit-more");
    assert_eq!(ui.read(|f| f.config().result_limit), 1000);
    assert_eq!(engine.search_settings(), search(&ui));
    ui.close_dialog();

    assert!(engine.wait_ready(std::time::Duration::from_secs(20)));
    ui.click("search");
    ui.type_text("notes");
    assert!(
        ui.names().contains(&"notes.txt".to_string()),
        "{:?}",
        ui.names()
    );
}

#[gpui_kit::test]
fn settings_terminal_choice(cx: &mut TestAppContext) {
    let fx = Fixture::new("terminal-choice");
    let ghostty = Terminal {
        id: "ghostty".into(),
        name: "Ghostty".into(),
        location: Some("/Applications/Ghostty.app".into()),
        launch: Launch::Ghostty {
            bundle: "/Applications/Ghostty.app".into(),
        },
    };
    let other = Terminal {
        id: "terminal".into(),
        name: "Terminal".into(),
        location: Some("/usr/bin/term".into()),
        launch: Launch::Exec {
            program: "/usr/bin/term".into(),
            args: vec!["--cwd".into(), "{dir}".into(), "{nu}".into()],
        },
    };
    let nu = PathBuf::from("/opt/nu/bin/nu");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu.clone()),
            terminals: vec![ghostty, other],
            ..Default::default()
        },
    );
    let dir = fx.dir.display().to_string();
    let last_launch = |ui: &Ui| match ui.system_calls().last() {
        Some(SystemCall::Terminal { program, args }) => {
            (program.display().to_string(), args.clone())
        }
        other => panic!("no terminal was started: {other:?}"),
    };
    let label = |ui: &mut Ui| ui.snapshot("terminal-app").label().map(str::to_string);

    ui.press(&cmd(","));
    assert_eq!(label(&mut ui).as_deref(), Some("Automatic (Ghostty)"));
    ui.click("terminal-app");
    assert_eq!(
        ui.menu_labels(),
        [
            "Automatic (Ghostty)",
            "Ghostty",
            "Terminal",
            "Custom Command",
            "Detect Again"
        ]
    );
    ui.click_menu_item("Terminal");
    assert_eq!(
        ui.read(|f| f.terminal_setting().app).as_deref(),
        Some("terminal")
    );
    assert_eq!(label(&mut ui).as_deref(), Some("Terminal"));
    ui.click("try-terminal");
    assert_eq!(
        last_launch(&ui),
        (
            "/usr/bin/term".into(),
            vec!["--cwd".into(), dir.clone(), nu.display().to_string()]
        )
    );

    // A custom command shows its field; words are split like a shell would.
    assert!(!ui.exists("terminal-input"));
    ui.click("terminal-app");
    ui.click_menu_item("Custom Command");
    ui.click("terminal-input");
    ui.type_text("foot -D {dir} {nu}");
    assert_eq!(
        ui.read(|f| f.terminal_setting().command).as_deref(),
        Some("foot -D {dir} {nu}")
    );
    ui.click("try-terminal");
    assert_eq!(
        last_launch(&ui),
        (
            "foot".into(),
            vec!["-D".into(), dir.clone(), nu.display().to_string()]
        )
    );

    // Automatic picks Ghostty, which goes through AppleScript; the custom
    // command is kept for later.
    ui.click("terminal-app");
    ui.click_menu_item("Automatic (Ghostty)");
    assert!(!ui.exists("terminal-input"));
    assert_eq!(ui.read(|f| f.terminal_setting().app), None);
    assert!(ui.read(|f| f.terminal_setting().command).is_some());
    ui.click("try-terminal");
    assert_eq!(last_launch(&ui).0, "/usr/bin/osascript");

    // A terminal that isn't installed any more falls back to Automatic.
    ui.update(|f, _, cx| f.set_terminal_app(Some("kitty".into()), cx));
    assert_eq!(label(&mut ui).as_deref(), Some("kitty (not installed)"));
    ui.click("try-terminal");
    assert_eq!(last_launch(&ui).0, "/usr/bin/osascript");

    ui.click("terminal-app");
    ui.click_menu_item("Detect Again");
    assert_eq!(ui.read(|f| f.terminals().map(<[Terminal]>::len)), Some(2));

    // The menu items and the shortcut use the same choice.
    ui.click("settings-done");
    ui.update(|f, _, cx| f.set_terminal_app(Some("terminal".into()), cx));
    ui.press(&cmd("alt-t"));
    assert_eq!(last_launch(&ui).1[1], dir);
    ui.click("nushell");
    ui.click_menu_item("Open Nushell Here");
    assert_eq!(last_launch(&ui).1[1], dir);
    let calls = ui.system_calls().len();
    assert_eq!(calls, 6, "one launch per request: {:?}", ui.system_calls());
}

#[gpui_kit::test]
fn settings_nu_choice(cx: &mut TestAppContext) {
    let fx = Fixture::new("nu-choice");
    // Any file can be chosen. An empty one can't be run, and what goes wrong
    // names it, which shows what was run.
    let fake = fx.path("fake-nu");
    std::fs::write(&fake, "").unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    let in_use = |ui: &mut Ui| ui.window(|_, cx| cx.global::<AppState>().nu.clone());
    let chosen = |ui: &Ui| ui.read(|f| f.config().nu_path.clone());
    let status = |ui: &mut Ui| ui.snapshot("nu-path").label().unwrap().to_string();
    let choose = |ui: &mut Ui, file: Option<&PathBuf>| {
        ui.click("choose-nu");
        assert!(ui.cx.did_prompt_for_paths());
        ui.cx.simulate_path_prompt_response(|options| {
            assert!(options.files && !options.directories && !options.multiple);
            file.map(|file| vec![file.clone()])
        });
        ui.settle();
    };

    // The window opened without a `nu`.
    ui.press(&cmd(","));
    assert!(status(&mut ui).contains("not found"), "{}", status(&mut ui));
    assert_eq!(ui.snapshot("nu-auto").label(), Some("Look Again"));

    // Choose… asks for a file, which is in use at once.
    choose(&mut ui, Some(&fake));
    assert_eq!(chosen(&ui), Some(fake.clone()));
    assert_eq!(in_use(&mut ui), Some(fake.clone()));
    assert_eq!(status(&mut ui), format!("Using {}", tildify(&fake)));
    let error = ui.read(|f| f.user_actions_error().map(str::to_string));
    assert!(
        error.as_deref().is_some_and(|e| e.contains("fake-nu")),
        "the actions are read again with it: {error:?}"
    );

    // Find Automatically goes back to looking for one.
    assert_eq!(ui.snapshot("nu-auto").label(), Some("Find Automatically"));
    ui.click("nu-auto");
    let found = fffinder::nushell::find_nu(None);
    assert_eq!(chosen(&ui), None);
    assert_eq!(in_use(&mut ui), found);
    assert_eq!(
        ui.read(|f| f.user_actions_error().map(str::to_string)),
        None
    );
    assert_eq!(
        ui.exists("nu-auto"),
        found.is_none(),
        "with one found there's nothing to go back to"
    );

    // Leaving the chooser without a file changes nothing.
    choose(&mut ui, None);
    assert_eq!(chosen(&ui), None);
    assert_eq!(in_use(&mut ui), found);

    // A chosen file that isn't there is named, with what's used instead.
    let gone = fx.path("gone");
    choose(&mut ui, Some(&gone));
    assert_eq!(chosen(&ui), Some(gone.clone()));
    assert_eq!(in_use(&mut ui), found);
    let gone = tildify(&gone);
    let says = match &found {
        Some(found) => format!("{gone} wasn’t found, so {} is used.", tildify(found)),
        None => format!("{gone} wasn’t found, and no other nu was found."),
    };
    assert_eq!(status(&mut ui), says);
    assert_eq!(ui.snapshot("nu-auto").label(), Some("Find Automatically"));
}

#[gpui_kit::test]
fn what_waited_for_a_nu_runs_with_the_one_chosen_in_settings(cx: &mut TestAppContext) {
    let fx = Fixture::new("nu-waited");
    // An empty file can't be run, and what goes wrong names it.
    let fake = fx.path("fake-nu");
    std::fs::write(&fake, "").unwrap();
    std::fs::write(fx.path("greet.nu"), "# Say hi\ndef greet [] { 'hi' }\n").unwrap();
    let mut ui = Ui::open(cx, &fx.dir);

    // A script in the preview pane, a filter, and Nushell's colors, with no
    // `nu` to run any of them.
    ui.click("view-list");
    ui.dispatch(TogglePreviewPane);
    ui.select("greet.nu");
    ui.update(|f, _, cx| {
        f.set_filter("size > 0b".into(), cx);
        f.set_use_nu_environment(true, cx);
    });
    let script = ui.read(|f| f.primary_entry()).expect("the script");
    let filter = |ui: &Ui| ui.read(|f| f.filter_error().unwrap_or_default().to_string());
    let colors = |ui: &Ui| ui.read(|f| f.nu_env_summary());
    // Nushell's own coloring finds the comment, which no other does.
    let colored_by_nu = |ui: &Ui| {
        ui.read(|f| match f.preview_of(&script) {
            Preview::Code { comments, .. } => !comments.is_empty(),
            _ => false,
        })
    };
    assert!(filter(&ui).contains("not found"), "{}", filter(&ui));
    assert!(colors(&ui).contains("not found"), "{}", colors(&ui));
    assert!(!colored_by_nu(&ui));

    // Each runs with the file chosen in Settings.
    ui.press(&cmd(","));
    ui.click("choose-nu");
    ui.cx
        .simulate_path_prompt_response(|_| Some(vec![fake.clone()]));
    ui.settle();
    assert!(filter(&ui).contains("fake-nu"), "{}", filter(&ui));
    assert!(colors(&ui).contains("fake-nu"), "{}", colors(&ui));
    assert!(!colored_by_nu(&ui));

    // And with a real one, where there is one. Its login files aren't a
    // test's to run, so the colors go off first.
    ui.click("use-nu-env");
    ui.click("nu-auto");
    if fffinder::nushell::find_nu(None).is_some() {
        assert_eq!(filter(&ui), "");
        let names = ui.names();
        assert!(
            !names.contains(&"fake-nu".to_string()),
            "the filter left the empty file out: {names:?}"
        );
        assert!(colored_by_nu(&ui), "the script is previewed again");
    }
}

#[gpui_kit::test]
fn a_preview_still_being_read_is_read_again_with_the_nu_found(cx: &mut TestAppContext) {
    if fffinder::nushell::find_nu(None).is_none() {
        eprintln!("nu not installed; skipping");
        return;
    }
    let fx = Fixture::new("nu-preview-waited");
    std::fs::write(fx.path("greet.nu"), "# Say hi\ndef greet [] { 'hi' }\n").unwrap();
    // The window opens without a `nu`, as if it were installed afterwards.
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    ui.dispatch(TogglePreviewPane);
    let names = ui.names();
    let script = names.iter().position(|name| name == "greet.nu").unwrap();

    // The script is selected, and its preview is still being read without a
    // `nu` when Look Again in Settings finds one.
    ui.update(|f, window, cx| {
        f.click_row(script, Default::default(), 1, window, cx);
        f.set_nu_path(None, cx);
    });

    let script = ui.read(|f| f.primary_entry()).expect("the script");
    // Nushell's own coloring finds the comment, which no other does.
    let colored_by_nu = ui.read(|f| match f.preview_of(&script) {
        Preview::Code { comments, .. } => !comments.is_empty(),
        _ => false,
    });
    assert!(
        colored_by_nu,
        "the preview is read again with the `nu` found"
    );
}

#[gpui_kit::test]
fn go_to_folder_lists_matches_for_the_path_it_opens_with(cx: &mut TestAppContext) {
    let fx = Fixture::new("goto-opens");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.go_to(&fx.path("Projects"));

    ui.press(&cmd("shift-g"));
    ui.settle();

    assert!(ui.exists(("goto-match", 0usize)), "before any typing");
    // Tab goes on from there, into the folder it opened with.
    ui.press("tab");
    ui.settle();
    ui.type_text("atlas");
    ui.settle();
    ui.press("enter");
    assert_eq!(ui.folder(), Some(fx.path("Projects/atlas")));
}

#[gpui_kit::test]
fn clicking_a_go_to_folder_match_keeps_the_keyboard_in_the_field(cx: &mut TestAppContext) {
    let fx = Fixture::new("goto-click");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-g"));
    ui.type_text("proj");
    ui.settle();

    ui.click(("goto-match", 0usize));
    ui.settle();
    ui.type_text("atlas");
    ui.press("enter");

    assert_eq!(ui.folder(), Some(fx.path("Projects/atlas")));
}

#[gpui_kit::test]
fn tab_waits_for_the_matches_of_what_is_typed_now(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt as _;
    let fx = Fixture::new("goto-stale");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-g"));
    ui.type_text("proj");
    ui.settle();

    // The matches for "proj" are in, and the read for "projx" hasn't finished.
    ui.window(|window, cx| {
        window.input("x", cx);
        window.press("tab", cx);
    });
    ui.settle();
    ui.press("enter");

    assert!(ui.dialog_open(), "projx doesn't exist");
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
}

#[gpui_kit::test]
fn go_to_folder_starts_from_home_where_there_is_no_folder(cx: &mut TestAppContext) {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let in_cwd = |name: &std::ffi::OsStr| std::path::Path::new(name).exists();
    let Some(name) = std::fs::read_dir(&home).ok().and_then(|items| {
        items
            .flatten()
            .map(|item| item.file_name())
            .find(|name| !in_cwd(name) && !name.to_string_lossy().starts_with('.'))
    }) else {
        return;
    };
    let fx = Fixture::new("goto-home");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.update(|f, window, cx| f.navigate(Location::Recents, window, cx));
    ui.press(&cmd("shift-g"));

    ui.type_text(&name.to_string_lossy());
    ui.settle();

    assert!(ui.exists(("goto-match", 0usize)), "{name:?} is in {home:?}");
}

#[gpui_kit::test]
fn go_to_folder_dialog(cx: &mut TestAppContext) {
    let fx = Fixture::new("goto");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("shift-g"));
    assert!(ui.dialog_open());
    // The field starts selected, so typing replaces it; Return goes.
    ui.type_text("Projects");
    ui.press("enter");
    assert!(!ui.dialog_open());
    assert_eq!(ui.folder(), Some(fx.path("Projects")));

    // Tab finishes the path as far as it can, one folder at a time.
    ui.go_to(&fx.dir);
    ui.press(&cmd("shift-g"));
    ui.type_text("proj");
    ui.settle();
    assert!(ui.exists(("goto-match", 0usize)), "the match is listed");
    ui.press("tab");
    assert!(ui.dialog_open(), "Tab stays in the field");
    ui.settle();
    ui.type_text("at");
    ui.settle();
    ui.press("tab");
    ui.settle();
    ui.press("enter");
    assert!(!ui.dialog_open());
    assert_eq!(ui.folder(), Some(fx.path("Projects/atlas")));

    // Clicking a match fills it in.
    ui.go_to(&fx.dir);
    ui.press(&cmd("shift-g"));
    ui.type_text("proj");
    ui.settle();
    ui.click(("goto-match", 0usize));
    ui.settle();
    ui.press("enter");
    assert_eq!(ui.folder(), Some(fx.path("Projects")));

    ui.go_to(&fx.dir);
    ui.press(&cmd("shift-g"));
    ui.type_text("does-not-exist");
    ui.click("goto-go");
    assert!(ui.dialog_open(), "a missing folder keeps the dialog open");
    ui.click("goto-cancel");
    assert!(!ui.dialog_open());

    ui.press(&cmd("shift-g"));
    ui.type_text(&fx.path("Roadmap.md").display().to_string());
    ui.click("goto-go");
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));
    assert_eq!(
        ui.selected(),
        ["Roadmap.md"],
        "a file opens its folder and selects it"
    );
}

#[gpui_kit::test]
fn rename_dialog_in_the_gallery(cx: &mut TestAppContext) {
    // The gallery has no room to edit a name in place, so it asks.
    let fx = Fixture::new("rename");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-gallery");
    ui.select("Roadmap.md");
    ui.press("f2");
    assert!(ui.dialog_open());
    // Only the name before the extension is selected.
    ui.type_text("Plan");
    ui.click("rename-ok");
    assert!(!ui.dialog_open());
    assert!(fx.path("Plan.md").exists() && !fx.path("Roadmap.md").exists());
    assert!(ui.names().contains(&"Plan.md".to_string()));

    ui.select("Plan.md");
    ui.press("f2");
    ui.type_text("Later");
    ui.click("rename-cancel");
    assert!(fx.path("Plan.md").exists(), "Cancel leaves the name alone");

    ui.select("Plan.md");
    ui.press("f2");
    ui.press(&cmd("a"));
    ui.type_text("notes.txt");
    ui.press("enter");
    assert!(
        ui.dialog_open(),
        "renaming onto an existing name is refused"
    );
    ui.press("escape");
    assert!(fx.path("Plan.md").exists());
}

#[gpui_kit::test]
fn quick_look_buttons_and_keys(cx: &mut TestAppContext) {
    let fx = Fixture::new("quick-look");
    let mut ui = Ui::open(cx, &fx.dir);
    let open = |ui: &Ui| ui.read(|f| f.quick_look_is_open() && f.quick_look_window().is_some());
    ui.select("Roadmap.md");
    ui.press("space");
    assert!(open(&ui), "Quick Look opens a window of its own");
    assert!(!ui.dialog_open());
    assert!(ui.read(|f| f.has_preview_for_test(&fx.path("Roadmap.md"))));
    assert!(ui.in_quick_look(|ui| ui.exists("ql-open")));
    ui.in_quick_look(|ui| ui.press("space"));
    assert!(!open(&ui), "Space in Quick Look closes it again");

    ui.press("space");
    ui.in_quick_look(|ui| ui.press("escape"));
    assert!(!open(&ui), "so does Escape");

    // Arrows in Quick Look move the browser's selection.
    ui.press("space");
    ui.in_quick_look(|ui| ui.press("left"));
    assert_ne!(ui.selected(), ["Roadmap.md"], "Left picks the item before");
    ui.in_quick_look(|ui| ui.press("right"));
    assert_eq!(ui.selected(), ["Roadmap.md"]);

    ui.in_quick_look(|ui| ui.click("ql-reveal"));
    assert!(open(&ui), "Show in Finder keeps the preview open");
    ui.in_quick_look(|ui| ui.click("ql-open"));
    assert!(!open(&ui));
    let roadmap = fx.path("Roadmap.md");
    assert_eq!(
        ui.system_calls(),
        [
            SystemCall::Reveal(roadmap.clone()),
            SystemCall::Open(roadmap)
        ]
    );
    ui.press("space");
    assert!(open(&ui), "one Space reopens it after Open");
    ui.press("space");
    assert!(!open(&ui), "Space in the browser closes it too");

    ui.press(&cmd("y"));
    ui.close_quick_look();
    assert!(
        !ui.read(|f| f.quick_look_is_open()),
        "closing its window resets the state"
    );
    ui.press("space");
    assert!(open(&ui), "and Space opens it again");
}

#[gpui_kit::test]
fn files_no_app_claims_open_in_an_editor(cx: &mut TestAppContext) {
    use fffinder::nu_env::NuEnv;
    let fx = Fixture::new("no-app");
    std::fs::write(fx.path("blob.bin"), [0u8, 159, 146, 150, 0, 1]).unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    let (roadmap, notes, blob) = (
        fx.path("Roadmap.md"),
        fx.path("notes.txt"),
        fx.path("blob.bin"),
    );
    ui.set_no_app(&roadmap);
    ui.set_no_app(&blob);

    // Quick Look's Open hands an unclaimed text file to the text editor.
    ui.select("Roadmap.md");
    ui.press("space");
    ui.in_quick_look(|ui| ui.click("ql-open"));
    assert!(ui.read(|f| f.quick_look_window()).is_none());
    assert_eq!(ui.system_calls(), [SystemCall::EditText(roadmap.clone())]);

    // A file with an app opens in it; a binary no app claims doesn't open.
    ui.select("notes.txt");
    ui.press("enter");
    ui.select("blob.bin");
    ui.press("enter");
    ui.dispatch(OpenInEditor);
    assert_eq!(
        ui.system_calls(),
        [
            SystemCall::EditText(roadmap.clone()),
            SystemCall::Open(notes)
        ]
    );

    // "Open in Editor" without Nushell's editor does the same as Open.
    ui.select("Roadmap.md");
    ui.dispatch(OpenInEditor);
    assert_eq!(
        ui.system_calls().last(),
        Some(&SystemCall::EditText(roadmap.clone()))
    );

    // With Nushell's editor on, Return opens the file in that editor.
    let env = NuEnv {
        buffer_editor: Some(json!(["code", "--wait"])),
        ..Default::default()
    };
    ui.update(|f, _, cx| f.set_nu_env_for_test(env, cx));
    ui.press("enter");
    assert_eq!(
        ui.system_calls().last(),
        Some(&SystemCall::Terminal {
            program: "code".into(),
            args: vec!["--wait".into(), roadmap.display().to_string()],
        })
    );
}

#[gpui_kit::test]
fn quick_look_text_size(cx: &mut TestAppContext) {
    let fx = Fixture::new("quick-look-text");
    let mut ui = Ui::open(cx, &fx.dir);
    let size = |ui: &Ui| ui.read(|f| f.config().preview_font_size);
    let open = |ui: &Ui| ui.read(|f| f.quick_look_window().is_some());
    ui.select("Roadmap.md");
    ui.press("space");
    assert_eq!(size(&ui), 11.5);
    ui.in_quick_look(|ui| {
        assert_eq!(ui.snapshot("ql-text-reset").label(), Some("11.5 pt"));
        ui.click("ql-text-bigger");
        assert_eq!(size(ui), 12.5);
        assert_eq!(ui.snapshot("ql-text-reset").label(), Some("12.5 pt"));
        ui.press(&cmd("="));
        assert_eq!(size(ui), 13.5);
        ui.press(&cmd("-"));
        ui.click("ql-text-smaller");
        assert_eq!(size(ui), 11.5);
        ui.press(&cmd("="));
        ui.click("ql-text-reset");
        assert_eq!(size(ui), 11.5, "the size label resets it");
        ui.press(&cmd("="));
        ui.press(&cmd("0"));
        assert_eq!(size(ui), 11.5, "and so does Cmd-0");
    });

    ui.update(|f, _, cx| f.set_preview_font_size(Some(31.5), cx));
    ui.in_quick_look(|ui| {
        ui.press(&cmd("="));
        ui.press(&cmd("="));
    });
    assert_eq!(size(&ui), 32., "it stops at the largest size");

    assert!(open(&ui));
    ui.in_quick_look(|ui| ui.press("space"));
    assert!(!open(&ui), "Space still closes it after the clicks");

    ui.select("Projects");
    ui.press("space");
    assert!(open(&ui));
    assert!(
        !ui.in_quick_look(|ui| ui.exists("ql-text-bigger")),
        "a folder has no text to size"
    );
    ui.in_quick_look(|ui| ui.press("space"));
    assert_eq!(size(&ui), 32., "the size stays for the next preview");
}

#[gpui_kit::test]
fn quick_look_arrows_move_through_items(cx: &mut TestAppContext) {
    let fx = Fixture::new("quick-look-arrows");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("view-list");
    let names = ui.names();
    let shown = |ui: &Ui| ui.read(|f| f.primary_entry().map(|e| e.name));
    let open = |ui: &Ui| ui.read(|f| f.quick_look_window().is_some());
    ui.select(&names[0]);
    ui.press("space");
    ui.in_quick_look(|ui| ui.press("down"));
    assert!(open(&ui), "the arrows keep Quick Look open");
    assert_eq!(ui.selected(), [names[1].as_str()]);
    assert!(ui.read(|f| f.has_preview_for_test(&fx.path(&names[1]))));
    ui.in_quick_look(|ui| ui.press("right"));
    assert_eq!(
        ui.selected(),
        [names[2].as_str()],
        "Right is next in the list"
    );
    ui.in_quick_look(|ui| {
        ui.press("left");
        ui.press("up");
    });
    assert_eq!(ui.selected(), [names[0].as_str()]);
    ui.in_quick_look(|ui| ui.press("space"));
    assert!(!open(&ui), "Space still closes it");

    // With several items selected, the arrows step through just those.
    ui.select(&names[0]);
    ui.select_also(&names[2]);
    ui.press("space");
    assert_eq!(shown(&ui).as_deref(), Some(names[2].as_str()));
    ui.in_quick_look(|ui| ui.press("right"));
    assert_eq!(
        shown(&ui).as_deref(),
        Some(names[0].as_str()),
        "wraps around"
    );
    ui.in_quick_look(|ui| ui.press("up"));
    assert_eq!(shown(&ui).as_deref(), Some(names[2].as_str()));
    assert_eq!(ui.selected(), [names[0].as_str(), names[2].as_str()]);
    assert!(ui.read(|f| f.has_preview_for_test(&fx.path(&names[0]))));
    ui.in_quick_look(|ui| ui.press("space"));

    // In columns, the arrows stay in the active column, even after opening
    // a folder, which moves the focus to the files.
    ui.click("view-columns");
    ui.click(item_id("crow", &fx.path("notes.txt")));
    ui.press("space");
    ui.in_quick_look(|ui| ui.press("down"));
    assert_eq!(ui.selected(), ["Projects"]);
    assert!(ui.read(|f| f.has_preview_for_test(&fx.path("Projects"))));
    ui.in_quick_look(|ui| ui.press("right"));
    assert_eq!(ui.selected(), ["Roadmap.md"]);
    ui.in_quick_look(|ui| ui.press("left"));
    assert_eq!(shown(&ui).as_deref(), Some("Projects"));
    assert!(open(&ui));
}

// ----- Nushell panel -----------------------------------------------------------------

#[gpui_kit::test]
fn nushell_panel_controls(cx: &mut TestAppContext) {
    let fx = Fixture::new("nu-panel");
    let nu = fffinder::nushell::find_nu(None);
    let have_nu = nu.is_some();
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu,
            ..Default::default()
        },
    );
    ui.press("ctrl-`");
    assert!(ui.read(|f| f.nu_panel_open()));

    // An example fills the field and runs it.
    ui.click(("nu-example", 0usize));
    assert!(!ui.read(|f| f.nu_busy()));
    assert_eq!(
        ui.exists("nu-error"),
        !have_nu,
        "an error shows only without nu"
    );

    ui.click("nu-input");
    ui.press(&cmd("a"));
    ui.type_text("length");
    ui.press("enter");
    assert!(!ui.read(|f| f.nu_busy()));
    assert_eq!(ui.exists("nu-error"), !have_nu);
    if have_nu {
        assert!(ui.exists("nu-text"), "`length` prints a number");
    }

    ui.click("nu-run");
    assert!(!ui.read(|f| f.nu_busy()));
    assert!(
        !ui.exists(("nu-example", 0usize)),
        "a result replaces the examples"
    );

    // Clear empties the field and brings the examples back.
    ui.click("nu-clear");
    assert!(ui.exists(("nu-example", 0usize)));
    assert_eq!(ui.update(|f, _, cx| f.nu_pipeline(cx)), "");
    ui.click(("nu-example", 1usize));
    assert!(!ui.exists(("nu-example", 0usize)));
    assert!(!ui.update(|f, _, cx| f.nu_pipeline(cx)).is_empty());

    ui.click("nu-close");
    assert!(!ui.read(|f| f.nu_panel_open()));
    assert!(ui.files_focused());
}

// ----- picker ------------------------------------------------------------------------

#[gpui_kit::test]
fn picker_bar_enables_choose_with_a_selection(cx: &mut TestAppContext) {
    // Choose and Cancel end the process, so only their state is checked.
    let fx = Fixture::new("picker");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            picker: Some(PickerOptions { multiple: true }),
            ..Default::default()
        },
    );
    assert!(ui.exists("picker-cancel"));
    // With nothing selected Choose does nothing (with a selection it would
    // print the paths and exit).
    ui.click("picker-choose");
    ui.select("Roadmap.md");
    assert_eq!(ui.snapshot("picker-choose").label(), Some("Choose"));
    ui.select_also("notes.txt");
    assert_eq!(ui.snapshot("picker-choose").label(), Some("Choose 2 Items"));
}

#[gpui_kit::test]
fn cmd_double_click_opens_a_folder_in_a_file_chooser(cx: &mut TestAppContext) {
    let fx = Fixture::new("picker-cmd");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            picker: Some(PickerOptions { multiple: false }),
            ..Default::default()
        },
    );
    let cmd_held = if cfg!(target_os = "macos") {
        gpui_kit::Modifiers::command()
    } else {
        gpui_kit::Modifiers::control()
    };
    ui.click("view-list");

    ui.double_click_with(item_id("row", &fx.path("Projects")), cmd_held);
    assert_eq!(ui.folder(), Some(fx.path("Projects")), "there are no tabs");

    ui.go_to(&fx.dir);
    ui.click("view-columns");
    ui.settle();
    ui.double_click_with(item_id("crow", &fx.path("Projects")), cmd_held);
    assert_eq!(
        ui.folder(),
        Some(fx.path("Projects")),
        "from the columns too"
    );
}

#[gpui_kit::test]
fn nushell_filter_and_smart_folders(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("filter");
    std::fs::write(fx.path("big.log"), "x".repeat(20_000)).unwrap();
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    ui.press(&cmd("alt-f"));
    assert!(ui.exists("filter-input"), "Cmd-Alt-F opens the filter bar");
    ui.type_text("size > 10kb");
    ui.press("enter");
    assert_eq!(ui.names(), ["big.log"]);

    // A mistake shows Nushell's message and leaves the items alone.
    ui.press(&cmd("a"));
    ui.type_text("sise > 10kb");
    ui.press("enter");
    assert!(ui.exists("filter-error"));
    assert_eq!(ui.names().len(), 5);
    ui.press(&cmd("a"));
    ui.type_text("| sort-by size --reverse | first 2");
    ui.press("enter");
    assert!(!ui.exists("filter-error"));
    assert_eq!(ui.names().len(), 2, "a pipeline works too");
    ui.press(&cmd("a"));
    ui.type_text("size > 10kb");
    ui.press("enter");

    // Searches are filtered too, and can be saved.
    ui.click("search");
    ui.type_text("log");
    assert_eq!(ui.names(), ["big.log"]);
    ui.click("save-smart-folder");
    assert!(ui.dialog_open());
    assert_eq!(
        ui.snapshot("smart-summary").label(),
        Some("Shows names matching “log” on This Computer, where size > 10kb. Kept up to date.")
    );
    ui.type_text("Big logs");
    ui.click("smart-save");
    assert!(!ui.dialog_open());
    let saved = ui.read(|f| f.smart_folders().to_vec());
    assert_eq!(saved.len(), 1);
    assert_eq!(
        (
            saved[0].name.as_str(),
            saved[0].query.as_str(),
            saved[0].filter.as_deref()
        ),
        ("Big logs", "log", Some("size > 10kb"))
    );

    ui.click("close-filter");
    assert!(!ui.exists("filter-input"));
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "");
    ui.click("search");
    ui.press("escape");
    assert!(!ui.read(|f| f.is_searching()));
    ui.select("notes.txt");

    // The sidebar runs it again: search, filter, and all.
    ui.click(("smart", 0usize));
    assert_eq!(ui.read(|f| f.query().to_string()), "log");
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "size > 10kb");
    assert_eq!(ui.names(), ["big.log"]);

    // Its filter ends with its search, and the folder is back as it was:
    // listed whole, with the same item selected.
    ui.click("search");
    ui.press("escape");
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "");
    assert!(!ui.exists("filter-input"));
    assert_eq!(ui.names().len(), 5);
    assert_eq!(ui.selected(), ["notes.txt"]);

    // Back ends its search the same way.
    ui.click(("smart", 0usize));
    assert_eq!(ui.names(), ["big.log"]);
    ui.press(&cmd("["));
    assert!(!ui.read(|f| f.is_searching()));
    assert_eq!(ui.names().len(), 5);
    assert_eq!(ui.selected(), ["notes.txt"]);
    ui.right_click(("smart", 0usize));
    ui.click_menu_item("Remove from Sidebar");
    assert!(ui.read(|f| f.smart_folders().is_empty()));
}

/// A Smart Folder for folders named like `atlas` in Projects, with a filter.
fn atlas_folders(fx: &Fixture) -> SmartFolder {
    SmartFolder {
        name: "Atlas folders".into(),
        query: "atlas".into(),
        kind: KindFilter::Folder,
        scope: Some(fx.path("Projects")),
        contents: false,
        filter: Some("size > 1kb".into()),
    }
}

/// The kind, scope, Name or Contents, and filter the window searches with.
fn search_setup(ui: &Ui) -> (KindFilter, SearchScope, bool, String) {
    ui.read(|f| {
        (
            f.config().kind_filter,
            f.search_scope(),
            f.searches_contents(),
            f.filter_text().to_string(),
        )
    })
}

#[gpui_kit::test]
fn a_search_is_saved_from_the_scope_bar(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-save");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.click("search");
    ui.type_text("atlas");
    assert!(!ui.exists("filter-input"), "no filter bar is open");

    ui.click("save-smart-folder");
    assert!(ui.dialog_open());
    ui.type_text("Atlas");
    ui.click("smart-save");
    assert!(!ui.dialog_open());
    assert_eq!(
        ui.read(|f| f.smart_folders().to_vec()),
        [SmartFolder {
            name: "Atlas".into(),
            query: "atlas".into(),
            kind: KindFilter::Any,
            scope: None,
            contents: false,
            filter: None,
        }]
    );
}

#[gpui_kit::test]
fn going_elsewhere_from_a_smart_folder_leaves_its_search_setup_behind(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-leave");
    let mut ui = Ui::open(cx, &fx.dir);
    let saved = atlas_folders(&fx);
    ui.update(|f, _, cx| f.add_smart_folder(saved, cx));
    let own = search_setup(&ui);
    assert_eq!(
        own,
        (KindFilter::Any, SearchScope::Computer, false, String::new())
    );

    ui.click(("smart", 0usize));
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert_eq!(
        search_setup(&ui),
        (
            KindFilter::Folder,
            SearchScope::Folder,
            false,
            "size > 1kb".to_string()
        )
    );
    assert!(ui.exists("filter-input"));

    // Another folder is listed whole: no kind, no filter, and the next
    // search is this window's own again.
    ui.go_to(&fx.dir);
    assert_eq!(search_setup(&ui), own);
    assert!(!ui.exists("filter-input"), "the filter bar went with it");
    assert_eq!(ui.names().len(), 4);
}

#[gpui_kit::test]
fn emptying_the_search_field_leaves_a_smart_folder_too(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-leave-field");
    let mut ui = Ui::open(cx, &fx.dir);
    // One that searches inside files, everywhere.
    let saved = SmartFolder {
        name: "Plain notes".into(),
        query: "plain".into(),
        contents: true,
        filter: Some("size > 1kb".into()),
        ..Default::default()
    };
    ui.update(|f, _, cx| f.add_smart_folder(saved, cx));
    let own = search_setup(&ui);
    ui.select("notes.txt");

    ui.click(("smart", 0usize));
    assert!(ui.read(|f| f.searches_contents()));
    assert_eq!(ui.folder().as_deref(), Some(fx.dir.as_path()));

    ui.click("search");
    ui.press("escape");
    assert!(!ui.read(|f| f.is_searching()));
    assert_eq!(search_setup(&ui), own);
    assert!(!ui.exists("filter-input"));
    assert_eq!(ui.selected(), ["notes.txt"], "the folder is as it was left");
}

#[gpui_kit::test]
fn what_is_changed_by_hand_in_a_smart_folder_stays_after_it(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-leave-changed");
    let mut ui = Ui::open(cx, &fx.dir);
    let saved = atlas_folders(&fx);
    ui.update(|f, _, cx| f.add_smart_folder(saved, cx));
    ui.click(("smart", 0usize));

    // A kind picked from the menu and a filter typed in the bar are choices
    // of their own, the filter even before the pause it's applied after is
    // over. The scope is still the Smart Folder's, and goes with it.
    ui.click("kind");
    ui.click_menu_item("Document");
    ui.click("filter-input");
    ui.press(&cmd("a"));
    ui.type_without_settling("size > 2kb");

    ui.go_to(&fx.dir);
    assert_eq!(
        search_setup(&ui),
        (
            KindFilter::Document,
            SearchScope::Computer,
            false,
            "size > 2kb".to_string()
        )
    );
    assert!(ui.exists("filter-input"));
}

#[gpui_kit::test]
fn back_and_forward_return_to_a_smart_folders_whole_search(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-back");
    let mut ui = Ui::open(cx, &fx.dir);
    let saved = atlas_folders(&fx);
    ui.update(|f, _, cx| f.add_smart_folder(saved, cx));
    let own = search_setup(&ui);
    ui.click(("smart", 0usize));
    let smart = search_setup(&ui);
    assert_ne!(smart, own);

    // Going elsewhere ends its search. Back returns to all of it, not only
    // to its words: the kind and the filter went with it, so they come back
    // with it.
    ui.go_to(&fx.dir);
    assert_eq!(search_setup(&ui), own);
    ui.press(&cmd("["));
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert_eq!(search_setup(&ui), smart);
    assert!(ui.exists("filter-input"));
    assert!(ui.read(|f| f.smart_is_active_for_test(0)));

    // Back from it ends it in its folder, and it's left behind like the
    // first time.
    ui.press(&cmd("["));
    assert!(!ui.read(|f| f.is_searching()));
    assert_eq!(ui.folder(), Some(fx.path("Projects")));
    assert_eq!(search_setup(&ui), own);
    assert!(!ui.exists("filter-input"));

    // Forward runs it again, and Escape leaves it again.
    ui.press(&cmd("]"));
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    assert_eq!(search_setup(&ui), smart);
    ui.click("search");
    ui.press("escape");
    assert!(!ui.read(|f| f.is_searching()));
    assert_eq!(search_setup(&ui), own);
}

#[gpui_kit::test]
fn a_filter_still_being_typed_doesnt_replace_a_smart_folders(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-typed-filter");
    let mut ui = Ui::open(cx, &fx.dir);
    let saved = atlas_folders(&fx);
    ui.update(|f, _, cx| f.add_smart_folder(saved, cx));
    ui.press(&cmd("alt-f"));

    // The filter bar applies what's typed after a pause, and the click comes
    // before the pause is over.
    ui.type_without_settling("size > 5kb");
    ui.click(("smart", 0usize));
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "size > 1kb");

    // It was the window's own filter all the same, and is back as one.
    ui.go_to(&fx.dir);
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "size > 5kb");
    assert!(ui.exists("filter-input"));
}

#[gpui_kit::test]
fn the_keyboard_stays_in_the_window_when_a_smart_folder_takes_the_filter_bar_away(
    cx: &mut TestAppContext,
) {
    let fx = Fixture::new("smart-filter-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    // One with a filter and one without, both in Projects.
    let filtered = atlas_folders(&fx);
    let plain = SmartFolder {
        name: "Atlas".into(),
        filter: None,
        ..filtered.clone()
    };
    ui.update(|f, _, cx| {
        f.add_smart_folder(filtered, cx);
        f.add_smart_folder(plain, cx);
    });

    // Its bar goes when its search ends, here with the keyboard in it.
    ui.click(("smart", 0usize));
    ui.click("filter-input");
    ui.press(&cmd("["));
    assert!(!ui.read(|f| f.is_searching()));
    assert!(!ui.exists("filter-input"));
    assert!(ui.files_focused());

    // The window's own bar goes when a Smart Folder with no filter comes
    // back, and keys still reach the window after that.
    ui.click(("smart", 1usize));
    ui.press(&cmd("["));
    ui.press(&cmd("alt-f"));
    assert!(ui.exists("filter-input"));
    ui.press(&cmd("]"));
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    assert!(!ui.exists("filter-input"));
    assert!(ui.files_focused());
    ui.press(&cmd("["));
    assert!(!ui.read(|f| f.is_searching()));
    assert!(ui.exists("filter-input"), "the window's own bar is back");
}

#[gpui_kit::test]
fn save_search_is_on_a_key_and_in_the_actions_menu(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-save-key");
    let mut ui = Ui::open(cx, &fx.dir);
    // Outside a search there is nothing to save: the key says so, and the
    // menu's item is off.
    ui.press(&cmd("s"));
    assert!(!ui.dialog_open());
    assert_eq!(ui.notification_count(), 1);
    ui.click("actions");
    ui.click_menu_item("Save Search…");
    assert!(!ui.dialog_open());
    assert_eq!(ui.notification_count(), 1, "the item did nothing");
    ui.press("escape");

    ui.click("search");
    ui.type_text("atlas");
    ui.press(&cmd("s"));
    assert!(ui.dialog_open(), "from the search field");
    ui.click("smart-cancel");
    assert!(!ui.dialog_open());

    ui.click("actions");
    ui.click_menu_item("Save Search…");
    assert!(ui.dialog_open());
}

#[gpui_kit::test]
fn the_save_dialog_says_what_is_saved_and_what_it_replaces(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-dialog");
    let mut ui = Ui::open(cx, &fx.dir);
    // One to replace, ahead of another in the sidebar.
    let (first, second) = (
        SmartFolder {
            name: "Docs".into(),
            query: "notes".into(),
            ..Default::default()
        },
        atlas_folders(&fx),
    );
    ui.update(|f, _, cx| {
        f.add_smart_folder(first, cx);
        f.add_smart_folder(second, cx);
    });
    ui.click("search");
    ui.type_text("road");
    ui.click("scope-folder");
    ui.click("kind");
    ui.click_menu_item("Document");

    ui.click("save-smart-folder");
    let folder = fx.dir.file_name().unwrap().to_string_lossy();
    assert_eq!(
        ui.snapshot("smart-summary").label(),
        Some(
            format!("Shows names matching “road” in “{folder}”, kind Document. Kept up to date.")
                .as_str()
        )
    );
    // The name starts as the words, which no Smart Folder is called.
    assert!(!ui.exists("smart-exists"));
    assert_eq!(ui.snapshot("smart-save").label(), Some("Save"));

    // Nothing is saved without a name: the button is off, and Return does
    // nothing.
    ui.press(&cmd("a"));
    ui.press("backspace");
    assert_eq!(ui.snapshot("smart-save").disabled(), Some(true));
    ui.press("enter");
    ui.click("smart-save");
    assert!(ui.dialog_open());
    assert_eq!(ui.read(|f| f.smart_folders().len()), 2);

    ui.type_text("Docs");
    assert!(ui.exists("smart-exists"), "one is called that already");
    assert_eq!(ui.snapshot("smart-save").label(), Some("Replace"));
    ui.click("smart-save");
    assert!(!ui.dialog_open());
    let saved = ui.read(|f| f.smart_folders().to_vec());
    assert_eq!(saved.len(), 2);
    assert_eq!(
        saved[0],
        SmartFolder {
            name: "Docs".into(),
            query: "road".into(),
            kind: KindFilter::Document,
            scope: Some(fx.dir.clone()),
            contents: false,
            filter: None,
        },
        "replaced where it was in the sidebar"
    );
}

#[gpui_kit::test]
fn only_the_smart_folder_whose_search_is_showing_is_marked(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-active");
    let mut ui = Ui::open(cx, &fx.dir);
    // The same words, for any kind and for folders.
    let any_kind = SmartFolder {
        name: "Atlas".into(),
        query: "atlas".into(),
        ..Default::default()
    };
    let folders = SmartFolder {
        name: "Atlas folders".into(),
        kind: KindFilter::Folder,
        ..any_kind.clone()
    };
    ui.update(|f, _, cx| {
        f.add_smart_folder(any_kind, cx);
        f.add_smart_folder(folders, cx);
    });
    let marked =
        |ui: &Ui| ui.read(|f| (f.smart_is_active_for_test(0), f.smart_is_active_for_test(1)));
    assert_eq!(marked(&ui), (false, false));

    ui.click(("smart", 0usize));
    assert_eq!(marked(&ui), (true, false));
    ui.click(("smart", 1usize));
    assert_eq!(marked(&ui), (false, true));

    // Searched inside files instead, it's neither one's search.
    ui.click("by-contents");
    assert_eq!(marked(&ui), (false, false));
}

#[gpui_kit::test]
fn a_smart_folder_whose_folder_is_gone_says_it_searches_everywhere(cx: &mut TestAppContext) {
    let fx = Fixture::new("smart-gone");
    let mut ui = Ui::open(cx, &fx.dir);
    // The same search twice: for a folder that isn't there, and for Projects.
    let (gone, there) = (
        SmartFolder {
            scope: Some(fx.path("gone")),
            ..atlas_folders(&fx)
        },
        SmartFolder {
            name: "Atlas in Projects".into(),
            ..atlas_folders(&fx)
        },
    );
    ui.update(|f, _, cx| {
        f.add_smart_folder(gone, cx);
        f.add_smart_folder(there, cx);
    });
    let marked =
        |ui: &Ui| ui.read(|f| (f.smart_is_active_for_test(0), f.smart_is_active_for_test(1)));

    ui.click(("smart", 0usize));
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    assert_eq!(ui.read(|f| f.search_scope()), SearchScope::Computer);
    assert_eq!(ui.notification_count(), 1);
    // It's the one that's open all the same, so the sidebar marks it, and
    // not the other, whose folder was found. A scope chosen by hand makes
    // the search neither one's.
    assert_eq!(marked(&ui), (true, false));
    ui.click("scope-folder");
    assert_eq!(marked(&ui), (false, false));
}

#[gpui_kit::test]
fn the_keys_on_the_filter_bars_close_button_remove_the_filter(cx: &mut TestAppContext) {
    let fx = Fixture::new("filter-keys");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("alt-f"));
    ui.type_text("size > 1kb");
    ui.press("enter");
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "size > 1kb");

    // The keyboard is in the filter's field, where the bar leaves it.
    ui.press(&cmd("alt-f"));
    assert!(!ui.exists("filter-input"));
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "");
    assert!(ui.files_focused());
}

#[gpui_kit::test]
fn the_files_get_the_keyboard_when_a_smart_folder_closes_the_filter_bar(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt as _;
    let fx = Fixture::new("filter-smart-folder");
    let mut ui = Ui::open(cx, &fx.dir);
    let atlas = SmartFolder {
        name: "Atlas".into(),
        query: "atlas".into(),
        kind: KindFilter::Any,
        scope: None,
        contents: false,
        filter: None,
    };
    ui.update(|f, _, cx| f.add_smart_folder(atlas, cx));
    ui.press(&cmd("alt-f"));
    assert!(ui.exists("filter-input"));

    // The click leaves the keyboard in the filter's field, and the Smart
    // Folder, which has no filter, closes the bar the field is in.
    ui.click(("smart", 0usize));
    assert_eq!(ui.read(|f| f.query().to_string()), "atlas");
    assert!(!ui.exists("filter-input"));
    assert!(ui.files_focused());

    // A shortcut still does its work: Cmd+F goes to the search field.
    ui.press(&cmd("f"));
    let typing_in = ui.window(|window, cx| window.focused_input(cx).map(|field| field.value(cx)));
    assert_eq!(typing_in.as_deref(), Some("atlas"));
}

#[gpui_kit::test]
fn escape_in_an_empty_filter_field_gives_the_files_the_keyboard(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt as _;
    let fx = Fixture::new("filter-escape");
    let mut ui = Ui::open(cx, &fx.dir);
    ui.press(&cmd("alt-f"));
    ui.type_text("size > 1kb");
    ui.press("enter");
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "size > 1kb");

    // With a condition in it, Escape empties the field, which turns the
    // filter off, and stays there for another condition.
    ui.press("escape");
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "");
    assert!(
        ui.window(|window, cx| window.has_focused_input(cx)),
        "the first Escape stays in the field"
    );

    ui.press("escape");
    assert!(ui.files_focused(), "the second Escape leaves the field");
    assert!(ui.exists("filter-input"), "the bar stays open");
}

#[gpui_kit::test]
fn a_filter_waits_for_the_rows_a_search_has_yet_to_find(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("filter-search-start");
    std::fs::write(fx.path("big.log"), "x".repeat(20_000)).unwrap();
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
    assert_eq!(ui.names(), ["big.log"]);

    // The folder's items are gone and the results aren't in. A run over no
    // rows would end with nothing kept, which would hide the results when
    // they come, until their own run had finished.
    ui.click("search");
    ui.type_without_settling("log");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(!ui.read(|f| f.is_filtering()));

    ui.settle();
    assert_eq!(ui.names(), ["big.log"]);
}

#[gpui_kit::test]
fn a_filter_that_keeps_nothing_says_so_and_offers_to_remove_it(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("filter-keeps-nothing");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    let filter_out_everything = |ui: &mut Ui| {
        ui.press(&cmd("alt-f"));
        ui.type_text("size > 1gb");
        ui.press("enter");
        assert!(ui.names().is_empty(), "{:?}", ui.names());
    };

    // The folder has four items, so it isn't empty.
    filter_out_everything(&mut ui);
    assert_eq!(
        ui.snapshot("message-title").label(),
        Some("No Items Pass the Filter")
    );
    assert_eq!(message_detail(&mut ui), "4 items are hidden by the filter.");
    ui.click("empty-remove-filter");
    assert_eq!(ui.names().len(), 4);
    assert_eq!(ui.read(|f| f.filter_text().to_string()), "");
    assert!(!ui.exists("filter-input"));
    assert!(ui.files_focused());

    // A search that found something has results, which the filter hides.
    filter_out_everything(&mut ui);
    ui.click("search");
    ui.type_text("atlas");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert_eq!(
        ui.snapshot("message-title").label(),
        Some("No Items Pass the Filter")
    );
    assert_eq!(message_detail(&mut ui), "2 items are hidden by the filter.");
    assert!(ui.exists("empty-remove-filter"));

    // One that found nothing has none for the filter to hide.
    ui.type_text("qqzz");
    assert_eq!(ui.snapshot("message-title").label(), Some("No Results"));
    assert!(!ui.exists("empty-remove-filter"));
}

#[gpui_kit::test]
fn an_item_in_a_collapsed_group_isnt_hidden_by_the_filter(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("filter-collapsed-group");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));
    ui.update(|f, _, cx| f.set_filter("name =~ 'Road'".into(), cx));
    assert_eq!(ui.names(), ["Roadmap.md"]);

    // The one item that passes the filter is in the one group left. With
    // that collapsed nothing is listed, and the filter isn't why.
    ui.click(gpui_kit::SharedString::from("group-Markdown"));
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert_eq!(ui.read(|f| f.group_headers()), ["Markdown"]);
    assert!(!ui.exists("empty-remove-filter"));
}

#[gpui_kit::test]
fn a_filter_that_is_still_running_doesnt_say_nothing_passes(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("filter-running");
    std::fs::write(fx.path("big.log"), "").unwrap();
    std::fs::write(fx.path("Projects/large.log"), "").unwrap();
    std::fs::create_dir(fx.path("Empty")).unwrap();
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    ui.press(&cmd("alt-f"));
    ui.type_text("name =~ 'log'");
    ui.press("enter");
    assert_eq!(ui.names(), ["big.log"]);

    // Go to a folder and stop where it has been read and the filter's run
    // over its items has only started.
    let go_until_filtering = |ui: &mut Ui, folder: &str| {
        let (view, dir) = (ui.view.clone(), fx.path(folder));
        ui.window(|window, cx| {
            view.update(cx, |f, cx| f.navigate(Location::Folder(dir), window, cx))
        });
        for _ in 0..1000 {
            if ui.read(|f| f.is_filtering()) {
                return;
            }
            ui.cx.executor().tick();
        }
        panic!("the filter never ran over the items of {folder}");
    };

    // What the filter kept in the folder before is none of this folder's
    // items, which says nothing about them.
    go_until_filtering(&mut ui, "Projects");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(ui.exists("skeleton"), "placeholders until the run ends");
    assert!(!ui.exists("message-title"));
    ui.settle();
    assert_eq!(ui.names(), ["large.log"]);

    // A run over no items has nothing to keep, so an empty folder says what
    // it is at once.
    go_until_filtering(&mut ui, "Empty");
    assert!(!ui.exists("skeleton"));
    assert_eq!(ui.snapshot("message-title").label(), Some("No Items"));
    ui.settle();
    assert_eq!(ui.snapshot("message-title").label(), Some("No Items"));
}

#[gpui_kit::test]
fn a_filtered_search_is_marked_when_more_match_than_the_filter_was_given(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("filter-search-limit");
    // A filter is given 5,000 of a search's results, whatever the result
    // limit is, and ten more than that match here.
    for n in 0..5010 {
        std::fs::write(fx.path(&format!("zlog{n:04}.txt")), "").unwrap();
    }
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    let status = |ui: &mut Ui| ui.snapshot("status-text").label().unwrap().to_string();
    ui.update(|f, _, cx| f.set_query("zlog".into(), true, cx));
    assert_eq!(status(&mut ui), "500+ items");

    ui.update(|f, _, cx| f.set_filter("size < 10kb".into(), cx));
    assert_eq!(status(&mut ui), "5,000 of 5,000+ items pass the filter");

    // Results shown is 500 here, and raising it wouldn't give the filter more.
    ui.hover("status-text");
    assert_eq!(
        ui.snapshot("status-more").label(),
        Some("More items match than the filter was given. Narrow the search.")
    );
}

#[gpui_kit::test]
fn batch_rename_with_a_closure(cx: &mut TestAppContext) {
    use fffinder::rename::Outcome;
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("batch-rename");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    ui.click("view-list");
    ui.select("Budget.csv");
    ui.select_also("notes.txt");
    ui.right_click(item_id("row", &fx.path("notes.txt")));
    assert!(ui.menu_labels().contains(&"Rename 2 Items…".to_string()));
    ui.press("escape");

    ui.press("f2");
    assert!(ui.read(|f| f.batch_rename_open()));
    let outcomes = |ui: &Ui| -> Vec<(String, Outcome)> {
        match ui.read(|f| f.batch_preview_for_test()) {
            Some(Ok(rows)) => rows.into_iter().map(|(_, new, o)| (new, o)).collect(),
            other => panic!("no preview: {other:?}"),
        }
    };
    // It starts as "keep every name": nothing to rename, so Rename does nothing.
    assert!(outcomes(&ui).iter().all(|(_, o)| *o == Outcome::Unchanged));
    ui.click("batch-apply");
    assert!(ui.dialog_open());

    ui.click(("batch-example", 0usize));
    assert_eq!(
        outcomes(&ui),
        [
            ("budget.csv".to_string(), Outcome::Rename),
            ("notes.txt".to_string(), Outcome::Unchanged)
        ]
    );
    assert!(ui.exists("batch-summary"));

    ui.click("batch-closure");
    ui.press(&cmd("a"));
    ui.type_text("{|f| 'same.txt' }");
    ui.press("enter");
    assert!(ui.dialog_open(), "Return previews instead of renaming");
    assert!(
        outcomes(&ui)
            .iter()
            .all(|(_, o)| matches!(o, Outcome::Problem(_)))
    );
    ui.click("batch-apply");
    assert!(fx.path("Budget.csv").exists(), "problems block the rename");

    ui.press(&cmd("a"));
    ui.type_text("{|f| $f.nope }");
    ui.press("enter");
    assert!(ui.exists("batch-error"));

    ui.press(&cmd("a"));
    ui.type_text("{|f| $\"($f.index)-($f.name)\" }");
    ui.press("enter");
    ui.click("batch-apply");
    assert!(!ui.dialog_open());
    assert!(fx.path("1-Budget.csv").exists() && fx.path("2-notes.txt").exists());
    assert!(!fx.path("Budget.csv").exists());
    assert!(ui.names().contains(&"1-Budget.csv".to_string()));

    ui.select("1-Budget.csv");
    ui.select_also("2-notes.txt");
    ui.press("f2");
    ui.click("batch-cancel");
    assert!(
        !ui.read(|f| f.batch_rename_open()),
        "Cancel forgets the batch"
    );
    assert!(fx.path("1-Budget.csv").exists());
}

#[gpui_kit::test]
fn nushell_actions_from_the_context_menu(cx: &mut TestAppContext) {
    let Some(nu) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("user-actions");
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            nu: Some(nu),
            ..Default::default()
        },
    );
    let labels: Vec<String> =
        ui.read(|f| f.user_actions().iter().map(|a| a.label.clone()).collect());
    assert!(
        labels.contains(&"Count Lines".to_string()),
        "starters load: {labels:?}"
    );
    assert!(fffinder::user_actions::actions_path().exists());

    ui.click("view-list");
    ui.right_click(item_id("row", &fx.path("notes.txt")));
    ui.click_menu_item("Nushell Actions");
    // The items are drawn at the Nushell text size, so they're found by id.
    let count_lines = labels.iter().position(|l| l == "Count Lines").unwrap();
    let item = ui.snapshot(("nu-action", count_lines));
    assert_eq!(item.label(), Some("Count Lines"));
    assert!(ui.exists("nu-actions-edit"));
    let height = item.bounds().size.height;
    ui.click(("nu-action", count_lines));
    assert!(ui.read(|f| f.nu_panel_open()));
    assert!(!ui.read(|f| f.nu_busy()));
    assert!(!ui.exists("nu-error"), "the action ran cleanly");

    // The toolbar's Nushell menu has them too, bigger after Settings changes.
    ui.update(|f, _, cx| f.set_nu_font_size(Some(20.), cx));
    ui.click("nushell");
    ui.click_menu_item("Nushell Actions");
    let total = labels.iter().position(|l| l == "Total Size").unwrap();
    assert_eq!(
        ui.snapshot(("nu-action", total)).label(),
        Some("Total Size")
    );
    let bigger = ui.snapshot(("nu-action", total)).bounds().size.height;
    assert!(bigger > height, "{height:?} -> {bigger:?}");
}

#[gpui_kit::test]
fn nushell_colors_and_editor(cx: &mut TestAppContext) {
    use fffinder::nu_env::NuEnv;
    let fx = Fixture::new("nu-env");
    let term = Terminal {
        id: "term".into(),
        name: "Term".into(),
        location: Some("/usr/bin/term".into()),
        launch: Launch::Exec {
            program: "/usr/bin/term".into(),
            args: vec!["--cwd".into(), "{dir}".into(), "-e".into(), "{nu}".into()],
        },
    };
    let mut ui = Ui::open_with(
        cx,
        &fx.dir,
        Open {
            terminals: vec![term],
            ..Default::default()
        },
    );
    let notes = fx.path("notes.txt");

    // Off: files open in their default app, names use the theme.
    ui.select("notes.txt");
    ui.dispatch(OpenInEditor);
    assert_eq!(ui.system_calls(), [SystemCall::Open(notes.clone())]);
    assert!(ui.read(|f| f.rows_for_test().iter().all(|r| r.name_style.is_none())));

    // On, with nvim: names colored like `ls`, the editor in the terminal.
    let env = NuEnv {
        ls_colors: Some("di=38;2;10;20;30:*.md=38;2;200;0;0".into()),
        buffer_editor: Some(json!("nvim")),
        ..Default::default()
    };
    ui.update(|f, _, cx| f.set_nu_env_for_test(env, cx));
    let color = |ui: &Ui, name: &str| {
        ui.read(|f| {
            f.rows_for_test()
                .iter()
                .find(|r| r.entry.name == name)
                .and_then(|r| r.name_style)
                .and_then(|s| s.color)
                .map(|c| {
                    let c = gpui_kit::Rgba::from(c);
                    (
                        (c.r * 255.).round() as u8,
                        (c.g * 255.).round() as u8,
                        (c.b * 255.).round() as u8,
                    )
                })
        })
    };
    assert_eq!(color(&ui, "Projects"), Some((10, 20, 30)));
    assert_eq!(color(&ui, "Roadmap.md"), Some((200, 0, 0)));
    assert_eq!(color(&ui, "Budget.csv"), None);

    ui.select("notes.txt");
    ui.dispatch(OpenInEditor);
    let dir = fx.dir.display().to_string();
    assert_eq!(
        ui.system_calls().last(),
        Some(&SystemCall::Terminal {
            program: "/usr/bin/term".into(),
            args: vec![
                "--cwd".into(),
                dir.clone(),
                "-e".into(),
                "nvim".into(),
                notes.display().to_string()
            ],
        })
    );

    // A windowed editor starts directly, with its own arguments.
    let env = NuEnv {
        buffer_editor: Some(json!(["code", "--wait"])),
        ..Default::default()
    };
    ui.update(|f, _, cx| f.set_nu_env_for_test(env, cx));
    ui.right_click(item_id("tile", &notes));
    ui.click_menu_item("Open in Editor");
    assert_eq!(
        ui.system_calls().last(),
        Some(&SystemCall::Terminal {
            program: "code".into(),
            args: vec!["--wait".into(), notes.display().to_string()],
        })
    );

    // The Settings switch turns it off again.
    ui.press(&cmd(","));
    assert!(ui.read(|f| f.config().use_nu_environment));
    ui.click("use-nu-env");
    assert!(!ui.read(|f| f.config().use_nu_environment));
    assert_eq!(color(&ui, "Projects"), None);
}

#[gpui_kit::test]
fn collapsing_every_group_keeps_its_headers(cx: &mut TestAppContext) {
    let fx = Fixture::new("groups-all");
    let mut ui = Ui::open(cx, &fx.dir);
    let header = |label: &str| gpui_kit::SharedString::from(format!("group-{label}"));

    ui.click("view-list");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));
    let labels = ui.read(|f| f.group_headers());
    assert_eq!(labels.len(), 4);
    for label in &labels {
        ui.click(header(label));
    }
    // Nothing is listed, and the folder is no emptier than it was: the
    // headers stay, each to open its group again.
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(!ui.exists("message-title"), "the folder has items");
    for label in &labels {
        assert!(ui.exists(header(label)), "{label}");
    }

    // A box dragged over the headers has no item to select, and the keys
    // have none to go to.
    let area = ui.snapshot("content").bounds();
    let under_the_headers = area.origin + at(40., f32::from(area.size.height) - 30.);
    let first_header = ui.snapshot(header(&labels[0])).bounds().center();
    ui.drag_points(under_the_headers, first_header, Default::default());
    assert!(ui.files_focused());
    assert!(ui.selected().is_empty());
    for key in ["down", "up", "end", "shift-pagedown", &cmd("a")] {
        ui.press(key);
        assert!(ui.selected().is_empty(), "after {key}");
    }
    ui.type_text("p");
    assert!(ui.selected().is_empty(), "no name to jump to");

    // The icon view keeps the headers too. Its keys find no item either,
    // and a header opens its group as before.
    ui.click("view-icons");
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(!ui.exists("message-title"));
    ui.drag_points(under_the_headers, first_header, Default::default());
    assert!(ui.files_focused());
    for key in ["down", "right", "end"] {
        ui.press(key);
        assert!(ui.selected().is_empty(), "after {key}");
    }
    ui.click(header("Folder"));
    assert_eq!(ui.names(), ["Projects"]);
}

#[gpui_kit::test]
fn a_place_whose_groups_are_all_collapsed_isnt_empty(cx: &mut TestAppContext) {
    let fx = Fixture::new("groups-elsewhere");
    let mut ui = Ui::open(cx, &fx.dir);
    let header = |label: &str| gpui_kit::SharedString::from(format!("group-{label}"));
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));

    // A tab remembers its collapsed groups from folder to folder, and
    // Projects holds nothing but a folder.
    ui.click(header("Folder"));
    ui.go_to(&fx.path("Projects"));
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(!ui.exists("message-title"), "Projects has a folder in it");
    ui.click(header("Folder"));
    assert_eq!(ui.names(), ["atlas"]);

    // Nor is a search without results when every group of them is collapsed.
    ui.click("search");
    ui.type_text("atlas");
    assert_eq!(ui.names(), ["atlas", "Project Atlas.txt"]);
    for label in ["Folder", "Plain Text"] {
        ui.click(header(label));
    }
    assert!(ui.names().is_empty(), "{:?}", ui.names());
    assert!(!ui.exists("message-title"), "the search found two items");
    ui.click(header("Plain Text"));
    assert_eq!(ui.names(), ["Project Atlas.txt"]);
}

#[gpui_kit::test]
fn a_gallery_shows_placeholders_while_it_is_read_after_collapsed_groups(cx: &mut TestAppContext) {
    let fx = Fixture::new("groups-then-gallery");
    let mut ui = Ui::open(cx, &fx.dir);
    // Projects opens as a gallery from here on, and the folder it is in as
    // a list.
    ui.go_to(&fx.path("Projects"));
    ui.click("view-gallery");
    ui.go_to(&fx.dir);
    ui.click("view-list");
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));
    for label in ui.read(|f| f.group_headers()) {
        ui.click(gpui_kit::SharedString::from(format!("group-{label}")));
    }
    assert!(ui.names().is_empty(), "{:?}", ui.names());

    // A gallery has no groups, so the headers the list is left with are
    // nothing for it to show until Projects has been read.
    let (view, projects) = (ui.view.clone(), fx.path("Projects"));
    ui.window(|window, cx| {
        view.update(cx, |f, cx| {
            f.navigate(Location::Folder(projects), window, cx)
        })
    });
    assert_eq!(ui.read(|f| f.effective_view()), ViewMode::Gallery);
    assert!(ui.exists("skeleton"), "placeholders, not an empty gallery");
    ui.settle();
    assert_eq!(ui.names(), ["atlas"]);
}

#[gpui_kit::test]
fn groups_collapse_and_hide_their_items(cx: &mut TestAppContext) {
    let fx = Fixture::new("groups");
    std::fs::write(fx.path("todo.txt"), "todo").unwrap();
    let mut ui = Ui::open(cx, &fx.dir);
    let headers = |ui: &Ui| ui.read(|f| f.group_headers());
    let header = |label: &str| gpui_kit::SharedString::from(format!("group-{label}"));

    ui.click("view-list");
    ui.click("sort");
    ui.click_menu_item("Group By");
    // "Kind" is also a sort; this label is only in the Group By menu.
    ui.click_menu_item("Date Last Opened");
    assert_eq!(ui.read(|f| f.config().group_by), GroupKey::LastOpened);
    assert!(!headers(&ui).is_empty());
    ui.update(|f, _, cx| f.set_group_by(Some(GroupKey::Kind), cx));
    assert_eq!(
        headers(&ui),
        ["Comma-Separated Values", "Folder", "Markdown", "Plain Text"]
    );
    assert_eq!(
        ui.names(),
        [
            "Budget.csv",
            "Projects",
            "Roadmap.md",
            "notes.txt",
            "todo.txt"
        ]
    );

    // A collapsed group's items leave the view, so the keys skip them.
    ui.click(header("Markdown"));
    assert_eq!(
        ui.names(),
        ["Budget.csv", "Projects", "notes.txt", "todo.txt"]
    );
    ui.select("Projects");
    ui.press("down");
    assert_eq!(
        ui.selected(),
        ["notes.txt"],
        "Down skips the collapsed group"
    );
    ui.click(header("Markdown"));
    assert_eq!(ui.names().len(), 5, "expanding brings them back");

    // The icon view has the same groups, and Down goes into the next one.
    ui.click("view-icons");
    assert!(ui.exists(header("Folder")));
    ui.select("Budget.csv");
    ui.press("down");
    assert_eq!(ui.selected(), ["Projects"]);
    ui.select("todo.txt");
    ui.press("down");
    assert_eq!(ui.selected(), ["todo.txt"], "nothing below the last group");

    // The gallery doesn't use groups.
    ui.click("view-gallery");
    assert!(headers(&ui).is_empty());
    assert_eq!(ui.names().len(), 5);

    // View › Use Groups turns them off and back on, by the same key.
    ui.click("view-list");
    ui.dispatch(ToggleGroups);
    assert!(headers(&ui).is_empty());
    ui.dispatch(ToggleGroups);
    assert_eq!(headers(&ui).len(), 4);
    assert!(ui.read(|f| f.config().use_groups));
}
