[confirmed] sev 4->4 known=False rename-ended-by-press-reselects-renamed-item: A rename ended by a press on another item selects the renamed item again a moment later; after a right-click, the open menu acts on the renamed item, 
    improvements: column-pane-selection-by-path
    scenario: As the finder wrote it, with three precisions.

1. The menu stays on screen only while the right-clicked row is still under the pressed point after the reload: context_menu.rs:358 `draws_menu.set(bounds.contains(&position))`. That is the usual case; the finder's notes.txt -> plan.txt keeps Roadmap.md at the same row. If the new name sorts across the pressed row, or `scroll_to(first)` scrolls the list, the kit stops drawing the menu and the focus-lost net gives the files the keyboard. Then no menu item can be chosen, but the renamed item is still the one selected.

2. Not every item goes to the renamed file. Action items do: Open, Quick Look, Get Info, Open in Editor, Show in File Manager, Cut, Copy, Copy Path, Copy as Nushell List, Rename…, Duplicate, Make Alias, Make Symbolic Link, Compress, Extract, Move to Trash. Items built with captured values still act on the item the menu names: O
    evidence: Traced by reading; nothing was run (rules). No guard, reset or early return found on the path.

1. The press ends the edit first. src/finder/inline_rename.rs:262-266 `.on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, window, cx| { this.leave_rename(window, cx) }))`; gpui-pre-0.3.10 src/elements/div.rs:269-271 runs it in the capture phase for any button. leave_rename (:145-156) -> finish_rename (:123-139) -> src/finder/dialogs.rs:884-887 `if !followed { self.load_listing(vec![target.clone()], true, cx); self.renamed_in_columns(path, target, cx); }`. An unchanged name returns at :852-854 before that.

2. Same press, bubble phase. src/finder/views.rs:488-493 -> src/finder/mod.rs:2677-26
    fix: Let the reload that follows a rename select the new name only when nothing else has been selected by the time it ends. In load_listing's completion, for a rename's `select`, apply it only if the selection is empty after rebuild_rows has remapped it (the old path is gone from it, so Return and a press on chrome still select the new name; a press on another item keeps that item). In load_column's completion, for the rename's `select`, use `wanted.filter(|_| kept.is_none())`. A flag on the two call
    test: tests/basics.rs, list view: ui.select("notes.txt"); ui.press("f2"); ui.type_text("plan"); ui.right_click(item_id("row", &fx.path("Roadmap.md"))); assert_eq!(ui.selected(), ["Roadmap.md"]) (today: ["plan.txt"]); ui.click_menu_item("Move to Trash"); assert!(fx.path("plan.txt").exists()); assert!(!fx.path("Roadmap.md").exists()). The same in view-columns with item_id("crow", ..), once in the first co

[confirmed] sev 3->3 known=False goto-folder-from-smart-folder-filter-field-dead-keyboard: Go to Folder confirmed with Return from the filter field of an open Smart Folder leaves the keyboard on a field that is no longer drawn: every key is 
    improvements: smart-folder-save-and-leaks, focus-lost-recovery
    scenario: As the finder wrote it, with these precisions.

1. The filter text must be as the Smart Folder set it. An edited filter counts as changed by hand (filter.rs:398), keeps its bar and its field, and nothing goes dead. This makes the flow narrow: a click in the field without a lasting edit, then Go to Folder by key, then Return.

2. Only Return ends this way. The Go button gives the files the keyboard itself (dialogs.rs:726-729).

3. Tab probably does not rescue, contrary to the finder's aside. The kit binds Tab in the "Root" context, which sits on a div below node 0 of the dispatch tree, and keys are dispatched on node 0 alone. Not verified by running.

4. Any press inside the browser does rescue: keyboard_stays (mod.rs:2631) gives the files the keyboard when the browser does not contain what has it, and a click on a row calls focus_files. Cmd+Q and Cmd+N still work.

5. The variant with an
    evidence: Traced by reading; nothing was run (rules). No guard found.

1. The dialog remembers the filter field. src/finder/dialogs.rs:30-35 `self.end_rename(window, cx); if !window.has_focused_input(cx) { self.focus_files(window, cx); }`: with the caret in the filter field the keyboard stays there (the kit registers a focused input as it renders, input/state.rs:371-393). gpui-component-0.8.1 src/root.rs:383 `let mut previous_focused_handle = window.focused(cx).map(|h| h.downgrade());`. The key reaches go_to_folder from that field: src/actions.rs:173 and :314 bind it in the Finder context, and no kit input binding takes those keys.

2. Return takes the bar away under the dialog. dialogs.rs:716-718 on_
    fix: A net at draw time, beside keyboard_lost: at the top of Finder's render, if `!self.filter_bar_open()` and the filter field's handle `is_focused(window)`, call focus_files and ask for one more frame with `window.defer(cx, |window, _| window.refresh())`. It covers the external navigations too. Narrower alternative, three lines: in focus_before_dialog, give the files the keyboard first when the focused field is the filter field and `smart_folder_setup.is_some()` (the bar can go with that search).
    test: tests/controls.rs, with atlas_folders(&fx) as in the test at :4137: ui.update(|f, _, cx| f.add_smart_folder(filtered, cx)); ui.click(("smart", 0usize)); ui.click("filter-input"); ui.press(&cmd("shift-g")); ui.type_text(&fx.path("Projects").display().to_string()); ui.press("enter"); assert!(!ui.dialog_open()); assert!(!ui.exists("filter-input")); assert!(ui.files_focused()) (fails today); ui.press(

[confirmed] sev 2->2 known=False relevance-rechosen-reruns-search: Choosing Relevance again in a search's Sort menu flips a direction it doesn't have and runs a cut search again
    improvements: sorted-search-results
    scenario: Results shown 3; a folder with a.log, b.log, c.log, d.log; search `*.log` (status bar `3+ items`, sort Relevance). Open Sort and View Options (or the empty-space menu's Sort By) and click the checked item Relevance.
What happens: `result_sort` becomes Relevance/descending, so the menu now shows Descending checked beside two disabled direction items. Because the answer on screen says a sort may choose its items, the search is also started again: spinner, and the rows are replaced by another run. On a broad search that run can list other items among ties, and a selected row that drops out loses its selection.
What should happen: nothing. Relevance has no direction (the manual: "Both are disabled for Relevance"). The review of sorted-search-results already made the checked Ascending/Descending items do nothing for this same reason; the checked key was missed.
    evidence: Traced by reading the snapshot at 7e8df53; no guard, reset or early return stops it.

1. src/finder/toolbar.rs:775-787 lists Relevance while searching. `.checked(sort.key == key)` only draws the mark; the item's `on_click` calls `this.set_sort(key, cx)`. The kit runs the handler of any item that is not disabled (gpui-component-0.8.1 src/menu/popup_menu.rs:228-240 `is_clickable`, 870-886 `activate`).
2. src/finder/mod.rs:3170-3175: `let current = self.sort_order(); let order = if current.key == key { SortOrder { key, ascending: !current.ascending } }`. There is no exception for Relevance.
3. mod.rs:3182-3185: `if self.is_searching() { let read_for_results = self.result_reads(); self.result_so
    fix: In `Finder::set_sort`, return early when `key == SortKey::Relevance` and the current key is Relevance (or never flip `ascending` for Relevance).
    test: In a_sort_chooses_among_every_match_not_among_the_ones_listed, right after `set_query("*.log")` with the limit at 3: `view.update(cx, |f, cx| { f.set_sort(SortKey::Relevance, cx); assert!(!f.search_in_flight_for_test()); assert!(f.current_sort().ascending); })`. Both asserts fail today.

[confirmed] sev 2->1 known=False sorted-cut-ignores-folders-on-top: With Keep Folders on Top on, a sorted cut search leaves matching folders out and lists files that belong below them
    improvements: sorted-search-results
    scenario: The list is as the finder says, under Size descending and under Name ascending. The defect is narrower than the title: the engine choosing by the sort alone was decided in the brief, because with folders on top first, a broad search's matching folders would fill the list and no file would show. What is missing is one sentence in the manual's sort paragraph (docs/manual.md:314) saying that a sort chooses the items before folders are put on top. That is the finder's "smallest alternative"; the engine change it leads with is the one the plan rejected.
    evidence: Traced by reading; the list comes out as the finder says. But the behaviour is the plan's choice, not an oversight, so what is confirmed is a gap in the manual.

The path:
1. src/finder/mod.rs:2203-2213 builds the request with `sort: Some(self.result_sort)` and nothing about folders. `folders_first` occurs nowhere in crates/fffinder-core/src/engine.rs, and `SearchRequest` (engine.rs:199-223) has no such field.
2. engine.rs:2150-2154 keeps the names that have the word: `ledger`, ledger.txt, my-ledger-notes.txt, xledgerx.txt (4 > 2).
3. engine.rs:2172-2175 orders them by `a.cmp(b).then_with(|| listing::natural_cmp(a_name, b_name))`. The key is `CutKey::Size(hit.size, !hit.is_dir)` (2214) and a
    fix: Add `folders_first: bool` to `SearchRequest`, set it from `config.folders_first` in `start_search`, and have `sort_before_cut` put folders first when it is set; let `folders_first_changed` call `run_search` when the answer on screen says `sort_chooses`. Smallest alternative: one sentence in the manual's sort paragraph saying a sort chooses the items before folders are put on top.
    test: Copy a_sort_by_a_word_stays_with_the_names_that_have_it with `folders_first` turned on before the search (through the config or the toggle_folders_first action), limit 2, query `ledger`, click `Size`, and assert `ui.names() == ["ledger", "my-ledger-notes.txt"]`. Today it is ["my-ledger-notes.txt", "xledgerx.txt"].

[confirmed] sev 1->1 known=False scope-chip-off-where-no-folder: In Recents, the Trash or sent results the scope bar's only chip shows as off while Search Only in is on, and clicking it restarts the same search from
    improvements: results-scroll-reset, back-forward-restore-search, scope-chips-keep-focus
    scenario: Turn on Search Only in "Folder" in a folder, or press Back to a search that was made in its folder, which turns it on. Go to Recents (Cmd+Shift+F), type a search, select a result and scroll down.
What happens: the search is of every location (`searched_folder()` is None), but the scope bar's one chip, This Computer, is drawn unselected. Click it: the search is the same, yet it runs as a new one, so the selection is cleared and the list jumps to the top. From then on Search Only in is off for the tab.
What should happen: the chip shows as on wherever the place has no folder to search, and a click that leaves the searched place unchanged is not a new search.
The chip's test (`scope == Computer`) is upstream's. This branch adds the reset of selection and scroll (7ef1f95) and a second way into the state through Back (323f173).
    evidence: Traced by reading; nothing prevents it.

The chip: src/finder/bars.rs:87-91 `chip("scope-computer", "This Computer".into(), scope == SearchScope::Computer)`. The folder chip is under `.when_some(folder_name, ..)` (bars.rs:97), and `current_folder()` is None in Recents, sent results and the Trash (src/model.rs:24-28).

The state lasts: `self.scope` is assigned only at src/finder/mod.rs:2137 (restore_search), 2362 (set_scope) and src/finder/filter.rs:361 and 393 (Smart Folders). `navigate` and `location_changed` (mod.rs:1269-1316) never touch it, so Folder stays on for the tab after the toolbar's button, a folder chip, `--here` (search_here, mod.rs:2371-2375) or Back to a folder's search. In t
    fix: In `render_scope_bar`, select the This Computer chip by `self.searched_folder().is_none()`. In `set_scope`, call `run_new_search` only when `searched_folder()` changed, and otherwise only store the scope.
    test: Open a folder, call `f.set_scope(SearchScope::Folder, cx)`, navigate to Recents, search for a word with several results, select the second result, click `scope-computer`. Assert `ui.selected()` is unchanged and `!f.search_in_flight_for_test()`. Today the selection is empty after the click.

5 findings; 5 confirmed, 0 uncertain, 0 refuted
