# Saved for later

36 improvements that are written up and not built. Each was found by reading the code; those marked
"verified" were checked against the code by a second reviewer before any work started, and those marked
"found in review" were found while another change was being reviewed and have not had that second check.
Highest value first. The full write-up of each, with the verifier's plan, is `briefs/<id>.json`.

| Value | Size | Area | Item | |
| --- | --- | --- | --- | --- |
| 5 | S | ui | [Column view: a right-click on the selected row of an earlier column opens the menu for another file, and its Move to Trash trashes that one](#column-row-menu-wrong-item) | found in review |
| 4 | M | ui | [Split view: show which pane is active, and make Tab switch panes instead of stranding focus in the toolbar](#split-active-pane) | verified |
| 3 | M | ui | [Collapsible sidebar sections (Favorites, Smart Folders, Locations) that stay collapsed](#collapsible-sidebar-sections) | verified |
| 3 | M | search | [Contents results do not show what matched: highlight the match in the Match column and start the snippet near it](#content-match-highlight) | verified |
| 3 | M | ui | [Menus that don't fit where they're shown: dead items in a file chooser, a mislabeled Actions item in the Trash, no menu on the Trash row](#context-menus-fit) | verified |
| 3 | M | search | [With a Nushell filter on, a new folder's items or a search's results are first shown through the filter's answer for the last rows, usually as an empty list](#filter-stale-keep-set) | found in review |
| 3 | M | ui | [Go to Folder: arrow keys choose a suggestion, Return takes it (or the only match), and suggestions can include indexed folders by name](#go-to-folder-keyboard) | verified |
| 3 | M | ui | [Icon view: the blank margins of a tile act as the item, leaving almost nowhere to start a box selection or click to deselect](#icon-tile-hit-target) | verified |
| 3 | M | ui | [After the search field has had the keyboard once, the window redraws about twice a second for good](#idle-redraw-after-search) | found in review |
| 3 | M | ui | [Image previews: show pixel dimensions, stop enlarging small images, and show a search hit's folder in the caption](#image-preview-details) | verified |
| 3 | S | ui | [Icon and gallery views always draw the selection in the active colour, so the focused pane can't be told apart](#inactive-selection-colour) | verified |
| 3 | S | search | [Multi-word and quoted searches highlight nothing or the wrong letters](#multi-word-highlights) | verified |
| 3 | M | ui | [Name-clash dialog: show which items clash and how the two copies differ](#name-clash-details) | verified |
| 3 | S | ui | [The preview pane and column card cap the preview at 240 points however tall the window is](#preview-card-height) | verified |
| 3 | S | ui | [Quick Look goes blank with a stale title when nothing is selected, and gives no position among several items](#quick-look-empty-state) | verified |
| 3 | M | search | [Recent searches: a menu on the search field's magnifier and Up to recall, plus a placeholder that names the scope](#recent-searches) | verified |
| 3 | S | search | [Recents can come up short or empty when many recently changed files are ones it hides](#recents-walk-index) | verified |
| 3 | M | search | [Regular-expression content search in the window, with a message when the pattern is invalid](#regex-content-search) | verified |
| 3 | M | ui | [Reopen the last window's tabs and split view at launch](#restore-session-tabs) | verified |
| 3 | S | search | [Settings › Search locations: say why a listed folder isn't indexed, and stop adding duplicates](#search-roots-notes-dedupe) | verified |
| 3 | M | search | [Settings › Search engine: correct the 'rebuilds the indexes' hint, explain the switches, and fix the steppers](#search-settings-hints-steppers) | verified |
| 3 | S | search | [Two Search engine settings do not do what they say: file-size limit above 10 MB, and 'Keep often-used files in memory'](#search-settings-without-effect) | verified |
| 3 | S | search | [Starting a search flashes placeholder bars for a few frames before the results](#search-start-no-flash) | found in review |
| 3 | M | ui | [Select All in a large folder makes every frame and every refresh scale with the selection size](#select-all-perf) | verified |
| 3 | S | ui | [Settings: rows run past the dialog's edge with wider fonts ('Remember each folder's vi', the letter case buttons, Try It)](#settings-rows-fit) | found in review |
| 3 | M | ui | [Split view clips the main pane's toolbar: the sidebar takes its room from the window, not the pane](#split-view-toolbar-clip) | verified |
| 3 | S | ui | [A table preview's columns ignore the room they have: an archive's two columns scroll sideways in the Preview Pane, and stay 140 points wide in Quick Look](#table-preview-column-widths) | found in review |
| 3 | S | ui | [Tree sidebar: the Down arrow can't get past the first Smart Folder, and a Smart Folder's row has no Cmd+click or middle-click](#tree-arrows-past-smart-folder) | found in review |
| 3 | S | ui | [Type-to-select cannot contain a space: Space opens Quick Look in the middle of a name](#type-select-space) | verified |
| 3 | M | ui | [Switching views loses the selected item: Columns drops it, Icons can scroll to the wrong rows](#view-switch-keeps-selection) | verified |
| 2 | M | search | [Temporary indexes for folders outside the search locations: no progress shown, rebuilt for every subfolder, and discarded indexes keep working](#adhoc-index-feedback) | verified |
| 2 | S | ui | [After a file operation during a search, every result is checked on disk on the window's thread](#files-changed-off-thread) | found in review |
| 2 | S | ui | [Get Info: stop adding up a folder when the panel closes](#get-info-stop-walk) | verified |
| 2 | S | ui | [The path bar rescans /Volumes on the window's thread every frame (macOS)](#path-bar-no-io) | verified |
| 2 | S | ui | [Page Up/Down and Cmd+=/−/0 act on the wrong thing when the item isn't a PDF or Quick Look has its own window](#preview-page-keys) | verified |
| 2 | M | search | [The 'opened before for this search' boost is lost when the search is scoped, kind-filtered, quoted, or uses a glob](#query-history-boost) | verified |

## column-row-menu-wrong-item

**Column view: a right-click on the selected row of an earlier column opens the menu for another file, and its Move to Trash trashes that one**

Value 5 of 5, size S, ui, bug.

**Problem.** Found with a probe by the reviewer of 'column-pane-selection-by-path' (done record: /home/user/polish-tools/done/column-pane-selection-by-path.json, first item under 'Found, not fixed'), and read by the orchestrator. In the column view every column but the one in use shows its selected row in grey: the folder you came through. A right-click on such a row opens the item menu, but for the item of the column in use, not for the row under the pointer: in the probe, the menu over the folder Projects read Cut "b.txt", and its Move to Trash would have trashed b.txt. The cause is in `render_column_row` (src/finder/views.rs): the row's right-press handler selects only when the row is not already the column's selected one (`if this.columns.get(col).is_some_and(|p| p.selected != Some(ix)) { this.select_in_column(col, ix, window, cx); }`), so for a row that is already selected nothing changes, the column in use stays the later one, and the menu is built from `selected_entries()`, which follows the column in use. A person right-clicks a folder and can move a different file to the Trash, rename it, or copy it, from a menu that names the other file only in some of its rows.

**Proposal.** Reproduce first with a headless test: column view, a folder selected in the first column and a file selected in the second (so the second is the column in use), right-click the first column's grey row; the item menu's entries name the second column's file (tests/controls.rs the_item_menu_shows_a_result_in_its_enclosing_folder and tests/columns.rs show how menus and columns are read in tests). Then make the right press act on the row under the pointer: when the row is already its column's selected one but its column is not the one in use, make that column the one in use, as a left click on the row or the Left arrow would (the reviewer's pointer: `|| col != this.active_column` in that condition, or what Left does). Read `select_in_column` whole first, as 'column-pane-selection-by-path' left it: it closes the columns to the right of the row's column and opens the folder's own column again, and that is also what a left click on the grey row does today; say whether the right press should do exactly the same (the columns to the right collapse to the folder's own) and make the menu and the highlight agree with whatever shows. Check every command of the menu against the row afterwards (Open, Cut, Copy, Rename…, Move to Trash, Get Info, Quick Look, Copy Path, Show in Enclosing Folder where it applies), and the keys pressed while the menu is open. Check the same press with several items selected in the rows' selection (Select All in column view is listed as an oddity in that record), with a rename in progress, and in a file chooser.

**Tests.** A headless test that fails today: with a later column in use, a right-click on the grey selected row of an earlier column opens a menu whose items name that row's item, and Move to Trash from it trashes that item and nothing else (assert on the file system: the other file is still there). The same for Rename… (the field opens on that row) and Copy (the clipboard holds that row's path). Guards: a right-click on an unselected row of any column, on the selected row of the column in use, and on a file's row behave as before; the keyboard is where it was after the menu closes with Escape.

**Risks.** Making the earlier column the one in use closes the columns to its right, which is a visible change on a right-click; a left click on that row already does it, so it is consistent, but look at it in a picture and say so in the record. 'column-pane-selection-by-path' (7e8df53) just rewrote how columns keep and remap their selection (by path) and 'stale-previews-and-cancel' (52e7de4) how the preview follows it: build on both, and keep their tests passing. `item_menu_at` is set by this handler and read by the empty-space menu ('collapsed-groups-empty-view' explains the mark); leave that line as it is.

**Notes from later reviews.** Queued after the cut-off of 20 Nov because it can send the wrong file to the Trash. Keep it small: the one fault, its test, a sentence in the manual only if the manual says something about right-clicks in columns that the fix makes untrue.

## split-active-pane

**Split view: show which pane is active, and make Tab switch panes instead of stranding focus in the toolbar**

Value 4 of 5, size M, ui, ux.

**Problem.** Nothing marks the active pane. Workspace::render draws both panes the same, with a 1px divider (src/workspace.rs:291-317), and neither toolbar changes. After Cmd+Backslash, focus moves to the new pane (workspace.rs:262) while the old pane's selection stays bright blue in icon view (see inactive-selection-colour). F5, F6 and Delete act on whichever pane has focus (src/finder/tab_bar.rs:219-255; src/actions.rs:153, 221). No key moves between panes: Tab isn't bound in src/actions.rs, so it falls to the kit root's `tab` -> focus_next (gpui-base-0.8.1/src/root.rs:44, 222-224). That jumps to a toolbar button, since buttons are tab stops (gpui-component-0.8.1/src/button/button.rs:286) and `files_focus` is a plain handle (src/finder/mod.rs:539). Shift+Tab can't return to the files, in one-pane windows too. Related gaps: Copy to Other Pane and Move to Other Pane are only in the macOS menu bar (actions.rs:422-423); item_menu in src/finder/menus.rs and the Actions menu (src/finder/toolbar.rs:727-733) don't have them. `frontmost()` always answers the control socket with `workspace.active()`, even when the user is in the split pane (src/lib.rs:190). Any pane's navigation sets the window title (mod.rs:922).

**Proposal.** src/workspace.rs: track `active_pane: EntityId`, set from `cx.on_focus_in` on each pane's root focus handle and in toggle_split()/focus_active(). Add `active_pane()`, `is_active_pane(id)` and `focus_other_pane(of, window, cx)`. In render(), while split, give the active pane's wrapper a 2px `theme.primary` top edge with id `pane-active`, transparent on the other. src/lib.rs `frontmost()`: return the active pane. src/actions.rs: add a `SwitchPane` action bound to `tab` and `shift-tab` in the FILES context, plus a Window menu item 'Switch Pane'. The handler in src/finder/tab_bar.rs focuses the other pane's files; with one pane it does nothing, so Tab no longer strands focus. src/finder/toolbar.rs:517-525: draw the title in `muted_foreground` in the inactive pane of a split window. src/finder/mod.rs:922: set the window title only from the active pane. src/finder/menus.rs `item_menu`: when `has_split_view`, add 'Copy to Other Pane' and 'Move to Other Pane' with `.action(..)`, which shows F5 and F6. The dimmed selection in the inactive pane's icon and gallery views comes from the inactive-selection-colour candidate (same views.rs change). Update docs/manual.md (Tabs and split view, shortcut tables) and CHANGELOG.md.

**Checked plan.** Keep the goal, but change five things in the proposal.

**1. Tab binding: split view only, not a no-op in one-pane windows**
- The proposal binds Tab in `FILES`, which also matches `"FinderFiles OnePane"`. That would remove the only Tab route from the file list to the toolbar and sidebar in every ordinary window, and still would not let Shift+Tab return to the files.
- In `src/actions.rs`, add `SwitchPane` to `actions!` and a context `context("FinderFiles && !OnePane")`. `!` is supported (`gpui-pre-0.3.10/src/keymap/context.rs:289-296, 389`), and `"FinderFiles && OnePane"` at `actions.rs:124` is the precedent.
- Bind `shift-tab` first and `tab` last, so the menu bar shows Tab (see the comment at `actions.rs:141`). The deeper context beats the kit root's `tab` (`gpui-pre-0.3.10/src/keymap.rs:216-217`).
- Add `MenuItem::action("Switch Pane", SwitchPane)` after Split View (`actions.rs:421`).
- Fix the stranding itself with one line at `src/finder/mod.rs:539`: `cx.focus_handle().tab_stop(true)`. The kit's Tab ring then includes the files, Shift+Tab returns, and one-pane Tab-from-files behaves as it does today.

**2. Handler**
- Add `Finder::switch_pane` in `src/finder/tab_bar.rs`: take `cx.entity_id()`, then `self.with_host(window, cx, move |w, window, cx| w.focus_other_pane(me, window, cx))`.
- Register it on the Finder root beside `copy_to_other_pane` (`src/finder/mod.rs:3131-3132`), not on the content div. `every_menu_bar_action_has_a_handler` (`tests/controls.rs:223-299`) expects menu-bar actions to be available with focus in the search field; add `SwitchPane` to its list.
- With no other pane it silently does nothing.

**3. Active-pane state in `src/workspace.rs`: do not rely on `cx.on_focus_in` alone**
- Focus events only fire in an active window, and a test window does not start active (`tests/common/mod.rs:551-552`). They would also need a stored subscription per tab.
- Use a `split_active: bool` field instead:
  - Set it explicitly in `toggle_split` (true on open, false on close) and in the new `focus_other_pane`.
  - For mouse activation, sync at the top of `Workspace::render`: if exactly one of `split.focus_handle(cx).contains_focused(window, cx)` and `self.active().focus_handle(cx).contains_focused(window, cx)` is true, store it.
- This works because `Window::focus` calls `refresh()` (`gpui-pre-0.3.10/src/window.rs:2418-2428`), so every focus change redraws; `views.rs:547` already relies on that. A dialog or menu holding focus leaves the remembered value, and hidden tabs are never consulted.
- Pitfall: `contains_focused` reads the last rendered frame (`window.rs:533-536`), so a pane that has never been drawn reports false. That is why `toggle_split` must set the field itself.
- Add `active_pane() -> &Entity<Finder>`, `is_active_pane(id)` and `focus_other_pane(of, window, cx)`. The last reuses `other_pane` (`workspace.rs:146`), focuses `files_focus_handle()`, sets the window title to that pane's `title()`, and calls `cx.notify()`.

**4. Marker**
- A 2px border on the pane wrapper would shift the toolbar down 2px whenever split is on, while the macOS traffic lights stay fixed at y=18 (`src/lib.rs:426`).
- Draw it as an absolutely positioned 2px `theme.primary` bar inside a `.relative()` wrapper, for the active pane only while split. `theme.primary` is the system accent (`src/theme.rs:238`; `bars.rs:29` uses it the same way).
- In `src/finder/toolbar.rs:517-525`, add `.when(split && !active, |d| d.text_color(muted_foreground))`. Compute `active` in render through `host.read(cx).is_active_pane(cx.entity_id())`.
- Only read the host from render paths, as `has_split_view` does (`tab_bar.rs:50`). Never read it from code reachable while the Workspace is being updated, such as `files_changed` → `reload_quietly`; `Entity::read` panics on a leased entity.

**5. Window title**
- Leave `src/finder/mod.rs:922` alone. Navigating a pane already requires interacting with it, which makes it active. A host read inside `location_changed` adds the lease hazard above for no gain.
- The real gap is that switching panes does not update the title. Set it in `focus_other_pane` and when the render sync flips `split_active`.

**Kept as proposed**
- `src/lib.rs:190`: `workspace.read(cx).active_pane().clone()`.
- `src/finder/menus.rs` `item_menu`: when `finder.has_split_view(cx)`, add "Copy to Other Pane" and "Move to Other Pane" with `.action(Box::new(..))` near the Cut/Copy group. The popup dispatches to the pre-menu focus and shows F5/F6 (`gpui-component-0.8.1/src/menu/popup_menu.rs:904-920, 1138-1164`). The Trash menu returns earlier, so it stays unchanged.

**Docs**
- `docs/manual.md`: split-view paragraph (line 160), Window menu row (784), item context menu list (~795), shortcut tables (~869, ~917).
- `docs/protocol.md:108, 121`: the active pane answers.
- `CHANGELOG.md`.

The dimmed selection in icons and gallery stays with the inactive-selection-colour candidate (`views.rs:519, 1309`).

**Tests.** Add the tests to `tests/file_ops.rs`, either extending `tabs_and_split_view` (line 1031) or as a new `tab_switches_panes_in_split_view`. Open with `Ui::open_with(cx, &fx.dir, Open { workspace: true, ..Default::default() })` and take `ui.workspace.clone().unwrap()`.

**Split view**
1. `ui.press(&cmd("\\"))`, then fetch the pane with `workspace.read_with(ui.cx, |w, _| w.split_pane().cloned())`.
2. Assert the split pane's files are focused with `ui.window(|window, cx| pane.read(cx).files_focused(window))`, and that `workspace.read_with(ui.cx, |w, _| w.active_pane().entity_id())` equals `pane.entity_id()`.
3. `ui.press("tab")`: assert `ui.files_focused()` (the main view) and that `active_pane()` is the main id.
4. `ui.press("shift-tab")`: back to the split pane.
5. Navigate the split pane to `Projects`, as lines 1099-1105 already do. Then `ui.press("tab")`, `ui.select("notes.txt")`, `ui.press("f5")`, and assert `fx.path("Projects/notes.txt").exists()`.

**Context menu**
- Set list view before splitting, as `tests/columns.rs:428-429` does, because element ids repeat across panes.
- With the split pane in another folder, `ui.right_click(item_id("row", &fx.path("notes.txt")))` and assert `ui.menu_labels()` contains "Copy to Other Pane" and "Move to Other Pane".
- In a one-pane `Ui::open`, assert they are absent.

**Mouse activation**
- Use `window.find_all("content")` sorted by x, the pattern at `tests/columns.rs:443-453`, and `ui.click_point(..)` in the left pane.
- Assert `active_pane()` is the main id.
- With the render-time sync this needs no `ui.activate()`; if `on_focus_in` is used instead, the test must call `ui.activate()` first.

**One pane**
- Close the split with `cmd("\\")`. `ui.press("tab")` and assert `!ui.files_focused()`, so toolbar traversal is kept. Then `ui.press("shift-tab")` and assert `ui.files_focused()`, which is the new tab stop.

**Marker**
- Assert through the `active_pane()` / `is_active_pane()` accessors. The proposed `pane-active` element is not findable by the harness without `.test_support()`.

**Other**
- Add `SwitchPane.boxed_clone()` to the list in `every_menu_bar_action_has_a_handler` (`tests/controls.rs:227-281`).
- `frontmost()` is private and the harness window is not registered in `Windows`, so it is covered only through `active_pane()`.

**Risks.** - **One-pane regression if implemented as proposed.** Binding Tab in `FILES` as a no-op removes keyboard traversal from the file list to the toolbar and sidebar in every ordinary window. The corrected plan avoids this with the split-only context.
- **Tab ring changes.** Making `files_focus` a tab stop means Tab from the search field or another control can now land on the file area. No test pins this, and it has not been run (cargo was not allowed).
- **macOS menu bar.** The Window menu will show a bare Tab key equivalent for Switch Pane. The kit root always consumes Tab first, and bare Delete/F5/F2 equivalents already exist, but this cannot be verified on Linux.
- **Render-time sync edge cases.** `contains_focused` reads the previous frame, so `toggle_split` must set the state explicitly. Mutating `split_active` in `Workspace::render` must not call `cx.notify()`.
- **Lease panic.** Reading the host Workspace from a Finder while the Workspace is being updated panics; keep `is_active_pane` reads to render paths.
- **Control socket behaviour change.** `fff selection`, show and open now answer from or act on the split pane when it is active. This is intended, but it is a visible change for scripts and needs the `docs/protocol.md` note.
- **Marker layout.** The marker must not change layout, or the split-view toolbar shifts against the fixed macOS traffic lights.

**Notes from later reviews.** 'split-view-toolbar-clip', 'tab-bar-overflow' and 'tab-drop-and-close' land before this one and touch src/workspace.rs and src/finder/tab_bar.rs; build on them. 20 Nov: 'tab-bar-overflow' (7e5689c) and 'focus-lost-recovery' (4e5b448) are on the trunk; read both done records, which have notes for this brief (a disabled toolbar button is not a tab stop; the root's `keyboard_stays` listener; which pane gets the keyboard after a press on the chrome; Ctrl+Tab pressed in the other pane switches the first pane's tab). 'tab-drop-and-close' is being worked on in another lane at the same time and changes src/workspace.rs and src/finder/tab_bar.rs: expect to rebase over it, or it over you; keep your changes to those files as small as they can be. 'split-view-toolbar-clip' was saved for later and is not on the trunk.

## collapsible-sidebar-sections

**Collapsible sidebar sections (Favorites, Smart Folders, Locations) that stay collapsed**

Value 3 of 5, size M, ui, feature.

**Problem.** The sidebar's section headings are inert labels. `Line::Heading(&'static str)` (src/finder/sidebar.rs:60-62) is drawn by `section_heading` (sidebar.rs:499, 1229-1240), a plain `div` with no id, hover state or click handler, and `build_sidebar_lines` always emits every row under it (sidebar.rs:301-328). There is no collapse state to remember in the config either (crates/fffinder-core/src/config.rs:204-282). Finder lets each section be hidden and remembers it. It matters most in tree mode, where an opened Favorites tree can push Locations and the Trash, which is a drop target, far down the list (sidebar.rs:308-309).

**Proposal.** config.rs: add `collapsed_sidebar_sections: Vec<String>` to `Config` (`#[serde(default)]` on the struct covers old files), default empty, and list it under `shared:` in `settings_by_scope!` (config.rs:609-615) so every tab and window agrees. sidebar.rs: in `build_sidebar_lines` push the heading, then skip that section's rows when its name is in the list. Keep the Trash line always, so it stays a drop target. Hash the list in `sidebar_key` (215-230) so the cached layout rebuilds. Make the heading a stateful row: `div().id(format!("heading-{label}"))`, a chevron (right when collapsed, down on hover when open), `.on_click(cx.listener(..toggle_sidebar_section(label, cx)))`, `.test_support()`. `toggle_sidebar_section` edits the config, then `settings_changed(cx)` and `cx.notify()`. Tree keyboard movement already walks the built lines and skips headings (src/finder/folder_tree.rs:967-989), so hidden rows drop out without more work. Document in docs/manual.md:188-195 and the settings list.

**Checked plan.** Corrected plan (all cross-platform gpui code).

config.rs
- Add `pub enum SidebarSection { Favorites, SmartFolders, Locations }` (derive Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize; `#[serde(rename_all = "snake_case")]`) with `title()` ("Favorites" / "Smart Folders" / "Locations") like `ViewMode::title` and a `key()` for element ids. Re-export from crates/fffinder-core/src/lib.rs:26.
- Add `pub collapsed_sidebar_sections: BTreeSet<SidebarSection>` to `Config` (default empty; struct-level `#[serde(default)]` covers old files; a set keeps the value canonical so two tabs compare equal). List it under `shared:` in `settings_by_scope!` (:609-616). (Acceptable alternative: `Vec<String>` of the same lowercase keys if you want typos ignored instead of making the file unreadable; do not store display labels.)

sidebar.rs
- `Line::Heading(SidebarSection)`; update the 6 match sites (sidebar.rs:101, 242, 285, 334, 350, 499; folder_tree.rs:929, 976 need no change in shape).
- `build_sidebar_lines` (:301-328): push each heading, skip that section's rows when collapsed; Smart Folders heading still only when non-empty; always push `Line::Trash`.
- `sidebar_key` (:215-230): hash `self.config.collapsed_sidebar_sections`.
- Replace free fn `section_heading` (:1229) with a `Finder` method taking `cx`: keep the 36pt layout, add `.id(SharedString::from(format!("heading-{}", section.key())))`, a 12pt chevron as in groups.rs:298-305 (ChevronRight when collapsed; when open either always ChevronDown or hover-only via `.group("sidebar-heading")` + a wrapping `div().invisible().group_hover("sidebar-heading", |s| s.visible())` - group_hover exists in gpui-pre div.rs:876 but has no precedent in this repo, so fall back to always-visible if fiddly), `.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_sidebar_section(section, cx)))`, `.test_support()` last.
- `pub fn toggle_sidebar_section(&mut self, section, cx)`: flip membership, `self.settings_changed(cx)`, then `self.sidebar_sections_changed(cx)`.

folder_tree.rs (the correction)
- `FolderTree::close_section(&mut self, section: Section)`: `expanded.retain(|k| k.section != section)`, clear `selected` and `reveal` if in that section, `changed()`. (Same rule as `collapse` :456-467: closing closes what is open inside; `prune` then drops the contents.)
- `Finder::section_collapsed(&self, section: Section) -> bool` mapping tree `Section` -> `SidebarSection`.
- `place_containing` (:661-671): filter out places whose section is collapsed. In `tree_follow_location` (:627-633) also drop a kept selection whose section is collapsed.
- `pub(super) fn sidebar_sections_changed(&mut self, cx)`: if `tree_on()`, `close_section` for each collapsed tree section, then `tree_follow_location(cx)` (re-homes the selection under a visible section, e.g. Locations > Home, or leaves it None); `cx.notify()`.

mod.rs / filter.rs
- `adopt_settings` (mod.rs:2347-2351): also call `sidebar_sections_changed(cx)` when `collapsed_sidebar_sections` differs from `before`.
- `add_favorite` (mod.rs:2373) removes `Favorites` from the set and `add_smart_folder` (filter.rs:201) removes `SmartFolders` before `settings_changed`, so something just added (menu, toolbar, or a drop on the sidebar at sidebar.rs:451-463) is visible; call `sidebar_sections_changed` when it actually expanded.

Docs
- docs/manual.md:188-197: clicking a Favorites / Smart Folders / Locations heading hides or shows its rows, every tab and window agrees, it is saved, the Trash always stays (line 195 currently says Locations "ends with the Trash"); in the Folder tree bullets (:217) say the tree follows under sections that are showing and collapsing a section closes what was open in it. Add a `collapsed_sidebar_sections` row (default `[]`, values `favorites`, `smart_folders`, `locations`) to the settings.json table after `expand_to_open_folder` (:1072). CHANGELOG.md [Unreleased] > Added.

Pitfalls
- `Finder.tree` fields are private to folder_tree.rs; keep the tree mutation there.
- A heading click inside `sidebar-scroll` still focuses the tree in tree mode (:426-431); harmless once the selection can't be hidden.
- Do not call `ui.activate()` in tests after `set_locations_for_test` (activation rereads real locations, mod.rs:522-527).

**Tests.** All runnable headless on Linux with the existing harness (tests/common/mod.rs: `Fixture::new`, `Ui::open`, `ui.click`, `ui.exists`, `ui.read`, `ui.update`, `ui.settle`, `ui.go_to`, `ui.press`).

1. Unit, crates/fffinder-core/src/config.rs tests: add the field to `round_trips` (:859); assert `collapsed_sidebar_sections.is_empty()` in `tolerates_partial_files` (:1239) and that `["favorites","smart_folders"]` parses; extend `taking_shared_changes_leaves_a_browsers_own_settings` (:906) so `copy_shared_from` carries the set.

2. tests/controls.rs, next to `sidebar_rows_and_menus` (:399): new `#[gpui_kit::test] fn sidebar_sections_collapse_and_expand`. Add favorite `fx.path("Projects")`; make Locations deterministic with `f.set_locations_for_test(vec![Place{..}])` + `cx.notify()` (precedent tests/file_ops.rs:988-994) instead of relying on `("loc", 0usize)`, which the existing test guards with `if ui.exists` (:417); add a smart folder with `f.add_smart_folder(SmartFolder{name, query, ..Default::default()}, cx)`. Then: `ui.click("heading-favorites")` -> `!ui.exists(("fav", ix))`, `ui.exists(("smart", 0usize))`, `ui.exists(("loc", 0usize))`, `ui.exists("trash")`, and `ui.read(|f| f.config().collapsed_sidebar_sections.clone())` == {Favorites}; `ui.click("heading-locations")` -> `!ui.exists(("loc", 0usize))`, `ui.exists("trash")`, `ui.click("trash")` reaches `Location::Trash`; `ui.click("heading-smart_folders")` hides `("smart", 0usize)`; `add_favorite` of another folder while collapsed re-expands Favorites; clicking a heading again brings rows back.

3. tests/settings.rs `shared_settings_show_in_every_tab_at_once` (:66, helpers `with_tabs`, `tabs`): `all[1].update(ui.cx, |f, cx| f.toggle_sidebar_section(SidebarSection::Favorites, cx)); ui.settle();` then assert `all[0]`'s `config().collapsed_sidebar_sections` contains it.

4. tests/folder_tree.rs (helpers `tree_window` :20, `selected` :32, `favorite_ix` :37): set Locations to one `Place` at `fx.dir` (same folder as the favorite, so Favorites wins the tie). Open the favorite (`ui.click(home.disclosure_id())`, `ui.exists(projects.row_id())`), `ui.click("heading-favorites")` -> `!ui.exists(("fav", ix))`, `!ui.exists(projects.row_id())`, and `selected(&ui) == Some(NodeKey::place(Section::Locations, fx.dir.clone()))`; `ui.go_to(&fx.path("Projects/atlas"))` -> `selected` is the Locations key `.under(atlas)` and `ui.exists(loc.under(fx.path("Projects")).row_id())`; `ui.click("recents")` then `ui.press("down")` lands on the Locations place (headings and hidden rows skipped); `ui.click("heading-favorites")` again -> the favorite row is back and closed, selection stays under Locations. If a heading's centre is clipped in the sideways-scrolling tree, use `ui.click_at("heading-favorites", at(12., 28.))`.

No existing assertion pins inert headings, so nothing existing should change.

**Risks.** - Without the folder_tree.rs correction the feature regresses tree mode: the tree follows the window invisibly inside the collapsed section, keeps reading and polling hidden folders, and F2 with the tree focused starts an in-place rename whose field is never drawn (keyboard goes nowhere). The correction changes which place the tree follows under, so it needs its own tree-mode test.
- Collapsing a section in tree mode closes the nodes open inside it (consistent with closing a node, CHANGELOG.md:69), so re-expanding does not restore the opened subtree; and with "Open the tree to the folder shown" on, collapsing Favorites makes Locations open down to the current folder. Both should be documented.
- The Trash stays as a lone row under a collapsed "Locations" heading; slightly odd visually and contradicts the current manual wording (manual.md:195) until updated.
- The setting is shared, so it also applies to other windows, tabs and `--pick` windows at once; an older fffinder writing the file drops the field (harmless). With an enum-typed set, a typo in a hand edit makes the whole settings file unreadable for that session, as with every other enum field (file is moved aside, not lost).
- `group_hover` for a hover-only chevron has no precedent in the repo; an always-visible chevron is the safe fallback.
- Value is moderate: in list mode with a handful of favorites it is Finder parity only; the real benefit is tree mode (the Windows default).

## content-match-highlight

**Contents results do not show what matched: highlight the match in the Match column and start the snippet near it**

Value 3 of 5, size M, search, ux.

**Problem.** For a contents search each row's Match cell is built as `format!("{}: {}", hit.line, hit.text.trim())` with an empty highlight list (src/finder/mod.rs:3013-3015). The engine already returns the match byte ranges for that line (`GrepHit.ranges`, crates/fffinder-core/src/engine.rs:230-231, 981-985; adjusted for the leading-whitespace trim by fff-search, grep/types.rs:72-84), but they are dropped. The cell is drawn as plain muted text truncated at the end (`list_cell` with an unstyled StyledText, src/finder/views.rs:838-846, used at 958-973), and the column is 220 pt wide by default (crates/fffinder-core/src/config.rs:158), roughly 35 characters, while fff-search returns up to 512 bytes of the line (grep/sink.rs:7). So for any match past the first few dozen characters (long prose lines, minified code, CSV rows) the user sees the line's beginning and never the matching text, and even when it is visible nothing marks it, unlike names, which get bold-on-tint highlights (`name_text`/`styled_name`, views.rs:228-274).

**Proposal.** src/finder/mod.rs: add `detail_highlights: Vec<(usize, usize)>` to `Row` (empty in `Row::new`). Add a pure helper `match_snippet(line: u64, text: &str, ranges: &[(usize, usize)]) -> (String, Vec<(usize, usize)>)`: trims the right side, and when the first range starts more than about 24 characters in, cuts the head at a char boundary so about 16 characters of context remain and prefixes an ellipsis; returns `"{line}: {snippet}"` with every range shifted by the prefix length minus the cut, dropping or clipping ranges that fall in the cut part. Use it in `content_search` (3013-3015). src/finder/views.rs: in the `searching` branch of `render_list_row` (958-972) draw the parent cell with highlights, either `styled_name(&row.detail, None, &row.detail_highlights, (!selected).then_some(pal.highlight))` through an optional StyledText argument on `list_cell`, or `StyledText::new(detail).with_highlights(..)` with the same bold plus `pal.highlight` background (no background on a selected row), passing it through `with_cut_off_text` so the full-text tooltip keeps working. Nothing changes for name searches or the Trash's Original Folder column. Mention it in docs/manual.md.

**Checked plan.** Corrected plan: keep `row.detail` as the full line, as today, so the tooltip and double-click fit are unchanged. Add the marks as data and choose what to show at draw time from the column's width.

1. **`src/finder/mod.rs`**
   - Add `pub detail_highlights: Vec<(usize, usize)>` to `Row`: byte ranges in `detail`, empty in `Row::new`, whose signature stays.
   - Add a pure `fn match_line(line: u64, text: &str, ranges: &[(usize, usize)]) -> (String, Vec<(usize, usize)>)` beside `content_search`:
     - `lead = text.len() - text.trim_start().len()` (0 today, since fff already trimmed), `body = text.trim()`.
     - If `body` contains U+FFFD, return no marks; the offsets cannot be trusted.
     - Otherwise, for each range: subtract `lead`, clip the end to `body.len()`, and drop it unless `a < b` and both ends are char boundaries. Sort, then merge overlapping or touching ranges.
     - Return `"{line}: {body}"` with the marks shifted by the prefix length.
   - In `content_search` (3013-3015), build the row with `Row::new(entry, Vec::new(), Some(detail))`, then set `row.detail_highlights`.
   - Add `pub fn match_cell_for_test(&self, ix: usize) -> (SharedString, Vec<(usize, usize)>)`, returning what the cell draws, in the style of `rows_for_test`.

2. **`src/finder/fit.rs`**
   - Add a pure `pub(super) fn match_window(detail: &SharedString, marks: &[(usize, usize)], room: usize) -> (SharedString, Vec<(usize, usize)>)`.
   - No marks, or `detail[..first_end].chars().count() <= room`: return the inputs unchanged, with no new string.
   - Otherwise: `prefix = detail.find(": ") + 2`; `lead = ((room - prefix_chars - 1 - match_chars) / 2).min(24)`, saturating. Find `start` by walking `lead` chars back from the first mark with `char_indices().rev()`, never before `prefix`. If `start == prefix`, return unchanged.
   - Return `"{prefix}…{detail[start..]}"` with every mark shifted by `head.len() - start`. No mark needs dropping, because the cut is always before the first one.

3. **`src/finder/views.rs`**
   - Split `list_cell` into `list_cell_showing(id, shown: StyledText, full: SharedString, width)`, with `list_cell` calling it.
   - In the `searching` branch of `render_list_row` (958-972), when `row.detail_highlights` is non-empty:
     - `room = ((parent_column_width() - 12.) / 7.) as usize`.
     - `(shown, marks) = match_window(..)` only if `parent_column_shows_match()`; otherwise use `detail` and its marks as they are.
     - Draw `styled_name(&shown, None, &marks, (!selected).then_some(pal.highlight))` through `list_cell_showing` with `full = row.detail.clone()`. A head-cut cell then always differs from `full`, so its tooltip shows the whole line.
   - Key the styling on the row's marks, not on `search_contents`: rows from the previous mode stay on screen until the new search lands.
   - Keep `.text_color(muted)`, the `text_ellipsis_middle` rule for folders, and `.test_support()`.
   - Rewrite the comment at 959-960.
   - Do not touch `list.rs`: `fitting_width` keeps measuring the full line.

4. **Docs**
   - `docs/manual.md:234` and `:335`: the match is bold on a tint; a line whose match is further along than the column shows starts shortly before it after an ellipsis; the tooltip still has the whole line; matches past the 512 bytes fff keeps are not marked.
   - Add an entry under Unreleased in `CHANGELOG.md`.

**Pitfalls**
- Never pass raw `hit.ranges` to `StyledText`. A bad range is a panic in debug builds and an invalid text run in release.
- The tests module in `mod.rs` deliberately avoids `use super::*`; import `match_line` by name.
- `room` is a character estimate (about 7 pt per character, 29 at 220 pt). An exact check is possible with `cx.text_system().line_wrapper(font, ui(12.)).should_truncate_line(.., TruncateFrom::End)`, but it is optional.

**Tests.** I ran nothing: the task was read-only with no cargo, so everything below is planned, not tried.

**Unit tests (plain `#[test]`)**
- `match_line`, in the tests module of `src/finder/mod.rs` (import it by name):
  - `(12, "let needle = 1;", [(4,10)])` gives `"12: let needle = 1;"` with a mark that slices to `needle`.
  - Trailing whitespace is trimmed and a range running into it is clipped.
  - `"aaaa"` with (0,2),(1,3),(2,4) merges into one mark.
  - A range splitting `é` is dropped.
  - Text containing U+FFFD yields no marks.
  - Unsorted input comes out sorted.
- `match_window`, in the tests module of `src/finder/fit.rs`:
  - A mark ending within `room` returns the same string and marks.
  - `"1: " + 200 x + "needle"` with `room` 29 starts with `"1: …"`, its mark slices to `needle` and starts before byte 40.
  - 100 `é` before the match cuts on a char boundary.
  - Two marks both survive, shifted.
  - A needle longer than `room` does not panic.

**Headless UI test in `tests/columns.rs`**
This file already rests the mouse on list cells to check tooltips (`cut_off_text_shows_in_a_tooltip`). Uses `Fixture`, `Ui::open`, `item_id` and `ui.hover_point` from `tests/common/mod.rs`.
1. Before `Ui::open`, write:
   - `far.txt`: 200 `x` followed by `needle`.
   - `latin1.txt`: the bytes `b"caf\xe9 \xe9t\xe9 needle\n"`.
2. `ui.click("view-list")`. The default view is Icons, which draws no Match cell.
3. `ui.update(|f, _, cx| f.set_search_contents(true, cx))`, then `ui.update(|f, _, cx| f.set_query("needle".into(), true, cx))`. Do not click `by-contents` first; it is not drawn until a search is active.
4. Assert:
   - Both files are in `ui.names()` and the frame drew. This is the regression check for the lossy-offset panic.
   - For `far.txt` in `rows_for_test()`: `detail` is still the whole line and `detail_highlights` slices to `needle`.
   - `match_cell_for_test(ix)` starts with `"1: …"` and its mark slices to `needle`.
   - Hovering `window.within(item_id("row", &path)).find("parent")` gives a `cut-off-text` tooltip whose `label()` equals the full `detail`.
   - After `ui.drag_by("resize-Match", 2000.)`, `match_cell_for_test` returns the full `detail` again. If the test text system's glyphs are too wide for 206 characters, use a shorter line for this step.
5. Query `plain`: the `notes.txt` row is `"1: plain notes"` with a mark slicing to `plain`, drawn uncut.

**Existing tests that must stay green**
- `scope_bar_controls` in `tests/controls.rs`.
- All of `tests/columns.rs` (Folder column resize, sort and tooltips).

**Risks.** - **Panic on bad ranges.** A contents search that hits a non-UTF-8 line or an overlapping match will crash unless `match_line` sanitises as described. This is the main risk, and the Latin-1 UI test exists to catch it.
- **Wrong-but-harmless marks.** Lossy-decoded lines get no mark, by design.
- **No mark past 512 bytes.** Rows whose match lies beyond fff's display cut look as they do today. The benefit for minified files is partial.
- **Width estimate.** `room` counts characters, so wide glyphs or CJK can still push the match out of view, and bold runs are slightly wider than the truncation maths assumes. Overflow is clipped, as it is for names.
- **Tooltip is still plain.** It shows the full line without a mark.
- **List view only.** The default Icons view and the Gallery show nothing for content matches, so the change is invisible there.
- **Per-frame work.** `match_window` runs for each visible row on each frame. It is a scan of at most 512 bytes plus one small allocation when it cuts, which is negligible.
- **Merge conflicts.** This touches `Row` and reuses `styled_name`, as the multi-word-highlights candidate does. Keep the field name `detail_highlights` and land the two in sequence.
- **Existing quirk, left alone.** Renaming a content result rebuilds its row (`src/finder/dialogs.rs:847-850`), so that row loses its match line and marks until the next search.

**Notes from later reviews.** 'cancel-content-search' and 'search-truncation-honesty' land before this one and change how a contents search is run in src/finder/mod.rs; build on them. 'multi-word-highlights' changes how names are highlighted.

## context-menus-fit

**Menus that don't fit where they're shown: dead items in a file chooser, a mislabeled Actions item in the Trash, no menu on the Trash row**

Value 3 of 5, size M, ui, ux.

**Problem.** 1. In a `--pick` window `open_in_new_tabs` and `open_in_new_windows` return at once (src/finder/tab_bar.rs:113-115, 129-131), and the item menu hides the two entries (src/finder/menus.rs:170-174). But `open_elsewhere_items` is added unconditionally to sidebar place rows (src/finder/sidebar.rs:761), tree folders (sidebar.rs:1132) and path crumbs (src/finder/bars.rs:289-294), so those menus offer 'Open in New Tab' and 'Open in New Window' that do nothing. 2. In the Trash the toolbar's Actions menu still lists 'Move to Trash' (src/finder/toolbar.rs:784-789), but `move_to_trash` there calls `put_back` (src/finder/trash.rs:33-36): the item restores the file under the wrong name. The item menu gets this right (menus.rs:134-136, 337-371). 3. Rows without a place return before any menu is attached (sidebar.rs:740-742), so right-clicking the Trash row (sidebar.rs:546-566) or Recents (500-512) shows nothing: no Empty Trash..., no Open in New Tab, although both actions exist and work from anywhere (trash.rs:164-181; `open_in_new_tabs` takes any `Location`). 4. In list mode a Favorites or Locations row has no Get Info or Copy Path (sidebar.rs:756-807). Get Info appears only in tree mode through `tree_items` (sidebar.rs:1086-1095), and Copy Path only on tree folders (1171-1180) and crumbs (bars.rs:296-300).

**Proposal.** 1. Give `open_elsewhere_items` (menus.rs:97-120) a `cx: &App` and return the menu unchanged when `view.read(cx).picker.is_some()`. `PopupMenu::separator` already drops a leading separator (gpui-component popup_menu.rs:682-693). 2. In `actions_menu` (toolbar.rs:670-828), when `finder.in_trash()`, show 'Put Back' with `PutBack`, the Undo2 icon and `.disabled(!can_put_back(..))` as menus.rs:343-348 does, in place of 'Move to Trash'; disable 'Rename...' there. 3. Put a context menu on the Trash row's wrapper (sidebar.rs:564): Open in New Tab, Open in New Window (through `open_elsewhere_items` with `Location::Trash`), a separator, and 'Empty Trash...' with `.action(Box::new(EmptyTrash))`. Give Recents the two open items. 4. Add 'Get Info' (`show_info(vec![path], ..)`) and 'Copy Path' (`copy_path_text`) to place rows when the tree is off. Update docs/manual.md:199-207 and :847.

**Checked plan.** Corrected plan. All of it is cross-platform code in src/finder with no cfg.

1. menus.rs `open_elsewhere_items`: add `cx: &App` and start with `if view.read(cx).picker.is_some() { return menu; }`. Pass `cx` at the four call sites:
- menus.rs:173; its own `picker ||` guard can stay.
- sidebar.rs:761; the closure's `cx` derefs to `&App`, as the `tree_items` call on :763 already relies on.
- sidebar.rs:1132.
- bars.rs:287; change the closure's `_` to `cx`.
In a chooser the remaining menus are never empty: place rows keep Open Nushell Here and Show in File Manager, crumbs keep Copy Path, tree folders keep the rest.

2. toolbar.rs `actions_menu`: bind `let entries = finder.selected_entries();`, `has_selection = !entries.is_empty()`, `in_trash = finder.in_trash()`.
- When `in_trash`, replace the "Move to Trash" item with `PopupMenuItem::new("Put Back").icon(Lucide::Undo2).action(Box::new(PutBack))`.
- CORRECTION: disable it with `!has_selection || !super::trash::can_put_back(&finder.trashed, &entries)`. `can_put_back` is true for an empty slice, so the proposal's condition leaves it enabled with nothing selected.
- Disable "Rename…" with `!has_selection || in_trash`.
- Outside the Trash, keep "Move to Trash" exactly as it is.

3. sidebar.rs `sidebar_row`: clone `target` into `menu_target` before the `on_click` closure moves it. In the `let Some((place, is_favorite)) = place else { .. }` branch:
- `let is_trash = menu_target == Location::Trash;`
- CORRECTION: `if self.picker.is_some() && !is_trash { return row.into_any_element(); }`. In a chooser the Recents menu would be empty, and an empty context menu takes focus without being drawn or dismissed, so Finder-context keys (Return, Escape, Cmd shortcuts) stop working until a click.
- Otherwise return `div().id(if is_trash { "trash-menu" } else { "recents-menu" }).child(row).context_menu(..)`.
- The menu is `open_elsewhere_items(&view, menu, vec![menu_target.clone()], window, cx)`, then for the Trash `.separator().item(PopupMenuItem::new("Empty Trash…").icon(Lucide::Trash).action(Box::new(EmptyTrash)))`, with the real ellipsis as in menus.rs:390. `.action` also shows the shortcut hint.
- Leave the outer `trash_drop_target(div().id("trash-drop")..)` wrapper at sidebar.rs:564 untouched, so the drop target is unchanged.

4. sidebar.rs place menu (756-807):
- Add "Get Info" (`IconName::Info`, `this.show_info(vec![path], window, cx)`) only when `node.is_none()`; `tree_items` already adds it in tree mode.
- CORRECTION: add "Copy Path" (`Lucide::Link`, `this.copy_path_text(std::slice::from_ref(&path), window, cx)`) after the reveal item in BOTH modes. Adding it only when the tree is off would leave tree-mode place rows without it.
- The menu closure is `Fn`, so clone each path inside it, as the existing items do.

5. Docs:
- docs/manual.md:199-207: add Get Info and Copy Path, plus a line for the Recents and Trash row menus.
- :225: drop "and Get Info" from the tree-only sentence.
- :140 and :158: say a `--pick` window leaves out Open in New Tab and Open in New Window.
- :579: add the Trash row's menu to the ways to empty the Trash.
- :847: "Move to Trash (Put Back in the Trash)".
- CHANGELOG.md [Unreleased]: an Added entry for the row menus, and a Fixed section for the Actions label and the chooser's dead items.

Out of scope: F2 still starts a rename in the Trash, and the macOS menu bar's static File › Move to Trash still aliases Put Back.

**Tests.** Headless harness in tests/common/mod.rs. `Ui::open` installs `TrashOverride(FolderBin)`, so nothing touches the real Trash. I did not run cargo; these are designed from reading the code.

tests/controls.rs:

(a) New `a_file_chooser_has_no_new_tab_items`, beside the picker tests at :1361. Open with `Ui::open_with(cx, &fx.dir, Open { picker: Some(PickerOptions { multiple: false }), ..Default::default() })`.
- `ui.update(|f, _, cx| f.add_favorite(projects.clone(), cx))`, find `ix` as `sidebar_rows_and_menus` does, `ui.right_click(("fav", ix))`. Assert `ui.menu_labels()` has neither "Open in New Tab" nor "Open in New Window" and does contain "Open Nushell Here". Close it with `ui.click_menu_item("Copy Path")`.
- `ui.right_click(("crumb", fffinder_core::places::breadcrumbs(&fx.dir).len() - 1))`. Assert `menu_labels() == ["Copy Path"]`, click it, compare `ui.clipboard()` with `fx.dir`.
- `ui.right_click("recents")`, `assert!(!ui.menu_open())`, then `ui.press(&cmd("2"))` and assert `effective_view() == ViewMode::List`. This fails if an empty menu took the focus.
- Never press Escape or Return in a picker window: `picker::cancel` and `choose` call `process::exit`.

(b) New `the_actions_menu_puts_back_in_the_trash`, beside `actions_menu_items` (:150).
- `ui.click("view-list")`, `ui.select("notes.txt")`, `ui.press(&cmd("backspace"))`, `ui.click("trash")`.
- `ui.click("actions")`: labels contain "Put Back" and not "Move to Trash". With nothing selected, `click_menu_item("Put Back")` leaves the file gone; `ui.press("escape")`.
- `ui.select("notes.txt")`; `click_menu_item("Rename…")` leaves `!ui.exists("rename-input")`; `ui.press("escape")`.
- `ui.click("actions")`, `click_menu_item("Put Back")`, `assert!(fx.path("notes.txt").exists())`.
- `ui.go_to(&fx.dir)`, `ui.click("actions")`: labels contain "Move to Trash" again.

(c) Extend `sidebar_rows_and_menus` (:400), before "Remove from Sidebar".
- Right-click the favorite, click "Copy Path", compare `ui.clipboard()` with `projects`.
- Right-click it again, click "Get Info", `assert!(ui.dialog_open())`, `ui.click("info-done")`.
- `ui.right_click("trash")`: labels are ["Open in New Tab", "Open in New Window", "Empty Trash…"]. Click "Empty Trash…", `assert!(ui.dialog_open())`, `ui.click("cancel")`.
- `ui.right_click("recents")`: labels are the two open items.
- Do not click the open items here: a window without a workspace would open a second window.

tests/basics.rs, beside `folders_open_in_new_tabs` (:253), using `with_tabs` and `tabs`:

(d) `ui.right_click("trash")`, `ui.click_menu_item("Open in New Tab")`, assert `tabs(&ui).0 == 2` and that the last tab's `location()` is `Location::Trash` (add `use fffinder::model::Location;`).

Existing tests that must still pass unchanged:
- controls.rs `actions_menu_items` (clicks "Move to Trash" in a folder)
- file_ops.rs:661 `trash_puts_back_and_empties` (drags onto "trash")
- file_ops.rs:1000 (Eject on a location row)
- folder_tree.rs:252, :334 (drag onto "recents") and :356

**Risks.** - Empty context menu: gpui-component hides an empty menu (context_menu.rs:310) but still focuses it (:505) and never dismisses it. Any row whose menu can come out empty must get no `context_menu` at all; Recents in a `--pick` window is the case here. Implemented literally, the proposal would leave the chooser's keyboard dead after a right-click on Recents.
- Picker tests: Escape with nothing selected and Return with a selection call `process::exit`, so a careless test kills the whole test binary.
- Clicking Open in New Tab or Open in New Window in a test window without a workspace opens a real second window; only click them with `Open { workspace: true }`.
- Put Back and Undo would both carry `Lucide::Undo2` in the Actions menu. This is cosmetic; `Lucide::ArchiveRestore` exists in the icon set if a distinct icon is wanted, but then the item menu (menus.rs:345) should change with it.
- "Empty Trash…" on the row is always enabled, like the menu bar item and Cmd+Shift+Backspace, because the Trash's contents aren't known unless the window shows it. The confirmation dialog still guards it.
- If the manual is not updated, :225 will contradict the new list-mode Get Info and :199-207 will be incomplete.
- Low regression risk otherwise: outside a chooser and outside the Trash, every existing menu keeps its current items and order; the only additions are Get Info and Copy Path on place rows.

**Notes from later reviews.** From the implementer of 'history-menus' (on the trunk): `Cut “name”` and similar item-menu labels run out of the menu for long names. Look at it in a picture with a long file name; if it is real, shorten the name in the label (in the middle, as the app does elsewhere) so the label fits, and give it a picture or a test. If you leave it, say so. From the reviewer of 'show-in-enclosing-folder-key' (on the trunk; read its done record's notes for this brief): `actions_menu` (src/finder/toolbar.rs) has a new row, Show in Enclosing Folder, after Copy Path, and `item_menu` (src/finder/menus.rs) offers its item by `finder.enclosing_target().is_some()`; keep both as tests/controls.rs pins them. Optional, only if it is small and certainly right: the Actions menu draws no key beside commands whose keys are bound only in the file views (Quick Look, Rename…, Move to Trash, Show in Enclosing Folder), because a dropdown menu looks keys up from its button's place in the toolbar. On Linux and Windows that menu stands in for the menu bar, so the keys would be worth showing. `PopupMenu::action_context` with the files' focus handle would show them, but it also changes where every row's action is sent and where the focus goes afterwards; if you cannot prove those unchanged with tests, leave it and say so.

## filter-stale-keep-set

**With a Nushell filter on, a new folder's items or a search's results are first shown through the filter's answer for the last rows, usually as an empty list**

Value 3 of 5, size M, search, bug.

**Problem.** Found from two sides by reviewers, by reading, and read again by the orchestrator; nobody has seen it in the window yet. `apply_filter_to` (src/finder/filter.rs) starts a new `nu` run when the rows change (`filter_source != paths`), and goes on applying `self.filter_keep`, the set of paths the last run kept, until the new run answers: `if let Some(keep) = &self.filter_keep { rows.retain(|r| keep.contains(&r.entry.path)); }`. Only `set_filter` (a changed filter text) resets `filter_keep`. So when the rows are those of another place, the new rows are held against what the filter kept of the old place's rows, which is usually none of them: the list is empty for as long as `nu` takes to start and answer, then fills. From the reviewer of 'smart-folder-save-and-leaks' (done record, third item under 'Found, not fixed'): with a filter on, a search's first results can show cut down by a keep-set that wasn't made for them; for a Smart Folder the folder's run is the one `set_filter` starts just before the words are set. From the reviewer of 'results-scroll-reset' (done record): with a Nushell filter on, a search that ends shows the folder's items through the search's filter result until the filter has run again. By the same reading, every step from one folder to another with a filter on does it. What is not known: what the window shows in that time (an empty list, or also an empty-state message such as 'No Items'), and for how long on a real machine. The doc comment on `apply_filter_to` says 'Rows stay as they are until the first run finishes', which is true only of the first run after the filter text changes.

**Proposal.** Reproduce first. The test executor runs a `nu` to its end in one step, so a test has to hold the run back: `AppState.nu` is the path of the program that is run (see how tests set it, and `nushell::filter_paths` in src/nushell.rs for what the program is given and must print), so a stand-in script that waits for a file to appear before answering would do, or a seam that lets a test hold `filter_task`'s answer; find the smallest honest way. Show, with a filter on: (a) folder A to folder B, (b) a folder to a search, (c) a search ending back to its folder, and what the list holds between the rows changing and the run's answer. Then decide what the list shows while a run for new rows is out, and say why. The cases differ: when the rows are the same place's, read again (a quiet reload after a file was added, the index finishing, a sort), the last keep-set is nearly right and keeping it avoids a flash of unfiltered items, which is surely why it is kept; when the rows are another place's or another search's, the last keep-set means nothing. So the keep-set should be dropped, or not applied, when the rows it was made for and the new rows are not the same place's and search's (the place, the words, the scope: find what identifies 'the same list' in the code; `filter_source` holds only paths). What to show instead while waiting: the first run after a filter text change shows the rows unfiltered until it answers, by design; doing the same here is consistent but flashes unfiltered items on every step; showing no rows with the filter bar's busy state (`is_filtering()`) and no empty-state message is the other choice. Look at both in pictures or frames and pick the one that a user stepping through folders with a filter on would rather see; the empty-state message ('No Items', 'This folder is empty') must not show while a run is out, whichever you pick. Selection: a `select` handed to a navigation (Back selects the folder you came from) is applied when the listing arrives; make sure it still lands when the filtered rows arrive later.

**Tests.** Tests that fail today, with the run held back: with a filter on, after going from one folder to another the list never holds the new folder's items cut by the old folder's keep-set (it holds what your design says: all of them, or none with the busy state), and no empty-state message is shown while the run is out; the same from a folder to a search and from a search back; a quiet reload of the same folder (a file added) keeps the filtered list steady, with no flash of unfiltered items; when the run answers, the list is the filtered one and a selection asked for by the navigation is on its item. These need a real or stand-in `nu`: follow tests/controls.rs nushell_filter_and_smart_folders for how tests that need `nu` are written and skipped where it is missing (it is installed here at /usr/local/bin/nu).

**Risks.** The filter runs on every change of rows, so flicker is the risk on both sides: unfiltered items flashing on each step, or the list blanking on each quiet reload. A slow filter (a pipeline that takes seconds) makes whatever is shown meanwhile stay for seconds: it must be honest (not look like an empty folder) and must not let the user act on items the filter will remove if that matters to a command. `set_filter` turns `run_search` on or off when the filter goes on or off during a search ('Turning it on or off changes how many results a search fetches'); a Smart Folder's `enter_smart_folder` and `leave_smart_folder` put filters in and take them out through `replace_filter`: read 'smart-folder-save-and-leaks' before touching the order of things there. 'live-search-results' (in flight in lane d) re-runs searches quietly; its rows are the same search's, so the keep-set should stay applied across them.

**Notes from later reviews.** Read the done records of 'smart-folder-save-and-leaks', 'results-scroll-reset' and 'search-skeleton-flash' first (all on the trunk): they describe the order in which a search's rows, the filter and the placeholders are set up, and each has tests around `apply_filter_to`'s early return for a search that hasn't answered. From the reviewer of 'empty-state-messages' (f60d2e4, on the trunk; read its done record's note for this brief, which also shows how its test holds a filter run back without a stand-in script: `navigate` inside `ui.window`, then `ui.cx.executor().tick()` until `is_filtering()`). That change made one thing worse that is yours to calm: the empty-state message no longer shows while a run is out over items (render_content's message branch has `&& !(self.filtering && self.unfiltered_len > 0)`), so when 'No Items Pass the Filter' is up and the list under it changes (typing on in a search, a folder that gains an item, a quiet rerun from 'live-search-results', another sort) the area goes message, placeholders, message for the length of each `nu` run. When the rows are the same place's and search's, keep it steady, for instance by leaving the message up with its old count until the run answers. Do not widen that guard to plain `!is_filtering()`: an empty folder would flash placeholders.

## go-to-folder-keyboard

**Go to Folder: arrow keys choose a suggestion, Return takes it (or the only match), and suggestions can include indexed folders by name**

Value 3 of 5, size M, ui, ux.

**Problem.** The Go to Folder dialog lists matches under the field, but the keyboard can't choose one. Rows in src/finder/go_to.rs:107-129 only have `.hover` and `.on_click`; the dialog captures a single key, Tab (`IndentInline`, src/finder/dialogs.rs:672-681), which only fills the prefix every match shares (`finish`, go_to.rs:83-90) and does nothing once the matches differ. Only 8 matches are drawn (`SHOWN`, go_to.rs:13-14) and the rest are an unreachable 'and N more' line (go_to.rs:130-137). Return ignores the list entirely: `submit_go_to` (dialogs.rs:715-738) resolves the raw text, so typing `proj` shows `Projects/` (matching is by case-insensitive prefix, crates/fffinder-core/src/listing.rs:257-263) and Return then answers with a toast '.../proj doesn't exist' (dialogs.rs:728-735) while the one match sits right under the field; nothing in the dialog itself marks the error, and the tests show Tab is needed first (tests/controls.rs:1004-1016). Suggestions also come only from reading the one folder the typed path has reached, by prefix (`listing::complete_path`, listing.rs:240-287), so the dialog cannot find a folder by name even though the window holds a live index of every folder. Finder's Go to Folder and every fzf-style picker let Down/Up move through the suggestions and Return accept one.

**Proposal.** In `PathCompletions` (go_to.rs): add `selected: Option<usize>`, cleared in `refresh` when the matches change, and an `error: Option<String>` cleared on the next `InputEvent::Change`. Add `move_selection(delta)` that moves over all matches and draw a window of `SHOWN` rows that follows the selection, so matches past the eighth are reachable. Draw the selected row with the theme's selection background and mark it `.selected(true)` so tests can read it. `finish` (Tab) fills the selected row's text when one is selected, otherwise the shared prefix as now. In `go_to_folder` (dialogs.rs:674), add `capture_action` for `gpui_kit::component::input::MoveDown` and `MoveUp` on the same wrapper div that already captures `IndentInline`, calling `move_selection(±1)` (alternative from the other reviewer: `.capture_key_down` for `down`/`up`, the technique the search field uses at src/finder/toolbar.rs:449-460). Give `submit_go_to` the completions entity, and use it from both `on_ok` and the Go button. Order of resolution: a selected row's text; else the typed path if it exists; else the single match, but only when `matched == input value` (the existing stale guard at go_to.rs:84). When nothing resolves, set `error` and render it as a small danger-coloured line with id `goto-error` under the field instead of the toast. Optional second part (index suggestions): when the typed text has no separator and is not `~`, let `refresh`'s background closure also call `engine.search(&SearchRequest { query, kind: KindFilter::Folder, limit: SHOWN, .. })` (engine from `AppState`), drop folders already listed, and append them under the prefix matches with the tildified parent in muted text; their `text` is the absolute path plus a separator, so choosing one works with the existing `submit_go_to`.

**Checked plan.** Do the keyboard part only; drop the index suggestions.

src/finder/go_to.rs (`PathCompletions`):
1. Add `selected: Option<usize>` and `error: Option<SharedString>`.
2. Add `fn fresh(&self, cx: &App) -> bool { self.matched == self.input.read(cx).value().as_ref() }` and use it in `finish` in place of the inline check at line 84.
3. In `refresh`, clear `selected` and `error` and call `cx.notify()` synchronously, before spawning the read. `InputEvent::Change` arrives as an effect and `set_value` emits none (gpui-base state.rs:1226-1247, hence `fill` calling `refresh` itself), so clearing only when the read lands leaves a stale selection usable.
4. `pub(super) fn move_selection(&mut self, delta: isize, cx)`: no-op when `!fresh` or there are no matches. Down: None -> 0, then clamp at `len - 1`. Up: Some(0) -> None (back to the typed text), no wrap.
5. `finish`: when fresh and a row is selected, `fill(matches[ix].text.clone())`; otherwise the existing `common_completion` path, unchanged.
6. Add `pub(super) fn choice(&self, cx) -> Option<String>` (the selected row's text, only if fresh), `pub(super) fn only_match(&self, cx) -> Option<String>` (only if fresh and `matches.len() == 1`), and `pub(super) fn set_error(...)`.
7. Render: `start = selected.map_or(0, |s| (s + 1).saturating_sub(SHOWN))`, then `.iter().enumerate().skip(start).take(SHOWN)`. Keep the absolute index in the id `("goto-match", ix)` so existing tests using index 0 still hold. Add `.aria_selected(self.selected == Some(ix))` and `.when(is_selected, |d| d.bg(theme.list_active))`. Keep "and N more". Draw the error as `div().id("goto-error").text_size(ui(11.)).text_color(theme.danger).child(msg).test_support()`; `theme.danger` is already used in batch_rename.rs:379.

src/finder/dialogs.rs:
8. On the wrapper div at 674, add two `capture_action` handlers typed with the full path `gpui_kit::component::input::MoveDown` / `MoveUp` (or an `as InputMoveDown` alias), each calling `cx.stop_propagation()` and `move_selection(±1)`. This is the same mechanism that already works for `IndentInline`. `capture_key_down` as in toolbar.rs:449 also works but must check modifiers.
9. Change `submit_go_to(&mut self, input, completions: &Entity<PathCompletions>, window, cx)` and update both callers (on_ok 684-686, Go button 690-699). Resolve in this order: `completions.read(cx).choice(cx)`; else the typed text if `path.exists()`; else `only_match(cx)`. All go through `listing::resolve_typed_path` with `go_to_base()`; row text is `{folder}{name}{sep}`, so it resolves exactly as typed text does. When nothing resolves, call `set_error` and return false in place of `push_notification`.

Docs: update the Go to Folder row at docs/manual.md:500 (Down/Up highlight a match, Return opens it, Return opens a lone match, Tab fills the highlighted match) and add an entry under CHANGELOG.md [Unreleased].

Pitfalls:
- Never read `matches` or `selected` unless `fresh()`.
- Do not put anything but prefix matches in `matches`.
- Existing tests at controls.rs:909-1045 must pass unchanged. In all of them the typed path either exists or has no match, so they are unaffected.

**Tests.** Headless UI tests in tests/controls.rs beside `go_to_folder_dialog` (line 990), using the harness in tests/common/mod.rs: `Fixture::new`, `Ui::open`, `ui.press(&cmd("shift-g"))`, `ui.type_text`, `ui.settle`, `ui.press("down" | "up" | "tab" | "enter")`, `ui.snapshot((..)).selected()`, `ui.exists`, `ui.notification_count`, `ui.dialog_open`, `ui.folder`, and `ui.window(|window, cx| ...)` with `gpui_kit::test::TestWindowExt`. Completions read the disk, not the index, so extra folders can be created with `std::fs::create_dir_all(fx.path(..))`.

1. Lone match: type `proj`, Return. The dialog closes and the folder is `Projects`.
2. Arrows: create `Alpha`, `Alps`, `Alto`; type `al`; press Down twice. `ui.snapshot(("goto-match", 1usize)).selected() == Some(true)` and row 0 is `Some(false)`. Return opens `Alps`.
3. Ends: Up from the first row clears the highlight, and Return then reports the error for `al`. Down at the last row stays there. Typing a letter clears the highlight.
4. Window: create `d01`..`d10`; type `d`; `!ui.exists(("goto-match", 8usize))`. After nine Downs it exists and is selected; Return opens `d09`.
5. Tab with a highlight: type `al`, Down, Down, Tab, settle, Return opens `Alps`. The existing Tab tests at 909 and 1001-1016 cover the unselected case.
6. Missing path: type `does-not-exist`, note `notification_count()`, Return. The dialog stays open, `ui.exists("goto-error")`, and the count is unchanged. Typing a letter removes `goto-error`.
7. Stale guard, new: type `proj`, settle, then in one `ui.window` closure call `window.input("x", cx); window.press("enter", cx);`. The dialog stays open and the folder is unchanged. Repeat with a highlighted row (`al`, Down, then `x` + Return in one closure).
8. Regression: the existing go-to tests at 909-1045, including `tab_waits_for_the_matches_of_what_is_typed_now`, pass unchanged.

I ran nothing (read-only review, no cargo); the plan is derived from reading the code and harness.

**Risks.** - Return on a lone match changes behaviour: a mistyped prefix with exactly one match now navigates instead of erroring. This is benign, since Go to Folder never creates anything, but it must be documented.
- Because of the fresh guard, Return pressed before a slow folder read lands reports "doesn't exist" rather than taking the match; a second Return works. This is preferable to using stale matches or reading the folder on the UI thread.
- Using the app's own `MoveUp`/`MoveDown` types in dialogs.rs fails silently; tests 2-5 catch it.
- Hover and keyboard highlights can show on different rows at once; this is cosmetic.
- The inline error changes the dialog's height and replaces the toast. No test pins the toast.
- Clamp versus wrap is a judgment call. I recommend clamping, with Up from the first row returning to the typed text, so Return on an existing typed path is always reachable.
- Arrow keys during IME composition were not checked. Capturing the bound actions rather than raw key-down is the safer choice.

## icon-tile-hit-target

**Icon view: the blank margins of a tile act as the item, leaving almost nowhere to start a box selection or click to deselect**

Value 3 of 5, size M, ui, ux.

**Problem.** In the icon view every tile is a fixed `tile_w x tile_h` box (`tile_size`: `max(icon + 52, 96)` wide, `icon + 58` tall, src/finder/views.rs:99-101) and ALL of it is the item's hit target: the click/select, drag and stop-propagation handlers are attached to the whole `v_flex().w(tile_w).h(tile_h)` (views.rs:489-498, 527-528, via `item_drag` and `item_interactions`, views.rs:336-351). Tiles are packed edge to edge in a row with no gap (views.rs:455-463) and lines are stacked with no gap, so at the default 64-pt size there are 44 pt of 'blank' space between neighbouring icons that select or drag the item instead of behaving as empty space. `press_in_empty_space`, which deselects and starts the marquee, only receives presses on the grid background (views.rs:423-428), which in a folder that fills the view means just the 18-pt side padding and whatever is left to the right of the last column. The marquee code itself already treats those margins as blank ('A tile's sides are blank margin; the box has to reach its icon', src/finder/marquee.rs:222-223), and the list view was deliberately arranged so a box can start inside a full list (comment at views.rs:928-929). Finder treats only the icon and the label as the item.

**Proposal.** In `render_tile` (views.rs:469-529) split the tile into a passive layout box and an interactive core: keep an outer non-interactive `div().w(tile_w).h(tile_h).items_center()` for layout, and move `.id(("tile", ..))`, the cut opacity, `item_drag`, the left/right/middle mouse handlers, `on_click`, the folder drop target / spring-loading and the context-menu wrapper onto an inner `v_flex().items_center()` that shrink-wraps the icon box and the label (so its bounds are the icon's padded square plus the label's width). Presses in the surrounding margin then bubble to the grid's existing `press_in_empty_space` handler (deselect + marquee) and right-clicks there open the background menu. No change is needed in marquee.rs: `tiles_in_box` already tests against the icon area.

**Checked plan.** Corrected plan: two hit parts per tile (icon square and name); the cell stays as it is for layout, tests and drops.

1. `render_tile` (src/finder/views.rs:469-529).
   - Keep the outer `v_flex().id(("tile", id)).w(tile_w).h(tile_h).items_center().pt(4)` as a passive cell with no mouse-down listener.
   - The cell keeps the `tile` id and `.test_support()`, so every existing test that clicks, hovers or measures `item_id("tile", ..)` stays valid (the cell's centre is on the icon).
   - The cell also keeps the cut opacity, the cut-off-name tooltip (`with_cut_off_text(cell, ..)`, fit.rs:31), and for folders the drop target and spring-loading, so drops stay as forgiving as today.
   - Add two interactive children. Each is built as: Left stop_propagation (added first) -> `self.item_drag(part, ix)` -> `self.item_interactions(.., wrapper, cx)`.
     - Icon square: `div().id(("tile-icon", id)).p(ICON_PADDING/2).rounded(6).when(selected, bg tile_selected).child(file_icon(..))`.
     - Name: `div().id(("tile-name", id)).pt(4).child(<the existing label div>)`, built inside the `wrap` closure of `with_cut_off_text`.
   - Drop `gap(4)` from the cell. The 4 pt under the icon belongs to the name part, so there is no dead strip between icon and name.
   - While renaming, the field replaces the name part as now: `div().mt(4).w(tile_w-8).child(field)`.

2. `item_interactions` (views.rs:315-375).
   - Let the caller name the menu wrapper (take a `Stateful<Div>` or an id kind). Two wrappers in one cell cannot both be `("item", id)`.
   - Move `spring_loaded` + `folder_drop_target` (:324-327) into a small `folder_target(el, ix, cx)`. Call it on the cell for tiles, and on the same element as before in `render_list_row` (:976) and `render_strip_tile` (:1314), which otherwise stay unchanged.
   - Middle-click stays with the presses. Leave the gallery strip's full-tile target alone; it has no empty space or box.

3. `tiles_in_box` (src/finder/marquee.rs:212-242): apply the icon test up and down too.
   - Tiles sit at the bottom of their line (line 0 has 9 pt above), so `icon_top = end - ui(tile_h) + ui(4)`.
   - Require `icon_top <= bottom && icon_top + ui(icon_size + ICON_PADDING) >= top`.
   - Share the 4 as a const in views.rs beside ICON_PADDING.
   - tests/basics.rs:171-194 still passes: its box ends at the cell centre, inside the icon.

4. Docs: docs/manual.md:276 and the Empty-space menu section (around 820); CHANGELOG.md [Unreleased].

Pitfalls:
- Bubble runs listeners in reverse registration order, and stop_propagation also silences the element's own listeners that would run after it (gpui-pre window.rs:6002-6008). Keep the stop listener the first one added, as at views.rs:527.
- The item-menu wrappers must be the parts, not the cell. ContextMenu stops propagation on a right press (gpui-component-0.8.1/src/menu/context_menu.rs:404), so a cell-wide wrapper would swallow the background menu.
- Checking the press position inside the existing handlers is not enough. GPUI's `on_drag` has no position filter (div.rs:3016-3027), so the handlers have to live on elements whose bounds are the item.
- `item_drag` (:279-310) clones the whole selection for every selected tile it is called on. With two parts per tile, build the payload once and clone it.
- `cx.theme()` borrows cx. Copy `muted_foreground` out before the closure that calls `item_interactions(.., cx)`.

Fallback if two parts prove troublesome: the proposal's single inner wrapper (id such as `tile-item`), still with the cell keeping `tile`, drops on the cell, and step 3. It only frees the flanks for short names.

**Tests.** Add to tests/basics.rs beside `a_box_selects_tiles_in_the_icon_view` (:171), using the headless harness in tests/common/mod.rs.

Helpers: `Fixture::new`, `Ui::open`, `ui.click("view-icons")`, `item_id`, `ui.snapshot(id).bounds()`, `ui.click`, `ui.double_click`, `ui.right_click`, `ui.click_point`, `ui.drag_points`, `ui.drag`, `ui.right_click_point`, `ui.menu_labels`, `ui.select`, `ui.selected`, `ui.files_focused`. The fixture shows Budget.csv, notes.txt, Projects, Roadmap.md in one row.

1. Layout pins. For notes.txt take `cell = tile`, `icon = tile-icon`, `name = tile-name`. Assert the icon is 72 x 72, `icon.top() - cell.top() == 4`, `icon.center().x` equals `cell.center().x` within 1 px, `name.top() == icon.bottom()`, and `name.size.width <= 108`.
2. The parts are the item. `ui.click(tile-icon)` and `ui.click(tile-name)` each select notes.txt. `ui.double_click(tile-name of Projects)` opens it.
3. The margin is empty space. `ui.select("notes.txt")`, then `beside = point(icon.left() - px(8.), icon.center().y)` (still inside the cell). `ui.click_point(beside)` gives `ui.selected().is_empty()` and `ui.files_focused()`.
4. A drag from the margin draws a box and moves nothing. `ui.drag_points(beside, <centre of Projects' tile-icon>, Modifiers::none())` gives `ui.selected() == ["notes.txt", "Projects"]`, `fx.path("notes.txt").exists()` and `!fx.path("Projects/notes.txt").exists()`. Today this drag moves the file.
5. Marquee up and down. `under = point(icon.center().x, cell.bottom() - px(6.))`; `ui.drag_points(under, under + point(px(30.), px(0.)), ..)` selects nothing. This fails without the `tiles_in_box` change.
6. Menus. `ui.select("notes.txt"); ui.right_click_point(beside)`: `menu_labels()` has "Select All" and not "Move to Trash"; then `ui.press("escape")`. `ui.right_click(tile-name)` shows "Move to Trash".
7. Drags still work from either part. `ui.drag(tile-icon of notes.txt, tile of Projects, ..)` leaves `Projects/notes.txt`. `ui.drag(tile-name of Budget.csv, tile of Projects, ..)` likewise. A drop in the folder cell's margin, `ui.drag_points(<icon centre>, point(projects_cell.left() + px(6.), projects_cell.center().y), ..)`, still lands in Projects.

Existing tests that must stay green unchanged: tests/basics.rs:171-194, tests/columns.rs:191-221 (tooltip anywhere on the tile, none while renaming), tests/controls.rs:47, 525, 1703.

**Risks.** - Smaller click targets: a near miss beside an icon now clears the selection instead of selecting the item, as in Finder. Cmd or Shift held keeps the selection.
- A box that touches only a name no longer selects; the icon rule now applies both ways. The manual's "everything the box touches" wording needs the nuance.
- Each part gets a wrapper div, so name wrapping or centring could shift. Test 1 pins the bounds, but the harness has no pixel check.
- The `item_interactions` signature change touches list rows and the gallery strip, and may conflict with other lanes editing views.rs.
- `item_drag` runs twice per tile unless the payload is shared, doubling an existing per-frame clone of the selection.
- All of this comes from reading fffinder and GPUI sources; nothing was run.

**Notes from later reviews.** 'select-all-perf' may land before this one and changes how a tile's drag is built (item_drag in src/finder/views.rs); build on it. Clicking and double-clicking an icon or its name, dragging items, dropping onto a folder tile, and the box selection must all keep working: test each.

## idle-redraw-after-search

**After the search field has had the keyboard once, the window redraws about twice a second for good**

Value 3 of 5, size M, ui, bug.

**Problem.** Found by the reviewer of 'live-search-results' while measuring that change's idle cost (done record: /home/user/polish-tools/done/live-search-results.json, first item under 'Found, not fixed'). In their words: once the search field has had the keyboard, the window is drawn about twice a second for good, with nothing happening: 31 calls of render_content in 16 s under gdb with no search, no reload and no row rebuilt, after Ctrl+F and then Escape in a window that showed a folder. Idle CPU was 6 ticks in 15 s in a fresh window and 109 after (debug build, software rendering; 100 ticks = 1 s of CPU). It looks like the field's caret going on blinking after the keyboard has left. It is on the trunk without that change. Nobody has found the cause yet. What the orchestrator read in the kit, as a lead and no more: the caret's timer is `BlinkCursor` (gpui-base-0.8.1/src/input/base/blink_cursor.rs), which notifies the input every 500 ms while it runs, and the input notifies the view it is in. It is started by `InputState::focus` (state.rs, near line 1653), by the focus observer `on_focus`, and by window activation while the field is focused; it is stopped only by `on_blur`, and `on_blur` returns before stopping it while the field's context menu is open. `pause` is a no-op on a stopped cursor. So a start with no blur after it would blink for good: for instance `InputState::focus` called and the focus then given to something else in the same update (the focus never arrives, so no blur is sent), or a blur taken while a context menu was open. A file browser is left open all day; one search should not keep it awake from then on.

**Proposal.** Reproduce first, headlessly: the test executor has a clock, so count how often the window is drawn (or how often the Finder or the search field's state is notified) while nothing happens, for a fresh window and for a window after Cmd+F and Escape, over ten simulated seconds. tests/finder.rs a_search_run_again_for_the_index_shows_no_sign_of_running ('live-search-results') counts redraws for its own purpose; see how. Then find the cause before fixing anything: which entity notifies every 500 ms, who started it, and why nothing stopped it. Try each way the keyboard leaves the search field: Escape with words and without, Return or Down into the results, a click on a file, a click on the sidebar, Tab, Cmd+L or another field taking the keyboard, a dialog opening, a Smart Folder clicked, a tab switched, the window deactivated and activated. Do the same for the app's other fields if the cause is general (the Nushell filter field, the Nushell panel's field, a rename field, Go to Folder's field, a dialog's field). Then find where it began: check the base commit 66c919b (upstream's) the same way. If upstream has it, it is theirs and we fix it in the app; if it began on this branch, find the commit (`git log 66c919b..search-ui-polish`; 'escape-ends-search', 'scope-chips-keep-focus', 'search-enter-before-results', 'quick-look-window-mixup' and 'back-forward-restore-search' all move the keyboard around the search field) and say which. Fix it in the app's code: the kit is a dependency and is not ours to patch. If the cause is the app starting the field's caret without the focus arriving (focusing the field and then something else in one update), the fix is in the order or the choice of those calls. If the only cause is inside the kit and no call the app makes can stop the caret, say so with the evidence, write the closed record, and return skipped: do not ship a workaround that blurs or rebuilds fields on a timer. A second, smaller thing from the same record, to do only if it is small and certainly right, else leave and say so: while a search shows, the folder it was made in is still looked at, and each change in it reads the folder again and rebuilds the rows twice (load_listing, then load_folder_stats), which draws the window, 12 reloads and 24 rebuilds for 12 files made and removed there, though the results don't change by it.

**Tests.** A headless test that fails today: after the search field has had the keyboard and lost it (at least Cmd+F then Escape, and Cmd+F, words, Return into the results), ten simulated seconds with nothing happening draw the window no more often than in a fresh window (state the bound you assert and why: the two-second look for changes and the index status poll may notify legitimately; measure a fresh window first). The same for each other way out of the field that you found to leak, and for each other field that leaked. A guard that the caret still blinks while the field has the keyboard (the window is redrawn about twice a second then, by design).

**Risks.** The fix must not cost the field its caret, its selection, or the keyboard: every test that types into the search field and every focus test ('scope-chips-keep-focus', 'escape-ends-search', 'quick-look-window-mixup', 'focus-lost-recovery' if on the trunk) must keep passing. A test that counts redraws is brittle if it counts legitimate ones; assert against a fresh window's count measured in the same test, not against zero. On a real display the measure is CPU, not frames: if you can, run the debug app under Xvfb as the reviewer of 'live-search-results' did (their notes say how they read rows and sent keys) and report ticks before and after.

**Notes from later reviews.** 'live-search-results' (153eb8f) is on the trunk: every tab's browser looks for changes every two seconds whether it shows or not, and that look costs two atomic loads when nothing changed and should not draw anything. If your counting shows that it does draw, that is a finding of its own: say so.

## image-preview-details

**Image previews: show pixel dimensions, stop enlarging small images, and show a search hit's folder in the caption**

Value 3 of 5, size M, ui, feature.

**Problem.** The large image preview is `img(entry.path.clone()).size_full().object_fit(ObjectFit::Contain)` (src/finder/preview.rs:905-907). `Contain` always scales to the box, up as well as down (gpui-pre-0.3.10/src/style.rs:55-74), so a 16x16 favicon or a 64-pixel icon fills the whole gallery or Quick Look window as a blur; `ObjectFit::ScaleDown` exists (style.rs:76-98) but is not used. No surface shows an image's pixel size: `load_preview` returns a bare `Preview::Image` (preview.rs:244), the caption's `info_line` lists kind, pages, duration, size and date (preview.rs:840-861), the card lists Kind, Size, Duration, Modified and Where (preview.rs:814-836), and the reviewer's grep for 'dimension' across src and crates/fffinder-core found nothing relevant. The surfaces also disagree on location: the card has a 'Where' row (preview.rs:833-836) but the gallery and Quick Look caption does not, so Quick Look on a This Computer search hit shows only name, kind, size and date. The browser's path bar would show the path, but Quick Look's window covers 80% by 85% of the browser, centred (src/finder/quick_look.rs:255-260).

**Proposal.** preview.rs: 1) Change `Preview::Image` to `Preview::Image { dimensions: Option<(u32, u32)> }`, filled in `load_preview` (already on a background thread) with `image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_dimensions().ok()`, which reads only the header; update the matches at preview.rs:46, 244 and 926. 2) `info_line`: push '4032 × 3024' after the kind when known; `render_preview_card`: add a 'Dimensions' row after Size. 3) In the `is_image` arm (preview.rs:899-919), when dimensions are known and the file is not SVG, add `.max_w(px(w as f32)).max_h(px(h as f32))` to the `img` and give it `id("preview-image")` with `.test_support()`, so it is never drawn larger than its pixels and still shrinks to fit; SVG keeps `Contain`. 4) `info_line`: when `finder.shows_parent_column()` (src/finder/mod.rs:772-774: searches, Recents, sent results), append the tildified parent folder so the gallery and Quick Look captions say where a hit lives, as the card does. Update the Previews table in docs/manual.md.

**Checked plan.** Corrected plan. All code changes are in src/finder/preview.rs unless noted.

1) Stop enlarging with `ObjectFit::ScaleDown`, chosen from the file name. Do not use the proposal's `max_w/max_h` clamp or put an id on the img.
- Add `fn raster_format(entry: &FileEntry) -> Option<image::ImageFormat> { image::ImageFormat::from_path(&entry.path).ok() }`. It is extension-only and case-insensitive: Some for png, jpg, jpeg, gif, webp, bmp, tif, tiff, ico; None for svg, heic, heif, psd, icns, raw, cr2, nef.
- Add `fn image_fit(entry) -> ObjectFit`: ScaleDown when `raster_format` is Some, else Contain. Use it at preview.rs:907: `.object_fit(image_fit(entry))`.
- SVG keeps Contain because GPUI rasterises it at 2x nominal size. RAW keeps today's behaviour.

Why not the proposal's clamp:
- An id on Img turns on its per-element frame state, so animated GIF and WebP previews start playing and request a frame every refresh (gpui-pre-0.3.10/src/elements/img.rs:289-346, 382-388). Today they are static. In --term every frame would go to the terminal.
- The clamp waits for the async Preview. The icon view and gallery strip load the same `img(path)` asset (views.rs:129-135), so a cached image would be drawn enlarged until the dimensions arrive, then snap small.
- GPUI turns Exif-rotated photos upright before drawing (gpui-pre-0.3.10/src/platform.rs:3338-3342), so a stored-size clamp mis-sizes small rotated images.
- A NEF would be clamped to its thumbnail size.
ScaleDown uses the decoded size, so none of these arise. `paint_image` snaps to device pixels (window.rs:5108), so 1:1 images stay sharp.

2) Dimensions.
- Change the variant to `Preview::Image { dimensions: Option<(u32, u32)> }`. Update line 46, line 244 (`Preview::Image { dimensions: image_dimensions(entry) }`) and line 926 (`Preview::Image { .. } | Preview::Unavailable`).
- Add `fn image_dimensions(entry) -> Option<(u32, u32)>`:
  - `raster_format(entry)?;` first, so a RAW file's TIFF header is never probed.
  - `let mut d = image::ImageReader::open(&entry.path).ok()?.with_guessed_format().ok()?.into_decoder().ok()?;`
  - `Some(upright(d.dimensions(), d.orientation().unwrap_or(Orientation::NoTransforms)))`
  - Needs `use image::ImageDecoder as _;` and `use image::metadata::Orientation;`.
- Add `fn upright((w, h), o)` that swaps for Rotate90, Rotate270, Rotate90FlipH and Rotate270FlipH. This mirrors what GPUI does before drawing, so the text matches the picture.
- It runs inside `load_preview`, already on the background executor (preview.rs:322-326).
- Decoders come from gpui-pre's `image` features through Cargo unification (fffinder's own dep is `default-features = false`); no manifest change is needed.
- `info_line`: push `format!("{w} × {h}")` right after the kind.
- `render_preview_card`: add `row("Dimensions", ..)` after Size. Check the label fits the 70pt label column (preview.rs:779); widen it if it wraps.
- Read the cache by reference (`finder.previews.get(&entry.path)`) in one match. `info_line` already clones the whole Preview once per frame for Media; do not add a second clone.

3) Folder, in Quick Look only.
- Give `preview_caption` and `info_line` a `surface: Surface` parameter. Callers: preview.rs:711 passes its own `surface`; src/finder/quick_look.rs:185 passes `Surface::QuickLook`.
- Append `entry.parent().map(super::tildify)` as the last part when `surface == Surface::QuickLook && finder.shows_parent_column() && !finder.in_trash()`.
- The gallery keeps relying on the path bar. The --term dialog gets the folder through `render_preview_large(.., Surface::QuickLook, ..)`.

4) Make the caption readable by tests.
- In `preview_caption`, build `let info = SharedString::from(info_line(..))` and render it as `div().id("preview-info").aria_label(info.clone()).text_color(muted).child(info).test_support()`.
- This is the status-text pattern (bars.rs:391-396). `TestSupportExt` already comes in with `gpui_kit::*`. Layout is unchanged.

5) Docs.
- docs/manual.md:390: images are shown at up to their own size, SVGs fill, and the caption and card give dimensions.
- docs/manual.md:134: the preview pane lists dimensions.
- docs/manual.md:424: Quick Look's bar gives dimensions, and the folder for search, Recents and sent results.
- CHANGELOG.md `[Unreleased]`: one Added entry and one Changed entry.

**Tests.** Unit tests in src/finder/preview.rs `mod tests`. Use explicit imports, not `super::*`, as the module comment says.
- Extend `previews_by_kind`:
  - `image::RgbaImage::new(40, 20).save(dir.join("tiny.png"))`, then assert `matches!(load(&entry), Preview::Image { dimensions: Some((40, 20)) })`.
  - Save the same image with `save_with_format(dir.join("shot.nef"), image::ImageFormat::Png)` and assert `Preview::Image { dimensions: None }`. This pins the RAW guard.
- `upright((40, 20), Orientation::Rotate90) == (20, 40)`; unchanged for NoTransforms and Rotate180.
- Fit geometry through GPUI's public `ObjectFit::get_bounds`, which is exactly what Img::paint calls. With an 800x600 area:
  - `image_fit(&png).get_bounds(area, size(DevicePixels(40), DevicePixels(20))).size == size(px(40.), px(20.))`
  - a 1600x600 image gives 800x300
  - `image_fit(&svg)` with a 40x20 image gives 800x400
- The drawn size cannot be asserted in the headless harness: it sees layout bounds only, and GPUI's test-support exposes painted quads and underlines but not image sprites (gpui-pre-0.3.10/src/window.rs:2897-2906).

Headless UI test in tests/finder.rs, next to `quick_look_footer_wraps_instead_of_cutting_the_caption` (368-391).
- Setup: `Fixture::new`, then write the 40x20 PNG to `fx.path("Projects").join("atlas").join("tiny.png")` before `Ui::open`.
- Gallery: `ui.go_to(..)`, `ui.click("view-gallery")`, `ui.select("tiny.png")`. Assert `ui.snapshot("preview-info").label()` contains "40 × 20" and does not contain "atlas".
- Quick Look: `ui.update(|f, window, cx| f.show_results("Pictures".into(), vec![png], window, cx))` (or `f.set_query("tiny".into(), true, cx)`), `ui.select("tiny.png")`, `ui.press("space")`, then `ui.in_quick_look(|ui| ..)`.
- Assert the label contains "40 × 20" and `format!("Projects{}atlas", std::path::MAIN_SEPARATOR)`. Do not use a literal "Projects/atlas"; it fails on Windows.

Existing tests are unaffected: the footer test uses notes.txt in a plain folder, and nothing else matches on `Preview::Image`.

**Risks.** - Small raster images now show at their own size on every surface, including the 240pt card and the preview pane. There is no zoom to get the enlargement back, which icon and pixel-art users may miss.
- The JPEG probe reads the whole file once more on the background thread, without decoding. GPUI reads and decodes it anyway, so the cost is small.
- The caption gives the upright size, which differs from the stored size for Exif-rotated photos.
- A folder in Quick Look's caption makes the footer wrap to two lines sooner for search hits.
- The "Dimensions" label is tight in the card's 70pt label column on wide fonts.
- Decoder coverage depends on gpui-pre's `image` features being unified into fffinder's build.
- The `Preview` enum and the `preview_caption`/`info_line` signatures change, so other candidates editing the same functions will conflict.
- docs/images/gallery-dark.png and quicklook-dark.png are generated on macOS and will be slightly stale.
- Not addressed: NEF previews stay blurry enlargements of the embedded thumbnail, and the icon-view tiles still upscale small images (views.rs:131-135).

**Notes from later reviews.** Other preview changes ('preview-card-height', 'stale-previews-and-cancel', 'thumbnail-memory') may land before this one; build on them. Image orientation must not regress.

## inactive-selection-colour

**Icon and gallery views always draw the selection in the active colour, so the focused pane can't be told apart**

Value 3 of 5, size S, ui, ux.

**Problem.** The list and column views dim the selection when the files don't have the keyboard: `active = self.files_focused(window)` picks `pal.selection` vs `pal.selection_inactive` and the text colour (src/finder/views.rs:547, 863-867, 921-927 for the list; 988, 1086-1091, 1117-1123 for columns). The icon view, the default view, never does: `let focused_selection = selected;` (views.rs:481) and the label is always `pal.selection` / `pal.selection_text` (views.rs:503, 518-520); the gallery strip is the same (views.rs:1298, 1308-1310). In split view the two panes are separated only by a 1-px line with no active-pane marker (src/workspace.rs:291-317), and F5/F6 copy or move the FOCUSED pane's selection to the other pane (src/actions.rs:153, 221): with both panes in icon view each showing a full-strength blue selection, there is no cue which pane will act. The same applies when the keyboard is in the search field, the sidebar tree or the Nushell panel. In addition `files_focused` is `files_focus.is_focused(window)` (src/finder/mod.rs:1636-1638), which stays true when the whole window is in the background, so even the list keeps the accent colour in an inactive window, unlike Finder.

**Proposal.** In views.rs: compute `let active = this.files_focused(window)` inside the `v_virtual_list` / `h_virtual_list` closures (they already receive the window; views.rs:412, 1236) and pass it to `render_icon_line` -> `render_tile` and to `render_strip_tile`. Replace `focused_selection` with `selected && active`; for a selected-but-inactive label use `pal.selection_inactive` with `theme.foreground`, exactly as `render_list_row` does, and keep the name's LS_COLORS style suppressed whenever selected. Put the choice in one small pure helper (e.g. `fn selection_colors(active: bool, pal: &Palette, theme: &Theme) -> (Hsla, Hsla)`) used by all four views so they can't drift again. Second, smaller step: add `fn selection_active(&self, window: &Window) -> bool { self.files_focused(window) && window.is_window_active() }` for the colour only (not for `files_focused`, which tests and logic rely on) and add an unconditional `cx.notify()` to the `observe_window_activation` callback (mod.rs:522-528) so the colour updates when the window loses or gains focus.

**Checked plan.** Do step 1 only, all in `src/finder/views.rs`. Drop the window-activation step from this change.

1. **Helper.** Add a private pure function near `name_text` (views.rs:228): `fn selection_colors(active: bool, pal: &Palette, foreground: Hsla) -> (Hsla, Hsla)`. It returns `(pal.selection, pal.selection_text)` when active and `(pal.selection_inactive, foreground)` otherwise. `Palette` is already imported (views.rs:9). Taking `foreground: Hsla` avoids naming the kit's `Theme` type.

2. **Icon view.**
   - In `render_icons` (views.rs:379), rename `_window` to `window` and compute `let active = self.files_focused(window);` in the body, as `render_list` (547) and `render_columns` (988) do. Computing it inside the virtual-list closure is not needed.
   - The closure at views.rs:412 is already `move`; capture `active` and pass it through `render_icon_line` (437) to `render_tile` (469).
   - Delete `focused_selection` (481). Replace 518-520 with `.when(selected, |this| { let (bg, fg) = selection_colors(active, &pal, theme.foreground); this.bg(bg).text_color(fg) })`.
   - Leave alone: the `pal.tile_selected` backdrop (503), the `!selected` LS_COLORS filter (485, matching manual.md:693), and the hidden-file rule (521).

3. **Gallery.**
   - In `render_gallery` (1217), rename `_window` to `window` and compute `active`.
   - The strip closure at 1236 is not `move`; add `move` (`active` is `Copy`) and pass it to `render_strip_tile(ix, active, cx)`.
   - Replace 1308-1310 the same way, using `cx.theme().foreground`.

4. **List and columns (optional).** Route the background pick in `render_list_row` (921-927) and `render_column_row` (1117-1123) through the helper. Do not change the columns' `current = col == active_column && active` rule or their LS_COLORS filter (1099).

5. **Docs.** Add a Changed entry under `[Unreleased]` in `CHANGELOG.md`. Add one sentence to the Split view paragraph at `docs/manual.md:160`: the pane with the keyboard shows its selection in the accent colour, the other in grey.

Why not the window-activation step here:
- Quick Look opens its own window, which deactivates the browser while arrow keys still drive its selection. An exemption for `quick_look_window.is_some()` would be needed.
- `examples/screenshot.rs` would need `window.activate_window()` or the README images regress.
- Every test window starts inactive.

If it is done later, use `files_focused(window) && (window.is_window_active() || self.quick_look_window.is_some())` for colour only, with no `cx.notify()` and no term-mode special case.

Pitfall: both panes start at the same folder, so `("tile", path_id)` ids are duplicated across panes.

**Tests.** Colours are not observable in the headless harness, so the test is split in two.

**1. Unit test in `src/finder/views.rs` `mod tests`**, beside `drag_frames_set_a_readable_text_color_in_both_themes` (views.rs:1367), with the same setup: `gpui_kit::init(cx); crate::theme::install(cx);` then `crate::theme::apply(appearance, None, cx)` for Light and Dark. With `let pal = crate::theme::palette(cx); let fg = cx.theme().foreground;` assert:
- `selection_colors(true, &pal, fg) == (pal.selection, pal.selection_text)`
- `selection_colors(false, &pal, fg) == (pal.selection_inactive, fg)`
- the two backgrounds differ
- `(fg.l - pal.selection_inactive.l).abs() > 0.5` (light is about 0.86, dark about 0.72)

**2. Headless test in `tests/settings.rs`**, which already has `with_tabs` (line 12) and reads `split_pane()` (170, 299). It pins the predicate that drives the colour:
- Open with `with_tabs` (`Open { workspace: true }`); the default view is Icons.
- `ui.select("notes.txt")`, then `ui.press(&cmd("\\"))`.
- Get `pane` from `ui.workspace` via `w.split_pane().cloned()`.
- Assert `!ui.files_focused()` and `ui.window(|w, cx| pane.read(cx).files_focused(w))`.
- Navigate the split pane to `Projects` as `tests/file_ops.rs:1097-1104` does, so tile ids are unique.
- `ui.click(item_id("tile", &fx.path("notes.txt")))`; assert `ui.files_focused()` is true and the pane's is false.
- `ui.click("search")`; assert both are false.

No new accessor is needed: `Finder::files_focused` is already `pub` and `Ui::files_focused` exists (`tests/common/mod.rs:710`).

The headless test covers focus, not paint. The wiring from predicate to tile is covered only by the unit test on the shared helper plus review.

`ui.activate()` / `ui.deactivate()` (`tests/common/mod.rs:553-562`) are only needed if the window-activation step is ever added.

**Risks.** - **More visible greying in the default view.** The icon-view label will go grey whenever focus is not exactly `files_focus`. That happens while a context menu is open (the popup takes focus and restores it on dismiss), while a dialog is open, during inline rename, and after a click on the toolbar, status bar or sidebar list, which focuses the root `self.focus` (gpui `div.rs:2826-2838`). The list and columns already do this, and it is honest because the arrow keys are dead in those states. It will be noticed more because Icons is the default.
- **Gallery without focus.** A gallery that auto-selects its first item without the files focused shows a grey strip label. This only affects the screenshot tool when `--view gallery` is used without `--select`; the current scripts pass `--select 1`.
- **Window-activation step, if someone adds it anyway.** It greys the selection under the Quick Look window, greys the README screenshots (headless windows are never active), and reads inactive in every test that does not call `ui.activate()`.
- **Existing tests.** None are affected: nothing asserts colours, and `files_focused` semantics are unchanged.

## multi-word-highlights

**Multi-word and quoted searches highlight nothing or the wrong letters**

Value 3 of 5, size S, search, bug.

**Problem.** `Engine::search` builds one needle from all fuzzy words (`let needle = fuzzy_text(&req.query)`, crates/fffinder-core/src/engine.rs:589-593) and `highlight()` strips the spaces and matches the concatenation as a single in-order subsequence of the file name, returning nothing unless every letter is found (engine.rs:1690-1725, bail-out at 1720-1723). Matching itself treats each word independently and against the whole path (`strict`, engine.rs:570-577; docs/manual.md:356). So the commonest multi-word cases show no highlight at all: `src main` on `main.rs` (needle `srcmain` is not in the name), `tax report` on `report-tax-2024.pdf` and `atlas project` on `Project Atlas.txt` (word order differs), `docs readme` on docs/readme.md although `readme` is the whole stem, `budget 2024` where 2024 is the folder. Quoted phrases (the new Unreleased feature) go through the same path: `split_quoted` hands the phrase's words back as fuzzy text (engine.rs:1575-1599), so the phrase `tax exemption` on `The tax exemption list.pdf` greedily highlights the `T` of `The`, then `ax`, then `exemption`, not the phrase that was required letters-in-a-row (`contains_phrase`, 1617-1622); a phrase such as `[draft]` or `notes: v2`, whose tokens fuzzy_text drops (1549-1561), gets no highlight on the names it was required to match. Highlights are also list-only: the icon and gallery tiles pass `&[]` (src/finder/views.rs:483-488, 1281-1286), and a word that matched in a folder name is marked nowhere because the Folder cell is plain text (`Row.detail`, src/finder/mod.rs:102-104, 114-115; views.rs:838-846, 958-972).

**Proposal.** In crates/fffinder-core/src/engine.rs change `highlight` to take the words and phrases separately, e.g. `highlight(name, words: &[String], phrases: &[String]) -> Vec<(usize, usize)>`: for each quoted phrase take its case-insensitive substring position; for each word take a contiguous case-insensitive occurrence, else the greedy in-order subsequence if the whole word is found within the name, else nothing (the word matched a folder); sort the ranges and merge overlapping or touching ones, because `styled_name` skips any range that starts before the previous one ended (views.rs:255-257). Keep the existing `lower.len() == name.len()` guard (1702). Thread `words` (already computed at 548-551) and `exact` (525) into `RawHit::into_hit` (1376, 1403) instead of `needle`, and pass empty slices from recents (847). Optional follow-ons: (a) in src/finder/views.rs pass `&row.highlights` with `pal.highlight` for unselected tiles in `render_tile` and `render_strip_tile`; (b) add `detail_highlights` to `Row` (shared with the content-match-highlight candidate), compute it in `run_search`'s background closure by running the same word matcher on the Folder text with the words that did not match in the name, and when the scope is the folder build `detail` relative to the scope folder (starting at its name) instead of repeating the whole `~/...` prefix.

**Checked plan.** Keep the core idea (per-word and per-phrase ranges, sorted and merged), with four corrections. All of it is in crates/fffinder-core/src/engine.rs unless noted.

**Corrections to the proposal**

1. **Phrases must follow the letter-case setting.** The proposal says "case-insensitive substring", but `contains_phrase` (1617-1622) matches exactly when `ignores_case(case, phrase)` is false. A case-insensitive find for `"Tax"` on `tax Tax.pdf` would mark (0,3), not the (4,7) that satisfied the filter. Add a `phrase_range(name, phrase, case) -> Option<(usize, usize)>` beside `contains_phrase`: `name.find(phrase)` when case matters, otherwise a case-folded search.

2. **Skip words that came from a located phrase.** `split_quoted` leaves each phrase's words in the query (1587-1589), so `words` (548-551) contains them too. Without a skip, `"tax exemption"` on `tax notes - tax exemption.pdf` also marks the first `tax`. Skip any word that is a substring of a lowercased phrase whose range was found. This needs no change to `split_quoted`, so the `quoted_phrases_come_out_of_the_query` test is untouched.

3. **Restrain the scattered-letter fallback.** "Else nothing (the word matched a folder)" does not hold as proposed. `test main` on `tests/integration_main_set.rs` would mark `te`, `s`, `t` scattered through the name, although `test` matched the `tests` folder. Per-word rule:
   - contiguous in the name: mark it (first occurrence, as `lower.find` does today);
   - else contiguous in the scope-relative parent path, lowercased: mark nothing;
   - else a full in-order subsequence of the name: mark the greedy ranges;
   - else nothing (typo-only or cross-folder match, as today).
   Compute the parent text in `into_hit` from `entry.path.parent()` stripped of `req.scope`, the same way `strict` builds `shown` (569-575).

4. **Keep offsets on char boundaries.** The `lower.len() == name.len()` guard (1702) only compares totals. Safer: do the contiguous search over `name.char_indices()`, comparing `c.to_lowercase()` with the word's chars as the greedy loop already does (1709-1718). Offsets then always index the original name and the guard can go. `styled_name` hands the ranges to `StyledText`, so an off-boundary range risks a panic.

**Shape**

- Replace `needle` with a small struct built once in `search`, e.g. `Marks { words: &[String], phrases: &[String], case: SearchCase, base: Option<&Path> }`, with a `Default`/empty value for `recents` (847).
- `into_hit(self, marks: &Marks, show_hidden)` calls `highlight(&entry.name, &folder, marks)`.
- `highlight` collects phrase ranges, then word ranges, sorts by start, and merges overlapping or touching ranges (required by views.rs:255).
- Drop `pub` on `highlight` or keep it; there is no caller outside engine.rs.
- Delete line 589.

**Follow-ons**

- (a) is fine and tiny: in src/finder/views.rs pass `&row.highlights` and `(!selected).then_some(pal.highlight)` at 483-488 and 1281-1286, mirroring `name_text`.
- Drop (b) from this change. Folder-cell highlights belong with the content-match-highlight candidate. Rewriting `detail` relative to the scope changes what the Folder column shows (docs/manual.md:364, "Search results always show where each item is") and feeds `fitting_width` (src/finder/list.rs:421-430).

**Changelog**

Add a `### Fixed` section under Unreleased in CHANGELOG.md.

**Not in scope**

`main.rs:42` highlights nothing because `fuzzy_text` drops any token with `:` (1555). That is a separate issue.

**Tests.** **Unit test**: extend `highlights` in the `mod tests` of crates/fffinder-core/src/engine.rs (line 1800). The three existing cases are kept, re-expressed through the new signature.

- [atlas, project] on `Project Atlas.txt` gives [(0,7),(8,13)]
- [atlas, proj] gives [(0,4),(8,13)]
- [src, main] on `main.rs` gives [(0,4)]
- [tax, report] on `report-tax-2024.pdf` gives [(0,6),(7,10)]
- [docs, readme] on `readme.md` gives [(0,6)]
- [mrs] on `main.rs` gives [(0,1),(5,7)]
- [xyz] on `abc` gives nothing
- [proj, project] gives one merged (0,7)
- phrase `tax exemption` with words [tax, exemption] on `The tax exemption list.pdf` gives [(4,17)]
- the same phrase on `tax notes - tax exemption.pdf` gives only the phrase range
- phrase `[draft]` on `[draft] plan.txt` gives [(0,7)]
- phrase `Tax` under `SearchCase::Sensitive` on `tax Tax.pdf` gives [(4,7)]
- word `test` with folder text `tests` on `integration_main_set.rs` gives nothing
- a non-ASCII name such as `Résumé Müller.pdf` with [muller, resume]: check that every offset satisfies `name.is_char_boundary`

**Engine end-to-end test**: a new test beside `quoted_words_match_names_letter_for_letter` (1987), with its own temp dir rather than that test's fixture, because the existing `"taxexemption"` case already yields (0,12) today and would not distinguish the fix.

- Same setup pattern: `Engine::new(EngineOptions { roots, app_dirs: Vec::new(), db_dir: None, watch: false, search: Default::default() })`, `engine.wait_ready`, `SearchRequest { files_only: true, limit: 50, .. }`.
- Files: `docs/readme.md`, `The tax exemption list.pdf`, `report-tax-2024.pdf`.
- Assert `hit.highlights` for `docs readme`, `"tax exemption"`, and `tax report`.

**Headless UI test**: in tests/finder.rs next to `searches_by_name_and_returns_to_browsing` (line 67), using the helpers in tests/common/mod.rs.

- `Fixture::new("search-marks")`, `Ui::open(cx, &fx.dir)`, `ui.press(&cmd("f"))`, `ui.type_text("atlas project")`.
- `ui.read(|f| f.rows_for_test().iter().find(|r| r.entry.name == "Project Atlas.txt").map(|r| r.highlights.clone()))` should equal `Some(vec![(0,7),(8,13)])`. It is empty today.
- The fixture already has `Projects/atlas/Project Atlas.txt`, and `type_text` with spaces is used elsewhere (tests/basics.rs:646).

All of this runs on Linux. I did not run cargo, per the read-only brief.

**Risks.** - **Cosmetic only.** Results and ranking do not change, which is why I rate the value 3, not 4.
- **Scattered-letter noise.** Per-word fallback marks stray letters for short words more often than the single-needle code did. The folder-contiguous tie-break in the notes reduces this but cannot remove it (`ui` on `Button_guide.rs` with no `ui` folder still marks `u` and `i`).
- **Non-ASCII offsets.** Ranges must stay on char boundaries or `StyledText` may panic; match char by char rather than via offsets from a lowercased copy.
- **First occurrence wins.** `rs` on `parsers.rs` marks the first `rs`, not the extension; same as today.
- **Fewer marked letters than typed.** A word that matched only a folder leaves nothing marked in the name. That is correct but visible.
- **Tiles (follow-on a).** Bold match letters can change where a two-line icon label wraps or truncates.
- **Follow-on (b).** A scope-relative Folder text would change documented behaviour and column fitting; leave it out of this change.

**Notes from later reviews.** From the reviewer of 'typo-fallback-near-miss' (5624239, on the trunk; read its done record's note for this brief): `checked_words(&words)` is what a result is held to, and the highlight needle is still every word, so a row that lacks a lone letter or digit of the search has no highlight at all (`photo 1.jpg` for `photo 2`; picture /home/user/polish-tools/shots/typo-fallback-near-miss-review-after-photo-2.png if it exists), and neither has a near-miss row. Highlight by the checked words first, and the lone letters where a name has them. 'result-tie-break' added a step and a helper `spells(name, words)` to `rerank`; `into_hit` and `highlight` are untouched by both.

## name-clash-details

**Name-clash dialog: show which items clash and how the two copies differ**

Value 3 of 5, size M, ui, ux.

**Problem.** The Replace / Skip / Keep Both dialog gives almost nothing to decide with. For several clashes the title is only 'N items already exist in <folder>' (src/finder/transfer.rs:366-372), with no names. The body is one fixed sentence for every case (transfer.rs:377, 396-401): 'Replace them with the ones you're copying, skip them, or keep both?' It says 'them' and 'the ones' even when the title names a single item. Neither copy's size or date is shown, so there is no way to tell which is newer before choosing Replace; Explorer and the Linux file managers show both. The footer has four equal buttons and no default (transfer.rs:403-408). Return falls through to the kit's default confirm handler, which just closes the dialog, so Return silently means Stop.

**Proposal.** In `confirm_clashes` (transfer.rs:357-410): One clash: under the sentence add two rows, 'Already there' and 'Copying'/'Moving', each with `format::human_size` and `format::human_date` from `symlink_metadata` of `transfer.target(source)` and of the source (folders show 'Folder' and the date); ids `clash-existing` and `clash-incoming`, with `.test_support()`. Several clashes: list the first five names (truncated) and 'and N more', with id `clash-names`. Word the sentence for one item ('Replace it with the one you're copying, skip it, or keep both?') and for many. Make Keep Both the `.primary()` button and wire `.on_ok` to it, so Return does the choice that loses nothing and one Cmd+Z undoes; Escape stays Stop. Read the metadata once before `open_dialog`, not in the builder closure, which runs every frame.

**Checked plan.** All changes are in `Finder::confirm_clashes` (src/finder/transfer.rs:357-410). Compute every string before `window.open_dialog` and clone `SharedString`s inside the builder, as `title` and `detail` are now; the builder reruns on every render (gpui-component root.rs:247-250).

1. **Wording.** For `[one]`: "Replace it with the one you’re {verb}, skip it, or keep both?". Keep the current sentence for several.

2. **One clash: two detail rows.**
   - Read `FileEntry::from_path(transfer.target(one)?)` and `FileEntry::from_path(one)` (crates/fffinder-core/src/entry.rs:36). It follows links as the list's Size column does and costs at most two stats.
   - Text: `entry.size.map(format::human_size).unwrap_or_else(|| entry.kind_label())`, then " · ", then `entry.modified.map(format::human_date)`. A folder reads "Folder · date".
   - Render with the existing `row(id, label, value, cx)` in src/finder/info.rs:227-250, made `pub(super)`. Ids: `clash-existing` ("Already there") and `clash-incoming` ("Copying" or "Moving").
   - **Correction:** `.test_support()` alone leaves `snapshot(id).label()` as `None`. The label is the aria label only (gpui-base test_support.rs:415, gpui-pre div.rs:3593). `row` already does `div().id(id).aria_label(value.clone()).child(value).test_support()`. If you don't reuse it, add `.aria_label(text)` and `use gpui_kit::TestSupportExt as _;`.
   - A failed read omits that row.

3. **Several clashes: names.** `div().id("clash-names").aria_label(joined).test_support()` holding up to five `div().truncate()` lines of `display_name`, plus "and N more" when there are more than five. The aria label is the same text joined with ", ".

4. **Default button and Return.**
   - Add `.primary()` to the `clash-keep` button.
   - Add `.on_ok` following the Rename and Go to Folder pattern (src/finder/dialogs.rs:684-686, 766-770): `view.update(cx, |this, cx| this.run_transfer(transfer.clone(), clash, window, cx)); true`.
   - Return `true` and let the kit close. Do not also call `window.close_dialog`, or the deferred close can take a second dialog.
   - **Pitfall the proposal misses:** Return reaches the dialog even when another button has keyboard focus, so a bare on_ok → KeepBoth would keep both after Tab onto Stop or Replace. Fix: create four `FocusHandle`s once in `confirm_clashes` (`cx.focus_handle()`), give each button its handle with `Button::track_focus` (gpui-component button/button.rs:486), and in `on_ok` run the choice whose handle `is_focused(window)`. Stop just returns `true`. With no button focused, fall back to `Clash::KeepBoth`.
   - Leave Escape, the × button and the backdrop alone; they stay Cancel, which is Stop.

5. **Docs.** docs/manual.md:516-523: one item shows each copy's size and date, several are listed by name, Return keeps both, Escape stops. Add an entry under CHANGELOG.md [Unreleased] › Changed.

**Tests.** File: tests/file_ops.rs, on the headless harness in tests/common/mod.rs. Helpers: `Fixture::new/path`, `Ui::open`, `select`, `select_also`, `go_to`, `press`, `click`, `snapshot(id).label()` / `.focused()`, `exists`, `dialog_open`, `cmd`, `common::trash_dir`. All of it runs on Linux.

1. **In `replacing_on_paste` (line 68), before the first `ui.click("clash-stop")` (line 76):**
   - `ui.snapshot("clash-existing").label()` contains "5 bytes" (the file holds "older").
   - `clash-incoming` contains "12 bytes" ("plain notes\n").
   - `!ui.exists("clash-names")`.
   - Add nothing else to this test. An extra Keep Both would shift its two-step Cmd+Z assertions (lines 92-96), so the proposal's "extend this test with enter" would break it.

2. **New test `return_keeps_both_and_escape_stops`** (same setup):
   - Cmd+V, `ui.press("escape")`: dialog closed, Projects/notes.txt is still "older", no "notes 2.txt".
   - Cmd+V, `ui.press("enter")`: dialog closed, "Projects/notes 2.txt" is "plain notes\n", notes.txt is still "older".
   - One `cmd("z")` removes "notes 2.txt".

3. **Focus-aware Return** (same test or its own):
   - Cmd+V, press "tab" until `ui.snapshot("clash-replace").focused() == Some(true)` (cap at 6), then `ui.press("enter")`.
   - Projects/notes.txt is "plain notes\n" and `trash_dir(&fx.dir).join("notes (replaced).txt")` is "older".

4. **New test `several_clashes_are_named`:**
   - Write Projects/notes.txt and Projects/Roadmap.md; `select("notes.txt")`, `select_also("Roadmap.md")`, Cmd+C, `go_to` Projects, Cmd+V.
   - The `clash-names` label contains both names; `!ui.exists("clash-existing")`; `ui.click("clash-skip")` leaves both files as written.

Do not assert dates; they read "Today at HH:MM". The existing clicks on clash-stop/skip/keep/replace (file_ops.rs:35-160) must stay green.

**Risks.** - **Return becomes an action.** It changes from a silent Stop to Keep Both. That is the one default that never touches an existing item, and one Cmd+Z undoes it. Finder and Explorer users expect Return to mean Replace, so the manual must say what it does. If the owner prefers no default, drop step 4; steps 1-3 stand alone.
- **Without the focus check, the change adds a trap.** Tab onto Stop or Replace, then Return, would duplicate files instead.
- **Keep Both on a large folder starts a full copy.** The progress row's Stop button and Undo cover it.
- **`info::row` becomes shared.** Don't change its layout; the Get Info tests read info-size and info-where through it.
- **Two more stats on the window's thread when the dialog opens.** `clashes()` and `check()` already stat every source there, so a stalled network volume is no worse than today.
- **Two filled buttons side by side.** Primary Keep Both sits next to danger Replace. Keep the ids and the order so existing tests and habits hold.

## preview-card-height

**The preview pane and column card cap the preview at 240 points however tall the window is**

Value 3 of 5, size S, ui, ux.

**Problem.** `render_preview_card`, used by the preview pane (src/finder/preview_pane.rs:91) and the column view's last column (src/finder/views.rs:1040-1049), puts the preview in a fixed-height box: `div().h(ui(240.)).w_full()...child(preview_body(...))` (src/finder/preview.rs:794-807), followed by the name and four or five one-line rows (preview.rs:808-837). The pane can be dragged from 180 to 640 points wide and is as tall as the window (preview_pane.rs:49-55), but the preview never grows: on a 900-point-tall window roughly 500 points under the metadata stay empty, a text file shows about 15 lines in the 240-point box, and a portrait image is letterboxed into it however wide the pane is made.

**Proposal.** preview.rs `render_preview_card`: make the preview box take the height left over. Replace `.h(ui(240.))` with `.flex_1().min_h(ui(160.))`, add `id("preview-card-body")` and `.test_support()` to it, and mark the name and each metadata row `.flex_shrink_0()` so they keep their size at the bottom. Keep `min_h_0`/overflow on the body so a long text preview scrolls inside the box rather than pushing the rows out. Update the Preview pane paragraph in docs/manual.md.

**Checked plan.** Only `render_preview_card` needs to change (src/finder/preview.rs:768-838). Both callers already give the card a definite height, so views.rs and preview_pane.rs stay as they are.

1. Preview box (preview.rs:795-807): change it to `div().id("preview-card-body").flex_1().min_h_0().w_full().rounded(ui(8.)).overflow_hidden().child(preview_body(..)).test_support()`.
   - This is the idiom already wrapped around the same `size_full()` preview bodies in quick_look.rs:161 and views.rs:1247-1249, so text, Markdown, font and table previews keep scrolling inside the box.
   - `.test_support()` goes last, as elsewhere. The trait is already in scope through `use gpui_kit::*;` (quick_look.rs:182 relies on the same glob).
   - `.h(ui(240.)).flex_grow()` is an equivalent minimal diff, but `flex_1().min_h_0()` matches the house style.

2. Do not add `.min_h(ui(160.))`. With a 160 pt floor the card needs about 341 pt of file-area height (28 padding + 160 + 153 details), and neither the pane (preview_pane.rs:55) nor "content" (mod.rs:3278) clips, so the Size/Modified/Where rows would be painted over the bars below. That happens in ordinary states:
   - the 850x470 minimum window (src/lib.rs:422) with a tab bar (36 pt, tab_bar.rs:407) or the scope bar (bars.rs:64);
   - the default 700 pt window with the Nushell panel open (260 pt, nu_panel.rs:204), which leaves 332 pt;
   - `--term` windows.
   Today the box shrinks in all of these and the details stay put. Keeping a zero minimum (`min_h_0` plus the existing `overflow_hidden`) gives exactly today's layout whenever the pane is shorter than about 420 pt, and grows the preview otherwise.

3. Add `.flex_shrink_0()` to the name div (preview.rs:809) and to the `h_flex()` in the `row` closure (:774). They already cannot shrink below their text, so this is belt and braces. Do not add `.test_support()` to the rows: their ids ("Kind", "Size", ...) equal the list headers' ids that tests look up (tests/file_ops.rs:931, :934), and `find` panics on an ambiguous id (gpui-base-0.8.1/src/test_support.rs:209-217).

4. Update the doc comment at preview.rs:767 ("A compact card for the column view's last column") to mention the preview pane and that the preview takes the height the details leave.

5. Docs: add a sentence to the Preview pane paragraph (docs/manual.md:134) and optionally the Columns bullet (:235), saying the preview fills the height above the details. Add an entry under [Unreleased] > Changed in CHANGELOG.md; the proposal omitted the changelog.

6. Design consequence to accept: the details become anchored to the bottom of the pane, as in Finder's preview column. Previews that don't fill the box (a folder or unknown-file icon, a landscape image in a 280 pt pane, a two-line text file's grey box) sit centred in a taller box, further from their details. The gain is for text, code, tables, PDF thumbnails and portrait images, and for landscape images once the pane is widened; today nothing past about 390 pt of width helps a 3:2 image.

**Tests.** Headless UI test in tests/basics.rs, in the "the preview pane" section after `the_preview_pane_resizes_from_its_left_edge` (line 965). Helpers from tests/common/mod.rs: `Fixture::new`, `Ui::open`, `ui.click`, `ui.press(&cmd("shift-p"))`, `ui.select`, `ui.resize`, `ui.snapshot(id).bounds()`, `ui.settle`, `item_id`.

List view with the pane:
1. `ui.click("view-list")`, `ui.press(&cmd("shift-p"))`, `ui.select("notes.txt")`.
2. Define a measure closure returning the height of `ui.snapshot("preview-card-body").bounds()` and the gap `pane.bottom() - body.bottom()`, with pane = `ui.snapshot("preview-pane").bounds()`.
3. `ui.resize(1100., 600.)` first, then measure. The test window starts at 1536x1080, so the proposal's single resize to 1000 would shrink it.
4. `ui.resize(1100., 1000.)`, measure again. Assert the body grew by about 400 pt and the gap under it is unchanged, both within 1 pt for device-pixel rounding. The old code gives 240 both times, so the first assertion pins the fix.
5. `ui.resize(1100., 360.)`. Assert the body is shorter than at 600 and the gap is still unchanged. This pins that a short pane takes height from the preview, not the details, and would fail with a 160 pt floor.

Column view:
6. `ui.click("view-columns")`, `ui.settle()`, `ui.click(item_id("crow", &fx.path("notes.txt")))`.
7. Repeat the 600 and 1000 measurements against `ui.snapshot("content").bounds()`. "content" is observed at src/finder/mod.rs:3279-3280; "preview-pane" does not exist in column view (preview_pane.rs:16-19).

No existing test depends on the card's geometry. `holding_a_drag_over_what_covers_a_folder_leaves_it_closed` (tests/basics.rs:441) relies on the row's clipped hitbox (src/finder/transfer.rs:749-760), not on what the pane draws.

**Risks.** - If the proposal's `.min_h(ui(160.))` is kept, the detail rows overflow the pane onto the Nushell panel, path bar or status bar in short file areas. That is a regression from today's shrinking box; the corrected plan drops the floor.
- The details move to the bottom of the pane or column. Icon-only previews, landscape images in a narrow pane and very short text files get a taller, mostly empty box between picture and details. It matches Finder, but it is a visible layout change on every selection, not only tall content.
- `ObjectFit::Contain` (preview.rs:907, :1125) upscales, so a small portrait image is drawn larger in a taller box. Landscape and square ones stay limited by the pane width.
- In split view both panes can show a preview pane, giving two "preview-card-body" elements in one window. A test that looks the id up without `within(..)` would panic as ambiguous; "preview-pane" already has the same property.
- Exact-pixel assertions can be off by half a point from device-pixel rounding, so the test should compare with a tolerance.
- Not built or run here (the review was read-only, no cargo). The layout conclusions come from reading GPUI and Taffy and from the identical `flex_1().min_h_0()` wrappers already used around these preview bodies.

## quick-look-empty-state

**Quick Look goes blank with a stale title when nothing is selected, and gives no position among several items**

Value 3 of 5, size S, ui, ux.

**Problem.** The Quick Look window renders nothing when the browser has no primary entry: `let Some(entry) = browser.primary_entry() else { return root; };` (src/finder/quick_look.rs:150-152), and the title is only set after that point (quick_look.rs:153). The `--term` dialog does the same (`return dialog`, src/finder/preview.rs:428-430). Quick Look deliberately stays open and follows the browser, and nothing closes it or redraws a placeholder when the selection empties: clicking empty space (`clear_selection`, src/finder/mod.rs:1697-1718), editing the search (`self.selection.clear()`, mod.rs:1336 and 1341), or the previewed file being deleted all leave an empty window still titled with the last file's name, with no caption or buttons. The preview pane has an explicit empty state for this ('No Selection', src/finder/preview_pane.rs:92-101). With several items selected the arrows cycle through them (preview.rs:552-557), but neither the caption (`preview_caption`, preview.rs:717-733) nor the title says which of how many is showing.

**Proposal.** quick_look.rs `render`: when `primary_entry()` is None, call `window.set_window_title("Quick Look")` and return the root with a centred muted 'No Selection' element (`id("ql-empty")`, `.test_support()`), styled like `preview-pane-empty`; keep the key handlers so the arrows still select an item and Space/Escape still close. Give the `--term` dialog the same body in preview.rs:428-430 instead of an empty dialog. When `selection.len() > 1` (outside Columns), add a muted '2 of 5' element (`id("ql-position")` with a matching `aria_label`, as `preview-pane-many` does at preview_pane.rs:75-80) to `preview_caption`, computed from the cursor's index among `selection.indices()` through a small `Finder::selection_position() -> Option<(usize, usize)>` helper in mod.rs.

**Checked plan.** Not compiled; this review was read-only.

1. **Shared placeholder** (`src/finder/preview.rs`)
   - Add `pub(super) fn no_selection(id: &'static str, cx: &App) -> AnyElement`, moving the element from `preview_pane.rs:92-101` (v_flex, id, size_full, centred, `ui(12.)`, muted, "No Selection", `.test_support()`).
   - Have `preview_pane.rs` call it with `"preview-pane-empty"`.

2. **Window** (`src/finder/quick_look.rs:150-152`)
   - On None: `window.set_window_title("Quick Look"); return root.child(no_selection("ql-empty", cx));`
   - Nothing else is needed; the root already carries the key handlers.
   - Do not touch the open-time guards (`quick_look.rs:249-251`, `preview.rs:412-414`). Quick Look must still refuse to open with nothing selected; `tests/controls.rs:154-183` relies on it.

3. **`--term` dialog** (`preview.rs:422-533`) — this is the correction
   - Swapping in "the same body" for `return dialog` is not enough: it leaves the keys dead and never resets `quick_look_open`.
   - Restructure the builder: take `let entry = finder.primary_entry();` as an Option.
   - Always build the `div().track_focus(&focus).key_context(QUICK_LOOK)` with all existing `on_action` handlers and `.h(px(height))`.
   - Always apply `.w(px(width))` and `.on_close(..)`.
   - None: `.title("Quick Look")` plus `keys.child(no_selection("ql-empty", cx))`, no footer.
   - Some(entry): today's title, `render_preview_large` and footer.

4. **Position**
   - Add a pure `Selection::cursor_position(&self) -> Option<(usize, usize)>` in `src/model.rs`: 1-based place of the cursor among `items`, plus `len`. A cursor outside the selection counts as first, matching `cycle_cursor` (`model.rs:315-318`) and `primary_entry` (`mod.rs:1780-1783`).
   - Add `Finder::quick_look_position()` beside `quick_look_step` in `preview.rs` (rather than `mod.rs`), with the same guard as the cycling branch: `effective_view() != ViewMode::Columns && self.selection.len() > 1`.
   - Give `preview_caption` a `position: Option<(usize, usize)>` parameter. When set, append `div().id("ql-position").aria_label(label.clone()).text_color(muted).child(label).test_support()` with `format!("{n} of {of}")`.
   - `quick_look.rs:185` passes `browser.quick_look_position()`.
   - `render_preview_large` passes it only when `surface == Surface::QuickLook`. The gallery (`views.rs:1252-1258`, `Surface::Gallery`) must never get a `ql-position`.

5. **Docs**
   - `docs/manual.md` §Quick Look (lines 424-429): the title is "Quick Look" and the window says "No Selection" when nothing is selected, and the arrows select the first or last item; with several selected, the caption says which one, such as "2 of 5".
   - `CHANGELOG.md` [Unreleased]: a Fixed entry and an Added entry. The candidate's file list omits the changelog.

**Pitfalls**
- `ElementSnapshot::label()` is the accessibility label, not rendered text, so the `aria_label` is required.
- Do not call `ensure_*` from render.
- CI runs `cargo fmt --check` and `clippy -D warnings` on macOS, Linux and Windows.

**Tests.** Headless harness in `tests/common/mod.rs` (`Fixture`, `Ui::open`, `select`, `select_also`, `press`, `update`, `go_to`, `in_quick_look`, `exists`, `snapshot`, `selected`, `names`). All of it runs on Linux.

1. **New test in `tests/controls.rs`**, beside `quick_look_buttons_and_keys` (line 1083):
   - `ui.select("notes.txt"); ui.press("space");` then `ui.update(|f, _, cx| f.clear_selection(cx));` (public; it is what Escape and an empty-space click call).
   - Assert `ui.in_quick_look(|ui| ui.exists("ql-empty"))`, `!ui.in_quick_look(|ui| ui.exists("ql-open"))`, and `ui.read(|f| f.quick_look_window()).is_some()`.
   - Assert the title is `Some("Quick Look")`. Add a small helper to `tests/common/mod.rs`: `pub fn window_title(&mut self) -> Option<String> { gpui_kit::VisualTestContext::from_window(self.handle, self.cx).window_title() }`. It works inside `in_quick_look`, which swaps `self.handle`; the test window records `set_title`.
   - `ui.in_quick_look(|ui| ui.press("down"))`: `ui.selected().len() == 1`, `ql-empty` gone, `ql-open` back, title equals the selected name.
   - `ui.go_to(&fx.path("Projects"))` (the navigation path, `mod.rs:906`): `ql-empty` again.
   - `ui.in_quick_look(|ui| ui.press("space"))`: `quick_look_window()` is None.

2. **Extend `quick_look_arrows_move_through_items`** (`tests/controls.rs:1270-1285`):
   - After `select` + `select_also` + Space: `ui.in_quick_look(|ui| assert_eq!(ui.snapshot("ql-position").label(), Some("2 of 2")));`
   - After `right`: `Some("1 of 2")`.
   - In the single-selection part: `!ui.in_quick_look(|ui| ui.exists("ql-position"))`.
   - Assert inside the closure. The candidate's form returns `label()` out of it, which borrows a temporary snapshot (`label()` returns `Option<&str>`) and will not compile.

3. **Unit test in `src/model.rs`**, next to `cycle_cursor_wraps_within_the_selection` (line 477), for `Selection::cursor_position`: three items with a moving cursor, a deselected cursor counting as first, and empty giving None.

4. **New test in `tests/term_mode.rs`**:
   - Do not use `open_term` (lines 13-27): it skips when `nu` is missing, as it is in this sandbox. Call `fffinder::term::pretend_active_for_tests(); let mut ui = Ui::open(cx, &fx.dir);`.
   - `ui.select("notes.txt"); ui.press("space"); assert!(ui.dialog_open());`
   - `ui.update(|f, _, cx| f.clear_selection(cx));` then assert `ui.exists("ql-empty")`, `!ui.exists("ql-open")`, `ui.dialog_open()`.
   - `ui.press("down")`: one item selected and no `ql-empty`. This proves the key context survived.
   - Clear again, `ui.press("space")`: `!ui.dialog_open()` and `!ui.read(|f| f.quick_look_is_open())`. This proves the handlers and close path survived.

5. **Existing tests that must stay green**: `tests/basics.rs:930-937` (`preview-pane-empty` / `preview-pane-many`), `tests/finder.rs:369-391` (`ql-caption` / `ql-controls` wrap, single selection), `tests/controls.rs:154-183` (Quick Look does nothing without a selection), `tests/term_mode.rs`.

**Risks.** Low overall.

- **`--term` dialog restructure** is the only part with regression potential. The focus-tracked div, all `on_action` handlers, the width and `.on_close` must be present in both branches. The existing dialog tests in `tests/term_mode.rs` are skipped without `nu`, so run them where `nu` is installed (CI installs it).
- **`preview_caption` is shared** with the gallery through `render_preview_large`. The position must be passed only for `Surface::QuickLook`, or the gallery grows a `ql-position` element.
- **Headless tests cannot catch a missing redraw**: `render_frame` refreshes the whole window (gpui-kit `src/test.rs:348-351`). The Quick Look window already observes the Finder (`quick_look.rs:42`), so it does redraw in the real app.
- **Design choice**: macOS Finder previews the enclosing folder when the selection is cleared. "No Selection" is simpler, matches the preview pane, and is needed anyway for Recents, search and the Trash, which have no folder.
- **Adjacent bug, out of scope**: `select_all` (`src/finder/mod.rs:1957-1960`) does not call `selection_changed`. Cmd+A from the empty state leaves Quick Look on the Loading spinner for a row never previewed. It exists today (blank, then spinner) and is a one-line fix if wanted.
- **Unchanged wart**: Space in Quick Look switches mode instead of closing when it is in explore mode, including in the empty state. The new tests open with plain Space so the first Space closes.

## recent-searches

**Recent searches: a menu on the search field's magnifier and Up to recall, plus a placeholder that names the scope**

Value 3 of 5, size M, search, feature.

**Problem.** There is no search history anywhere (no `recent_search`/`search_history` in src or crates; no such field in Config, crates/fffinder-core/src/config.rs:205-283). The search field is a plain input with a static magnifier icon (src/finder/toolbar.rs:461-475) whose key handler only intercepts `down` (toolbar.rs:449-460); Return only reruns the query (src/finder/mod.rs:1308-1318); ending a search empties the field (mod.rs:1350-1356). The only record of a query is fff's ranking database, written when a result is opened (mod.rs:2019, 2031-2038 -> crates/fffinder-core/src/engine.rs:1308-1313), and it is never shown. A query with globs, a folder filter or quoted words has to be retyped every time; the only way to keep one is a Smart Folder. Spotlight, Finder's search field and Everything all offer recent searches. The app already keeps a comparable list for servers (`recent_servers`, config.rs:241, maintained at src/finder/volumes.rs:202-205). Separately, the field's placeholder is a fixed 'Search' (mod.rs:479-483) even though the scope can be set before typing (toolbar.rs:413-437, default `SearchScope::Computer` at mod.rs:549), so until the first keystroke nothing in the field says whether it will search the folder or everything.

**Proposal.** 1. Config: add `recent_searches: Vec<String>` (newest first, deduplicated, at most 10; the second reviewer suggested 20) next to `recent_servers` in crates/fffinder-core/src/config.rs, default empty, and add it to the `shared` list of `settings_by_scope!` (config.rs:609-616; a missing entry is a compile error). Document it in the settings.json table of docs/manual.md. 2. Record with a small `fn remember_search(&mut self, cx)` (dedupe, insert at 0, truncate, `settings_changed`, following the insert/retain/truncate pattern at volumes.rs:202-205) called when Return is pressed in the field (mod.rs:1308), when a result is opened (`open_entries`, mod.rs:2019) and from `reveal_in_place` while searching. Never per keystroke, and not in `--pick` windows. 3. UI in src/finder/toolbar.rs:464-468: replace the prefix icon with a ghost icon button `search-history` that opens a dropdown (`dropdown_menu`, as the other toolbar buttons do) listing the entries, then a separator and 'Clear Recent Searches'. Choosing an entry sets the field and calls `set_query(text, true, cx)`. With no entries it stays a plain icon. 4. Optional: in the key handler at toolbar.rs:449-460, Up in an empty field (or while stepping) recalls older entries and Down steps back; Down keeps moving into the results otherwise. 5. Placeholder: add `update_search_placeholder(window, cx)` that calls `InputState::set_placeholder` (gpui-base src/input/base/state.rs:1177) with 'Search <folder>' or 'Search This Computer', invoked from `set_scope` and `location_changed` only (not from render, since it notifies).

**Checked plan.** Corrected plan. Nothing here was compiled or run (read-only review).

**1. Config** (crates/fffinder-core/src/config.rs)
- Add `pub recent_searches: Vec<String>` beside `recent_servers` (line 241), `Vec::new()` in `Default` (near 309), and add it to the `shared:` list of `settings_by_scope!` (609-616). The destructuring at 597 makes a missing entry a compile error.
- `#[serde(default)]` on the struct already covers older files.
- Cap at 10, matching `RECENT_SERVERS` (src/finder/volumes.rs:20) and the "up to ten" wording at docs/manual.md:1068.

**2. Recording** (src/finder/mod.rs)
- Add `fn remember_search(&mut self, cx)`: return if `self.query` is empty or `self.picker.is_some()`; then retain/insert(0)/truncate as at volumes.rs:202-205, and call `settings_changed(cx)`.
- Call it from:
  - `InputEvent::PressEnter` (mod.rs:1308);
  - the Down branch of the toolbar key handler (toolbar.rs:450), because the manual treats Down and Return as equal ways into the results;
  - the top of `open_entries` (mod.rs:2019), before the folder branch navigates;
  - the top of `reveal_in_place` (mod.rs:1005), before `end_search`.
- Never call it from `InputEvent::Change`, `open_request` or `open_smart_folder`.

**3. Menu** (src/finder/toolbar.rs:461-475)
- When `config.recent_searches` is non-empty, replace the prefix icon with `Button::new("search-history").ghost().xsmall().icon(IconName::Search).tab_stop(false).tooltip("Recent Searches").dropdown_menu(...)`. Otherwise keep the plain icon.
- Build items the way `view_mode_items` does (toolbar.rs:260-277): `PopupMenuItem::new(text).on_click(window.listener_for(&view, ...))`, then a separator and "Clear Recent Searches".
- An entry's click does what `open_request` does (mod.rs:819-824): `search_input.set_value`, then `set_query(text, true, cx)`. It also focuses the field and calls `remember_search` so the entry moves to the top. Consider factoring a `search_for(query, window, cx)` helper shared with `open_request` and `open_smart_folder`.
- **Mandatory fix:** guard the existing `capture_key_down` body with `this.search_input.focus_handle(cx).is_focused(window)`. `InputState` is `Focusable` (gpui-base state.rs:4944). Without the guard, Down inside the open history menu is stolen.
- The Input frame keeps its width, so the toolbar arithmetic (toolbar.rs:27-81) and the basics.rs fold tests are unaffected. Do not use the fallback of a separate pill; that would change `TOOLBAR_FULL_WIDTH` and those tests.

**4. Up recall** (optional, same handler)
- Add `search_recall: Option<usize>` to `Finder`.
- On `up`, with the field focused and either empty or still showing the recalled entry, step to the next older entry via `set_value` + `set_query(.., true, ..)` and stop propagation.
- Reset the index in `InputEvent::Change` and in `end_search`. `set_value` emits no Change event (state.rs:1218-1220), so recalls do not reset it.
- Do not implement "Down steps back". Down keeps its documented meaning.

**5. Placeholder**
- Add `fn update_search_placeholder(&mut self, window, cx)` that computes the text from the same `in_folder` test as toolbar.rs:415-418 and calls `set_placeholder` only when the text differs.
- Call it from:
  - `set_scope` (add a `window` parameter; its three callers at toolbar.rs:429-435 and src/finder/bars.rs:80-93 are click listeners that have one);
  - `location_changed` (mod.rs:900);
  - the end of `open_smart_folder` (src/finder/filter.rs:215-237).
- Pin `.aria_label("Search")` on the Input (input.rs:269) so the accessible name stays stable.
- Wording: the field narrows to 120 pt, then 80 pt (toolbar.rs:23-25), so "Search This Computer" will clip. I suggest keeping "Search" for the default scope and showing `Search “Projects”` only for folder scope.

**6. Docs**
- docs/manual.md: the toolbar table row (185), Searching (324), the "In the search field" keys table (964-969), and a `recent_searches` row after 1068.
- CHANGELOG.md: under `[Unreleased]` → Added.

**Tests.** **Headless UI tests** go in tests/controls.rs, in the search section beside `search_field_typing_escape_and_down_arrow` and `search_scope_button` (lines 301-358). Helpers come from tests/common/mod.rs: `Fixture::new`, `Ui::open`, `ui.click`, `ui.type_text`, `ui.press`, `cmd("f")`, `ui.exists`, `ui.menu_open`, `ui.menu_labels`, `ui.click_menu_item`, `ui.read(|f| f.config().recent_searches.clone())`, `ui.read(|f| f.query().to_string())`, `ui.names`, `ui.selected`, `ui.files_focused`, `ui.go_to`, `ui.update`.

1. **Records on Return.** `ui.click("search")`, type `roadmap`, press `enter`, press `escape` twice (same flow as tests/finder.rs:86-98). Assert `recent_searches == ["roadmap"]` and `ui.exists("search-history")`.
2. **Typing alone records nothing.** Type `atlas`, press `escape`. Assert the list is unchanged. On a fresh window, assert `!ui.exists("search-history")`.
3. **Down records.** Type `atlas`, press `down`. Assert the list is `["atlas", "roadmap"]`.
4. **Repeat moves to the top.** Search `roadmap` again with Return. Assert `["roadmap", "atlas"]` with no duplicate.
5. **Menu.** `ui.click("search-history")`. Assert `ui.menu_labels()` starts with the entries and ends with "Clear Recent Searches". `ui.click_menu_item("atlas")`, then assert `query() == "atlas"` and `ui.names()` contains "Project Atlas.txt".
6. **Regression for the key-capture pitfall.** Open the menu, press `down`. Assert `ui.menu_open()` is still true and `!ui.files_focused()`.
7. **Clear.** `ui.click_menu_item("Clear Recent Searches")`. Assert the list is empty and `search-history` is gone.
8. **Up recall.** With history present, press `cmd("f")`, then `up`. Assert `query()` equals the newest entry. Press `up` again for the older one. Then press `down` and assert `ui.files_focused()`, which pins the documented Down behaviour.
9. **Placeholder.** Add `pub fn search_placeholder_for_test(&self, cx: &App) -> String`, reading `self.search_input.read(cx).presentation().placeholder()`. Read it with `let view = ui.view.clone(); ui.window(|_, cx| view.read(cx).search_placeholder_for_test(cx))`, the pattern `files_focused` uses at common/mod.rs:710-713.
   - `ui.go_to(&fx.path("Projects"))`, `ui.click("search-scope")`: assert it contains "Projects".
   - Click again: assert the default text.
   - Navigate to `Location::Recents`: assert the default text.
   - `add_smart_folder` with a folder scope, then `open_smart_folder(0, ..)`: assert it names that folder.

**Config unit tests** go in the `mod tests` of crates/fffinder-core/src/config.rs (line 854), not tests/settings_file.rs:
- extend `round_trips` (859) with `recent_searches`;
- assert a JSON file without the key loads with an empty list;
- assert `take_shared_changes` carries the field.

**Existing tests that must stay green:** `the_toolbar_folds_what_does_not_fit` and `the_toolbar_folds_just_when_the_drawn_controls_run_out_of_room` (tests/basics.rs:519-614), `search_scope_button` (controls.rs:328), and `searches_by_name_and_returns_to_browsing` (tests/finder.rs:66).

**Risks.** - **Key capture (highest risk).** The history popup is a dispatch-tree descendant of the search wrapper, so the existing Down handler steals Down from the open menu unless it is guarded on the field's own focus. Test 6 covers this.
- **Focus after dismissing the menu.** The Input frame tracks its own focus handle (gpui-component input.rs:675-683, 737), and `PopupMenu::dismiss` restores whatever was focused when the menu was built (popup_menu.rs:1074-1091). After Escape, focus may land on the frame or the button, not the editor. Entry clicks should focus the field explicitly; check the Escape case by hand or with `window.has_focused_input`.
- **Prefix button size.** A ghost xsmall button is wider than the 14 px icon, so text in the 80-120 pt narrow field loses a few points. The frame width is unchanged, so the fold tests should hold.
- **Discoverability.** A magnifier with no caret does not look like a menu. The tooltip and manual help; a caret would cost more text room.
- **Privacy.** Queries are written in plain text to settings.json, which some users sync or share. `recent_servers` and `last_folder` set the precedent, and "Clear Recent Searches" plus recording only committed searches mitigates it. Recording also rewrites settings.json on each committed search, as `last_folder` already does on every navigation.
- **Placeholder.** Long folder names clip in a narrow field. `set_placeholder` notifies, so it must be called only when the text changes and never from render.
- **Value is moderate, not high.** Search is as-you-type and takes milliseconds, so the gain is mostly for long glob or exclusion queries, which Smart Folders already cover for deliberate reuse. Hence 3 rather than the proposed 4.

## recents-walk-index

**Recents can come up short or empty when many recently changed files are ones it hides**

Value 3 of 5, size S, search, bug.

**Problem.** Engine::recents asks fff for the top `(limit * 4).max(4_000)` files of an empty query, per root (crates/fffinder-core/src/engine.rs:811-823), and only afterwards drops files older than 30 days, anything under a dot-folder or a `Library` folder, and package contents (825-838). An empty query is ordered by frecency score, then newest modification (fff-search score.rs:911-970, 1029-1041), so the 4,000 candidates are essentially the 4,000 most recently modified files. If more than 4,000 hidden-from-Recents files changed more recently than the user's documents, the documents never make the cut: a Photos import into `Photos Library.photoslibrary`, an Xcode build into `~/Library/Developer/Xcode/DerivedData` (not on fff's ignore list, ignore.rs:5-64), mail arriving in `~/Library/Mail`, or files inside an `.app`/`.framework` being updated. The window calls recents(500) (src/finder/mod.rs:1044-1045) and then shows fewer days than the promised 30, or 'No Recent Files'. The call also scores and partially sorts every indexed file just to keep 4,000.

**Proposal.** Rewrite the body of recents to walk the index directly, as list() and folder_stats_for() already do: for each root iterate `picker.get_files().iter().filter(|f| !f.is_deleted() && f.modified >= cutoff)`, build the relative path only for those, apply the existing dot/Library and inside_package filters, and push (modified, path, size). Then sort newest first and truncate to `limit` (or keep a BinaryHeap of size `limit`). The comparison on `modified` happens before any path is materialised, so the pass is cheaper than the scored search it replaces. Keep the final `RawHit::file(...).into_hit(..)` mapping unchanged.

**Checked plan.** Rewrite only the per-root body of `Engine::recents` (`engine.rs:803-840`). Keep the cutoff computation, the final sort and truncate (841-842), and the `RawHit::file(..).into_hit("", false)` mapping (843-850).

**Per root, under the existing picker read guard**
1. Collect the recent files: `let mut recent: Vec<_> = picker.get_files().iter().filter(|f| f.modified >= cutoff && !f.is_deleted()).collect();`
   - Type inference avoids importing `FileItem`.
   - `get_files()` already includes overflow files added since the scan, as the comments at `engine.rs:1027` and `1189` note.
2. Sort with `recent.sort_by_key(|f| std::cmp::Reverse(f.modified));`
   - Use the stable sort: the index is path-sorted, so same-second files come out in a deterministic order.
3. Materialise paths lazily, newest first: `found.extend(recent.into_iter().filter_map(|f| { ... }).take(limit))`.
   - Inside the closure: `let rel = f.relative_path(picker);`, apply the existing dot/`Library` part check, `join_rel(&base, &rel)`, then `!inside_package(&path, None)`, yielding `(f.modified, path, f.size)`.

**Why lazy rather than the proposal's order**
- The proposal builds a path string for every file inside the 30-day window before truncating. After a fresh clone or a big build that can be hundreds of thousands of allocations.
- Sorting first and stopping at `limit` accepted files bounds this. The worst case, where everything recent is hidden, is unavoidable.
- A size-`limit` `BinaryHeap` also works but is more code for no visible gain.

**Clean-up and docs**
- Delete the now-unused `QueryParser::default().parse("")` and the comment at 812. `QueryParser`, `FuzzySearchOptions` and `PaginationArgs` are still used elsewhere in the file, so the imports stay.
- Rewrite `docs/manual.md:382` to describe the real behaviour: up to 500 files changed in the last 30 days, newest first, leaving out hidden items, `Library` folders and package contents. Keep "It waits for the index to be ready."
- Drop "rank Recents" from `docs/manual.md:1173`; frecency no longer touches Recents at all.
- Optionally reword `docs/manual.md:192` from "files you used recently" to "files changed recently".
- Add a `### Fixed` entry under `[Unreleased]` in `CHANGELOG.md`.

**Pitfalls**
- Compare on `f.modified`, a `u64` of seconds. A file whose metadata failed has `modified == 0` and stays excluded, as today.
- `limit == 0` still returns an empty list.
- Do not add `par_iter`; a sequential pass over the item slice is a simple comparison per file.

**Tests.** Add a unit test to the `#[cfg(test)] mod tests` block in `crates/fffinder-core/src/engine.rs` (starts at 1778). Model it on `glob_finds_files_inside_bundles` (1823-1852) and `adds_up_folders_from_the_index` (1878-1913). No UI harness is needed.

**Set-up**
- Temp root: `std::env::temp_dir().join(format!("fff-recents-{}", std::process::id()))`.
- Write `doc.txt`, `newer.txt`, `old.txt` and `Library/notes.txt`.
- Write 4,100 small files under `Big.app/Contents/Resources/`, left at the current mtime.
- Set mtimes with the idiom from `crates/fffinder-core/src/ops.rs:1041-1046` (`File::options().write(true).open(p).unwrap().set_modified(t)`):
  - `doc.txt` to now minus 1 hour
  - `newer.txt` to now minus 10 minutes
  - `old.txt` to now minus 40 days
- Build the engine with `Engine::new(EngineOptions { roots: vec![dir.clone()], app_dirs: Vec::new(), db_dir: None, watch: false, search: Default::default() })`, then `assert!(engine.wait_ready(Duration::from_secs(20)))`.

**Assertions**
- `engine.recents(10)` mapped to `entry.name` equals `["newer.txt", "doc.txt"]`.
  - Compare names, not paths: fff respells the root on Windows, as the comment at 1841 notes.
  - This fails today with an empty list, because all 4,000 candidates are the newer package files. It also covers the 40-day cutoff, the `Library` filter, the package filter and newest-first order.
- `engine.recents(1)` returns `["newer.txt"]`.

Remove the temp dir at the end. The test is platform-neutral: the `Library` and package filters work by name, and `Library/notes.txt` is not on fff's ignore list on any OS. Existing UI tests in `tests/finder.rs`, `tests/controls.rs` and `tests/folder_tree.rs` only check navigation to `Location::Recents`, so they are unaffected.

**Risks.** Low.
- **No ordering regression.** The final list is already sorted by mtime at `engine.rs:841`. The new result differs from today's only where today's is missing files.
- **Docs.** The manual's frecency wording (`docs/manual.md:382`, `:1173`) is already wrong and must be corrected in the same change, or the docs will describe ranking that does not exist.
- **Cost.** One sequential pass over each root's file items plus a sort of the files inside the 30-day window. That is cheaper than the scored search except in the pathological case where most of a huge index was modified in the last 30 days and nearly all of it is hidden; then a path string is built per recent file, on the background listing thread.
- **Test time.** The regression test writes 4,100 files, needed to exceed the 4,000-candidate floor. That is fine on Linux and may add a second or two on Windows CI.
- **Value depends on the machine.** The effect is mostly on macOS home roots with Full Disk Access. On Linux, non-git roots skip hidden folders at index time, so the gap only opens with large package-like or `Library` trees.

## regex-content-search

**Regular-expression content search in the window, with a message when the pattern is invalid**

Value 3 of 5, size M, search, feature.

**Problem.** The engine already supports plain, regex and fuzzy content search (GrepModeKind, crates/fffinder-core/src/engine.rs:195-204, mapped at 907-911) and the Nushell/HTTP side exposes it (src/server/mod.rs:467-476), but the window always searches plain text: content_search builds its GrepRequest with `..Default::default()` (src/finder/mod.rs:2999-3006) and the scope bar has only Name and Contents chips (src/finder/bars.rs:99-114). There is no way to search for `TODO|FIXME` or a pattern with character classes. Regex mode also fails silently: when a pattern does not compile fff falls back to literal matching and reports it in GrepResult::regex_fallback_error (fff-search grep/types.rs:190-193, grep/grep.rs:330-336), but Engine::grep never reads that field (engine.rs:960-992), so `fff search --regex` with an unbalanced parenthesis returns literal matches or nothing with no error. In regex mode the pattern is still run through the constraint parser (engine.rs:933-936), which swallows tokens that mix a wildcard with '/' (a path-like regex) as a path glob.

**Proposal.** Engine: add `regex_error: Option<String>` to GrepResponse, filled from the first root's result.regex_fallback_error. In Regex mode with one pattern, build the query unparsed: an FFFQuery with an empty raw_query, constraints holding only the scope, fuzzy_query FuzzyQuery::Text(pattern) and no location. Window: add `search_regex: bool` to Finder next to search_contents (mod.rs:300), a `set_search_regex` like set_search_contents (1478-1483), and a third chip with id `contents-regex` and label `.*` (tooltip 'Regular expression') shown only when Contents is on, after the by-contents chip in bars.rs. content_search takes the mode. Carry regex_error in SearchStats and append ', invalid regular expression - searched as text' to the status bar timing string (bars.rs:361-372). Save it in Smart Folders: add `regex: bool` to SmartFolder (crates/fffinder-core/src/config.rs:411-423, struct already has serde(default)) and set/read it in src/finder/filter.rs:186-227. Server: return Failure::bad(error) from `search` when response.regex_error is Some. Document the chip in manual.md's scope-bar table.

**Checked plan.** ENGINE (crates/fffinder-core/src/engine.rs)
1. GrepResponse (247-257): add `pub regex_error: Option<String>`. It is only built at 865 with `..Default::default()`, so nothing else changes.
2. Add `Constraint, FFFQuery, FuzzyQuery` to the existing `use fff_search::{...}` (lib.rs re-exports fff_query_parser::*; no new dependency).
3. In the per-root loop, branch on `req.mode == GrepModeKind::Regex` before the `if let [only]` at 934. After the join at 880-887 regex mode always has exactly one pattern.
   - text: `if only.starts_with('\\') { format!("(?:{only})") } else { only.to_string() }` to defeat grep_text()'s leading-backslash strip.
   - constraints: `scope_glob.as_deref().map(str::trim_start).map(Constraint::Glob).into_iter().collect()` (reuses today's scope glob, so it is self-contained; if scoped-search-special-folder-names lands first, use its helper instead).
   - `FFFQuery { raw_query: "", constraints, fuzzy_query: FuzzyQuery::Text(&text), location: None }` then `picker.grep(&query, &options)`. The empty raw_query is load-bearing: it switches off fff's whole-query retry (grep.rs:227-230), which would otherwise drop the scope constraint and rescan every file.
   - Leave Plain and Fuzzy on the parser.
4. Right after the call, for EVERY root (not only the first): if response.regex_error is None, take result.regex_fallback_error. Shorten it: the regex crate's message is multi-line ("regex parse error:\n    (\n    ^\nerror: unclosed group"); keep the last line without the "error: " prefix.
5. Known gap to state in a comment, not paper over: fff reports nothing when no file is a candidate (grep.rs:379-381), which can happen with content_index on. Closing it means validating in the engine with `regex::bytes::RegexBuilder` using fff's flags (multi_line(true), unicode(false), grep/regex.rs:13-32) and adding `regex = "1"` to fffinder-core (already in Cargo.lock at 1.13.1). Optional; do not write tests that depend on the error with content_index on.

WINDOW
- src/finder/mod.rs: `search_regex: bool` beside search_contents (300, init at 550); `pub fn searches_regex()` beside 727; `pub fn set_search_regex()` copying set_search_contents (1478-1483); capture it in run_search (1373) and pass to content_search, which sets `mode: if regex { GrepModeKind::Regex } else { GrepModeKind::Plain }` (import GrepModeKind at line 44). Add `regex_error: Option<String>` to SearchStats and set it in both constructors (1411 -> None, 3021 -> response.regex_error).
- src/finder/bars.rs: after the by-contents chip, `.when(contents, |this| this.child(chip("contents-regex", ".*".into(), regex).tooltip(...Tooltip::new("Regular expression")...).on_click(cx.listener(|this, _, _, cx| this.set_search_regex(!this.search_regex, cx))).test_support()))`; same pattern as the scope-folder chip at 85-97 and the filter-toggle tooltip at 130-133. In the timing closure (361-372) append ", invalid regular expression, searched as text" when stats.regex_error is Some, and give the timing div `.id("search-timing").aria_label(...)` plus `.test_support()` like status-text at 392-396.
- src/finder/filter.rs: `regex: self.search_contents && self.search_regex` at 196; `self.search_regex = folder.regex` at 227.
- crates/fffinder-core/src/config.rs: `pub regex: bool` on SmartFolder (old files still load via serde(default)).
- src/finder/dialogs.rs:949-976: optional extra row in the Search Syntax table (array length 11 -> 12).

SERVER (src/server/mod.rs, after engine.grep at 477): `if let Some(err) = response.regex_error { return Err(Failure::bad(format!("invalid regular expression: {err}"))); }`. The nu client already surfaces the body's `error` (nu/fff.nu:70-73).

DOCS
- docs/manual.md: scope-bar table (after line 335), Smart Folder sentence (643), `smart_folders` row (1078), `fff search` row (1132).
- docs/protocol.md:83-84: in regex mode the whole pattern is the expression (no inline globs; use `scope`), and an invalid expression is a 400.
- CHANGELOG.md Unreleased: Added (the chip) and Changed (regex patterns no longer parsed for globs; invalid regex is an error from `fff search`).
- Worth one sentence in the manual: with Letter case = Smart, fff treats any capital in the expression as a request for case, including `\S`, `\W`, `\D`, `\B` (Casing::is_insensitive_for, grep/types.rs:19-25); `(?i)` overrides. Classes are ASCII-only (unicode(false)).

**Tests.** Engine unit test in crates/fffinder-core/src/engine.rs `mod tests`, modelled on search_settings_change_content_search (2252-2306): temp dir, Engine::new(EngineOptions{..}), engine.wait_ready, engine.grep(&GrepRequest{ mode: GrepModeKind::Regex, .. }).
- `sub/t.txt` = "foo food\na/xxb\n", `other.txt` = "a/yyb\n".
- `fo+d` -> 1 hit, regex_error None.
- `(` -> regex_error Some (default settings, content_index off).
- Parser bypass must be proven WITH a scope: pattern `a/.*b`, scope dir.join("sub") -> exactly the sub/t.txt line (0 hits before the change, and other.txt stays out). The unscoped version in the proposal passes today via fff's raw-query retry. A scope-free alternative: file with lines "x !abc" and "x only", pattern `x !abc` -> 1 hit (2 today).
- Leading backslash: file containing "**bold", pattern `\*\*bold` -> 1 hit, regex_error None.

crates/fffinder-core/src/config.rs tests: serde round trip of a Config whose smart_folders holds `SmartFolder { regex: true, contents: true, .. }` via save_to/load_from as in round_trips (859-885), plus loading JSON without the key gives false.

tests/server.rs, inside answers_index_queries_over_a_unix_socket (cfg(unix)), using existing helpers get / status_of / client::json:
- `status_of(&endpoint, "GET", "/v1/search?pattern=%28&mode=regex", None) == 400` and the client::json error text contains "invalid regular expression".
- existing `pattern=atl.s+t&mode=regex` (line 165) still returns 1 row.
- add a scoped regex: `pattern=atl.s&mode=regex&scope=<tree>/Projects` -> 1 row.

tests/controls.rs, new #[gpui_kit::test] next to scope_bar_controls (361), with Fixture::new and Ui::open from tests/common/mod.rs:
- `ui.update(|f, _, cx| f.set_query("atl.s".into(), true, cx)); ui.click("by-contents");` -> `ui.names()` empty, `ui.exists("contents-regex")`.
- `ui.click("contents-regex")` -> `ui.read(|f| f.searches_regex())`, `ui.names() == ["Project Atlas.txt"]`.
- `ui.click("scope-folder")` (folder scope) -> still found.
- `set_query("atl(")` -> `ui.snapshot("search-timing").label()` contains "invalid regular expression".
- `ui.click("by-name")` -> `!ui.exists("contents-regex")`.
No existing UI test asserts content-search rows, so this is the first; the fixture file holds "atlas" with no newline, which the engine test searches_a_folder shows grep handles.

**Risks.** - Documented protocol behaviour changes for regex mode: inline globs in a pattern (protocol.md:83) stop filtering files, and an invalid expression becomes a 400 instead of literal matches. Both only affect `fff search --regex`; the protocol is marked experimental, but it needs the doc and CHANGELOG entries.
- The invalid-regex message and the 400 are best effort unless the engine validates the pattern itself: fff drops the error when no file is a candidate, which happens with `search.content_index` on. Same pattern can then give 400 or an empty 200 depending on the index.
- The `(?:...)` guard means an invalid pattern that also starts with a backslash is "searched as text" in its wrapped form, so it finds nothing. Negligible, but the status text is slightly off in that case.
- Smart case surprises: `todo\S+` is case-sensitive because of the capital S. Upstream behaviour; document it or pre-decide the casing in the engine.
- Search-as-you-type runs a regex scan per pause in typing. Regex mode has no literal prefilter (grep.rs:394), each scan can run to the 5 s time limit, and the engine passes no abort signal, so superseded searches keep running. Plain content search already behaves this way; regex makes each one more expensive.
- Engine::grep's scope handling is also what scoped-search-special-folder-names changes, and other lanes are editing the same files (bars.rs scope bar, mod.rs content_search). Sequence after that candidate or expect a merge conflict in engine.rs:897-959.

**Notes from later reviews.** 'content-match-highlight' lands before this one: the Match column's highlight has to be right for a regular-expression match too. Plain text stays the default, and the search bar must not get busier for people who never use this.

## restore-session-tabs

**Reopen the last window's tabs and split view at launch**

Value 3 of 5, size M, ui, feature.

**Problem.** Quitting saves only where the window was (WindowPlacement, crates/fffinder-core/src/config.rs:510-556) and one `last_folder`, written by whichever tab navigated last (src/finder/mod.rs:918-921). Workspace::new always builds a single tab (src/workspace.rs:55-83), and run() opens one window at start_location() (src/lib.rs:148, 321-338). So a window with several tabs and split view comes back as one tab, at the folder last navigated to in any tab or pane, which may not be the tab that was showing.

**Proposal.** crates/fffinder-core/src/config.rs: add `Session { tabs: Vec<PathBuf>, active: usize, split: Option<PathBuf> }` with load/save to `session.json` beside window.json, using the same `write_file`. src/workspace.rs: add `fn session(&self, cx: &App) -> Session`. Folder tabs are saved as they are; a Recents, Trash or Results tab falls back to its last folder or is skipped. Save it debounced from tabs_changed(), toggle_split() and tab navigation, and from the existing on_release/on_app_quit hooks (64-72), all gated on `settings::persists`. src/workspace.rs: add `Workspace::restore(session, window, cx)`, which builds the tabs, sets `active` and opens the split pane; folders that no longer exist are dropped. src/lib.rs run()/open_window(): for the first window only, restore the session when there is no path or query argument, it isn't `--pick` or `--term`, and the session has at least one existing folder. New Window keeps opening one tab. Update docs/manual.md (Tabs and split view; Files, folders, and environment variables) and CHANGELOG.md.

**Checked plan.** The plan works with these corrections.

1. Gate on the resolved folder. In `run()` (src/lib.rs:43-154), reopen tabs only when `folder.is_none() && args.query.is_none() && picker.is_none() && !args.term`, where `folder` is the value at lib.rs:46 after `shell_folder`. Load the file after `forward_open` returns false. Put the condition in a small pure function so `mod tests` (lib.rs:483) can cover it.

2. Core type, beside `WindowPlacement` (crates/fffinder-core/src/config.rs:510-556): `SavedTabs { tabs: Vec<PathBuf>, active: usize, split: Option<PathBuf> }` with `load`/`load_from`/`save`/`save_to` through `write_file`, at `config_dir().join("session.json")`. Avoid the name `Session`; src/term/tty.rs:80 already has one. Add one pure constructor that takes `Vec<Option<PathBuf>>`, drops the `None`s, moves `active` to the nearest kept tab at or before it, clamps it, and returns `None` when no tab is left. Build `keeping(self, keep: impl Fn(&Path) -> bool)` on it for the `is_dir` check at launch; it also drops a split folder that fails.

3. A tab that isn't showing a folder. `Finder.config.last_folder` is not that tab's folder: a tab opened straight at Recents or the Trash (src/finder/sidebar.rs:720-727) starts with the app-wide value. Add `History::last_folder()` in src/model.rs (the current location, else the newest folder in `back`) and a `Finder` accessor. A tab with none is skipped.

4. src/workspace.rs:
- `saved_tabs(&self, cx: &App) -> Option<SavedTabs>` from each tab's folder, `active`, and the split pane's folder.
- `Workspace::restored(&SavedTabs, window, cx)`, sharing a private constructor with `new`. Build each tab with `new_finder`, and the pane with `new_finder` plus `become_split_pane`. Don't go through `new_tab`/`toggle_split`, which focus and retitle on every call.
- Saving copies `remember_placement` (workspace.rs:92-121): a `tabs_unsaved` flag set only when `settings::persists(cx)`, a 500 ms task, and a flush in the existing `on_release`/`on_app_quit` closures only if the flag is still set. Writing unconditionally there lets two open windows overwrite each other in release order at quit.
- Mark unsaved from `tabs_changed()`, `toggle_split()`, a tab's navigation, and when the window becomes active (`cx.observe_window_activation`, next to `observe_window_bounds`). The last one makes the window you went back to win after you close another.
- Every save path, the debounced one included, needs the condition `_placement_watches` uses (workspace.rs:64): no picker and not `term::active()`. A `--pick` window navigates too and shares the settings folder.
- Don't write when `saved_tabs` is `None`; a window showing only `fff show` results would otherwise wipe the saved tabs.

5. Navigation. At the end of `Finder::location_changed` (src/finder/mod.rs:900-926), tell the host with `cx.defer`, as `tell_others` does (src/finder/tab_bar.rs:71-79). Never read the tabs from inside a `Finder` update; that entity is leased then.

6. src/lib.rs: move the body of `open_window` (lib.rs:435-481) into a private helper that takes the closure building the `Workspace`, and add `open_saved_tabs(saved, cx)`. Keep `open_window`'s signature; examples/drive.rs:42 and tests/settings_file.rs:211 call it. `on_reopen`, New Window, and the control socket's opens stay one tab.

7. Docs: docs/manual.md:74 (Starting fffinder), :154-162 (Tabs and split view), :1086 (`last_folder`), :1170 (add `session.json`), and CHANGELOG.md under [Unreleased] › Added. Say what isn't reopened: history, searches, filters, selection, and tabs showing Recents, the Trash, or results with no folder behind them.

Pitfalls:
- examples/drive.rs:42-45 and examples/screenshot.rs:169-191 run on the real settings folder and turn writing off only after the window is built. Building a `Workspace` must not save or schedule a save.
- serde refuses a non-UTF-8 `PathBuf`, which would fail the whole save; skip such a tab.

**Tests.** All of it runs headless on Linux.

1. Core unit tests in crates/fffinder-core/src/config.rs `mod tests`, beside `window_placement_round_trips` (config.rs:1089-1107):
- `save_to`/`load_from` round trip in a temp file; a missing or garbled file loads as `None`.
- The constructor and `keeping`: a dropped active tab moves `active` to the kept tab before it, an out-of-range `active` is clamped, a dropped split folder leaves no split, and no tabs left gives `None`.

2. src/model.rs `mod tests`, beside `history_back_forward` (model.rs:391): `History::last_folder` after folder → Recents is the folder, and `None` for a history that starts at Recents.

3. src/lib.rs `mod tests` (lib.rs:483): the gate function is false with a folder, a query, `--pick`, or `--term`, and true with none of them.

4. Headless UI test in tests/file_ops.rs, beside `tabs_and_split_view` (file_ops.rs:1030-1124):
- Open with `Ui::open_with(cx, &fx.dir, Open { workspace: true, ..Default::default() })`.
- Add tabs with `ui.window(|window, cx| workspace.update(cx, |w, cx| w.new_tab(Location::Folder(..), window, cx)))` at `fx.path("Projects")` and `fx.path("Projects/atlas")`, then `w.select_tab(1, ..)` and `ui.settle()`.
- Turn on split view with `ui.press(&cmd("\\"))` and move the pane with `pane.update(cx, |f, cx| f.navigate(..))`.
- Assert `workspace.read_with(ui.cx, |w, cx| w.saved_tabs(cx))` lists the three folders, `active == 1`, and the pane's folder.
- Open a second window from it the way tests/common/mod.rs:198-209 does: `gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| cx.new(|cx| Workspace::restored(&saved, window, cx)))`. Assert `tabs().len()`, `active_index()`, each tab's `current_folder()`, and the split pane's folder.
- `std::fs::remove_dir_all(fx.path("Projects/atlas"))`, apply `keeping(|p| p.is_dir())`, restore again, and assert two tabs.

5. Writing to the file, in tests/settings_file.rs, the only test file that turns writing on. Use `take_turn()` and `open_writing`, and remove `session.json` first as the `window.json` tests do (settings_file.rs:159, 181); the shared folder is named by process ID and isn't cleared.
- `ui.press(&cmd("t"))` settles for 600 ms, past the 500 ms wait, so `SavedTabs::load()` then has two tabs.
- Add a tab through `ui.window(..)` without settling, assert nothing is saved yet, call `ui.cx.quit()`, and assert it was saved. The model is `quitting_right_after_resizing_saves_the_windows_place` (settings_file.rs:176-189). The test macro's own teardown does not run quit handlers; only `TestAppContext::quit()` does.
- The same for closing: `ui.workspace = None`, `remove_window()`, `run_until_parked()`, as at settings_file.rs:154-174.
- Nothing is written with writing off (plain `Ui::open_with`), nor from `Open { picker: Some(PickerOptions { multiple: false }), workspace: true, .. }` with writing on after `ui.go_to(..)`.

Existing tests that must stay green: `tabs_and_split_view`, the tests in tests/settings.rs, and `a_file_chooser_opens_in_the_middle_and_a_browser_where_the_last_was` (settings_file.rs:191-228), which calls `fffinder::open_window` directly and must not reopen saved tabs.

**Risks.** - It changes documented launch behaviour with no switch to turn it off. A launch from the Dock, Explorer, or a desktop menu now reopens every tab instead of one window at the last folder (docs/manual.md:74). One-tab users see the same folder as before. A `settings.json` field would be a cheap follow-up; a Settings switch costs more, since tests/controls.rs clicks every control.
- Launch gets heavier with many tabs. Each restored tab is a whole `Finder`: a listing, a read of the volumes, a `nu` run to find actions (and `nu --login` when Nushell's environment is on), and a two-second watch loop, all started at once.
- A one-off window replaces the saved tabs. Launching with a PATH, or from a terminal, opens one tab, and that window's tabs are then what is saved.
- With several windows, the one used last wins. On macOS, closing the last window and clicking the Dock icon still opens one tab; only a real relaunch reopens the tabs.
- `is_dir()` runs on the main thread for every saved folder before the window opens. A stale network mount can delay launch, as the single `last_folder` check already can.
- A save path left ungated would write `session.json` from `--pick` and `--term` runs, which share the settings folder, or from the example tools that run on the real settings.
- Each navigation now writes two small files after its debounce, `settings.json` and `session.json`.

**Notes from later reviews.** The tab and split view changes ('tab-bar-overflow', 'tab-drop-and-close', 'split-view-toolbar-clip', 'split-active-pane') land before this one; build on them. Restoring must never leave a tab on a folder that is gone, must not slow the launch (nothing slow on the window's thread), and must not change what `fffinder PATH`, `--pick` or a second launch open. From the reviewer of 'query-scope-from-cli' (on the trunk; read its done record): Workspace::move_tab_to_new_window carries only the place and the words, so the new window loses the tab's Search Only in, Contents and filter, and a tab that `--here` narrowed searches every location after Move Tab to New Window. If you build a description of a tab's state for restoring, use it there too so the moved tab keeps all of it; give that its own test and its own sentence in the commit message. If it doesn't fit, say so in your notes.

## search-roots-notes-dedupe

**Settings › Search locations: say why a listed folder isn't indexed, and stop adding duplicates**

Value 3 of 5, size S, search, bug.

**Problem.** The engine quietly drops some of the folders Settings lists. `Engine::set_roots` runs `normalize_roots` (crates/fffinder-core/src/engine.rs:322-339, 1727-1741), which skips any root that isn't a directory and any root inside another root. `config.search_roots` keeps them all, and the Settings list draws one row per configured root, finding its count with `status.iter().find(|s| &s.path == root)` and drawing nothing when there is no status (src/finder/dialogs.rs:358-367, 377). So with the home folder indexed (the default), Add Folder... -> `~/Projects` produces a row that looks like a search location but has no file count and no explanation. tests/controls.rs:652-660 builds exactly this state (fixture dir plus `Projects` inside it). A root that was deleted or is on an unmounted volume looks the same. Separately, `prompt_add_root` de-duplicates with `roots.dedup()` (dialogs.rs:629-632), which only removes adjacent repeats. With roots `[~, /Volumes/X]`, adding `~` again gives three rows, two of them identical, and the X on either removes both because removal filters by value (dialogs.rs:385-387).

**Proposal.** In `prompt_add_root`, replace `extend` + `dedup` with: for each picked path, push it only if no existing root equals it once trailing separators are trimmed. Add a pure helper `fn root_note(root: &Path, roots: &[PathBuf], missing: bool) -> Option<String>`: another configured root is a strict ancestor -> 'inside <tildified ancestor>, already searched'; `missing` -> 'folder not found'. In the root-list builder (dialogs.rs:358-391), when no status exists for a root, draw that note in the muted 11px slot where the file count normally goes, with id `("root-note", ix)`. To avoid a stat per frame, compute the set of missing roots in `show_settings` and again in `set_search_roots`, on the background executor, and keep it in a `Finder` field.

**Checked plan.** **1. Dedupe (src/finder/dialogs.rs `prompt_add_root`, 626-637).** Replace `extend` + `dedup` with `for p in paths { if !roots.contains(&p) { roots.push(p) } }`. No separator trimming is needed. Keep nested roots in the config; tests/controls.rs:658-661 pins that.

**2. Pure helper `fn root_note(root: &Path, roots: &[PathBuf], missing: bool) -> Option<String>`.** Put it in dialogs.rs, or next to `tildify` at src/finder/mod.rs:191.
- `missing` wins and returns "folder not found". A child of a missing ancestor is missing too.
- Otherwise mirror `normalize_roots` exactly: among `roots` with `r != root && root.starts_with(r)`, take the outermost (fewest components).
- Word it "searched as part of {tildify(ancestor)}" rather than "already searched". Per docs/manual.md:374-378 a nested git repository does not get its own hidden-file index, so "already searched" over-promises.

**3. Row (dialogs.rs:358-391).** Keep the count when a status exists. Otherwise draw the note:
`div().id(("root-note", ix)).aria_label(SharedString::from(note.clone())).text_size(ui(11.)).text_color(muted).child(note).test_support()`
The path label is `flex_1().truncate()`, so cap or truncate the note; a long ancestor path would otherwise squeeze the folder name to nothing.

**4. Missing set, without a second task.** Add a `missing_roots: Vec<PathBuf>` field beside `index_status` (mod.rs:401, initialised near 623).
- Compute it inside the existing background closure of `index_search_roots` (mod.rs:1558-1566), right after `engine.set_roots(&roots)`: `roots.into_iter().filter(|r| !r.is_dir()).collect()`.
- Store it in the following `this.update`, together with `poll_index_status`, and call `cx.notify()`.
- In `show_settings` (next to `self.detect_terminals(cx)`, dialogs.rs:324) call `self.index_search_roots(cx)` when `!crate::disk_access::index_held(cx)`.

This keeps the note taken at the same instant as the engine's decision. It also makes opening Settings pick up a volume that came back, since `set_roots` keeps unchanged indexes (engine.rs:328-330), and `Engine::generation()` has no readers in src/ or crates/. `set_search_roots` already calls `index_search_roots`, so Add, Remove and Reset refresh it.

**Pitfalls.**
- The dialog builder closure runs every frame, so never stat in it; read `finder.missing_roots` only.
- Do not stat while indexing is held for Full Disk Access; nested notes are pure and still show.
- The note must not depend on "no status yet". `roots_task` lands a moment after Add Folder.
- A new `mod tests` in dialogs.rs must import by name, not `super::*`, because the file glob-imports `gpui_kit::*` (see the comment at mod.rs:3434).

**Docs.** Add a sentence at docs/manual.md:977 and a Fixed entry under `[Unreleased]` in CHANGELOG.md.

**Tests.** **Headless UI, tests/controls.rs `settings_dialog_controls` (619-695).** Helpers from tests/common/mod.rs: `Fixture::new`, `Ui::open`, `ui.press(&cmd(","))`, `ui.click`, `ui.settle`, `ui.read`, `ui.update`, `ui.exists`, `ui.snapshot(..).label()`, plus `ui.cx.did_prompt_for_paths()` and `ui.cx.simulate_path_prompt_response`.

After the existing assertion at 658-661 (roots are `[fx.dir, projects]`):
1. `assert!(!ui.exists(("root-note", 0usize)))` — the fixture root shows its count.
2. `ui.snapshot(("root-note", 1usize)).label()` contains "part of".
3. `ui.click("add-root")`, answer with `Some(vec![fx.dir.clone()])`, `ui.settle()`, and assert the roots are still `[fx.dir, projects]`. This fails today with three entries.

Leave the existing `remove-root` step at 662 as is.

**Missing folder.** Add a small separate test (or do it before 662 and restore the list): `ui.update(|f, _, cx| f.set_search_roots(vec![root.clone(), fx.path("gone")], cx))` with Settings open, then assert `ui.snapshot(("root-note", 1usize)).label()` contains "not found". The background `is_dir` runs on the test executor and `ui.update` settles it.

**Unit test for `root_note`** (std `#[test]`, in the mod.rs tests module at 3432 or a new by-name-import module in dialogs.rs). Use paths outside the home folder so `tildify` is deterministic:
- `/a/b` with `[/a, /a/b]` gives a note naming `/a`.
- The root itself or an equal path gives `None`.
- Sibling prefix `/a/bc` against `/a/b` gives `None`.
- A trailing-slash spelling still matches.
- `[/, /a, /a/b]` picks the outermost ancestor.
- `missing = true` returns "folder not found" and wins over nested.

The existing `roots_are_normalized` test (engine.rs:1808) stays untouched.

**Risks.** - **Behaviour change on opening Settings.** It now re-applies the roots when indexing is not held. This is a no-op for unchanged roots, but a volume that came back starts indexing at that moment; say so in the changelog. The existing test at tests/controls.rs:668-683 is unaffected: it swaps the fixture root back before `roots_task` runs and never opens Settings while the disk or home is configured.
- **Other windows.** `missing_roots` is per-Finder, so another window's copy is stale until that window opens Settings. It is only read in the dialog, so this is acceptable.
- **Windows letter case.** `C:\Users\Me` and `c:\users\me` are not deduped or seen as nested. That matches the engine's own `starts_with`, so the note and the engine stay consistent.
- **Hand-edited settings.json.** Duplicates written there still render as twin rows; only Add Folder is fixed.
- **Layout.** A long note can crowd the path label unless it is capped or truncated.
- **Rescan.** It still does not retry missing roots by itself. Only the reopen-Settings path heals, unless `Finder::rescan` (mod.rs:1615) is also made to call `index_search_roots`.

**Notes from later reviews.** From the reviewer of 'choose-nu-path' (on the trunk; read its done record's note for this brief): Add Folder… in Settings does nothing, and says nothing, when the file chooser can't open (`prompt_add_root` in src/finder/dialogs.rs drops the error), which is the case in `--term` and on Linux without a desktop portal. `prompt_choose_nu`, right after it, shows a Warning notification in that case ('Couldn’t open the file chooser: <reason>'). You are editing `prompt_add_root`: make it say the same, with a test if the harness can stage it (the reviewer found it cannot answer a path prompt with an error; if so, say so and keep the change to the few lines that mirror `prompt_choose_nu`).

## search-settings-hints-steppers

**Settings › Search engine: correct the 'rebuilds the indexes' hint, explain the switches, and fix the steppers**

Value 3 of 5, size M, search, ux.

**Problem.** Three defects in one section of Settings. 1. Wrong hint. The section says 'fff-search's options. Changing a switch rebuilds the indexes.' (src/finder/dialogs.rs:484) above six switches drawn as one wrapping row (dialogs.rs:46-78, 97-111). The sixth, 'Allow typos' (dialogs.rs:77), is a per-search option: `SearchSettings::index_options` holds only the first five (crates/fffinder-core/src/config.rs:387-395) and `Engine::set_search_settings` re-indexes only when those differ (crates/fffinder-core/src/engine.rs:372-385). The manual gets it right ('the first five switches', docs/manual.md:1002). The dialog therefore warns of a full re-scan for the one switch a user is most likely to flip while searching. 2. No explanations. None of the switches has a tooltip (dialogs.rs:98-110), so 'Keep often-used files in memory', 'Index file contents' and 'Watch folders for changes' are only explained in the manual (manual.md:990-995). 3. Stepper behaviour. In `choice_stepper` the value between − and + is a ghost `Button` with no click handler (dialogs.rs:229-235), so it highlights on hover and does nothing, while the text-size steppers in the same dialog reset when their value is clicked (src/finder/preview.rs:660-692). For a hand-edited value between choices, `at = position(c >= current)` (dialogs.rs:207-211) makes + skip a step: `result_limit` 300 puts `at` on 500, so + goes straight to 1000 (dialogs.rs:237-241).

**Proposal.** In `search_engine_controls` (dialogs.rs:38-196): split the switches into two rows. 'Indexing': the index switches under a muted caption with id `index-switch-hint`, 'Changing one of these rebuilds every index'. 'Matching': Allow typos beside the Letter case row. Replace the sentence at dialogs.rs:484 with plain 'fff-search's options.' Give each `Switch` a `.tooltip(...)` with the manual's one-line description (`Switch::tooltip` is already used at dialogs.rs:418). Give `choice_stepper` a `default: u64` argument (500 results, 10 MB, 5 s, Auto threads); clicking `{id}-value` picks the default, with tooltip 'Reset to <label(default)>', disabled when already there. Compute neighbours from the value, not from `at`: less = `choices.iter().rposition(|&c| c < current)`, more = `choices.iter().position(|&c| c > current)`. Put this in a small pure function so it can be unit tested.

**Checked plan.** All UI code is in src/finder/dialogs.rs.

**A. Hint and grouping (`search_engine_controls`, dialogs.rs:38-196)**
- Cut the `switches` array to the five index switches. Keep the ids `follow-symlinks`, `watch-changes`, `git-recency`, `content-index`, `preload-files` and keep `h_flex().flex_wrap()`.
- Do not add a second caption line. Replace the sentence at dialogs.rs:484 with the index hint, directly above the five: `div().id("index-switch-hint").test_support()` with text "fff-search's options. Changing one of these switches rebuilds every index." The `.test_support()` is required for the harness to find it.
- Move Allow typos into its own row just before "Letter case in searches": `setting_row("Allow typos", "Show near misses when no name has the letters in order", Switch::new("typos").small().accessibility_label("Allow typos").checked(search.typos).on_change(..update_search_settings(|s| s.typos = on)..), muted)`. A switch without a label inside a row has precedent at dialogs.rs:415. This row also gives typos its explanation without a tooltip.

**B. Tooltips on the five index switches**
- Do not use `Switch::tooltip`; it covers only the track.
- Wrap each switch: `div().id(SharedString::from(format!("{id}-tip"))).child(switch).tooltip(move |window, cx| Tooltip::new(tip).build(window, cx))`. Import `gpui_kit::component::tooltip::Tooltip`; the pattern is at src/finder/sidebar.rs:686.
- Keep tips to about 70 characters, since `Tooltip::new` does not wrap (dialogs.rs:456 is a precedent of that length):
  - "Index what symbolic links point to"
  - "Keep indexes current while fffinder runs; off, they update on Rescan"
  - "In a git repository, files from the last 10 commits rank higher"
  - "Content searches skip non-matching files sooner; uses more memory"
  - "Load the most-opened files into memory for faster content search"
- For a hover test, build the tooltip as fit.rs:59-67 does: `Tooltip::element` with `div().id("setting-tip").aria_label(tip).max_w(..).test_support()`.

**C. Steppers (`choice_stepper`, dialogs.rs:200-242)**
- Add a pure helper: `fn neighbours(choices: &[u64], current: u64) -> (Option<u64>, Option<u64>)`, returning `(choices.iter().rev().find(|&&c| c < current).copied(), choices.iter().find(|&&c| c > current).copied())`.
- Change `step` to take `Option<u64>` and drop `at`. Replace the comment at dialogs.rs:207.
- Add a `default: u64` parameter, threaded through the `pick` closure (dialogs.rs:84-94).
- Take defaults from the core, not literals: `fffinder_core::Config::default().result_limit as u64`, and `SearchSettings::default()` for `max_file_size_mb`, `time_limit_secs`, `threads`. These are 500, 10, 5 and 0, and each is in its choice list.
- Value button: `.disabled(current == default)`, `.tooltip(format!("Reset to {}", label(default)))`, `.on_click(pick(default))`. Keep the `{id}-value` id and `label(current)`.
- A value at its default will draw in the disabled colour. "100%" and "12 pt" already do this in the same dialog.

**D. Docs**
- docs/manual.md:986-1004: reword line 1004 to match the grouping ("Changing one of the five switches at the top rebuilds every index…"). Add that the value between − and + resets to the default when clicked.
- CHANGELOG.md [Unreleased]: add Fixed entries for the + skip and the dead value button. The hint needs no entry, because the switch that broke it is itself unreleased.

**Pitfalls**
- B and C add no height, but the new Allow typos row adds about 42 px at 100%. The test window is 1536x1080 and the Settings dialog already scrolls there; tests/folder_tree.rs:448 scrolls before clicking `sidebar-list`.
- Tests that click lower controls without scrolling may land below the fold. `settings_text_sizes` clicks `nu-text-bigger` and `nu-text-reset` at 110% scale (tests/controls.rs:730-733); others click `use-nu-env` (controls.rs:1716) and `terminal-app` / `terminal-input` (controls.rs:831-892).
- macOS has less headroom because of the Applications row and the PRIVACY section.
- If any of those fail, add `ui.scroll(<id>, point(px(0.), px(-2000.)))` before the click, as folder_tree.rs:448 does. Do not drop the row.
- The zero-height fallback is a text-only fix at dialogs.rs:484: "Changing any switch but Allow typos rebuilds the indexes."
- Coordinate with the candidate `search-settings-without-effect`, which may remove `preload-files`.

**Tests.** Headless UI tests go in tests/controls.rs, using `Fixture`, `Ui::open`, `ui.press(&cmd(","))`, `ui.click`, `ui.snapshot(..).label()`, `ui.exists`, `ui.update`, `ui.read` and `ui.close_dialog` from tests/common/mod.rs.

1. **Extend `settings_search_engine` (controls.rs:743).**
   - After line 770, click `time-limit-value`. Assert `search(&ui).time_limit_secs == 5`, the label is "5 s", and `ui.snapshot("time-limit-value").disabled() == Some(true)`.
   - Assert `ui.exists("index-switch-hint")`.
   - The existing `ui.click("typos")` and `engine.search_settings() == search(&ui)` assertions must still pass.

2. **New test `settings_steppers_step_from_values_between_choices`.**
   - Set the value before opening the dialog, because the dialog only redraws on `window.refresh()`.
   - `ui.update(|f, _, cx| f.set_result_limit(300, cx))`, open Settings, assert `result-limit-value` reads "300", click `result-limit-more`, assert 500.
   - Close, set 300 again, reopen, click `result-limit-less`, assert 200.
   - Close, set 9000, reopen, click `result-limit-less`, assert 5000.

3. **Optional hover test** (needs the `Tooltip::element` form with id `setting-tip`): `ui.hover("watch-changes")`, then assert `ui.snapshot("setting-tip").label()`. Model it on `cut_off_text_shows_in_a_tooltip` in tests/columns.rs:136. This proves the tooltip shows over the label, not only the track.

4. **Drop the proposed `engine.is_scanning()` assertion.** If a no-rebuild check is wanted, put it in the core unit test `typos_can_be_turned_off` (crates/fffinder-core/src/engine.rs:2047): read `engine.generation()` before `set_search_settings` and assert it rose by exactly 1. A rebuild also goes through `set_roots` (engine.rs:338), so it would rise by 2.

5. **Unit test for `neighbours`** in a new `#[cfg(test)] mod tests` at the bottom of src/finder/dialogs.rs (the style is at src/finder/fit.rs:129). Over `[100, 200, 500, 1000]`:
   - 300 gives (200, 500)
   - 50 gives (None, 100)
   - 9000 gives (1000, None)
   - 200 gives (100, 500)
   - 100 gives (None, 200)
   - 1000 gives (500, None)

6. **Re-run for the scroll-fold risk:** `settings_text_sizes`, `settings_terminal_choice` and the Nushell-environment test around controls.rs:1716, plus `the_sidebar_switches_between_list_and_tree` in tests/folder_tree.rs.

**Risks.** - **Dialog height.** The Allow typos row adds about 42 px to a dialog that already scrolls in the 1536x1080 test window. Lower controls clicked without scrolling (`nu-text-*` at 110% scale, `use-nu-env`, `terminal-app`) may land below the fold; macOS has the least headroom. I could not run the tests to measure it. The fix is a `ui.scroll` before the click, or the text-only hint fallback.
- **Tooltip placement.** `Switch::tooltip` covers only the track, so the tooltips must go on a wrapper div. A long manual sentence in `Tooltip::new` would not wrap, so keep tips short.
- **Disabled look.** With reset-on-click, a value at its default draws in the disabled colour, as "100%" and "12 pt" already do. Most users will see all four values dimmed.
- **Accidental reset.** A stray click on a value now resets it silently. The settings are cheap to restore and the tooltip says what the click does.
- **Overlap with another candidate.** `search-settings-without-effect` may remove `preload-files`, which changes the switch count and the tooltip list.

## search-settings-without-effect

**Two Search engine settings do not do what they say: file-size limit above 10 MB, and 'Keep often-used files in memory'**

Value 3 of 5, size S, search, bug.

**Problem.** (1) 'Content search skips files over' offers 1 to 1000 MB (src/finder/dialogs.rs:157-163; docs/manual.md:1000) and Engine::grep passes it as GrepSearchOptions::max_file_size (crates/fffinder-core/src/engine.rs:918). fff uses that value only to pre-filter the file list; when it reads a file it checks the picker's own ContentCacheBudget instead: `let max_file_size = budget.max_file_size; if ... self.size > max_file_size { return None; }` (fff-search types.rs:786-788, and 725 for the cached path). RootIndex::start passes no cache_budget (engine.rs:62-77, `..Default::default()`), so the budget is auto-sized with max_file_size fixed at MAX_FFFILE_SIZE = 10 MB (types.rs:978-1002, constants.rs:5). Any setting above 10 MB is silently ignored: a 12 MB log is never searched at 20, 100 or 1000 MB. The existing test only uses a 2 MB file (engine.rs:2260, 2298-2303). (2) 'Keep often-used files in memory' (search.preload_files, dialogs.rs:71-76, manual.md:996) maps to FilePickerOptions::enable_mmap_cache (engine.rs:68), whose only effect in fff-search 0.11.0 is commented out (scan.rs:355-358, 'Skipped as potentially unsafe'). Yet it is one of index_options() (config.rs:387-394), so flipping the switch throws away and rebuilds every index (engine.rs:372-383): a rescan plus minutes of 'Finishing index...' for no change.

**Proposal.** (1) Make the limit real. In Engine::grep, before taking the read guard for a root, compare `picker.cache_budget().max_file_size` with the wanted bytes; when they differ take the write lock briefly and call the public `picker.set_cache_budget(ContentCacheBudget { max_files, max_bytes, max_file_size: wanted, cached_count: AtomicUsize::new(old count), cached_bytes: AtomicU64::new(old bytes) })`, copying the other fields from the current budget (all fields are pub). Doing it lazily at grep time keeps fff's size-based auto budget and survives rescans, which reset the budget (scan.rs:198-200). Extend the unit test with a file over 10 MB. If the owner prefers the smaller change, instead cut the Settings choices to [1, 2, 5, 10], clamp on load, and say so in the manual. (2) Remove preload_files from index_options() (make it [bool; 4]) so it no longer rebuilds, stop passing enable_mmap_cache, drop the switch from the `switches` array in dialogs.rs, keep the serde field so old settings files still load, and update manual.md rows 996 and 1097.

**Checked plan.** The proposal works as written; these are refinements.

**(1) Make the limit real** — crates/fffinder-core/src/engine.rs
- Add a helper on `RootIndex`, e.g. `fn allow_file_size(&self, bytes: u64)`, and call it in the `Engine::grep` loop just before `root.picker.read()` (around line 890). This also covers ad-hoc indexes.
- Only ever raise the budget: `let want = bytes.max(fff_search::constants::MAX_FFFILE_SIZE)`. Settings of 10 MB or less are already enforced by fff's prefilter, so default users never take a write lock.
- Check `picker.cache_budget().max_file_size != want` under the read lock first. If it differs, drop the read guard, take `root.picker.write()`, and re-check.
- Build the replacement before calling `set_cache_budget` (borrow order): `ContentCacheBudget { max_files, max_bytes, max_file_size: want, cached_count: AtomicUsize::new(old.cached_count.load(Relaxed)), cached_bytes: AtomicU64::new(old.cached_bytes.load(Relaxed)) }`. All fields are pub and the type is re-exported (`lib.rs:136-137`).
- Carrying the counters over matters, because existing cached mmaps decrement them on invalidation (`types.rs:645-650`).
- Do not pass `FilePickerOptions::cache_budget` instead. That sets `has_explicit_cache_budget`, which loses the size-based auto budget and cannot follow later setting changes.
- A brief engine-side write lock has precedent in `track_open` (engine.rs:1303).
- Keep 20 in the choice list; tests/controls.rs:774 pins 10 to 20.

**(2) Remove the dead switch**
- crates/fffinder-core/src/config.rs: make `index_options()` a `[bool; 4]` without `preload_files`.
- The field can be deleted outright rather than kept. `SearchSettings` has `#[serde(default)]` and no `deny_unknown_fields` (config.rs:340), and loading is plain `serde_json::from_str` (config.rs:644), so an old `"preload_files"` key is ignored. Keeping the field also works but leaves a dead key in every saved file.
- engine.rs:68: drop the `enable_mmap_cache` line (the fff default is false). Fix the doc comment at engine.rs:370-371, which lists "preloading".
- src/finder/dialogs.rs:46 and 71-76: the array becomes `; 5]` and the `preload-files` tuple goes.

**Docs**
- docs/manual.md: delete rows 996 and 1097; change line 1004 to "first four switches".
- manual.md:1000: add that files over 10 MB are only skipped as binary by extension, so a large binary with an unknown extension is searched as text.
- CHANGELOG.md `[Unreleased]`: a Fixed entry for the limit and a Removed entry for the switch.

**Optional hardening**
- In `Engine::grep`, drop hits whose file is over 10 MB and whose line contains NUL.

**Tests.** All of this runs on Linux; nothing is platform-specific.

**Engine unit test** — extend `search_settings_change_content_search` in crates/fffinder-core/src/engine.rs:2253, reusing its `grep` and `set` closures.
- Before `Engine::new`, write `huge.txt`: "needle\n" followed by about 12 MB of text. Newline-separated lines of x are safer than one 12 MB line.
- The existing asserts stay valid, because `huge.txt` is prefiltered out at 1 and 10 MB.
- Append: `set(max_file_size_mb = 20)`, then `grep("needle") == ["big.txt", "huge.txt", "small.txt"]`. This is the assert that should fail today.
- Then `set(10)` and expect `["big.txt", "small.txt"]`.
- For the "survives a rescan" case, do not rely on `engine.rescan()`. `ScanJob::new_rescan` returns `None` while the post-scan phase is active (`scan.rs:73-81`), so the test would not reliably hit the reset.
- Instead reset the budget directly. The tests module can reach private fields: `engine.read_indexes().roots[0].picker.write()`, then `set_cache_budget(ContentCacheBudget::default())`. With the setting at 20, grep again and expect `huge.txt`.

**Config unit tests** — crates/fffinder-core/src/config.rs, `mod tests` at line 855.
- `serde_json::from_str::<SearchSettings>(r#"{"preload_files":true,"content_index":true}"#)` parses and has `content_index == true`, so old files still load.
- `index_options()` differs when `content_index` differs. If the field is kept, also assert it is equal for two settings differing only in `preload_files`.
- Skip the proposed "does not restart indexing" engine test. `is_scanning()` right after a rebuild of a four-file folder is a race in the failing direction and adds little over the config test.

**Headless UI test** — tests/controls.rs, `settings_search_engine` at line 743, using `Ui` from tests/common/mod.rs.
- After `ui.press(&cmd(","))`, add `assert!(!ui.exists("preload-files"))`. `Ui::exists` is at common/mod.rs:634.
- Keep the existing `ui.click("max-file-size-more")` and `assert_eq!(search(&ui).max_file_size_mb, 20)`.

**Risks.** All of these apply only once a user raises the limit above 10 MB; the default path is unchanged. The proposal named the binary-sniff and cache-share risks; the other three are additions.

- **Unsniffed binaries.** The initial scan only sniffs files up to 10 MB (`bigram_filter.rs:1180`) and the searcher has no binary detection. Larger binaries without a known extension (`.pack`, `.img`, `.dat`) are searched as text. Matched lines are cut to 512 bytes (`sink.rs:7`). The cost is bounded by the time limit, so searches may end with "stopped at the time limit" more often.
- **Windows memory (not in the proposal).** fff has no mmap path on Windows. `get_content_for_search` does `buf.resize(size)` plus `read_exact` into a per-worker Vec (`types.rs:803-808`), so a 500 or 1000 MB limit can transiently allocate that much per search thread. Either document it or drop the top choices; `choice_stepper` tolerates saved values that are off the list (dialogs.rs:207-211).
- **Persistent mmap of large files (proposal mentions only the budget share).** The same budget field gates fff's persistent mmap cache (`types.rs:725`), and the public API cannot separate the two. Files up to the new limit stay mapped until the watcher invalidates them, within `max_bytes` of 128 to 512 MB. They crowd small files out of the cache. They also widen the existing SIGBUS-on-truncate exposure that fff's own comment notes (`types.rs:740-742`), mostly for rotated logs and with Watch folders off.
- **Write-lock wait (not in the proposal).** The first grep after a settings change or a rescan takes the picker write lock. It waits for any in-flight grep, up to the time limit, and parking_lot holds new readers behind it meanwhile. This is the same contention fff's scan commit and watcher already cause.
- **Removing the switch.** A saved `preload_files: true` is silently dropped. If a later fff release re-enables warmup, the switch has to be re-added deliberately.

## search-start-no-flash

**Starting a search flashes placeholder bars for a few frames before the results**

Value 3 of 5, size S, search, ux.

**Problem.** Raised as a doubt by the reviewer of 'search-skeleton-flash' (done record: /home/user/polish-tools/done/search-skeleton-flash.json, 'One doubt for the owner'). Since that change, the first letter of a search takes the folder's items away at once and the file area shows placeholder bars until the first answer. A name search answers about 55 to 85 ms later (the 50 ms pause after typing plus the search), so every search now starts with a flash of placeholder bars for a few frames, most visible in icon view. Before that change the folder's items simply stayed until the results replaced them, which was calmer, though for a slow search (contents, or while indexing) they stayed for seconds and looked like results. Loading indicators that show for under about 150 ms read as flicker.

**Proposal.** Give the first answer a short grace period. At the first letter of a search (set_query's branch for a search that was not showing, where drop_stale_results is called today) keep the rows that are there and start a timer of about 150 ms tied to the search generation. If the first answer arrives before it fires, the results replace the rows directly, as before 'search-skeleton-flash'. If it fires first and the search still awaits its first results, drop the rows then (drop_stale_results), so a slow search shows placeholders and never leaves stale rows up for long. Use the app's executor timer as run_search's 50 ms pause does, so tests can drive it with the harness clock. Keep the immediate drop at a Name/Contents switch: there the columns change meaning and stale rows are wrong, not just old. Return and Down Arrow already wait for the results (results_pending), so stale rows on screen for 150 ms cannot be acted on from the search field; check what a click on a stale row during the grace period does and make sure nothing breaks (it was possible before 'search-skeleton-flash' too). Update the tests that pin the immediate drop (the_first_letter_of_a_search_takes_the_folders_items_away, a_filter_waits_for_the_rows_a_search_has_yet_to_find) to pin the new rule instead, and the manual's sentence.

**Tests.** Headless UI tests with the harness clock: (1) first letter typed, clock advanced less than the grace period, the answer arrives: the folder's items are replaced by the results and placeholders were never drawn; (2) first letter typed, the answer held back past the grace period (a contents search, or the test hook the earlier change added): the rows are dropped and placeholders show; (3) the query is cleared during the grace period: the folder's items are still there and nothing is dropped later; (4) a Name/Contents switch still drops at once.

**Risks.** A timer that outlives its search must do nothing: tie it to the search generation and to the search still awaiting its first results. The Nushell filter must not run over the folder's rows as if they were results during the grace period (the earlier change added a guard for the empty rows of a search that has not answered; see how it interacts). The selection: it was cleared with the rows; decide what happens to it during the grace period and keep 'search-enter-before-results' working. Quick Look and the preview pane follow the selection.

**Notes from later reviews.** This refines 'search-skeleton-flash', which is on the trunk: read its commit and done record first. 'results-scroll-reset', 'empty-state-messages' and 'collapsed-groups-empty-view' work in the same functions and should be on the trunk before this one; build on them.

## select-all-perf

**Select All in a large folder makes every frame and every refresh scale with the selection size**

Value 3 of 5, size M, ui, perf.

**Problem.** Two hot paths are proportional to the number of selected items. (1) Per frame: `item_drag` builds the drag payload eagerly for every visible item while drawing; for a selected item it calls `self.selected_entries()` and collects all paths (src/finder/views.rs:279-289), and `selected_entries` clones a whole `FileEntry` (PathBuf + String + metadata) per selected row (src/finder/mod.rs:1765-1773). It is called for every visible list row (views.rs:931), icon tile (views.rs:528) and gallery tile (views.rs:1314). After Cmd+A in a folder of N items that is (visible items) x N entry clones per frame: about 40 x 10,000 = 400,000 clones (twice that in allocations) for every scroll frame in the list, more in the icon grid. (2) Per rebuild: `Selection::remap` does a linear `new.iter().position(..)` for every selected index (src/model.rs:324-336), i.e. O(selected x rows) path comparisons on the UI thread each time `rebuild_rows` runs (mod.rs:1219, 1268-1272): on every sort click, folder-stats arrival, expanded-folder load, filter result and the 2-second change watcher. With everything selected in a 20,000-item folder that is about 2x10^8 `PathBuf` comparisons per rebuild (a multi-second freeze); `rebuild_rows` also clones every path twice into `old`/`new` even when nothing is selected. The status bar already memoises its selection total by `selection.revision()` for this reason (mod.rs:432-434, 1736-1753).

**Proposal.** (1) model.rs `remap`: return early when `items`, `anchor` and `cursor` are all empty (still bump `revision`); otherwise build one `HashMap<&Path, usize>` over `new` and look paths up in it: O(rows + selected). (2) mod.rs `rebuild_rows`: only build the `old`/`new` path vectors when the selection or a marquee needs them; otherwise call a `selection.touch()` that just bumps the revision so `selected_size_memo` stays correct. (3) Memoise the selected paths the same way as the size: `selected_paths_memo: RefCell<Option<(u64, Arc<[PathBuf]>)>>` keyed by `selection.revision()`, filled from `rows[ix].entry.path` without cloning whole entries. Change `DraggedFiles(pub Vec<PathBuf>, ..)` (views.rs:48) to hold `Arc<[PathBuf]>`, have `item_drag` clone the Arc for selected items, and take the count for the 'N items' label from `selection.len()`. Update the other constructors/consumers: views.rs:1190, sidebar.rs:869, and the drop handlers that do `files.0.clone()` (mod.rs:3366-3368, transfer.rs:811, trash.rs:408, sidebar.rs:456) to `files.0.to_vec()`. Also use `selection.len()` instead of `selected_entries().len()` in `render_picker_bar` (src/finder/bars.rs:451).

**Checked plan.** Corrected plan. Do it in one commit, because the payload type changes across files.

1. **`Selection::remap` (src/model.rs:324).**
   - Keep `self.revision += 1` first, then return early when `items`, `anchor` and `cursor` are all empty.
   - Otherwise build one `HashMap<&Path, usize>` over `new` with `entry(path).or_insert(ix)`, and map items, anchor and cursor through it.
   - `or_insert` matters: `position` returns the first row for a repeated path, and a sent-results list can repeat one. `Path`'s `Hash` is component-wise like `PathBuf ==`, so lookups match what `==` found.
   - Make it generic, `remap<P: AsRef<Path>>(&mut self, old: &[P], new: &[P])`, so the existing unit tests (model.rs:386, 505) compile unchanged. `HashMap` and `Path` are already imported (model.rs:3-4).

2. **`rebuild_rows` (src/finder/mod.rs:1218-1279).**
   - Skip the proposed `touch()`; borrow instead of cloning. `self.rows` is written only at mod.rs:1273, so delete the clone at 1219.
   - After `group_rows` returns, build `old: Vec<&Path>` from `self.rows` and `new: Vec<&Path>` from the local `rows`, then call `self.selection.remap(&old, &new)` and `marquee.rows_changed(&old, &new)` as now.
   - The borrow must start after `apply_filter_to` and `group_rows`, which take `&mut self`. Field-disjoint borrows of `self.rows` and `self.selection` are fine.
   - Change `Marquee::rows_changed` (src/finder/marquee.rs:44) to take `&[&Path]`.
   - `remap` still runs on every rebuild, so the revision bump that `selected_size_memo` relies on is unchanged.

3. **`load_listing` (mod.rs:1091-1094), beyond the proposal but the same bug.** When `select` has more than a handful of paths, build the same first-wins path-to-index map over `this.rows` once instead of a `position` per path. Keep the `select_only(first)` + `toggle(rest)` sequence as is.

4. **Selected-paths memo (mod.rs).**
   - Add `selected_paths_memo: RefCell<Option<(u64, Arc<[PathBuf]>)>>` beside `selected_size_memo` (mod.rs:434, initialised at 644; import `RefCell`).
   - Add `fn selected_paths(&self) -> Arc<[PathBuf]>` keyed by `selection.revision()`, filled from `selection.indices().filter_map(|ix| rows.get(ix))` cloning only `entry.path`.
   - Release the `borrow()` before the `borrow_mut()`.
   - Document that it serves the list, icon and gallery views only. The column view keeps its own one-path payload.
   - Add `#[doc(hidden)] pub fn selected_paths_for_test()`, following `rows_for_test` (mod.rs:683).

5. **Payload type (src/finder/views.rs:48).**
   - Change to `DraggedFiles(pub Arc<[PathBuf]>, pub DragSource)`.
   - `item_drag`: use `self.selected_paths()` when `self.selection.contains(ix)`, else `Arc::from([row.entry.path.clone()])`. Take the label count from `drag_paths.len()`; it is exactly today's value, so `selection.len()` is not needed.
   - Constructors: views.rs:1190 and sidebar.rs:869 become `vec![..].into()`.
   - Consumers: `files.0.clone()` becomes `files.0.to_vec()` at mod.rs:3368, transfer.rs:812 and trash.rs:409.
   - These compile unchanged: `.contains(..)` at transfer.rs:775 and 808-810, and `.iter()` at sidebar.rs:460, views.rs:307, views.rs:1202 and sidebar.rs:876.

6. **Count without cloning (replaces the proposal's bars.rs change).**
   - Add `pub(super) fn selected_count(&self) -> usize`: `usize::from(self.column_selection().is_some())` in the column view, `self.selection.len()` otherwise.
   - Use it at src/finder/bars.rs:451 and for both calls at src/finder/nu_panel.rs:41 and 44.
   - Leave toolbar.rs:642 and 701 alone; they run only when a menu opens.

7. **CHANGELOG.md.** Add a "### Fixed" entry under [Unreleased]: with thousands of items selected, scrolling, sorting, and folders changing on disk no longer slow down with the size of the selection. docs/manual.md needs no change.

**Tests.** Everything below runs on Linux with the existing headless harness; nothing is platform-specific.

**Unit tests in src/model.rs** (beside `selection_survives_resort` and `the_revision_changes_with_the_selected_items`, which must pass unchanged):
- 30,000 paths, `select_all`, `remap` onto the reversed list. Assert every index is still selected, anchor and cursor moved from 0 to 29,999, and `Instant::elapsed() < 5 s`. Measured in a debug build: the map version takes about 40 ms, the current code about 50 s. src/nushell.rs:819 is the precedent for a time bound.
- A path that appears twice in `new` maps to its first index.
- With `items` empty but anchor and cursor set (toggle the only item off), both still map.
- A blank selection stays blank and its revision still changes.

**tests/file_ops.rs** (helpers: `Fixture::new`, `Ui::open`, `ui.click("view-list")`, `ui.select`, `ui.select_also`, `ui.drag`, `item_id`, `ui.update` with `reload_quietly`):
- `dragging_one_of_several_selected_items_moves_them_all`: select notes.txt and Budget.csv, drag `item_id("name-cell", notes.txt)` onto `item_id("row", Projects)`, assert both are in Projects. This covers the path being changed.
- Memo invalidation on selection change: select three items, toggle one off with `select_also`, drag. The deselected item stays.
- Memo invalidation on rebuild: with two selected, `std::fs::rename` one on disk, call `reload_quietly`, drag the other. Only it moves.

**tests/basics.rs** (owns `long_folder`):
- In list view press `cmd("a")`, read `selected_paths_for_test()` twice with `ui.frame()` between, and assert `Arc::ptr_eq` and `len() == ui.names().len()`.
- Then `select_also` one name and assert the Arc changed and is one shorter.

**tests/controls.rs** (beside `picker_bar_enables_choose_with_a_selection`, line 1362):
- Open with `Open { picker: Some(PickerOptions { multiple: true }), .. }`, press `cmd("a")`, click "view-columns", and assert `ui.snapshot("picker-choose").label() == Some("Choose")`. This pins the column-view count that the proposal's `selection.len()` would break.

**Existing tests that must stay green:**
- `a_reload_during_a_box_keeps_what_cmd_kept_selected` (basics.rs:63) for the marquee remap.
- `the_status_bar_keeps_up_with_the_selection_and_the_files` (basics.rs:984) for the revision bump on rebuild.
- The single-item drag tests in file_ops.rs.

**Risks.** - **Stale memo.** It is correct only while `rebuild_rows` stays the sole writer of `self.rows` (true today, mod.rs:1273) and always calls `remap`. Say so in a comment on the field. If it went stale, a drag would carry the wrong files.
- **Duplicate paths.** The map must keep the first index, or selections in lists with a repeated path land on a different row than today.
- **Borrow order in `rebuild_rows`.** Moving the `old` borrow to the end relies on nothing replacing `self.rows` earlier in the function.
- **Column view count.** The picker bar and Nushell panel must keep following `selected_entries()` there, not `selection.len()`.
- **Timed unit test.** It is wall-clock based; keep the bound generous (5 s against about 40 ms expected) so slow CI runners on macOS and Windows don't flake.
- **Type change.** `Arc<[PathBuf]>` touches five source files, but it is mechanical and compiler-checked.
- **Out of scope.** `selected_trashed` (trash.rs:112-123) is still quadratic for Put Back with a large selection.

**Notes from later reviews.** 'column-pane-selection-by-path' and 'view-switch-keeps-selection' may land before this one and touch how the selection is kept; build on them. This is a performance change: behaviour must stay exactly the same, and the test has to show the work no longer grows with the selection. From the reviewer of 'results-scroll-reset' (on the trunk): Selection::remap costs selected x rows on every rebuild of the rows. It belongs with this brief.

## settings-rows-fit

**Settings: rows run past the dialog's edge with wider fonts ('Remember each folder's vi', the letter case buttons, Try It)**

Value 3 of 5, size S, ui, bug.

**Problem.** Found by the reviewer of 'choose-nu-path' (done record: /home/user/polish-tools/done/choose-nu-path.json, third item under 'Found, not fixed') and seen by the orchestrator in that change's pictures (/home/user/polish-tools/shots/choose-nu-path-review-found.png, and -before-settings.png for the same before that change). The Settings dialog is 600 points wide and several of its rows are laid out as one line that doesn't give way. With the fonts on Linux here: under Appearance the third control's label reads 'Remember each folder's vi' and is cut at the dialog's edge (the row holds System/Light/Dark, Show hidden files, and Remember each folder's view); under Search Engine the Smart / Match Case / Ignore Case buttons end about 10 points past the content's right margin; under Nushell, with a long terminal name ('Automatic (System Default (x-terminal-emulator))', which is what Linux shows by default), Try It hangs over the right margin. In a narrow window at 150% text size the letter case buttons and the terminal menu run off the edge as well (picture: /home/user/polish-tools/shots/choose-nu-path-review-narrow-150.png). On a Mac, whose system font is narrower, the rows at 100% may just fit, which would be why it was not seen; nobody has looked there. Text size is a setting (up to 150% at least), so the rows must give way on every system.

**Proposal.** Reproduce in pictures first: Settings at 100%, 125% and 150% text size, light and dark, in a 1100x1150 window (so the whole dialog shows) and in a 700-point-wide one. List every row whose content passes the content's right edge or is cut without an ellipsis. Then make each such row give way in the way that suits it, in the dialog's own style (src/finder/dialogs.rs; read how the rows are built and what helpers the file has, and 'choose-nu-path' for `fit::with_cut_off_text` on a `flex_1().min_w_0().truncate()` div, which is how its status text gives way to its buttons): a row of several switches can wrap to a second line (the Search Engine switches above it already sit in rows of two); a label beside a fixed control can shrink and wrap or be cut with an ellipsis and a tooltip; a menu button with a long value can cut its value with an ellipsis so the button after it stays inside the margin. Nothing may be cut without an ellipsis, nothing may pass the right margin, and a control must never be pushed out of reach. Do not change the dialog's width or the wording of labels. Keep the rows that fit today looking exactly as they do (compare pictures before and after at 100% on a wide window: only the rows you meant to change may differ).

**Tests.** A headless test that fails today with the test fonts: open Settings, and for every control and label the harness can see in the dialog (they have IDs: read tests/controls.rs settings_* tests for them), assert its right edge is inside the dialog's content edge, at 100% and at 150% text size; and that 'Remember each folder's view' is whole (its element is as wide as its text needs, or it has wrapped). If the harness's fonts happen to make everything fit at 100%, assert at the size where it first fails and say so. Pictures before and after for each row changed, light and dark, at 100% and 150%.

**Risks.** Settings is taller than a small window and scrolls; tests click controls low in it, some without scrolling first, and on macOS there is one more row (Applications) so they have little room to spare: the reviewer of 'choose-nu-path' measured `nu-text-bigger` at 10.75 points above the fold at 110% text size with that row forced on. A row that wraps to a second line makes everything below it lower: check every settings test (tests/controls.rs settings_*, tests/settings.rs, tests/folder_tree.rs) and make those that click below your change scroll to their control first if they would otherwise depend on the spare room, as tests/folder_tree.rs does for `sidebar-list`. 'search-settings-hints-steppers', 'search-settings-without-effect' and 'search-roots-notes-dedupe' also change Settings; if they are on the trunk, build on them and read their done records.

**Notes from later reviews.** 'choose-nu-path' (ca4fbee) is on the trunk: the first line under NUSHELL is `nu_controls`, whose text gives way to its buttons; read its done record's notes on Settings' heights (under 'Doubts that remain' and 'For briefs still to come').

## split-view-toolbar-clip

**Split view clips the main pane's toolbar: the sidebar takes its room from the window, not the pane**

Value 3 of 5, size M, ui, bug.

**Problem.** `sidebar_width_shown` (src/finder/sidebar.rs:193-200) caps the sidebar at `window.viewport_size().width - TOOLBAR_MIN_WIDTH`. In split view a browser only gets half the window (src/workspace.rs:299-316, two `flex_1` panes), so the cap never bites. At a 1000pt window the main pane is 499.5pt, the sidebar stays at its 212pt default (crates/fffinder-core/src/config.rs:164) and the toolbar gets 287.5pt. Fully folded with the search field at SEARCH_LAST_WIDTH it needs 334pt (TOOLBAR_MIN_WIDTH 374 - 40; src/finder/toolbar.rs:23-25, 80-81). The toolbar is `overflow_hidden` (toolbar.rs:485), so the search field, its last child, is cut off. At the 850pt minimum window (src/lib.rs:422) the pane's toolbar is 212.5pt: the search field is gone entirely and Actions is cut. Every split window under about 1093pt is affected; the default is 1100 (lib.rs:371). docs/manual.md:114 and :160 promise the sidebar gives up width and each toolbar folds. No test covers it (the toolbar tests at tests/basics.rs:520-614 are single-pane). Same narrow pane, second symptom: the status text has no `min_w_0`/`truncate` (src/finder/bars.rs:391-397, unlike the picker hint at bars.rs:468), so text cannot shrink below its full width and '2 of 12 selected (1.4 MB), 214 GB available' pushes the icon-size slider (bars.rs:428-444) past the pane's edge at the default window size.

**Proposal.** 1. Add `fn pane_room(&self, window, cx) -> f32`: the viewport width, or `(viewport - 1px) / 2` when `self.has_split_view(cx)` (src/finder/tab_bar.rs:50-57). Use it at sidebar.rs:194 and pass `cx` from the four callers (sidebar.rs:209, 384, 437; list.rs:327). This fixes windows of 949pt and up. 2. Below that the sidebar is at its 140pt floor and the toolbar is still up to 50pt short, so add one more fold step to `ToolbarFit` (toolbar.rs:85-116): `ViewsInActions`, below `SortInActions`, saving `MENU_PILL + TOOLBAR_GAP` (66pt). The view menu button becomes a 'View' submenu at the top of the Actions menu, built with the existing `view_mode_items` (toolbar.rs:260-277), the way Sort and Nushell already move (toolbar.rs:676-699). Subtract it from TOOLBAR_MIN_WIDTH and use SEARCH_LAST_WIDTH for both lowest steps (toolbar.rs:442). At 850pt split that is 140 + 284.5, and the folded toolbar needs 268. 3. Status bar: `.min_w_0().truncate()` on `status-text` with the cut-off tooltip from fit.rs:31, and `.flex_shrink_0()` on the slider group. Update docs/manual.md:166-171 and the unit test at toolbar.rs:834-850.

**Checked plan.** Corrected plan: apply today's rules to the pane, and add the fifth fold only where the toolbar clips today.

1. Pane room
- In src/workspace.rs, add a shared divider constant and a small `pane_width(viewport_width) -> Pixels` helper, `(w - px(1.)) / 2.`. Use the constant in `render` so the two cannot disagree. The divider is `px`, not `ui`.
- In src/finder/sidebar.rs, change `sidebar_width_shown(&self, window, cx: &App)` to use that width when `self.has_split_view(cx)` (tab_bar.rs:50-57), divided by `ui_scale`.
- Extract a pure `sidebar_width_for(chosen, room) = chosen.min(room - TOOLBAR_MIN_WIDTH).max(140)` for a unit test.
- Pass `cx` from sidebar.rs:209, 384, 437 and list.rs:327; all four already have a `Context<Self>`.
- Reading the host during render is already done at mod.rs:3284 and tab_bar.rs:283.

2. Fifth fold as a last resort
- Keep TOOLBAR_MIN_WIDTH at 374 as the sidebar's reserve. Do not subtract 66.
- In src/finder/toolbar.rs, add `ToolbarFit::ViewsInActions` as the lowest variant. After the existing loop in `for_room`, return `SortInActions` when `room >= TOOLBAR_MIN_WIDTH - (SEARCH_MIN_WIDTH - SEARCH_LAST_WIDTH)` (334), otherwise `ViewsInActions`.
- Rendering at toolbar.rs:528-531 becomes three-way: `fit >= NoFolderSearch` shows `views`, `fit >= SortInActions` shows `views_menu`, otherwise neither.
- Search minimum at toolbar.rs:442: `fit <= SortInActions` uses SEARCH_LAST_WIDTH.
- In `actions_menu` (toolbar.rs:676-699), when `fit < SortInActions`, prepend `menu.submenu("View", ...)` built with `view_mode_items`, exactly as src/finder/menus.rs:478-480 does.
- Resulting sums: 1100 split gives sidebar 175.5 and toolbar 374 with everything that shows today. 1000 gives sidebar 140, toolbar 359.5, search 105.5. 850 gives sidebar 140, toolbar 284.5, and the folded toolbar needs 268.
- Single-pane layouts that fit today are unchanged.

3. Status bar, src/finder/bars.rs:391-397
- Build `status-text` with `super::fit::with_cut_off_text(div().id("status-text").min_w_0().truncate().aria_label(..), StyledText::new(text.clone()), text.into(), |shown| shown).test_support()`, following sidebar.rs:1220-1226.
- Keep the `aria_label`: tests/basics.rs:980-993 and tests/file_ops.rs:974 read `.label()`.
- Add `.flex_shrink_0()` to the slider group, and give it an id plus `.test_support()` so a test can find it.

4. Docs
- docs/manual.md: line 114 (a split pane counts as narrow), line 118 (add status text to the cut-off list), lines 166-171 (step 5, after the sidebar is at its narrowest).
- Add a Fixed entry under [Unreleased] in CHANGELOG.md.

Pitfalls
- A fit change takes two frames (measure in prepaint, then refit), so tests need `settle(); frame(); frame();`.
- On Linux client-side decorations the main pane still draws duplicate window controls mid-window, costing 96pt. After this fix they are what gets clipped, not the search field. Optionally skip them in the non-split pane while the window is split, in both `toolbar_room` and the render; this cannot be tested headlessly because the test platform reports server decorations.
- Dragging the sidebar edge in a narrow split changes the saved width while the shown width stays capped. This already happens in narrow single windows.
- With `ui_scale` well above 1 at the minimum window, nothing can fit; do not chase it.
- Giving the main wrapper `.id("main-pane")` changes descendants' element paths. That is harmless at runtime.

**Tests.** Headless UI test in tests/basics.rs, beside the toolbar tests, using the existing `with_tabs` helper (basics.rs:21-30, `Open { workspace: true, .. }`).

Setup
- `ui.resize(1000., 600.)`, then `ui.press(&cmd("\\"))` as tests/columns.rs:429 does (or `ui.dispatch(ToggleSplitView)`), then `ui.settle(); ui.frame(); ui.frame();`.
- IDs repeat in both panes, so do not use `ui.snapshot` or `ui.exists`. Use `ui.window(|w, _| w.within("main-pane").find("search").bounds())` and `.try_find("view-menu")` after adding `.id("main-pane")` in workspace.rs. `within` resolves a scope from its descendants, so no `test_support` is needed for scoping. The alternative is `find_all("search")` sorted by x, as columns.rs:443-453 does.
- Pane edge: `find_all("content")[1].bounds().left() - px(1.)`, or (W-1)/2.

Assertions
- 1000pt: the main pane's `search.right()` and `actions.right()` are at or before the pane edge, and `view-menu` is still there. This fails today: search ends at 536 against an edge of 499.5.
- 850pt: the same bounds hold and `view-menu` is absent in `main-pane`. `ui.click_in("main-pane", "actions")` then `ui.menu_labels()` contains "View", "Sort and View Options" and "Nushell".
- Back at 1400pt: `view-menu` or `view-list` returns in the main pane.
- Status bar at 1000pt: `ui.select` and `ui.select_also` act on the main Finder directly (tests/common/mod.rs:729-743), so they work in split view. Assert the main pane's `status-text` right edge and the new slider-group id are inside the pane edge, and that the `label()` is still the full text. The test platform's NoopTextSystem gives 0.6 em per character (about 284pt for the 43-character example), so this fails today on Linux.

Unit tests
- In toolbar.rs `the_toolbar_folds_one_step_at_a_time` (834-850): replace the last assert with `for_room(TOOLBAR_MIN_WIDTH - 40.) == SortInActions` and `for_room(TOOLBAR_MIN_WIDTH - 41.) == ViewsInActions`.
- In sidebar.rs: test `sidebar_width_for(212., 499.5) == 140.`, `(212., 549.5) == 175.5`, `(212., 1100.) == 212.`.

Existing tests to rerun
- tests/basics.rs:519-614 (single-pane folds; "four folds" in 640..=1000 is unchanged because TOOLBAR_MIN_WIDTH is not touched).
- tests/columns.rs:354-384 (sidebar resize and the narrowed sidebar).
- Split tests in tests/columns.rs:415, tests/file_ops.rs, tests/settings.rs, tests/folder_tree.rs. They run at the 1536pt default test window or wider, where the sidebar stays 212.

**Risks.** - Toggling split view at the default 1100pt window now narrows the sidebar from 212 to about 175pt. This matches manual.md:114, but it is a visible change.
- The fifth fold puts the view switcher in an Actions submenu in 850-948pt split windows. It stays reachable through the background context menu's View submenu, Cmd+1..4 and the macOS View menu.
- Truncating the status text must keep its `aria_label`, or three existing tests break.
- Linux client-side decorations: the main pane's duplicate window controls are still clipped in narrow splits unless the optional follow-up is done.
- toolbar.rs is shared with the tooltip-shortcuts and history-menus candidates, so expect merge conflicts.
- The real-font status bar overflow at 1100pt is estimated from character widths, not measured; the headless test proves it only under the test platform's fixed-width text.

**Notes from later reviews.** From the reviewer of 'search-truncation-honesty': in a window about 1000 pt wide with the sidebar showing, the toolbar's title is cut down to a single '.' while the buttons keep their full width. Check single-pane windows at 900 to 1100 pt wide as well as split view, in pictures, and make the title degrade sensibly (keep a readable, truncated title; let something else give first) if it is the same layout fault. From the reviewer of 'empty-state-messages' (on the trunk): the scope bar's folder chip is never cut short, so a folder with a long name takes a line of its own and runs past the pane's edge (picture: /home/user/polish-tools/shots/empty-state-messages-review-long-folder-name.png, at 900 pt; in a narrow split pane a much shorter name is enough). It is the same kind of fault as this brief's, one bar lower (src/finder/bars.rs, `render_scope_bar`): if it is small, give the chip a width it can't pass and cut the name in the middle with an ellipsis, as the app does for names elsewhere, with a picture and a test; if not, leave it and say so. Since 'smart-folder-save-and-leaks' the scope bar also holds a Save button, and since 'empty-state-messages' the Kind button is tinted while a kind narrows the search.

## table-preview-column-widths

**A table preview's columns ignore the room they have: an archive's two columns scroll sideways in the Preview Pane, and stay 140 points wide in Quick Look**

Value 3 of 5, size S, ui, bug.

**Problem.** Found by the reviewer of 'table-preview-scroll' (done record: /home/user/polish-tools/done/table-preview-scroll.json, under 'Found, not fixed'), and seen by the orchestrator in that change's pictures. In the table arm of preview_body (src/finder/preview.rs) every column is a fixed 140 points at the default text size (`let cell = 140. * text_size / DEFAULT_PREVIEW_FONT_SIZE`, and table_cell gives each cell `.w(ui(width)).flex_shrink_0()`), and the table is at least 300 points wide (`let width = ui((ncols as f32 * cell).max(300.))`, used as `.min_w(width)`). Two things follow. (1) Narrow places: the Preview Pane at its default width leaves the table about 245 points and the column view's card about 270, so every archive's listing (Name, Size) is wider than its card by 55 or 30 points. It scrolls sideways by that little for no reason, and since 'table-preview-scroll' made tables scroll down, the horizontal scrollbar's thumb is drawn over the bottom visible row whenever such a table is scrolled down (picture: /home/user/polish-tools/shots/table-preview-scroll-after-pane-long-wheel.png, the grey bar across 'echo/item-012.txt'). That thumb is new with our change, so this is partly ours to clean up. (2) Wide places: in Quick Look's window and the gallery the Name column is still 140 points wide while the rest of the row is empty (picture: /home/user/polish-tools/shots/table-preview-scroll-after-quicklook-long.png, where the two columns use 190 of 880 points). A longer name in an archive is cut with an ellipsis there, next to hundreds of points of empty table. Two smaller things in the same arm, from the same reviewer: the line under the table is cut at the card's edge with no ellipsis when it is long ('Opened with Nushell · 300 rows, the first 200 shown' ends at 'the first 20' in the Preview Pane; sheet /home/user/polish-tools/shots/table-preview-scroll-review-sheet-pane-2.png); and in the column view a sideways swipe over a table in the card moves the table and the columns together (a delta of -10 moved the columns 10 and the table 20 on screen), because the table's scroll area doesn't stop a sideways event and the columns' plain overflow_x_scroll takes it too.

**Proposal.** Reproduce first, headlessly and in pictures: (a) an archive's two-column table in the Preview Pane at its default width, and in the column view's card, can be scrolled sideways (its content is wider than its area), and (b) in Quick Look's window a name longer than 140 points' worth is cut although the table has room. Then let the columns use the room there is. One way: the cells share the area's width when the table's natural width is less than it (grow), and for a table of few columns they may shrink to a floor that keeps a size or a short name readable, so that Name and Size fit the Preview Pane's and the column card's default widths without a sideways scroll. A table of many columns (a database's rows, a spreadsheet) must keep readable columns and scroll sideways as now. Whatever you choose: the heading row stays lined up with its columns at every width and text size (the heading and the rows are separate flex rows today, which works only because every cell has the same fixed width; if cells grow, both must grow by the same rule), text size changes in Quick Look still widen the columns, and a table at its natural width or wider looks as it does now. Then the footer: give the line under the table an ellipsis when it doesn't fit (it is an h_flex with a plain child today). The column view's double move on a sideways swipe: fix it only if it is small and certainly right (the table's area taking the sideways event when it can scroll that way, and leaving it to the columns when it cannot, the way the wheel behaves vertically since 'table-preview-scroll'); otherwise leave it and say so in left_out.

**Tests.** Headless UI tests that fail today: an archive's table in the Preview Pane at its default width does not move for a sideways wheel event and its last column ends inside the card (read the bounds of `preview-table` against the card's); the same in the column view's card; in Quick Look's window the table is as wide as its area (so a long name has the room). Keep, and keep meaningful, the five tests 'table-preview-scroll' added (tests/basics.rs: a_table_preview_scrolls_down_to_its_last_row, a_long_table_wider_than_the_pane_scrolls_both_ways, a_table_scrolls_down_in_the_gallery_and_in_quick_look, a_table_in_the_column_views_card_takes_the_wheel_only_when_it_scrolls; tests/term_mode.rs: the_dialog_scrolls_a_table_down_to_its_last_row) and a_table_preview_starts_at_its_left_edge_however_far_the_last_one_was_scrolled: some of them need a table that is wider than its area. a_long_table_wider_than_the_pane_scrolls_both_ways drags the pane to its least width and relies on the table's 300-point minimum being wider than that; if your change lets a two-column table fit there, give that test a table that is still wider (more columns need Nushell, so prefer a floor that the pane's least width stays under, or say what you did). Pictures before and after, light and dark: an archive in the Preview Pane (at rest and scrolled down: no horizontal thumb), in the column view's card, in the gallery and in Quick Look (with a long name), and a many-column table that still scrolls sideways.

**Risks.** The same arm draws in five places (Preview Pane, column view's card, gallery, Quick Look's window, Quick Look's dialog in --term mode): check each. The vertical scrolling that 'table-preview-scroll' just fixed depends on `.self_start()` on the element the kit's both-axes wrapper scrolls and on that element being as tall as the table; do not disturb it (its five tests will tell you). A heading that no longer lines up with its columns is worse than today's fixed widths. Very wide tables (many columns from Nushell) must not get columns too narrow to read.

**Notes from later reviews.** 'table-preview-scroll' (66d4fad) is on the trunk; build on it. 'preview-card-height' changes the height of the card this table sits in (the column view's card and the Preview Pane's): if it has landed, read its done record; if not, nothing to wait for. The manual's Previews table says a table 'wider than its space scrolls sideways': keep that true and say, if you change when it happens, that a table of few columns fits its space.

## tree-arrows-past-smart-folder

**Tree sidebar: the Down arrow can't get past the first Smart Folder, and a Smart Folder's row has no Cmd+click or middle-click**

Value 3 of 5, size S, ui, bug.

**Problem.** Found by the reviewer of 'smart-folder-save-and-leaks' (done record: /home/user/polish-tools/done/smart-folder-save-and-leaks.json, first two items under 'Found, not fixed'), and read in the code by the orchestrator. (1) With the tree sidebar (`sidebar_mode: "tree"`) and the keyboard in it, Down cannot get past the first Smart Folder while a line above it is still marked. `tree_cursor` (src/finder/folder_tree.rs) takes the first line that `line_is_active` (src/finder/sidebar.rs) marks as the place the arrow keys are at: `lines.iter().position(|line| self.line_is_active(line))`. A Smart Folder that searches This Computer doesn't change the window's place, so Recents, or the tree's selected folder under Favorites, stays marked above it; `move_in_tree` then steps from that line again, and every Down opens the same Smart Folder again. A folder-scoped Smart Folder does the same when its folder's line is above the Smart Folders. Probed by the reviewer on 323f173 and on the trunk with two Smart Folders for different words, from a favorite and from Recents: nine presses of Down never reach the second Smart Folder, nor Locations below. Up from below works, since the Smart Folder's line then comes before the place's. (2) The manual says of sidebar rows: 'Click a row to open it. Cmd+click or middle-click a row to open it in a new tab.' (docs/manual.md, the Sidebar section, and again under 'Opening a folder in a new tab'). A Smart Folder's row has only a plain click (`smart_row` in src/finder/sidebar.rs: one `on_click` that calls `open_smart_folder`), so Cmd+click opens it in the same tab and a middle click does nothing.

**Proposal.** Reproduce (1) first with a headless test in tests/folder_tree.rs beside the_arrows_go_on_past_a_smart_folder_whose_folder_is_gone (which walks Up through the Smart Folders): tree sidebar, two Smart Folders for different words that search This Computer, the keyboard in the tree on a favorite above them; Down, Down, Down must open the first, then the second, then go on to the first line under Locations. Do the same starting from Recents. Then fix where the keys think they are. The reviewer's suggestion: in `tree_cursor`, let a marked `Line::Smart` win over the other marked lines, since the search is what shows. Check Up and Down across the whole sidebar in both directions afterwards, with a This Computer Smart Folder and a folder-scoped one whose folder's line is above and one whose folder's line is below the Smart Folders, and after the search ends (Escape) the keys must be at the place's line again. Page Up, Page Down, Home and End go through the same cursor. Look at what the sidebar draws meanwhile: while a This Computer Smart Folder's search shows, both its row and the place's row may be highlighted; say what you find and leave the look alone unless it is plainly wrong. For (2): either give a Smart Folder's row the Cmd+click (Ctrl+click on Linux and Windows) and middle-click that the other rows have, opening its search in a new tab, if the new tab can be given the Smart Folder's whole search with what is already there (`open_smart_folder` and `enter_smart_folder` in src/finder/filter.rs; see how a place's row opens a new tab), or, if that is not small and certainly right, correct the two sentences in the manual to say that a Smart Folder's row opens in the same tab. Say which you did.

**Tests.** tests/folder_tree.rs: Down from a favorite and from Recents walks through two This Computer Smart Folders and on into Locations (fails today: the words never change after the first); Up walks back the same way; with a folder-scoped Smart Folder whose folder is a favorite above it; after Escape the next Down starts from the place's line. If you add the new-tab clicks: a test that Cmd+click (secondary) and middle-click on a Smart Folder's row open a second tab showing its search with its kind, scope and filter, and leave the first tab as it was.

**Risks.** The tree's keyboard cursor is shared by the arrows, Page Up and Down, Home and End, and by typing to select; a change in which marked line wins must not move the cursor for places and folders when no Smart Folder's search shows. 'smart-folder-save-and-leaks' made the mark exact (`shows_smart_folder`: the whole search must be the saved one), so at most one Smart Folder is marked; rely on that only after checking it. 'collapsible-sidebar-sections' will let the Smart Folders section be collapsed: a hidden marked line must not capture the keys either; leave a note for it.

**Notes from later reviews.** 'smart-folder-save-and-leaks' (60df179) is on the trunk; read its done record first: it explains the mark (`shows_smart_folder`), the test the_arrows_go_on_past_a_smart_folder_whose_folder_is_gone, and what entering and leaving a Smart Folder does to a tab's kind, scope and filter.

## type-select-space

**Type-to-select cannot contain a space: Space opens Quick Look in the middle of a name**

Value 3 of 5, size S, ui, bug.

**Problem.** The type-to-select key handler explicitly intends to accept a space once a prefix is in progress: `(text != " " || !this.type_select.is_empty())` at src/finder/mod.rs:3343-3350. That branch can never run for the real Space key: `space` is bound to `QuickLook` in the file views' key context (src/actions.rs:187), the root handles that action without propagating (`.on_action(cx.listener(Self::quick_look))`, mod.rs:3054), and GPUI dispatches matching key bindings before key-down listeners, returning as soon as an action is handled (gpui-pre-0.3.10 src/window.rs, `dispatch_key_event`: the `for binding in match_result.bindings` loop runs before `finish_dispatch_key_event`). So typing `my plan` quickly selects `my notes.txt` at `my`, then the space pops up a Quick Look window for it and the rest of the letters start a different prefix. Names with spaces ('New Folder', 'Project Atlas') can't be reached past the first word. The existing test only types letters (tests/finder.rs:318-365); the test harness's `type_text(" ")` produces key `" "` rather than `space`, which is why the dead branch looks covered. Separately, the check uses `type_select.is_empty()` although the buffer is only cleared lazily on the next typed character (mod.rs:1906-1917), so it would treat a prefix typed minutes ago as still in progress.

**Proposal.** In mod.rs: add `fn type_select_in_progress(&self, cx: &App) -> bool` = buffer non-empty AND `type_select_at` within the same 1-second window `typed_prefix` uses. In `render_content` (the `content` div that tracks `files_focus`, mod.rs:3278-3351) add `.on_action(cx.listener(|this, _: &QuickLook, window, cx| if this.type_select_in_progress(cx) { this.type_to_select(" ", window, cx) } else { cx.propagate() }))`; it runs before the root's `quick_look` because it is nearer the focused node. Replace `!this.type_select.is_empty()` at mod.rs:3346 with the same helper so both paths agree. Add one sentence to docs/manual.md 'Type to select' (line 289) saying a space typed mid-name is part of the name.

**Checked plan.** Two corrections to the proposal, then the concrete steps.

**Correction 1: do not swallow every Space typed within a second.**
As proposed, typing `ro` and pressing Space straight away (jump, then peek) would stop opening Quick Look. Each swallowed space also resets `type_select_at`, so repeated presses stay swallowed. Take the space only when a prefix is in progress and some candidate name starts with the lowercased prefix plus a space. Then `my` + Space + `p` reaches `my plan.txt`, while `ro` + Space still opens Quick Look.

**Correction 2: use a dedicated action instead of intercepting `QuickLook`.**
Cmd+Y, File > Quick Look, the context menus (menus.rs:184, 352) and the toolbar (toolbar.rs:744) all send `QuickLook` and should never type a space.

**Steps**

1. src/actions.rs
   - Add an action such as `TypeSpace` to the `actions!` list.
   - Add `key("space", TypeSpace, &files, cx)` after line 187, in the same array, with a comment that order matters.
   - Why order matters: `resolve_bindings_for_input` (gpui-pre keymap.rs) sorts by context depth, then later index first, so `TypeSpace` is tried first. The loop at window.rs:6196 moves on to the `QuickLook` binding when the handler propagates.
   - Do not bind it in the `QUICK_LOOK` or `TREE` contexts. The menu still shows Cmd+Y, because `QuickLook`'s bindings are unchanged.

2. src/finder/mod.rs
   - Pull the one-second window into a const shared with `typed_prefix` (line 1910).
   - Extract `fn type_select_names(&self) -> Vec<String>`: the active column's entries in Columns view, otherwise `rows`. Reuse it inside `type_to_select` (1939-1949) so both paths agree.
   - Add `fn space_continues_typing(&self, cx: &App) -> bool`:
     - buffer non-empty, and
     - `type_select_at` within the window, using `cx.background_executor().now()` as `typed_prefix` does, and
     - any name's lowercase starts with `format!("{} ", self.type_select.to_lowercase())`.
   - On the `content` div (next to the other `.on_action` calls, around 3294-3336) add a `TypeSpace` listener: if `this.space_continues_typing(cx)` call `this.type_to_select(" ", window, cx)`, otherwise `cx.propagate()`. The propagate is required, or Space stops opening Quick Look.
   - Replace `!this.type_select.is_empty()` at 3346 with `this.space_continues_typing(cx)`. This fixes the stale-buffer Shift+Space jump.
   - No `renaming` check is needed in the action handler: the context is `FinderFilesRenaming` while renaming, so the binding is inactive.

3. Optional, same idea: src/finder/folder_tree.rs:1134 rejects every space. Space is unbound in the tree context, so it could accept one when a tree prefix is in progress. Leave it out if scope matters.

4. Docs
   - docs/manual.md:289: add one sentence, for example that a space typed within a second of the letters is part of the name when a name goes on with one, and otherwise opens Quick Look.
   - CHANGELOG.md: add a "Fixed" entry under [Unreleased].

**Tests.** Headless UI test in tests/finder.rs, next to `typing_jumps_to_matching_names` (line 318).

Helpers from tests/common/mod.rs: `Fixture::new`, `fx.path`, `Ui::open`, `ui.press`, `cmd("2")`, `ui.type_text`, `ui.selected()`, `ui.read(|f| f.quick_look_is_open())`, `ui.in_quick_look`, `ui.cx.executor().advance_clock`.

Setup: write `my notes.txt` and `my plan.txt` with `std::fs::write(fx.path(..))` before `Ui::open`, then `ui.press(&cmd("2"))`.

1. **Space joins the prefix.** `ui.type_text("my")` selects `my notes.txt`. Then `ui.press("space")`: this sends the real `space` key through the bindings, as tests/controls.rs:1088 does. Then `ui.type_text("p")`. Assert `ui.selected() == ["my plan.txt"]` and Quick Look is not open.
2. **Jump then peek still works.** `advance_clock(2 s)`, `ui.type_text("r")` selects `Roadmap.md`, `ui.press("space")`. Assert Quick Look is open, because no name starts with `r `. Close it with `ui.in_quick_look(|ui| ui.press("space"))`.
3. **Normal Space is unchanged.** `advance_clock(2 s)`, `ui.press("space")`. Assert Quick Look is open, then close it.
4. **Stale buffer.** `advance_clock(2 s)`, note the selection, `ui.press("shift-space")`. Assert the selection is unchanged; today it jumps to the first item.

Timing: every harness step calls `settle()`, which advances the test clock 600 ms (mod.rs:246-253). Keep type, space, type as consecutive steps with no other `ui.*` step between them, or the one-second window expires.

Existing tests that must stay green: `typing_jumps_to_matching_names` in tests/finder.rs, the Quick Look tests at tests/controls.rs:1083-1300, and tests/term_mode.rs. None of them type letters before Space.

I did not run cargo, as instructed; this plan is from reading the code.

**Risks.** - **Swallowed Space (main risk).** With the proposal as written, any Space within a second of a typed letter no longer opens Quick Look, and repeated presses keep extending the window. That regresses the common jump-then-peek habit. The "a name must continue with a space" condition limits it to folders that really have such a name; there, a second Space or a one-second pause opens Quick Look.
- **Quick Look already open.** With its window open and the browser focused, a mid-name Space extends the prefix instead of closing Quick Look (docs/manual.md:424 says Space in the browser closes it). The same narrow condition applies.
- **Binding order.** The dedicated action depends on its binding coming after the `QuickLook` one, and on the handler calling `cx.propagate()`. Getting either wrong breaks Space for Quick Look everywhere. The existing Quick Look tests in tests/controls.rs would catch that.
- **If the proposal's `QuickLook` interception is kept instead.** Cmd+Y and the menu items would also type a space during that window.
- **Shared buffer.** `type_select` is shared with the sidebar tree (folder_tree.rs:1045). A prefix typed in the tree could count as in progress in the files for up to a second after focus moves. This is negligible.

## view-switch-keeps-selection

**Switching views loses the selected item: Columns drops it, Icons can scroll to the wrong rows**

Value 3 of 5, size M, ui, bug.

**Problem.** `set_view_mode` (src/finder/mod.rs:2106-2131) tries to keep the selection in view (`if let Some(ix) = self.selection.cursor() { self.scroll_to(ix) }`, mod.rs:2123-2125) but two cases fail. (1) To Columns: it clears the panes and calls `sync_root_column()`, which creates the first pane with `selected: None` (mod.rs:2119-2122, 2861-2870). The row selection still exists in `self.selection` but the column view reads only `column_selection()` (mod.rs:1765-1768), so after Cmd+3 nothing is highlighted and Space/Return/F2/Cmd+Backspace do nothing until you click again; switching back to List makes the old selection reappear. (2) To Icons: `scroll_to` converts the item to a grid line with `self.grid_cols` (mod.rs:1892-1896, src/finder/groups.rs:262-265), but `grid_cols` is only updated while the icon grid itself is being painted (src/finder/views.rs:390-405) and starts at 1 (mod.rs:564). If the width changed while another view was showing (window resized, sidebar or preview pane toggled) or this tab has never drawn icons, including a first listing that arrives while the skeleton is showing (mod.rs:3267-3268) and a folder whose remembered view is Icons opened from a list-view search via Show in Enclosing Folder (`reveal_in_place` -> mod.rs:1091-1100), the deferred scroll is consumed against a layout built with the stale column count (gpui-base-0.8.1 src/virtual_list.rs:641, 271-277), the next frame re-lays the grid with the real count and merely clamps the offset. The selected/revealed item ends up off-screen, typically with the view parked at the bottom.

**Proposal.** (1) Columns: in `set_view_mode`, after `sync_root_column()`, if the cursor row is selected, select that row in pane 0 without touching `self.selection` or focus (set `selected`, scroll the pane to it, and open the next pane when it is a folder: the body of `select_in_column`, mod.rs:2889-2919, factored into a window-less helper). (2) Icons: make the column count independent of the icon grid having been painted. Record the file-view width in a prepaint on the body container that every view shares in `render_content` (wrap `body` at mod.rs:3375-3383 the way `content_height` is recorded at mod.rs:3372-3374), store it as `view_width`, and derive `grid_cols` from `view_width`, `tile_size(icon_size)` and `ui_scale` in one function used by `icon_layout`, `icon_line_index`, `icon_step` and `move_by_page`; the existing prepaint in `render_icons` then just confirms it. Additionally keep the pending reveal (`Cell<Option<usize>>` set by `scroll_to` in icon view) and re-issue `scroll_to` from that prepaint when the measured count differs from the one the layout used, clearing it after one stable frame.

**Checked plan.** Two small, independent fixes; drop the `view_width` refactor entirely.

A. Columns (src/finder/mod.rs).
- Split `select_in_column` (2881-2920): keep `focus_files` and the `if col == 0 { self.selection.select_only(ix) }` in the public fn, and move the rest (set `pane.selected`, `pane.scroll.scroll_to_item(ix, Nearest)`, `active_column = col`, `columns.truncate(col+1)`, push + `load_column` for `is_navigable()` else `ensure_preview`, `columns_scroll.scroll_to_item`, `cx.notify()`) into a private window-less helper, e.g. `fn show_in_column(&mut self, col, ix, cx)`.
- In `set_view_mode`, right after `self.columns.clear(); self.sync_root_column();` (2120-2121): if `self.effective_view() == ViewMode::Columns` (guard: while searching / no folder the mode is Columns but the list is shown and pane 0 mirrors search rows) pick `self.selection.cursor().filter(|ix| self.selection.contains(*ix)).or_else(|| self.selection.indices().next())` (same rule as `primary_entry`, 1780-1783) and call the helper with col 0. Pane 0 entries mirror `self.rows` 1:1 (2850), so the row index is the pane index. Do NOT call `selection.select_only`: tests/basics.rs `the_preview_pane_follows_the_selection` (922-949) crosses Columns with two items selected, and the multi-selection must survive the round trip.
- The existing `selection_changed(cx)` at 2127 then loads the preview via `column_selection()`.

B. Icons: re-issue the reveal once the real column count is known.
- Add `icon_reveal: Cell<Option<(usize, usize)>>` (item, column count used) to `Finder` (next to `grid_cols`, mod.rs:314/564; `scroll_to` takes `&self`, and `selected_size_memo` is an existing Cell precedent).
- `scroll_to` Icons arm (1894-1896): also `self.icon_reveal.set(Some((ix, self.grid_cols.max(1))))`.
- `render_icons` (views.rs:379), BEFORE `self.icon_layout(...)` at 385: `if let Some((ix, cols)) = self.icon_reveal.get() && cols != self.grid_cols.max(1) { self.scroll_to(ix) }`. Doing it at render time (not in the canvas prepaint at 390, which runs before the list's prepaint) guarantees the deferred scroll is consumed against a layout built with the new count.
- In the existing prepaint closure (395-399): when `!changed`, `this.icon_reveal.set(None)` (one stable frame ends the reveal, so plain window resizes never yank the scroll back to the selection).
- Clear `icon_reveal` where `icon_layout` is reset in `group_rows` (src/finder/groups.rs:130) so a row index can't go stale across a rebuild; both callers that reveal (`load_listing` 1090-1100, `set_view_mode` 2118-2125) call `rebuild_rows` before `scroll_to`, so the order is safe.
- Frame trace to verify: T0 scroll_to -> (ix,c0); frame 1 lays out c0, canvas sets n and requests a frame, list consumes stale scroll; frame 2 render sees c0 != n, re-issues with n, lays out n, canvas unchanged -> clears, list consumes correct scroll.

C. Add a `### Fixed` entry under `[Unreleased]` in CHANGELOG.md (selection carried into the column view; icon view scrolls to the selection after a view switch). No manual change needed.

Out of scope but adjacent: `location_changed`/`load_listing` with a `select` path in Columns (Back, Go Up, reveal) also leaves pane 0 unselected; the window-less helper from A is what that separate fix would reuse.

**Tests.** Headless UI tests in tests/basics.rs (its header already covers "where the selection goes"; `long_folder` is defined at line 13), using `Ui::open`, `ui.click`, `ui.select`, `ui.select_also`, `ui.selected()`, `ui.read(|f| f.column_names_for_test())`, `ui.resize`, `ui.press(&cmd("1"))`, `ui.exists`, `ui.snapshot(..).visible()`, `item_id` from tests/common/mod.rs.

1. `the_selection_comes_along_to_the_column_view`: `Fixture::new`, `ui.click("view-list")`, `ui.select("notes.txt")`, `ui.click("view-columns")`, assert `ui.selected() == ["notes.txt"]` (empty today) and `ui.read(|f| f.has_preview_for_test(&fx.path("notes.txt")))`. Then `ui.click("view-list")`, `ui.select("Projects")`, `ui.click("view-columns")`, assert `column_names_for_test()` == [[Budget.csv, notes.txt, Projects, Roadmap.md], [atlas]] (ui.click settles, so the pane's listing has landed).
2. Multi-selection survives: list, `ui.select("Roadmap.md")`, `ui.select_also("notes.txt")`, `ui.click("view-columns")` -> `ui.selected() == ["notes.txt"]` (cursor row), `ui.click("view-list")` -> `ui.selected() == ["notes.txt", "Roadmap.md"]`.
3. `switching_to_icons_keeps_the_selection_in_view`: `long_folder`, open (icons drawn at the default ~1536pt width, ~11 columns), `ui.click("view-list")`, `ui.resize(900., 600.)` (NOT 620: below ~800pt the view buttons fold into `view-menu`, tests/basics.rs:541-544; 900 gives ~5 columns), `ui.select("item 150.txt")`, `ui.click("view-icons")` (or `ui.press(&cmd("1"))`), then assert `ui.exists(item_id("tile", &fx.path("item 150.txt")))` and that its snapshot is `visible()`. This fails today: the test-mode draw at effect flush consumes the scroll against the 11-column layout, leaving lines ~10-15 of the 5-column grid on screen while the item is on line ~30.
Drop the proposed reveal_in_place test (c): in the harness the dirty window is drawn with the old rows in icon view before the listing task runs, so grid_cols is already correct and the test passes without the fix.

Existing tests checked and unaffected: basics.rs 877-901, 922-949, 1013-1039 (nothing selected before the switch, cursor None), 1196-1211; controls.rs 520-559, 1289-1299, 1405-1412; finder.rs 143-150, 354-365; columns.rs 329-352.

**Risks.** Icons fix is frame-order sensitive: re-issuing the scroll from a prepaint that precedes the virtual list (the existing canvas at views.rs:390, or a wrapper built like mod.rs:3372) is consumed against the stale layout in the same frame and silently does nothing; it must happen at render time or after the list child. The pending reveal must be cleared after one stable frame, or resizing the window in icon view would snap the scroll back to the selection. The proposal's shared `view_width` would regress Gallery/Columns -> Icons with the Preview Pane open (pane only exists in List/Icons), so it should not be implemented. Columns: carrying the cursor must not call `selection.select_only` (would collapse a multi-selection kept across views) and must be guarded by `effective_view() == Columns` (searching / Recents show the list while the mode is Columns). A selected folder now opens a second pane and spawns one directory listing on each switch to Columns (matches Finder, cheap). Gallery's auto-selected first item will now also be carried into Columns, consistent with how it already carries into List/Icons. Could not run cargo here, so the frame trace is from reading gpui-pre app.rs:2156-2170 and virtual_list.rs:597-739, not from an executed test.

## adhoc-index-feedback

**Temporary indexes for folders outside the search locations: no progress shown, rebuilt for every subfolder, and discarded indexes keep working**

Value 2 of 5, size M, search, ux.

**Problem.** Searching 'this folder' outside every search location builds a temporary index (crates/fffinder-core/src/engine.rs:476-508). (1) No feedback: Engine::status() and roots_indexing() only cover roots (400-420), so the footer shows nothing, and the results pane uses indexing_progress() from roots only (src/finder/mod.rs:3236-3252). While the temporary index scans, the search returns `indexing: true` with no rows; the retry loop then waits a fixed 24 x 25 ms because `roots_indexing() < indexing` is `0 < 0` (mod.rs:1442-1448) and `searching` is false meanwhile, so the pane alternates between 'No Results - Try a different name...' and the loading skeleton until the scan ends. SearchStats.indexing is used only to decide the retry (mod.rs:1427). Engine::prepare(scope, timeout), which waits for exactly these indexes (engine.rs:439-445), is never called by the app. (2) Reuse is by exact path (`r.path == scope`, engine.rs:491): after indexing /Volumes/Disk, searching in /Volumes/Disk/Photos starts a second full scan, and with MAX_ADHOC = 3 (line 26) browsing a few subfolders evicts the parent. A reused index is not moved to the front, so eviction is by creation order, not by use as the comment says. (3) Discarded indexes are never cancelled: eviction (line 500), removed roots (337) and settings-triggered rebuilds (381) just drop the RootIndex, but fff's running ScanJob holds its own strong handle (fff-search scan.rs:50-62), so FilePicker::drop, which sets `cancelled` (file_picker.rs:1978-1986), does not run until the job ends. The old walk, the per-file binary sniff that takes minutes on a large folder, and the watcher install all complete for an index nobody can search. Only Drop for Engine cancels (engine.rs:1322-1329). Flipping two index switches during 'Finishing index...' stacks full passes over the disk.

**Proposal.** Engine: (a) in indexes_for, match a temporary index that covers the scope (`scope.starts_with(&r.path)`), and move the one used to the front under the write lock; re-check under the write lock before starting a new index so two concurrent searches cannot both start one. (b) Call `picker.cancel()` on every RootIndex being discarded: roots not kept in set_roots, everything in the old Indexes in set_search_settings, and entries cut by `adhoc.truncate`. Window: (c) in the retry branch of run_search replace the roots_indexing polling loop with `engine.prepare(scope.as_deref(), Duration::from_millis(600))` on the background executor, so the search reruns as soon as the covering index (root or temporary) is ready, and keep `this.searching = true` while a retry is pending so the pane does not flip to the empty state. (d) When search_stats.indexing is true and roots are not scanning, show 'Indexing this folder...' with detail 'It isn't one of the search locations, so it is indexed for this search.' instead of 'No Results' (the same rule as in empty-state-messages).

**Checked plan.** Engine (crates/fffinder-core/src/engine.rs)

1. Cancel what is discarded. Add a helper that calls `picker.cancel()` on a Vec<RootIndex>, and call it only after the engine's `indexes` write guard is dropped. SharedFilePicker::cancel takes the picker's read lock (fff shared.rs:130-136), which can wait behind a queued writer during a long content search, and Engine::status() runs on the UI thread (mod.rs:1495, 1504).
- set_roots: `let old = std::mem::replace(&mut indexes.roots, keep)`, then cancel only old entries whose path is not in the new `roots`. Kept roots are clones of the same Arc; cancelling one kills a live index.
- set_search_settings: `std::mem::take` the whole Indexes and cancel its roots and adhoc.
- indexes_for: cancel what `adhoc.split_off(MAX_ADHOC)` returns, and the loser of the insert race in step 2.
- Optional: in set_roots, also drop and cancel temporary indexes that a root now covers.

2. indexes_for: keep exact-path matching; do not adopt `scope.starts_with(&r.path)`.
- A parent index walked from a non-git base skips hidden entries and prunes node_modules, target/debug and the like (fff walk/ripgrep.rs:23-33, ignore.rs:5-65). A folder indexed on its own is walked from its own base.
- So after searching /Volumes/Disk, a search in /Volumes/Disk/proj/node_modules, in a hidden folder, or in a git repository below it would find nothing or lose hidden files. Today it works, and docs/manual.md:378 documents it.
- Do the safe parts: when the reused index is not first, take the write lock and move it to the front. Keep RootIndex::start outside the lock (it canonicalizes the path, which can stall on a dead mount). Then, under the write lock, look for the path again; if another search inserted one meanwhile, return that and cancel the one just started.

3. Add `pub fn scope_indexing(&self, scope: Option<&Path>) -> usize`: how many of the indexes a search of `scope` uses are still scanning.
- Count the roots containing it, else the temporary index with that exact path. Use RootIndex::scanning().
- Apply root_spelling as search() does, and never start an index.
- roots_indexing() becomes scope_indexing(None).
- prepare() can stay for tests; give it the same root_spelling step.

4. Add `#[serde(default)] pub temporary: bool` to SearchResponse and GrepResponse, true when no root covers the (root-spelled) scope. The server builds its JSON by hand (src/server/mod.rs:407-507), so the protocol does not change.

Window (src/finder/mod.rs)

5. run_search retry: clone the scope for the retry task (`scope` is moved into the search closure) and poll `scope_indexing(scope)` in place of roots_indexing().
- Break when the count drops or is 0, after at least one 25 ms tick. The scan of a small folder usually ends before the first check, and today's loop would then still wait 600 ms.
- Do not use prepare(scope, 600 ms). For This Computer it waits for every root in turn, so results from a location that finished early come up to 600 ms late. docs/manual.md:125 and CHANGELOG.md:188 promise otherwise.
- prepare() also blocks an executor thread, skips root_spelling, and can start an index for a stale scope.

6. Do not hold `searching` true while a retry is pending. The message branch requires `!self.searching` (mod.rs:3236), so that would replace both the new message and the documented "Indexing…" state with a skeleton for the whole scan.
- Instead, let that branch also apply while `searching` when the last answer said `indexing`. The message then stays up through each rerun.
- 17a460e's results_pending() uses the same test. If search-skeleton-flash edits this branch, make the change once there.

7. Add `temporary` to SearchStats at both construction sites (run_search and content_search). Move the title and detail choice into a pure function of (stats.indexing, stats.temporary, roots progress):
- indexing and temporary: "Indexing This Folder…" with "It isn’t one of the search locations, so it’s indexed for this search."
- otherwise roots progress Some(n): today's "Indexing…" text, unchanged.
- otherwise: "No Results".
- Key on `temporary`, not on "roots are not scanning": index_status is a 400 ms poll, and a watcher-triggered rescan of a root also returns `indexing` with idle status. Share the rule with empty-state-messages.

Optional: a wait_for_scan of about 100 ms on a just-started temporary index inside search()/grep() would return a small folder's results on the first try, with no message flash.

Docs: add the new message to the table at docs/manual.md:120-128. Add CHANGELOG [Unreleased] Fixed entries for the message and retry, and for stopping discarded indexes. manual.md:378 stays true.

**Tests.** All of this runs on Linux; nothing here was run (read-only review).

Engine unit tests, in `mod tests` of crates/fffinder-core/src/engine.rs. Follow `searches_a_folder`: a temp dir named with the pid, `Engine::new(EngineOptions { roots: vec![], app_dirs: Vec::new(), db_dir: None, watch: false, search: Default::default() })`.

1. A temporary index reports its own scan (deterministic).
- Push a hand-built `RootIndex { path, canonical, picker: SharedFilePicker::default() }` into `engine.indexes.write().unwrap().adhoc`, as `scope_follows_the_base_spelling` builds one. An empty picker counts as scanning (engine.rs:101-107, 128-135).
- Assert roots_indexing() == 0, scope_indexing(Some(path)) == 1, scope_indexing(None) == 0.
- Assert scope_indexing(Some(other)) == 0 and that adhoc.len() is still 1 afterwards.
- Assert search() with that scope returns `indexing && temporary` and no hits.

2. Last-use order and no duplicate.
- Make folders a, b, c, d with one file each. Search with scope a, b, c, calling prepare(Some(..), 10 s) after each. Search a again, then d.
- Assert the adhoc paths are [d, a, c] and the file under a is found.
- Assert a second search of an existing scope leaves adhoc.len() unchanged.

3. Discarded indexes are cancelled.
- Write 60 files containing "needle". Add a helper that greps a cloned SharedFilePicker directly: `picker.read().unwrap().as_ref().unwrap().grep(&QueryParser::new(GrepConfig).parse("needle"), &GrepSearchOptions { page_limit: 10_000, ..Default::default() }).files_with_matches`.
- page_limit must be raised: the default of 50 caps an uncancelled grep below 60, so the proposed `< 60` assertion would pass without any cancel.
- With no abort_signal, fff uses the picker's cancelled flag (file_picker.rs:1397-1400) and stops within the first 8 files.
- For each of set_roots(&[]), an index-shaping settings change (toggle follow_symlinks), and eviction by a fourth scope: clone the picker first, in a single expression so the read_indexes() guard is dropped before the mutator runs. Assert 60 before and < 60 after.
- Guard the kept root: after set_roots(&[a, b]) then set_roots(&[a]), a's picker still returns every file.

Window unit test, in `mod tests` of src/finder/mod.rs. Use plain #[test] and import the function by name, as that module's comment requires.
- (indexing, temporary) gives "Indexing This Folder…".
- (indexing, not temporary, roots progress Some(12)) gives "Indexing…" with the count.
- (indexing, not temporary, roots idle) and (not indexing) give "No Results".

Headless UI smoke test, in tests/controls.rs next to `search_scope_button`, using tests/common/mod.rs.
- `Fixture::new`, plus a sibling temp folder outside the fixture root holding one file. `Ui::open`, `ui.go_to(&outside)`, `ui.click("search-scope")`, `ui.update(|f, _, cx| f.set_query("lonely".into(), true, cx))`.
- The scan runs on fff's real threads while the test clock is simulated. So loop `std::thread::sleep(10 ms); ui.settle();` until `ui.names()` holds the file, or a 10 s deadline passes, then assert.
- This is a smoke test for a path with no coverage today. The 25 ms claim is proven by engine test 1, not here.

Existing tests to keep green: `settings_search_engine` (tests/controls.rs:743-788) rebuilds the index twice, and `Ui::open_with` calls set_search_roots with unchanged roots (tests/common/mod.rs:228), which must not cancel anything.

**Risks.** - cancel() takes the picker's read lock. If it is called while the engine's `indexes` write lock is held, it can stall Engine::status() on the UI thread behind a long content search. Call it after the guard is dropped.
- In set_roots the kept roots share their Arc with the old list. Cancelling by "everything in the old list" would silently kill live indexes: content searches on them would stop after a few files. The kept-root test guards this.
- A content search still running on an index that is discarded at that moment stops early with partial results, because Engine::grep passes no abort_signal. The window reruns the search after a settings change. After a root removal or an eviction, the partial rows stay until the next search.
- cancel() only acts at fff's checkpoints: the directory walk still runs to its end before the result is dropped.
- The message rule and the render branch overlap empty-state-messages, search-skeleton-flash and results_pending() from 17a460e. Implement the rule once.
- Not fixed here, by choice: a subfolder of a temporarily indexed folder still gets its own scan. Reusing the parent would need a manual change and would lose results in hidden, pruned or git-ignored subfolders.
- The same first-call-empty behaviour exists for /v1/find and /v1/search with an outside `scope` (src/server/mod.rs:407, 477). It is out of scope unless the optional wait in search()/grep() is taken.

**Notes from later reviews.** 'search-skeleton-flash' and 'empty-state-messages' land before this one and change what the results pane shows while a search has no rows yet; build on them, and make sure the pane no longer flips between 'No Results' and the placeholder while a temporary index is being built. Also, from the reviewer of 'search-skeleton-flash': the text 'Indexing… N files indexed so far' is chosen from the status of all search locations, not of the indexes the search used, so a search of one finished folder that finds nothing says 'Indexing…' for as long as any other location is scanning, and no retry is coming for it. Fix that here if it fits; say so if you leave it. From the reviewer of 'live-search-results' (on the trunk; read its done record's note for this brief first: `RootIndex` has a new field `followed`, and a temporary index reports changes only if `engine.follow_changes()` is called after its watcher is up). Two things they found and left, both about index status, to fix here only if they are small and certainly right, else say you left them: the sidebar's 'N files indexed' is read only while an index is being built, so it stays at the old count as the watcher adds and removes files; and `Engine::rescan` rescans the search locations only, not the temporary indexes.

## files-changed-off-thread

**After a file operation during a search, every result is checked on disk on the window's thread**

Value 2 of 5, size S, ui, perf.

**Problem.** Found by the reviewer of 'search-skeleton-flash' (done record: /home/user/polish-tools/done/search-skeleton-flash.json, under 'Found, not fixed'). Finder::files_changed (src/finder/trash.rs) runs after every copy, move, rename, trash or put back. While a search is showing it does `self.results.retain(|r| r.entry.path.exists())`: one stat call per search result, on the window's thread. On a local disk that is quick, but results can come from any search location, and a stat on a stalled network volume blocks; the window then freezes after a file operation. The project already moved this kind of check off the window's thread elsewhere (CHANGELOG 0.5.1: 'Checking folders for changes no longer runs on the window's thread, where a stalled network volume could freeze the window').

**Proposal.** Do the existence checks on the background executor and apply the answer on the window's thread: collect the result paths, check them in a background task, then drop the ones that are gone from self.results if the search is still the same one (compare the search generation) and rebuild the rows. Only the paths the operation could have touched need checking when files_changed is told what changed; if the callers can pass that cheaply, check only those, otherwise check all results in the background. Keep the rest of files_changed as it is.

**Tests.** A headless UI test: during a search, trash (or rename away) one result; after settling, that row is gone and the others stay; an answer for an older search generation is ignored (start a new search before the check answers). Existing file_ops tests keep passing.

**Risks.** The row of a file that was just trashed must still disappear promptly, in the same settle as before from a test's point of view. A check that answers after the search changed must not touch the new results. Selection after the operation (the `select` argument) must still work.

## get-info-stop-walk

**Get Info: stop adding up a folder when the panel closes**

Value 2 of 5, size S, ui, perf.

**Problem.** Closing Get Info doesn't cancel the folder-size walk it started. The walk stops only when the `InfoPanel` is dropped (`Drop` sets `stop`, src/finder/info.rs:36-40), and `self.info` is only replaced by the next Get Info (info.rs:99) or cleared when nothing could be read (info.rs:125). `open_info_dialog` registers no `on_close` (info.rs:171-197), unlike the batch-rename dialog, which clears its state there (src/finder/batch_rename.rs:130). So after Done or Escape, `info::folder_size` keeps running to the end. It walks with rayon's `par_bridge` on the global pool, one stat per item on every core (crates/fffinder-core/src/info.rs:271-298). Cmd+I on the home folder or the whole disk, a glance, then Done leaves every core walking the disk for a total nobody will see, competing with indexing and searches. While it runs, the Size row only says 'Calculating...' (info.rs:271-273), with no sign of progress.

**Proposal.** In `open_info_dialog`, add `.on_close(move |_, _, cx| view.update(cx, |this, _| if let Some(panel) = &this.info { panel.stop.store(true, Ordering::Relaxed) }))`. Set the flag rather than `this.info = None`: the builder returns an empty dialog when `info` is `None` (info.rs:176-178), which would blank the panel during its closing fade. In the second `update_in` (info.rs:150-157), skip writing sizes when `stop` is already set. Optional follow-up: give `folder_size` a shared `AtomicU64` item counter and poll it every 200 ms while `panel.folders.is_none()`, so the row reads 'Calculating... 12,340 items'.

**Checked plan.** All code changes are in src/finder/info.rs. `Ordering` is already imported (line 21).

1. **`open_info_dialog` (171-198).** After `let Some(panel) = &finder.info else { return dialog; };` add `let stop = panel.stop.clone();`, then chain `.on_close(move |_, _, _| stop.store(true, Ordering::Relaxed))` on the dialog.
   - The candidate's literal snippet does not compile: it moves `view` into a nested `move` closure inside the `Fn` builder that still uses `view` (E0507). Capturing the panel's own `Arc` avoids that and needs no entity update.
   - The house-pattern alternative is `let close_view = view.clone();` plus `close_view.update(cx, |this, _| ...)`, as in batch_rename.rs:120/130 and preview.rs:433/530.
   - Do not set `this.info = None` here. The builder runs during the exit animation and would return the bare dialog (info.rs:176-178). Tests run with `set_reduce_motion(true)` (tests/common/mod.rs:193) and would not catch it.

2. **First `update_in` (121-135).** Right after `let panel = this.info_panel(&stop)?;` add `if panel.stop.load(Ordering::Relaxed) { return None; }`.
   - This is an addition to the proposal. A panel closed before its items were read then starts no walk and no `default_app_name` lookup.
   - It also stops the items-empty branch from calling `window.close_dialog(cx)` on some other dialog opened after the user already closed this one.
   - Borrowck is fine: same shape as the existing early-return branch.

3. **Second `update_in` (150-157).** Add `if stop.load(Ordering::Relaxed) { return; }` before filling in. Otherwise the partial total is written and `window.refresh()` shows a wrong "x MB … for N items" during the fade. `stop` here is the `Arc` returned by the first update.

4. **Field doc (line 32).** Change to "Stops the folder walk when the panel is closed or replaced."

5. **Test accessor**, in this file's `impl Finder` because `InfoPanel`'s fields are private to it:
   `#[doc(hidden)] pub fn info_walk_for_test(&self) -> Option<(bool, bool)> { self.info.as_ref().map(|p| (p.stop.load(Ordering::Relaxed), p.folders.is_some())) }`
   It returns (stopped, total arrived), which is stronger than the proposed `Option<bool>`.

**Corrections to the candidate's scope**
- No change is needed in crates/fffinder-core/src/info.rs. `folder_size` already winds down once the flag is set, and the proposed unit test already exists at info.rs:386 (`folder_size(&dir, &AtomicBool::new(true)).items == 0` in `adds_up_a_folder`).
- Leave out the optional progress counter. It changes `folder_size`'s signature (second caller at engine.rs:1154), adds a 200 ms polling task needing its own cancellation, and contradicts docs/manual.md:547. The harness cannot observe mid-walk state, so it would ship untested. Make it a separate candidate if wanted.

**Docs**
- CHANGELOG.md: add a `### Fixed` entry under `## [Unreleased]`, e.g. "Closing Get Info stops adding up a folder's size. Before, the count ran to the end in the background on every core."
- docs/manual.md:547: add "Closing the panel stops the count."

**Why "the current panel" is always the one shown**
- Get Info cannot stack. `GetInfo` is handled on the Finder's own div (mod.rs:3063) and bound in the `finder` key context (actions.rs:159); the modal dialog layer sits outside that subtree.
- A closing dialog's handlers are inert (`!closing && …`, gpui-component dialog.rs:751-754), so `on_close` fires once.

**Tests.** Add `#[gpui_kit::test] fn closing_get_info_stops_adding_up_its_folder` to tests/file_ops.rs, next to `get_info_shows_details_and_changes_permissions` (line 727). It uses the headless harness in tests/common/mod.rs and runs on Linux.

Helpers: `Fixture::new`, `Ui::open`, `ui.select`, `ui.press(&cmd("i"))`, `ui.click("info-done")`, `ui.press("escape")`, `ui.press("enter")`, `ui.close_dialog()`, `ui.dialog_open()`, `ui.click_without_settling`, `ui.settle()`, `ui.window(..)`, and `ui.read(|f| f.info_walk_for_test())`.

1. **Each way of closing sets the flag.** For Done, Escape, Return and the ×:
   - select `Projects`, press Cmd+I, assert `Some((false, true))` (`settle()` runs the walk to the end);
   - close, assert `!ui.dialog_open()` and that the first element is `true`.

2. **Closing before anything is read adds nothing up.**
   - Select `Projects`, then `ui.window(|window, cx| window.press(&cmd("i"), cx))`. This needs `use gpui_kit::test::TestWindowExt as _;` and does not settle, so the spawned read has not run.
   - `ui.click_without_settling("info-done")`, then `ui.settle()`, then assert `Some((true, false))`.
   - Today this gives `Some((false, true))`, so the test fails without the fix.

3. **A new panel starts fresh.** Select `notes.txt`, press Cmd+I, assert `Some((false, true))`.

**Existing tests that must stay green**
- tests/file_ops.rs:727-769 opens and closes Get Info three times and expects "for 2 items" on the second open.
- tests/file_ops.rs:1047-1051 and tests/controls.rs:264.
- crates/fffinder-core/src/info.rs:381-388 `adds_up_a_folder` already covers `folder_size` with the flag pre-set.

**Limits**
- The harness cannot interrupt a walk mid-way; the deterministic executor finishes it inside `settle()`. Mid-walk wind-down rests on reading `folder_size` (the check precedes the stat and the recursion).
- The exit-animation behaviour is not testable under reduced motion.
- Nothing was run here, as instructed. Commands: `cargo test --test file_ops get_info` and `cargo test -p fffinder-core info::`.

**Risks.** Low; nothing changes while the panel stays open.

- **Direct closes skip `on_close`.** `window.close_dialog` does not run it, so those paths keep today's behaviour. The only direct close of this dialog is info.rs:125-126, which drops the panel first (`Drop` sets the flag).
- **Do not "simplify" to `this.info = None`.** That blanks the panel during the 0.25 s exit slide, and reduced-motion tests would not catch it.
- **Skipping the late write is required.** Without it a partial, wrong total flashes during the fade.
- **The closed panel stays in `self.info`**, stopped and with `folders: None`, until the next Get Info. That matches today; only the dialog builder and `toggle_permission` read it.
- **The early-close test assumes spawned work does not run until `settle()`.** `click_without_settling` at tests/controls.rs:671/678 relies on the same assumption.
- **Value is modest.** The waste is invisible until it bites: seconds on a warm SSD, minutes for `/` or a network share. During that time the app's own two-second folder-change checks queue behind the walk on rayon's global pool.

## path-bar-no-io

**The path bar rescans /Volumes on the window's thread every frame (macOS)**

Value 2 of 5, size S, ui, perf.

**Problem.** `render_path_bar` runs on every draw of the window (src/finder/mod.rs:3199) and calls `places::breadcrumbs(&target)` each time (src/finder/bars.rs:211-212). On macOS `breadcrumbs` calls `volumes()` just to name the root crumb (crates/fffinder-core/src/places.rs:269-286), and `volumes()` reads `/Volumes` and does a `read_link` and a `metadata` on every entry (places.rs:116-160). So each frame, on every scroll, hover and selection change, does directory and stat calls on the UI thread. Normally that is microseconds, but a stat on a stalled network mount blocks, and here it would block every frame. The code already treats this as a hazard elsewhere: the sidebar's locations are read off the UI thread 'since naming a volume can wait on its drive' (src/finder/volumes.rs:23-27), and CHANGELOG.md:26 records moving folder checks off the window's thread because 'a stalled network volume could freeze the window'. Linux and Windows are unaffected, since their `breadcrumbs` does no I/O.

**Proposal.** Make the path bar's crumbs free of I/O on every platform. Add `places::breadcrumbs_named(path, root_name: Option<&str>)` in places.rs holding the pure part, and keep `breadcrumbs(path)` as a wrapper for its existing callers (tests/controls.rs:438, tests/basics.rs:484, 505). In `render_path_bar` take the root's name from the locations the browser already loaded off-thread: `self.locations.iter().find(|p| p.path.parent().is_none()).map(|p| p.name.as_str())` (the boot volume's Place comes from the same `volumes()`, places.rs:151-158), and call `breadcrumbs_named`. Until locations arrive the root crumb falls back to `/` (or 'Macintosh HD' on macOS) for a frame or two. Optionally memoize the crumb list by target path in the Finder so the Vec is not rebuilt each frame.

**Checked plan.** Corrected plan. The sketches below were not compiled (cargo was off limits). Keep Linux and Windows labels exactly as they are; only macOS names the root.

1. crates/fffinder-core/src/places.rs — make `breadcrumbs` pure.
   - New signature: `pub fn breadcrumbs(path: &Path, root_name: Option<&str>) -> Vec<(String, PathBuf)>`.
   - Keep the ancestors/reverse body and delete the `#[cfg(target_os = "macos")]` block (275-284).
   - After reversing: if `root_name` is `Some` and the first crumb's path has no parent, set its name to `root_name`.
   - Say in the doc comment that it reads nothing from disk because the path bar calls it on every draw.
   - If you would rather not edit the three test call sites, add `breadcrumbs_named` and have `breadcrumbs(path)` call it with `None`. Either way, `breadcrumbs` must no longer call `volumes()`.

2. places.rs — add the platform policy beside it:
   `pub fn root_crumb_name(locations: &[Place]) -> Option<&str> { cfg!(target_os = "macos").then(|| locations.iter().find(|p| p.path == Path::new("/")).map_or("Macintosh HD", |p| p.name.as_str())) }`
   - Match by path, not "first place without a parent".
   - Use `cfg!`, not `#[cfg]`: the macOS branch is then type-checked and linted on Linux, and no dead code trips `clippy -D warnings` on the three CI systems (.github/workflows/ci.yml:14, 32).
   - `volumes()` and `device_id` stay in use through `locations()`.

3. src/finder/bars.rs:212 — `let parts = places::breadcrumbs(&target, places::root_crumb_name(&self.locations));`
   - `self.locations` is already filled off-thread by `reload_locations` (volumes.rs:25-38), which calls `cx.notify()`, so the bar redraws when the name arrives.
   - The borrow ends with the call because the Vec is owned.

4. bars.rs:235-236 — add `.aria_label(name.clone())` right after `.id(("crumb", ix))`, before the `.map` that moves `name`. This is what lets a test read the crumb, the same way `status-text` does.

5. Do not memoize the crumbs. The target changes with the selection while searching, and the Vec is trivial next to the elements built each frame.
   Do not read the name synchronously "just once" to avoid the fallback; that puts the blocking call back at window open.

6. CHANGELOG.md — add a `### Fixed` section under [Unreleased]: on macOS the path bar read the mounted volumes every time the window was drawn, so a stalled network volume could freeze the window; it now uses the name the sidebar's Locations already loaded in the background.

7. Update callers for the new parameter: tests/controls.rs:438, tests/basics.rs:484 and 505, places.rs:295 (pass `None`; they only use the length).

**Tests.** CI runs `cargo test --workspace` on macOS, Ubuntu and Windows, so every test must hold on all three.

Unit tests in crates/fffinder-core/src/places.rs `mod tests` (288-304):
- Update `breadcrumbs_root_first` for the new parameter.
- Add one for the pure function: `breadcrumbs(Path::new("/a/b"), Some("Disk"))` gives names `["Disk", "a", "b"]` and paths `["/", "/a", "/a/b"]`; with `None` the root name is `/`. Made-up paths prove it reads no disk, and this holds on Windows too.
- Add one for the policy: `root_crumb_name(&[Place named "Disk" at "/"])` equals `cfg!(target_os = "macos").then_some("Disk")`, and `root_crumb_name(&[])` equals `cfg!(target_os = "macos").then_some("Macintosh HD")`.

Headless UI test in tests/controls.rs, beside `path_bar_crumbs_navigate` (433), using tests/common/mod.rs:
- `Fixture::new("crumb-root")`, `Ui::open(cx, &fx.dir)`.
- `let root = fx.dir.ancestors().last().unwrap().to_path_buf();` so the place matches `C:\` on Windows.
- Build `fffinder_core::places::Place { name: "Disk".into(), path: root.clone(), icon: PlaceIcon::Drive, ejectable: false }`.
- `ui.update(|f, _, cx| { f.set_locations_for_test(vec![place]); cx.notify(); })`, the pattern at tests/file_ops.rs:988-994.
- Read `ui.snapshot(("crumb", 0usize)).label()`. Expect `"Disk"` on macOS and `fffinder_core::entry::display_name(&root)` (`/` or `C:\`) elsewhere. On Linux this pins that the label did not change even though Locations names that root.

Existing tests that must still pass: `path_bar_crumbs_navigate` (tests/controls.rs:433), `items_drop_on_the_path_bar` (tests/basics.rs:479), `the_current_folder_stays_in_a_long_path_bar` (tests/basics.rs:497), `volumes_eject_free_space_and_servers` (tests/file_ops.rs:967).

The freeze itself cannot be reproduced on Linux. The macOS-positive assertions only run on the macOS CI runner.

**Risks.** - The benefit is macOS-only and cannot be measured or compiled for macOS on Linux; keep the new code platform-neutral (`cfg!`) and rely on the macOS CI runner.
- Every Finder (each window, tab and split pane) starts with empty `locations` (mod.rs:616). A Mac whose boot volume is renamed shows "Macintosh HD" for a frame or so until Locations arrive, at the same moment the sidebar's Locations pop in. Default-named disks see no change.
- If a volume is stalled at launch, the background `places::locations()` never returns, so the root crumb stays "Macintosh HD" and the sidebar's Locations stay empty. The window keeps drawing, which is still better than today.
- A renamed boot volume shows after the next window activation (mod.rs:522-527) instead of on the next frame.
- As written, the candidate would have renamed the Linux root crumb to "Computer" and mislabelled every non-first Windows drive; the macOS gate and path match avoid both. Showing sidebar names on Linux and Windows would be a separate, visible change with its own label flip on every new tab.
- The new `aria_label` changes what assistive tech reads for a crumb (its name); this is harmless.

**Notes from later reviews.** From the implementer and the reviewer of 'show-in-enclosing-folder-key' (on the trunk; read its done record, under 'Found, not fixed'): `path_bar_target` (src/finder/bars.rs) treats the Trash like Recents (`is_searching() || current_folder().is_none()`), so with an item selected in the Trash the path bar shows the item's path inside the Trash's own storage folder, and a click on a crumb opens that folder as an ordinary one (probed: the location became Folder("…-trash")). You are in that function anyway: in the Trash the bar should not offer the storage folder's crumbs (show the Trash itself, as with nothing selected, or the place the item came from if that is at hand and certainly right). Give it a test that fails today. If it turns out bigger than a few lines, leave it and say so.

## preview-page-keys

**Page Up/Down and Cmd+=/−/0 act on the wrong thing when the item isn't a PDF or Quick Look has its own window**

Value 2 of 5, size S, ui, bug.

**Problem.** Page Down and Page Up are bound to `NextPdfPage`/`PreviousPdfPage` in both the files and Quick Look contexts (src/actions.rs:203-204, 210-211). In the browser the handlers branch on `shows_large_preview()`, which is `quick_look_open || Gallery` (src/finder/pdf.rs:558-560), and then always call `step_pdf_page(±1, Surface::Gallery, cx)` (pdf.rs:617-640), which returns when no PDF is open (pdf.rs:469-472). Results: (a) while Quick Look's window is open, Page Up/Down in the browser's list or icon view do nothing: they neither move a screenful (`move_by_page` is skipped) nor page the PDF Quick Look shows, since the surface is hard-coded to Gallery although `pdf_surface()` exists (pdf.rs:336-341). (b) Cmd+=/−/0 in the browser go to `step_preview_size(steps, Surface::Gallery, cx)` (pdf.rs:608-613), so they cannot zoom a PDF shown in Quick Look from the list view (`zoom_pdf` returns when that surface has no frame, pdf.rs:516-519). (c) In the gallery with any non-PDF selected the page keys are dead (`move_by_page` returns for Gallery, src/finder/mod.rs:1872), and in Quick Look they do nothing for text, code or table previews, so a long text preview cannot be scrolled from the keyboard because the arrows move between items. (d) `step_preview_size` changes the saved `preview_font_size` whenever the item is not a PDF (pdf.rs:573-577; mod.rs:2258-2261), even when the preview has no text (image, folder, video). Quick Look hides the size buttons then (src/finder/quick_look.rs:154-157; tests/controls.rs:1231-1237) but the keys still silently change the setting, so the next text preview opens at an unexpected size. (e) `step_pdf_page` acts on whatever PDF was opened last, not the selected item (pdf.rs:470).

**Proposal.** pdf.rs: 1) `next_pdf_page`/`previous_pdf_page`: compute `let pdf_shown = self.primary_entry().is_some_and(|e| self.pdf_view(&e).is_some());`. If `pdf_shown && self.shows_large_preview()` call `self.step_pdf_page(by, self.pdf_surface(), cx)`; else if a text-like preview is shown large, scroll it by a page (step 4); else `self.move_by_page(by, false, window, cx)`. 2) `preview_key`: pass `self.pdf_surface()` instead of `Surface::Gallery`. 3) `step_preview_size`: in the non-PDF arms, do nothing unless `self.preview_of(&entry).has_text() || self.explores(&entry, cx)`, the same condition that shows the size buttons. In `step_pdf_page`, also require `pdf.path == primary path`. 4) Keyboard scrolling of text, code, table and font previews in Quick Look and the gallery: hold a `ScrollHandle` per `Surface` on `Finder`, build those scroll areas with `track_scroll` plus `.vertical_scrollbar(&handle)` (the pattern at pdf.rs:679-688) in place of `overflow_y_scrollbar()` in src/finder/preview.rs:963, 990, 1013, 1057, and have the page handlers move the offset by 90% of `handle.bounds().size.height`, clamped to `handle.max_offset()`. Markdown keeps its own scrolling. Update the shortcut tables in docs/manual.md.

**Checked plan.** Do the corrected steps 1-3 only; split step 4 into its own candidate.

**`src/finder/pdf.rs`**
1. Add `fn previewed_entry(&self, surface: Surface) -> Option<FileEntry>`: `primary_entry()`, or for `Surface::Gallery` the first row. Use it in `render_gallery` (`src/finder/views.rs:1224-1226`) so the fallback has one source.
2. Replace `shows_large_preview()` with two routers:
   - **Zoom and size keys** (`preview_key`): `fn browser_surface(&self) -> Option<Surface>`.
     - `quick_look_open && quick_look_window.is_none()` (the `--term` dialog covering the view) gives `QuickLook`.
     - Gallery view gives `Gallery`.
     - Otherwise `quick_look_open` gives `QuickLook`.
     - Otherwise `None`, and `cx.propagate()` as today.
   - **Page keys** (`next_pdf_page`/`previous_pdf_page`):
     - `--term` dialog open: `step_pdf_page(by, Surface::QuickLook, cx)`.
     - Gallery view: `step_pdf_page(by, Surface::Gallery, cx)`.
     - Otherwise `move_by_page(by, false, window, cx)`, whether or not Quick Look's window is open. Quick Look follows through `selection_changed` (`mod.rs:1720-1731`).
   - Do not use `pdf_surface()` in either; it picks Quick Look over the gallery the buttons were clicked in.
3. `step_pdf_page(by, surface)`: act only when `previewed_entry(surface)` is the open PDF (reuse `pdf_view(&entry)`). This fixes (e) and keeps the first-row case working.
4. `step_preview_size(steps, surface)`:
   - Take `previewed_entry(surface)`; return if none.
   - If `pdf_view(&entry).is_some()`, call `zoom_pdf`.
   - Else change the font size only when `self.preview_of(&entry).has_text() || (surface == Surface::QuickLook && self.explores(&entry, cx))`.
   - Else do nothing.
   - `explores()` does not check `quick_look_open`, so the surface test is needed. Compute the PDF bool before the `&mut self` calls.

**Leave alone**
- Quick Look's own handlers (`quick_look.rs:99-103`, `140-148`; `preview.rs:595-605`, `622-633`) already pass `Surface::QuickLook` and pick up the fixes through the two shared functions.
- `actions.rs` needs no change.

**Docs**
- `manual.md:923`: say the keys also act on Quick Look's preview while it is open.
- `manual.md:455`: say previews without text ignore the keys.
- `manual.md:282`, `:903`, `:961` stay true.
- Add `### Fixed` entries under `[Unreleased]` in `CHANGELOG.md`.

**Pitfalls**
- `shown_pdf_page()` and `shown_pdf_zoom()` read Quick Look's surface while it is open (`pdf.rs:345-357`); tests must account for that.
- `zoom_pdf` needs the surface laid out, so let Quick Look's window draw before pressing keys in the browser.
- `pdf.zoom` is shared by both surfaces; that is unchanged.

**Step 4, if taken up later**
- Needs a per-surface `ScrollHandle` on `Finder`, a hand-built scroll-area helper, a `surface` parameter on `preview_body`, and a reset on selection change.
- Needs a decision on Markdown and the explore grid.

**Tests.** Headless UI tests with `Ui`, `Fixture`, `cmd`, `in_quick_look`, `press`, `click`, `select`, `settle` from `tests/common/mod.rs`. The red/green labels are predictions from reading the code; nothing was run.

1. **`tests/basics.rs`**, beside `page_up_and_down_move_through_a_long_list` (uses `long_folder`):
   - List view, `home`, `space`, then `pagedown` in the browser.
   - Assert the cursor moved more than 5 rows and `quick_look_is_open()`; `pageup` returns.
   - Red today.

2. **`tests/finder.rs`**, new test beside `pdfs_scroll_and_zoom`, copying `crates/fffinder-core/testdata/pages.pdf`:
   - **Zoom from a list.** List view, select the PDF, `space`, `ui.in_quick_look(|ui| ui.settle())`, record `shown_pdf_zoom()`. `ui.press(&cmd("="))` in the browser; zoom grew. Red today.
   - **Gallery keeps its own surface.** Gallery view with Quick Look open, `ui.click("pdf-next")` in the browser. `shown_pdf_page()` stays `Some((1, 2))` (Quick Look's copy). Close with `in_quick_look(|ui| ui.press("space"))`; then `Some((2, 2))`. Green today; guards against the `pdf_surface()` regression.
   - **Nothing selected.** Name the PDF `A.pdf` so it sorts first, gallery view, no selection. `ui.click("pdf-zoom-in")`: zoom grew and `config().preview_font_size == 11.5`. Red today. `ui.click("pdf-next")` gives `Some((2, 2))`.
   - **Stale PDF.** Gallery: select the PDF, select `notes.txt`, `pagedown`, select the PDF again. Assert `Some((1, 2))`. Red today.

3. **`tests/controls.rs`**, `quick_look_text_size`, in the existing `Projects` folder block (`:1231-1239`):
   - `ui.in_quick_look(|ui| { ui.press(&cmd("-")); ui.press(&cmd("0")); })`.
   - Assert the size is still 32. Red today: it would go to 31, then 11.5.

`pdfs_scroll_and_zoom` and `long_pdfs_keep_only_the_pages_near_view` (`tests/finder.rs:470-597`) must stay green.

**Risks.** - **`--term` dialog path is untestable headless.** It depends on `crate::term::active()`. Treating `quick_look_open && quick_look_window.is_none()` as the dialog also covers the brief moment before the Quick Look window opens (`quick_look.rs:243-275`), which is harmless.
- **Size keys ignored while loading.** With the `has_text()` guard, the keys do nothing during `Preview::Loading` and before a PDF's document loads; today they change the size at once.
- **Page keys with Quick Look showing a PDF.** From a list, icon or column view they will move the selection a screenful, not page the PDF. This matches `manual.md:282` and avoids trapping list paging, but it is a design choice the proposer argued the other way.
- **Step 4 left out.** Page keys still do nothing for text, code, table and Markdown previews in Quick Look and the gallery.
- **If step 4 is done as proposed.** It rebuilds four scroll areas by hand, with layout and bounce regressions that headless tests cannot see, and leaves Markdown inconsistent.

## query-history-boost

**The 'opened before for this search' boost is lost when the search is scoped, kind-filtered, quoted, or uses a glob**

Value 2 of 5, size M, search, bug.

**Problem.** track_open records the opened file under the text the user typed and the root's path (crates/fffinder-core/src/engine.rs:1308-1313). fff looks the boost up under `query.raw_query` (fff-search-0.11.0/src/file_picker.rs:1115-1128), which is the string the engine built, and gives the matching file +1000 (score.rs:805, since the engine leaves min_combo_count at 0 through `..Default::default()` at 672-680). The engine rewrites that string whenever the search is not a plain unscoped Any Kind query: quoted phrases are unwrapped and globs case-folded (525-529), a one-letter query is emptied (640-641), kind extensions are appended (645-648), the scope glob is appended (649-652), and a trailing folder token is moved (656-661). In all those cases the key never matches, so the promise in docs/manual.md:362 ('files you open often rank higher for the same search later') silently does not hold: only the weaker 1%-per-open frecency boost remains (score.rs:717). Folder searches never get it (fuzzy_search_directories takes no tracker, engine.rs:698), and opens from a temporary index outside the search locations are not recorded at all, because track_open walks `roots` only (1301-1302).

**Proposal.** Make the engine own the lookup so both sides use one key. In Engine::search, after the parts are merged and before rerank, read `self.queries` and call `get_last_query_entry(typed, &root.path, 0)` for each index searched, where `typed` is the original `req.query` trimmed (and lowercased if fuzzy-word-case is done; apply the same normalisation in track_open). Add +1000 to the RawHit whose path equals the entry's `file_path`. Pass `None` to fff as the tracker at 713 and 741 so the boost is not applied twice. In track_open, iterate `roots` and `adhoc` so folder-scoped searches outside the search locations are recorded too.

**Checked plan.** All code changes are in crates/fffinder-core/src/engine.rs. Keep the candidate's direction (the engine owns both sides of the key), with these corrections.

1. Add one key helper and one constant near `DEPTH_PENALTY` (engine.rs:1503).
   - `fn history_key(query: &str) -> String` returns `query.split_whitespace().collect::<Vec<_>>().join(" ")`.
   - `const OPENED_BEFORE_BONUS: i32 = 1000;` (the value fff adds at score.rs:805).
   - Do not lowercase unless the separate fuzzy-word-case change lands; the helper is then the single place to do it. Trim plus collapse keeps every existing single-spaced DB entry valid, since they were written with `query.trim()`.

2. `Engine::search` (engine.rs:510-601).
   - Compute `let key = history_key(&req.query);` before line 525. `req` is shadowed at 526 with the rewritten query, so reading it later gives the wrong text.
   - After the strict/typo `retain` (578-580), next to the rerank loop (581-583): if `key` is non-empty and `self.queries.read()` yields `Some(tracker)`, then for each `root in &indexes` call `tracker.get_last_query_entry(&key, &root.path, 0)`.
   - On `Ok(Some(entry))`, `saturating_add(OPENED_BEFORE_BONUS)` to every `raw` hit with `hit.path == entry.file_path`.
   - `rerank` is additive, so doing this before or after it is equivalent.
   - Keep `&root.path` as the project key on both sides so existing entries stay valid.

3. `search_root`: delete `qt_guard`/`qt` (670-671) and pass `None` at 713 and 741. `recents` (815) and `list` (1058) already pass `None`. This removes double-boosting and fff's suffix false positives.

4. `track_open` (1290-1315).
   - Compute `let key = query.map(history_key).filter(|k| !k.is_empty());`.
   - Clone `roots.iter().chain(adhoc.iter())` out of the read guard, not roots only.
   - Per index, read `picker.base_path()` under a short picker read lock.
   - Accept the index when `path.starts_with(&root.path) || path.starts_with(&base)`.
   - Keep the `update_single_file_frecency` call; it now also refreshes temporary indexes.
   - Record `qt.track_query_completion(&key, &root.path, &root.in_base(path, &base))`. `in_base` (engine.rs:91) turns a root-spelled path into fff's spelling and leaves a base-spelled one alone, so the stored path equals later `RawHit.path` values, which are `join_rel(&base, rel)`.
   - Without this step the feature and its test fail on windows-latest CI.

5. Do not claim folder searches learn. Folder opens are never tracked (src/finder/mod.rs:2020-2023), and tracking navigation is a separate product decision.

6. Docs.
   - CHANGELOG.md: add `### Fixed` under `[Unreleased]`. The file you opened for a search comes first the next time you type the same words. Before, that held only for a This Computer / Any Kind search without quotes or case-folded globs; a search in one folder, with a kind chosen, or from a Smart Folder forgot it.
   - docs/manual.md:362: optionally add that it holds whichever scope or kind is chosen.

Optional follow-up, not required: src/finder/mod.rs:2019 also passes the query for Contents searches, so a content search for `report` overwrites the file remembered for the name search `report`. A one-line fix is `(self.is_searching() && !self.search_contents).then(...)`. That file is busy, so leave it out unless it is cheap to merge.

Rejected alternative: overwrite `parsed.raw_query` with the typed text so fff keeps doing the lookup. fff also reads `raw_query` for the dirs-only decision (file_picker.rs:1260-1261), which engine.rs:654-661 deliberately works around. It would also leave one-letter queries, glob-only queries and the suffix false positives unfixed.

Pitfalls:
- Use let-chains as the file already does.
- `get_last_query_entry` is public and `QueryMatchEntry.file_path` is a public field (query_tracker.rs:16-20, 331).
- Do not hold a picker lock while taking `self.queries.write()`; today's code releases it first, so keep that order.

**Tests.** Add a unit test in the `mod tests` of crates/fffinder-core/src/engine.rs, e.g. `the_file_opened_for_a_search_comes_first_in_any_scope`, modelled on `searches_a_folder` (2084-2146) and `quoted_words_match_names_letter_for_letter` (1987-2044). No UI harness is needed: tests/common/mod.rs builds its Engine with `db_dir: None` (:165) and cannot see history.

Fixture:
- `dir = std::env::temp_dir().join(format!("fff-history-{}", std::process::id()))`
- `dir/root/sub/report.txt` and `dir/root/sub/quarterly-report-draft.txt`
- the same two files in `dir/outside/` (not a root)
- `Engine::new(EngineOptions { roots: vec![dir.join("root")], app_dirs: Vec::new(), db_dir: Some(dir.join("db")), watch: false, search: Default::default() })`
- `assert!(engine.wait_ready(Duration::from_secs(10)))`

Use these names, not the candidate's frecency setup, so the "before" order is deterministic through `rerank`: an exact stem gets +120 and a boundary match +55.

Helper: `first(query, scope, kind)` returns `engine.search(&SearchRequest { query, scope, kind, limit: 20, ..Default::default() }).hits[0].entry.name`.

Steps:
1. Before any open, `first("report", Some(root/sub), Any) == "report.txt"`.
2. Take the draft's path from a search hit (`hits.iter().find(..).entry.path`), as the UI does, and call `engine.track_open(&path, Some("report"))`. On Windows CI this exercises the base-spelled path.
3. Assert the draft is first for each of:
   - (a) scope `root/sub`, Any kind — fails today because fff looks up `report sub/**`;
   - (b) no scope, `KindFilter::Document` — fails today;
   - (c) query `"report"` in quotes — fails today;
   - (d) query `report` typed with a double space or surrounding spaces — covers key normalisation;
   - (e) no scope, Any kind — passes today; this is the regression guard for passing `None` to fff.
4. A different query (`draft`, or `quarterly` with no history) is not boosted beyond its natural order, and `first("rep", ..)` is still `report.txt`.
5. Temporary index: `assert!(engine.prepare(Some(&outside), Duration::from_secs(10)))`, then `track_open(outside/quarterly-report-draft.txt, Some("report"))`, then `first("report", Some(outside), Any)` is the draft. Fails today because `track_open` walks roots only.
6. Finish with `drop(engine); let _ = std::fs::remove_dir_all(&dir);`.

Run with `cargo test -p fffinder-core history`. The full `cargo test --workspace` on macOS, Ubuntu and Windows runs in CI.

**Risks.** - Semantics: the remembered file for a set of typed words is now boosted in every scope and kind where it appears among the results. That is already half true today, since the write key never included scope or kind. There is still one remembered file per (search location, words), so opening another file for the same words replaces it.
- Lost guarantee in the plain case: fff no longer adds +1000 before pagination, so the remembered file must be among the candidates fff returns unboosted (at least 2,000 per location, engine.rs:665-669). It was shown when the user opened it, so it normally is. It can drop out only after large index drift, or when the same words are searched in a much wider scope than where it was opened; in the second case there is no boost today either. If that matters, keep passing the tracker to fff and subtract `score.combo_match_boost` when building `RawHit`s, so fff only pulls the file into the page.
- Windows path spelling cannot be checked on Linux. Step 4 of the plan (base spelling via `in_base`) is what makes windows-latest CI pass; that CI runs `cargo test --workspace` (.github/workflows/ci.yml:14,34).
- This is the first test in the repo that opens the LMDB databases. Put the db dir outside the indexed root, and do not hard-fail on cleanup: background scan threads hold clones of the frecency handle, so the env files can still be mapped when the test ends, and Windows refuses to delete them.
- The server and the Nushell module share the GUI's engine (src/lib.rs:83-112), so their `search` results get the same boost for more query shapes than before. Plain queries already get it today.
- `SearchHit.score` values are unchanged in the plain case and rise by 1000 for the remembered hit in the newly covered cases.

**Notes from later reviews.** 'fuzzy-word-case' is already on the trunk and changed the text a search is remembered under (lowercase, unquoted, glob-folded): read its done record and its commit before anything else. Do only what is still missing after it (the scope, the Kind filter, a one-letter query). If nothing worth a change is left, return skipped with the evidence.

