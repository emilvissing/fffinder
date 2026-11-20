# Check of the assembled changelog (20 Nov)

A reader checked the [Unreleased] section, as assembled from the commit messages, against the code at the tip of
`search-ui-polish`, and a skeptic then tried to refute each finding. Eleven findings: ten confirmed, one refuted.
What was done with each is in `changelog-fixes.md`, from 9 on.

## 1. accent-typo-exception-gone: confirmed

- Entry: "With **Allow typos** on, a name search that" (Changed)
- Kind: untrue-at-tip, severity 3, the reader's confidence high
- Sentence: A word with an accent or in another alphabet is held to this only where a path is all in A to Z, digits, and symbols: a name can store a letter such as `ñ` as one character or as two, as macOS does, so other paths are still listed for such a word as loosely as before.
- Problem: 362de8f took this exception out. At the tip every word, whatever its letters, is held to letters in order or a near miss, and nothing is listed "as loosely as before" for a word with an accent. The entry at line 58 of the text says so itself ("A misspelled word with an accent or in another alphabet now lists only near misses"), so the two entries contradict each other.
- The reader's evidence: crates/fffinder-core/src/engine.rs:1061-1072 (one check for all words, no exception); `may_be_spelled_apart` is nowhere in the tree (rg finds none), removed by 362de8f (its diff has `-fn may_be_spelled_apart`, and its message says "the near-miss rule holds for every word"). docs/manual.md:1037 lost the same sentence in 362de8f, and docs/manual.md:376 now says that to Allow typos another accent, or none, is a wrong letter.
- The reader's correction: REMOVE
- The skeptic's evidence: crates/fffinder-core/src/engine.rs:1061-1072 at 362de8f holds every word to one check: `let strict = |hit: &RawHit| checked.iter().all(|w| is_subsequence(w, hit.looked_in(w)));` and, when no hit is strict, `is_subsequence(w, text) || near_miss(w, text, typo_budget(w))`. 362de8f's diff takes out `|| may_be_spelled_apart(w, &hit.rel)` and the function (`!word.is_ascii() && !text.is_ascii()`); rg finds the name nowhere in the tree. bf654d0 keeps one rule for all words (its engine.rs:1062-1075: `is_subsequence(w, text) || misspelled.contains(&w) && near_miss(w, text, typo_budget(w))`). 362de8f cut the same sentence from the Allow typos row of docs/manual.md (now line 1037). Line 58 of the text says the opposite. This is already correction 10 in changelog-fixes.md and in assemble-changelog.py; unreleased.md still has the sentence.
- The skeptic's correction: REMOVE (the whole sentence)

## 2. picture-kept-until-restart: confirmed

- Entry: "A preview no longer goes on showing a" (Fixed)
- Kind: untrue-at-tip, severity 3, the reader's confidence high
- Sentence: A file written over in place, which changes nothing about its folder, still shows as it was until you refresh, and a picture still shows as it was first drawn until fffinder is restarted.
- Problem: The second half was true when 52e7de4 landed and 1acaff3 made it untrue: a picture is now loaded by its path and date, so one that was saved again shows as it is once its folder is read again, as Refresh does. The entry at line 53 of the text says exactly that ("where it kept its old picture until fffinder was restarted"), so the two contradict each other. The first half still holds.
- The reader's evidence: src/finder/preview.rs:977 (`img(PreviewImages::source(&finder.preview_images, entry))`); src/finder/preview_images.rs:25 and :49-53 (the key is path and date), :66-73 (a file that has changed takes the place of the file as it was); docs/manual.md:412 ("An image that has changed on disk shows as it is now once its folder is read again, as Refresh does"), which 1acaff3 put in place of the old sentence. The record /home/user/polish-tools/done/stale-previews-and-cancel.json says in its notes that this clause "goes the same way when the changelog is assembled"; it is not among the eight in changelog-fixes.md.
- The reader's correction: A file written over in place, which changes nothing about its folder, still shows as it was until you refresh.
- The skeptic's evidence: Second half untrue at the tip. src/finder/preview.rs:977 `img(PreviewImages::source(&finder.preview_images, entry))`; src/finder/preview_images.rs:25 `type Key = (PathBuf, Option<SystemTime>);`, :49 `(entry.path.clone(), entry.modified)`, :68-75 a key not held starts a new `ImageCacheItem::new(&Resource::Path(..))`, which loads the file afresh. tests/controls.rs:2745-2773 (an_image_that_changed_on_disk_is_shown_as_it_is_now): after a save with a later date and Refresh, `assert_eq!(pixels(&after), (16, 4), "the photo as it is now")`. Thumbnails too go by date (src/thumbnails.rs:82). Line 53 of the text says the opposite ("where it kept its old picture until fffinder was restarted"). First half holds: src/finder/mod.rs:1699-1703 "only a folder whose time moved is read again", crates/fffinder-core/src/listing.rs:18-20 (the stamp is the folder's own modification time), docs/manual.md:141. Already correction 9 in changelog-fixes.md; unreleased.md still has the clause.
- The skeptic's correction: A file written over in place, which changes nothing about its folder, still shows as it was until you refresh.

## 3. picture-blank-while-read-again: confirmed

- Entry: "A preview no longer goes on showing a" (Fixed)
- Kind: untrue-at-tip, severity 1, the reader's confidence medium
- Sentence: The old preview stays until the new one is ready, and text stays where you were reading it.
- Problem: Not so for a picture since 1acaff3. Before it a picture was never read again, so the sentence had nothing to be wrong about. Now a changed picture is read again and nothing shows in its place meanwhile: the old one is let go at once and the image has no element for the wait.
- The reader's evidence: src/finder/preview.rs:971-991 (the image branch has only `.with_fallback`, no loading element); src/finder/preview_images.rs:66-73 (the file as it was is let go before the new one has loaded); docs/manual.md:428 ("An image differs in two ways: nothing shows in its place while it's read again, and it's read again only when its date has changed, after Refresh too"). Commit 1acaff3.
- The reader's correction: The old preview stays until the new one is ready, except a picture's, whose place is empty meanwhile, and text stays where you were reading it.
- The skeptic's evidence: For an image the old preview does not stay. src/finder/preview.rs:971-990: the image branch is `img(PreviewImages::source(..)).size_full().object_fit(ObjectFit::Contain).with_fallback(..)`, with no element for the wait. src/finder/preview_images.rs:68-75: for a key it doesn't hold, `load` first lets go of what it holds of the same file ("What's held of the same file is the file as it was before it changed, which won't be shown again") and returns None until the new one has loaded. tests/controls.rs:2768 `assert_eq!(held.len(), 1, "the photo as it was isn't kept beside it")`. docs/manual.md:428 "An image differs in two ways: nothing shows in its place while it's read again". The finder's sentence is still inexact, so the correction is mine: a preview drawn from a thumbnail (a video, an office document, HEIC; manual lines 412 and 414) goes empty the same way. src/thumbnails.rs:82 keys it by `(entry.path.clone(), entry.modified, px)` and line 91 returns None while it is made; `large_thumbnail` (preview.rs:1453-1457) draws it with an `img` that has no ID, and GPUI shows the `with_loading` element only for an image with element state (gpui-pre-0.3.10 src/elements/img.rs, the `None =>` arm: `if let Some(state) = &mut state`). That half is by reading (tests draw no thumbnails) and was so before the branch (66c919b:src/thumbnails.rs:77).
- The skeptic's correction: The old preview stays until the new one is ready, except an image or a thumbnail, whose place is empty meanwhile, and text stays where you were reading it.

## 4. accent-near-miss-clause-never-shipped: confirmed

- Entry: "A name with accented letters is found however" (Fixed)
- Kind: never-shipped, severity 2, the reader's confidence medium
- Sentence: A misspelled word with an accent or in another alphabet now lists only near misses, as other words do, and what a search opened is remembered for such names too.
- Problem: The first clause mends something no release had. The looser rule for words with an accent came with the near-miss rule itself (5624239, the Changed entry at line 24 of the text) and went again in 362de8f, both in this section. Once the sentence at line 24 is removed (see accent-typo-exception-gone), that entry already says it for every word. The clause is also no longer exact since bf654d0, the commit that was pending: a misspelled word lists its near misses beside the results that have its letters in order, not only near misses. The second clause is a real fix and stays.
- The reader's evidence: At 362de8f crates/fffinder-core/src/engine.rs:1061-1072 has one rule for all words; `may_be_spelled_apart` was added by 5624239 and removed by 362de8f. At bf654d0 engine.rs:1066-1075 keeps a hit when each word has its letters in order `|| misspelled.contains(&w) && near_miss(..)`, and bf654d0's own paragraph says "beside the results that have its letters in order". The other clause: engine.rs:1936-1950 (`track_open` remembers a search under each spelling).
- The reader's correction: What a search opened is remembered for such names too.
- The skeptic's evidence: At 362de8f the first clause holds (engine.rs:1061-1072). bf654d0 proves it wrong: its engine.rs:1062-1075 keeps a hit when each word has its letters in order `|| misspelled.contains(&w) && near_miss(..)`, and its test at 5420-5421 has `let meant = ["budget.xlsx", "target.txt"]; assert_eq!(names("bduget"), meant, ..)`: a misspelled word lists `build/debug/target.txt`, which is no near miss, beside the near miss. So no word "lists only near misses", and "as other words do" is not what other words do; bf654d0's own entry says "beside the results that have its letters in order". The finder's first reason is half right: v0.5.1 did list every typo match for such a word (its engine.rs:558-560, `if !words.is_empty() && raw.iter().any(&strict) { raw.retain(strict); }`, for any word), so the loose listing shipped; only the difference from other words never did (`may_be_spelled_apart`, added by 5624239, removed by 362de8f). Without "only" the clause says nothing that line 24 doesn't say for every word once correction 10 is applied, so it goes. The second clause stands: engine.rs:1936-1950 (`track_open` remembers a search under each spelling) and the test at engine.rs:5730. The finder's sentence leaves "such names" without the words it pointed back to, right after "`resume.pdf`", so I name them.
- The skeptic's correction: What a search opened is remembered for names with accented letters too.

## 5. accent-fault-in-unreleased-switch-and-quotes: confirmed

- Entry: "A name with accented letters is found however" (Fixed)
- Kind: never-shipped, severity 1, the reader's confidence medium
- Sentence: `señor` could miss `señor.txt`, or list it only among loose typo matches without a highlight, and never with **Allow typos** off, in a quoted word, or for an app.
- Problem: The **Allow typos** switch and quoted words are added by this same section (its first two Added entries, upstream's 66c919b), so no release ever missed `señor.txt` "with Allow typos off" or "in a quoted word". This is the case corrections 3 and 5 of changelog-fixes.md settled ("Allow typos never shipped"; "which it didn't"). The rest of the sentence is a fault the last release had. That both now work is still said two sentences on ("in words, quoted words and folder filters").
- The reader's evidence: Lines 5-6 of the text add quoted words and the switch; `git log -1 66c919b` is "add quoted exact words, case-folded globs, and an Allow typos switch", and CHANGELOG.md's [0.5.1] section has neither. Apps were matched by name before the branch (crates/fffinder-core/src/engine.rs:1033-1045, `app_hits`), so "for an app" stays.
- The reader's correction: `señor` could miss `señor.txt`, or list it only among loose typo matches without a highlight, and never for an app.
- The skeptic's evidence: No release had the switch or quoted words. `git show v0.5.1:crates/fffinder-core/src/config.rs` has no `typos` setting; v0.5.1's engine.rs has no quoted-word code (no `split_quoted`, no `contains_phrase`) and its fallback has no switch (558-560: `if !words.is_empty() && raw.iter().any(&strict) { raw.retain(strict); }`). `git describe 66c919b` is v0.5.1-3-g66c919b, "add quoted exact words, case-folded globs, and an Allow typos switch", and lines 5-6 of the text add both, the switch "on by default, as before". So the Fixed entry names faults of two things the Added entries call new, the case corrections 3 and 5 of changelog-fixes.md settled. The rest is a release's fault: v0.5.1 listed every typo match when nothing had the letters in order, with no highlight for them (engine.rs:1567 "Typo-tolerant matches may not be subsequences; show no partial noise"), and matched apps letter by letter (engine.rs:1419-1427, `is_subsequence(w, &name)`), so "for an app" stays. At the tip apps compare in one spelling (engine.rs:2454-2456).
- The skeptic's correction: `señor` could miss `señor.txt`, or list it only among loose typo matches without a highlight, and never for an app.

## 6. zero-items-under-headers-never-shipped: confirmed

- Entry: "New Folder left the keyboard in a name" (Fixed)
- Kind: never-shipped, severity 1, the reader's confidence high
- Sentence: The status bar now counts the items of collapsed groups too: it read "0 items" under headers that counted more, and "N of M items pass the filter" left out the items that pass in collapsed groups.
- Problem: "0 items" under headers was only ever seen on this branch, between f5562db and e2e1672. In the last release a view whose groups were all collapsed drew no headers at all: the message "No Items" took the view's place, which is what the entry at line 48 of the text says it fixed. What a release did show was a count that fell short with some groups collapsed.
- The reader's evidence: `git show 66c919b:src/finder/mod.rs` line 3236 (`} else if self.rows.is_empty() && !self.loading && !self.searching {` chooses the message, headers and all) and `66c919b:src/finder/bars.rs` line 327 (`let count = self.rows.len();`). f5562db's message: "Left as they were: the status bar counts the items that show, so it reads "0 items" under headers whose counts add up to more, as it already counted short with one group collapsed". e2e1672's message has the example a release had: "7 items" under headers that counted sixteen. Tip: src/finder/bars.rs:353 (`let count = self.listed_len;`).
- The reader's correction: The status bar now counts the items of collapsed groups too: it read "7 items" under headers that counted sixteen, and "N of M items pass the filter" left out the items that pass in collapsed groups.
- The skeptic's evidence: In v0.5.1 and 66c919b alike, src/finder/mod.rs:3236 `} else if self.rows.is_empty() && !self.loading && !self.searching {` puts the message (3262: "No Items", "This folder is empty.") in the view's place, and 3267-3268 `} else if self.rows.is_empty() { self.render_skeleton(view, cx)` covers the rest, so no header was ever drawn over zero rows. src/finder/bars.rs:327 `let count = self.rows.len();` and groups.rs:307 (the header shows `group.total`): a release read "0 items" only under "No Items", which line 48 of the text reports as fixed, and under headers it read short. f5562db's message: "it reads \"0 items\" under headers whose counts add up to more, as it already counted short with one group collapsed"; e2e1672's: "\"7 items\" under headers that counted sixteen, \"0 items\" with every group collapsed". Tip: bars.rs:353 `let count = self.listed_len;`. The finder's example is one a release shows (seven rows showing, headers adding up to sixteen), and the filter clause is a release's fault too (bars.rs:343-347 at 66c919b uses the same `count`).
- The skeptic's correction: The status bar now counts the items of collapsed groups too: it read "7 items" under headers that counted sixteen, and "N of M items pass the filter" left out the items that pass in collapsed groups.

## 7. quick-look-waiting-return-never-shipped: confirmed

- Entry: "With Quick Look open, Return in the search" (Fixed)
- Kind: never-shipped, severity 1, the reader's confidence high
- Sentence: Pressed before the results were in, Return did nothing.
- Problem: A Return that waits for the results is new in this section (17a460e, the entry at line 28 of the text). In the last release Return never waited: it ran the search and at once selected the first row the window showed, which with Quick Open's window drawn last is the fault the sentence before this one describes. "Did nothing" was the waiting Return failing to move in when the results arrived, which only the branch had between 17a460e and e746db6.
- The reader's evidence: `git show 66c919b:src/finder/mod.rs` lines 1308-1318 (`InputEvent::PressEnter` runs the search, then `focus_files` and `select_only(0)` when there are rows; nothing is kept for later). The wait is `enter_results`, added by 17a460e (src/finder/mod.rs:2103-2142 at the tip). e746db6's own message: "A Return that came before the results were in, and waits for them, did nothing when they arrived."
- The reader's correction: REMOVE
- The skeptic's evidence: `enter_results` is in src/finder/mod.rs 0 times at v0.5.1 and at 66c919b, and 10 times at 17a460e. 66c919b:src/finder/mod.rs:1308-1318: `InputEvent::PressEnter { .. } => { self.run_search(false, cx); if !self.rows.is_empty() { self.focus_files(window, cx); if self.selection.is_empty() { self.selection.select_only(0);`: a release's Return never waited. It selected the first row the window still showed, which is the fault of this entry's first sentence and of line 28 ("acted on what the window still showed"). "Did nothing" is the waiting Return of 17a460e: tip mod.rs:2058 `self.enter_results = Some((window.window_handle(), selection));`, and 2137-2141 `finish_enter` moves in only when the field `is_focused(window)` in that window, which it never was in Quick Look's. e746db6's message: "A Return that came before the results were in, and waits for them, did nothing when they arrived." The entry reads whole without the sentence.
- The skeptic's correction: REMOVE (the whole sentence)

## 8. renamed-item-stays-selected: refuted

- Entry: "The column view lost track of what was" (Fixed)
- Kind: untrue-at-tip, severity 2, the reader's confidence high
- Sentence: A renamed item stays selected, and in a further column shows its new name at once.
- Problem: Since 4efa254 a renamed item is selected again only when nothing else has been selected meanwhile. A rename ended by a click or a right-click on another item leaves that item selected, in the first column and in a further one, which is what the entry at line 57 of the text says ("The item pressed now stays selected"). When 7e8df53 landed, the renamed item was selected again in that case too, so the sentence was true then.
- The reader's evidence: src/finder/mod.rs:1677-1682 (`let another = ...; if !another && let Some(..)`: the reading after a rename selects the item only when no other selected item is still listed); src/finder/mod.rs:4343-4349 (`wanted.filter(|_| kept.is_none() && this.active_column >= col)` for a column after the first); tests/basics.rs:2118 (a_press_on_another_row_that_ends_a_rename_leaves_that_row_selected_in_its_column); docs/manual.md:513. Commit 4efa254.
- The reader's correction: A renamed item stays selected unless a click on another item ended the edit, and in a further column shows its new name at once.
- The skeptic's evidence: The app does what the sentence says of a rename. src/finder/mod.rs:1682-1688 `if !another && let Some((&first, rest)) = found.split_first() { this.selection.select_only(first); .. this.select_row_in_columns(first, cx); }`; mod.rs:4268-4281 (`renamed_in_columns`: "it's read again at once, and the item stays selected there if it was, unless something else has been selected by then"). tests/basics.rs:2070 `assert_eq!(ui.selected(), ["columns.txt"], "still the selected item");` (first column), 2092-2093 `assert_eq!(columns(&ui)[1], ["a.txt", "atlas", "c.txt", "zz.txt"]); assert_eq!(ui.selected(), ["zz.txt"], "where it sorts now");` (a further column), 2174 after Select All. The only time the item isn't selected afterwards is when the user has selected another by then, with a press or an arrow key. That is the subject of line 57, which calls the old behaviour (the renamed item selected again) the fault, so the two entries don't say opposite things of one case. The proposed sentence is itself untrue: tests/basics.rs:2205 and 2215, a click on another item, one inside the renamed folder, ends the edit and the renamed folder stays selected (`["Work"]`, `["maps"]`). If a qualifier is wanted all the same, the code's own is "unless something else has been selected by then".

## 9. pressed-item-inside-renamed-folder: confirmed

- Entry: "Ending a rename with a click or a" (Fixed)
- Kind: unclear, severity 2, the reader's confidence medium
- Sentence: The item pressed now stays selected, an arrow key pressed right after Return is no longer undone, and renaming a folder in the sidebar's tree no longer takes the selection from the items selected among the files.
- Problem: The first clause has an exception that the entry's own story makes matter. When the item pressed is inside the folder whose name is being edited, in a folder open in place in the list or in the column to the folder's right, the folder is still the one selected afterwards, and Cmd+Backspace then moves the whole folder to the Trash. A reader takes that case as fixed, because the sentence before says the same press on a result inside a renamed folder was mended.
- The reader's evidence: tests/basics.rs:2184-2220 (a_press_inside_a_folder_that_ends_its_rename_leaves_the_folder_selected: after the click on Projects/b.txt `ui.selected()` is ["Work"], and in the columns ["maps"]); src/finder/mod.rs:1671-1677 (the comment: an item inside a folder that was open in the list isn't kept). /home/user/polish-tools/done/fix-rename-press-selection.json, left_out, first item, and /home/user/polish-tools/final-pass-notes.md line 83 record it as left for later.
- The reader's correction: The item pressed now stays selected, unless it is inside the folder being renamed, in a folder open in the list or in the next column, where the folder is selected as before, an arrow key pressed right after Return is no longer undone, and renaming a folder in the sidebar's tree no longer takes the selection from the items selected among the files.
- The skeptic's evidence: tests/basics.rs:2184-2220 (a_press_inside_a_folder_that_ends_its_rename_leaves_the_folder_selected) pins the opposite of the first clause: with the folder open in the list, F2, "Work", a click on `Projects/b.txt`, then `assert_eq!(ui.selected(), ["Work"]);` (2205); in the columns a click on `Work/atlas/Project Atlas.txt` ends the rename of `atlas` and `assert_eq!(ui.selected(), ["maps"]);` (2215). src/finder/mod.rs:1671-1675: "An item inside a folder that was open in the list isn't [kept]: its row was under the folder's old name, and the folder is closed under its new one." 4efa254's message: "a press on an item inside a folder whose name is being edited, in the column to its right, still ends with the folder selected". The done record's left_out adds that Cmd+Backspace then moves the whole folder to the Trash. The sentence before this one names the same press among a search's results as mended, and there it is (tests/finder.rs:1066 `assert_eq!(ui.selected(), ["Atlas Plan.txt"]);`), so the limit has to keep to the list and the columns. The finder's wording puts the exception inside a list of three, where it can't be told where it ends, and says "as before", which holds for the list and wasn't traced for a release's column view. Mine leaves the sentence as it is and adds one after it.
- The skeptic's correction: The item pressed now stays selected, an arrow key pressed right after Return is no longer undone, and renaming a folder in the sidebar's tree no longer takes the selection from the items selected among the files. A press on an item inside the folder being renamed, in the list with that folder open in place or in the column to its right, leaves the folder selected, not the item.

## 10. recents-first-file-written-to: confirmed

- Entry: "Search results and Recents follow files that other" (Changed)
- Kind: unclear, severity 2, the reader's confidence medium
- Sentence: Recents is read again when a file it would show is added, removed, renamed, or written to.
- Problem: Not for the file Recents has first already, which is the common case of saving the file just worked on. When the only news is that file being written to, Recents is not read, and its row keeps the size and date it showed. No later commit caused this: it has been so since 153eb8f, and 5e68c44 left it ("Recents is not read for its newest file being written to again"). A quick save reaches the index as one write, so someone trying it sees nothing change.
- The reader's evidence: src/finder/mod.rs:1785-1797 (`let written = self.engine.only_written_since(seen); ... if news || written.as_deref() != first { self.recents_behind.fell_behind(None); }`); crates/fffinder-core/src/engine.rs:725-736; tests/finder.rs:1547-1554 ("Written to again it stays first, so Recents isn't read again"); docs/manual.md:141 ("it goes first, and stays there however often it's written to, with the size and date it showed until Recents is read again").
- The reader's correction: Recents is read again when a file it would show is added, removed, renamed, or written to, except for the file it has first already, which keeps the size and date it showed.
- The skeptic's evidence: src/finder/mod.rs:1786-1797: "So one that Recents has first already changes nothing but its own size and date, however often it's written to, and those wait for the next read"; `let written = self.engine.only_written_since(seen); let first = self.listing.first()..; news = written.is_none(); if news || written.as_deref() != first { self.recents_behind.fell_behind(None); }`. With writes to the first file as the only news, nothing falls behind and `reload_recents` isn't called. tests/finder.rs:1547-1554 "Written to again it stays first, so Recents isn't read again": `assert!(!lists(&ui, "unseen.txt"))` after three looks. docs/manual.md:141 "stays there however often it's written to, with the size and date it showed until Recents is read again". It was so at 153eb8f, but it shows only since 5e68c44, which has a read bring a row its new size and date (mod.rs:1818-1831); 5e68c44's message: "Recents is not read for its newest file being written to again". The finder's sentence can be read as excepting the first file from all four verbs, which is untrue: saved under another name that takes its own, it is reported as renamed and Recents is read (tests/finder.rs:1612-1646, `assert_eq!(size(&ui), "20 bytes")`). So the wording is mine.
- The skeptic's correction: Recents is read again when a file it would show is added, removed, renamed, or written to, though not for writes to the file it has first already, which keeps the size and date it showed until Recents is next read.

## 11. pending-paragraph-in-patch-is-stale: confirmed

- Entry: "With **Allow typos** on, a misspelled word now" (Fixed; the entry still to be added, as the patch file words it)
- Kind: untrue-at-tip, severity 2, the reader's confidence high
- Sentence: A name with a near miss of a word of six letters or more ranks higher, which as a rule puts the name that was meant first, and a near miss that is only in a folder's name ranks lower, so `mian` lists `main.rs` ahead of everything in a folder named `Many`.
- Problem: The paragraph in <scratch>/pending-near-miss-beside-scattered.patch disagrees with the patch's own code: a name is ranked up for a word of five letters or more, and only when a word of the name begins with the near miss (`mybudget.xlsx` gets nothing for `bduget`). The commit has since landed on search-ui-polish as bf654d0 with the same code (one test assertion more) and this sentence rewritten, and with ", or ahead of them" added to the sentence about `pckg`. Take the paragraph from bf654d0's message, not from the patch file. The rest of bf654d0's paragraph agrees with its code.
- The reader's evidence: Patch file line 338 (`const NEAR_NAME_LETTERS: usize = 5;`) against its line 115 ("six letters or more"). `git show bf654d0:crates/fffinder-core/src/engine.rs` lines 2930 (the constant), 2969 (`if long && begins_word() { NEAR_NAME } else { 0 }`), 4305 (`adds("budget.txt", "budgt") == NEAR_NAME`, "five letters") and 4310 (`adds("mybudget.xlsx", "bduget") == 0`); bf654d0's docs/manual.md:382 ("begins with a near miss of a word of five letters or more").
- The reader's correction: A name ranks higher when one of its words begins with a near miss of a word of five letters or more, which as a rule puts the name that was meant first, and a near miss that is only in a folder's name ranks lower, so `mian` lists `main.rs` ahead of everything in a folder named `Many`.
- The skeptic's evidence: The patch file's message disagrees with its own code: line 115 "A name with a near miss of a word of six letters or more ranks higher" against line 338 `+const NEAR_NAME_LETTERS: usize = 5;` and line 377 `+        if long && begins_word() { NEAR_NAME } else { 0 }`. The commit is on search-ui-polish as bf654d0. Its diff of config.rs, engine.rs and manual.md is the patch file's plus one test assertion (`adds("Live/Bjo\u{308}rkGudmunds.mp3", "bjökr")`), and its Changelog paragraph differs from the patch file's in two sentences only: this one, and ", or ahead of them" at the end of the sentence about `pckg`. bf654d0's engine.rs:2930 (the constant), 2969, 4305 `assert_eq!(adds("budget.txt", "budgt"), NEAR_NAME, "five letters");`, 4310 `assert_eq!(adds("mybudget.xlsx", "bduget"), 0);`, 4312 `assert_eq!(adds("Many/a.bin", "mian"), -NEAR_FOLDER);`, and its test at 5434-5437 (`mian`: plan.txt, main.rs, then the two files in Many) bear out the landed sentence, which is the finder's word for word. assemble-changelog.py reads `git log 66c919b..search-ui-polish` in /home/user/fffinder, where the branch is at bf654d0, so it takes the landed paragraph.
- The skeptic's correction: A name ranks higher when one of its words begins with a near miss of a word of five letters or more, which as a rule puts the name that was meant first, and a near miss that is only in a folder's name ranks lower, so `mian` lists `main.rs` ahead of everything in a folder named `Many`.

## What the reader read

Nothing was built or run; all of it is reading at 362de8f, plus bf654d0 (the pending commit, which landed while I read; its code is the patch's with one more test assertion).

The ten commits against the whole text:
- e2e1672: one finding ("0 items" under headers). Nothing else in the text describes the status bar's count, New Folder under a filter, or the group opening.
- ac62eaf: none. The older tab sentences (tab bar scrolling, tooltips, the double-click on a tab's close button) still hold.
- 1acaff3: two findings, both in the preview entry at line 49.
- ddefe96: none. The placeholder sentences at lines 30 and 45 and the Cmd+Return entry still hold (src/finder/mod.rs:1195-1203).
- ffb02de: none. No sentence describes Relevance chosen again, the double-click on a button under the message, or Go to Folder from a Smart Folder's filter field.
- 5e68c44: none made untrue. "Results the file made no difference to are left as they are" still holds for which rows are listed and where; only their size and date are brought up to date.
- a085120: none. The --here entry and the special-folder-names entry are unchanged by it.
- 4efa254: one finding (line 51).
- 362de8f: one finding (line 24). The folder-filter, capital-letter and quoted-word entries still hold.
- bf654d0: one finding on the clause at line 58, one on the stale paragraph in the patch file. The first sentence of the entry at line 24 and the Allow typos entry at line 6 stay true, since both speak of the case where nothing matches without typos.

Entries, by text line:
- 5 quoted words: example has a test (engine.rs:4780-4823). True.
- 6 Allow typos: true.
- 7 --here: src/args.rs, src/lib.rs:46-57, src/server/mod.rs:636-648, nu/fff.nu:127-421, upstream example at 66c919b nu/fff.nu:148. True.
- 8 history menu: toolbar.rs:307-394. True.
- 9 Search Syntax: dialogs.rs:1016-1155, Cmd+/ at actions.rs:194. True.
- 10 matching line: preview.rs:43, 49, 1190. True.
- 11 Cmd+Return: actions.rs:215, mod.rs:1187-1203. True.
- 12 nu: dialogs.rs:636-670, 1160-1222; labels as coded. True.
- 13 tabs: tab_bar.rs, workspace.rs, views.rs:134-136; the middle click on a folder is upstream's. True.
- 17 globs: label only.
- 18 tooltips: tooltips and key table (actions.rs:125-292), menu names (actions.rs:384-483). True.
- 19 Escape: toolbar.rs:672-683. True as far as read.
- 20 Back and Forward: mod.rs:2252-2289, toolbar.rs:309-337. True.
- 21 Smart Folders: bars.rs:129-142, labels and keys. True.
- 22 live results: one finding.
- 23 tie-break: engine.rs:2826-2929. True.
- 24 near misses: one finding; thresholds and `checked_words` as stated (engine.rs:2580-2599).
- 28 Return waits: true against upstream's 66c919b mod.rs:1308-1318.
- 29 capitals: engine.rs:950-964, 1033-1045. True.
- 30 placeholders: mod.rs:5111-5128. True as far as read.
- 33 scope-relative: engine.rs:1305-1313, examples check. True.
- 34 results at the top: mod.rs:2144-2225. True.
- 36 content search count: bars.rs:356-383, mod.rs:5131-5148. True.
- 37 folder filters: engine.rs:1150-1157. True.
- 38 Kind: kind.rs:120-170, 208-218. True.
- 39 sorting: engine.rs:921-937, 1082-1097. True.
- 40 Quick Look: one finding.
- 41 macOS keys: actions.rs:132-151, 276-290. True by reading.
- 42 special names: engine.rs:1128-1139 and the record. True as far as read.
- 43 dimmed buttons: toolbar.rs:633-655; the button is upstream's. True.
- 45 messages: mod.rs:4964-5192, bars.rs:181-190. True.
- 48 collapsed groups: groups.rs `shows_nothing`, mod.rs:5096-5109. True.
- 49 previews: two findings.
- 50 double-click: views.rs:122-176. True.
- 51 columns: one finding.
- 52 New Folder: filter.rs:151-162, bars.rs:353. One finding.
- 53 photos: thumbnails.rs:20-24, 156-157, 217; preview_images.rs:21. True.
- 54 place left: mod.rs:1389-1410, 4421. True.
- 55 filter: filter.rs:128-163; `kind == Folder` is valid (nushell.rs:77). True.
- 56 folder spelling: engine.rs:3534-3580, lib.rs:56, listing.rs:211-225, config.rs:370. True.
- 57 rename press: mod.rs:1573-1690, 4303-4349. One finding.
- 58 accents: engine.rs:965-1016, 1936-1950, 2646-2784. Two findings.

Every bold label, quoted message, menu path and key in the text was looked up in src/ and docs/manual.md; none is named otherwise.

## What the reader did not get to

- Entries at lines 31 (content search stops), 32 (preview scroll per file), 35 (scope bar keyboard), 44 (table preview scrolls), 46 (keyboard no longer goes dead) and 47 (tab bar scrolls): I checked their labels and keys, that tests named for their claims exist, and that none of the ten commits touches what they say. I did not trace each claim through the code again.
- Claims about what the last release did, where I had only the commit message or the done record to go by and did not read upstream's code: the "before" halves of lines 12, 21, 36, 37, 42, 45, 46, 50 and 53.
- Anything that needs macOS or Windows: what the macOS menu bar shows (lines 18, 41), letter-case and symbolic-link behaviour of a searched folder (line 56), names as Finder stores them (line 58). Read only.
- Figures that were measured, not derived: "about 100 of 500" (line 36), "about 48 MB" (line 53), "some nine hundred Rust crates" and `benchmrak` (the entry from bf654d0), "a few thousand" (line 23).
- The examples at lines 17, 37 and 42 (`*taxex*`, `/notes/ todo`, `drafts[old]` and the like) were checked against the code's rules and tests' fixtures by reading, not by searching a folder.
- Known open faults that touch entries without making a sentence false were left out of the findings: a search of one large folder for a word in the folder's own name can lose matches (/home/user/polish-tools/later.md, fix-folder-search-own-name-pool; lines 24 and 42 stay true as worded), and a new file that is only a near miss waits for Refresh ("or could list" at line 22; docs/manual.md:141 says so).

## The skeptic's notes

Ten confirmed, one refuted (renamed-item-stays-selected). Nothing was edited, built or run; my scratch folder is deleted.

What I held the text against: the snapshot at 362de8f, and bf654d0, which is the pending commit as it landed on search-ui-polish (same code as the patch file plus one test assertion). Only accent-near-miss-clause-never-shipped and pending-paragraph-in-patch-is-stale turn on bf654d0.

Already recorded: changelog-fixes.md and assemble-changelog.py now carry corrections 9 and 10, which are picture-kept-until-restart and accent-typo-exception-gone. The copy of unreleased.md I was given (written 08:32) does not have them applied yet.

Where the corrected sentence is mine and not the finder's:
- picture-blank-while-read-again: "or a thumbnail" is added. The image half is proven by a test and the manual; the thumbnail half is by reading only and predates the branch. Dropping it gives the finder's sentence, which then still claims too much for a video or a document shown from its thumbnail.
- accent-near-miss-clause-never-shipped: "such names" becomes "names with accented letters", because its antecedent leaves with the clause.
- pressed-item-inside-renamed-folder: the sentence stays as it is and one sentence is added after it.
- recents-first-file-written-to: the exception is tied to writes only.

On the two "never shipped" findings about typos and accents: v0.5.1 did list every typo match for a word with an accent, as for any word. What no release had is the difference between such words and others, and the switch and quoted words.

Seen while tracing, not judged:
- docs/manual.md:428 says a preview stays until the new one is ready and excepts only an image; a thumbnail-drawn preview goes empty the same way.
- docs/manual.md:513 (the Rename… row) says a click on another item leaves that one selected, without the case of an item inside the folder being renamed.
- In the entry at line 49 of the text, "another size or date" is date only for an image (src/finder/preview_images.rs:25; manual line 428).
