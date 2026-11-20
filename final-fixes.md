# Candidate fixes for the final pass

Faults in what this branch itself added, found after the change landed. Each needs a failing test first (GUIDE).
The whole-branch review's confirmed findings are added here when its workflows return.

1. `message_button_clicked` (empty-state-messages, src/finder/mod.rs), found by the reviewer of
   stray-double-click-opens-file (c0605b6): after a double-click on Search This Computer (or the other buttons under
   the message), every quick click on the results is stopped in the capture phase until a click the system counts as
   first, so a double-click on the result under the pointer opens nothing until the mouse rests or moves. Their
   pointer: clear the flag at the first left press on the file area, whatever its count; the press after the stopped
   one would then be left alone once more by stray-double-click's rule (`Presses`), unless the guard records its press
   as left alone too. Consider whether `Presses` now makes the guard unnecessary for the result under the button
   (their record says it covers that case but not "the other button").

## From the whole-branch review, previews-work and docs-tests lenses (04:57 UTC; all confirmed by the skeptic)

Full text: review/previews-work_docs-tests.md (and .json).

2. filter-answer-kept-while-paths-are-the-same (sev 3). A Nushell filter runs again only when the ordered list of
   paths changes: a file saved with another size, folder sizes that arrive later, Refresh, and Return with the same
   text all leave its old answer in force ("No Items Pass the Filter. 1 item is hidden" stays after the file has
   grown past the limit). Fix: keep (path, size, modified) in `filter_source` and compare that in `apply_filter_to`
   (src/finder/filter.rs:130-138); set `filter_source = None` in `refresh` (src/finder/mod.rs). Test needs nu.
3. listed-file-saved-keeps-old-row-and-preview (sev 2). Recents and a quiet name-search rerun drop an answer whose
   paths are the same in the same order, with its new sizes and dates, so a saved file keeps its old row and preview
   there although the manual says the preview is read again "when a search runs again, or when Recents is read
   again". Fix: in `reload_recents` take the answer when some entry's (modified, size) differs; in the quiet
   search's answer, when not fresh but the paths are equal, take the rows and call `selection_changed`.
4. selection-moved-without-asking-for-its-preview (sev 2, older). Left to the parent folder in the list
   (`tree_key`) and Select All change the shown item without `selection_changed`, so the Preview Pane or Quick Look
   shows a spinner. Two lines. Optional: only if a lane is idle.
5. Wording batch (ours to correct):
   - Search Syntax dialog's first row, docs/manual.md:358 and README.md:108 still say every word must match;
     since typo-fallback-near-miss a lone letter or digit beside a longer word is not looked for. Dialog: "Fuzzy
     words; every word of two letters or more must match (typos allowed)" / "(typos off in Settings)"; manual: "Fuzzy
     words. Every word must match, but for a single letter or digit beside a longer word. Typos are allowed unless
     **Allow typos** is off."; README: "Fuzzy words; near misses with a typo when nothing else matches. Every word
     of two letters or more must match." Two expectations in tests/controls.rs follow.
   - Search Syntax footer (src/finder/dialogs.rs:1129) says "Return searches now and Down Arrow moves into the
     results": both keys do the same now (search-enter-before-results). Say what docs/manual.md:333 says.
   - Save Smart Folder says "Kept up to date." for a contents search, the one search that isn't (dialogs.rs:937).
     Append it only when the search is by name; otherwise "Searched again when you open it or refresh." Test:
     the `smart-summary` label for a contents search.
   - docs/manual.md:337 and the F5 rows (manual.md:523 and :916 say "F5 outside split view"; on a Mac F5 refreshes
     in split view too since menu-bar-mac-keys): see changelog-fixes.md #4 and the review file.
6. Left for later, in the report: Quick Look's window holds its browser strongly, so closing the tab keeps that
   browser and its background work alive until Quick Look closes (quick-look-keeps-its-closed-browser-alive, sev 2);
   two "Indexing…" tests assert only when a scan thread loses a race; nu tests check less without nu and print no
   skip line; one watcher test sleeps on the real clock; six `_for_test` entry points are compiled into the app
   (house pattern, ten upstream).

## From the search-flow, focus-keys, engine and platforms-cli lenses (05:00 UTC; all confirmed)

Full text: review/search-flow_focus-keys.md and review/engine_platforms-cli.md (and .json).
Turned into fix briefs, queued in every lane (one more turn each): fix-rename-press-selection (sev 4),
fix-filter-reruns-and-fresh-rows (#2 and #3 above), fix-own-small-faults (#1 above, the Go to Folder dead keyboard,
Relevance chosen again), fix-here-folder-spelling (--here with `..`, a link, or another letter case),
fix-wording-after-later-changes (#5 above), fix-folder-search-own-name-pool (engine; after the two engine items).

Left for later, for the report and later.md:
- sort-fallback-flag-outlives-its-sort (sev 2, ours): after a date sort falls back at its read bound, a later Size,
  Name or Kind sort only reorders the best-ranked rows, until the next keystroke or Refresh. Needs more than 5,000
  matches and Results shown at 5,000 or a filter on.
- changes-for-counts-searched-folders-own-path (sev 2, ours): for a search of one folder `changes_for` looks for the
  words from the search location down, so files the search cannot list cost quiet reruns. Self-healing.
- evicted-temporary-index-leaves-results-unfollowed (sev 1): results of a folder search outside the search
  locations stop following changes once three other such folders have been searched.
- scope-chip-off-where-no-folder (sev 1): in Recents, the Trash or sent results the This Computer chip shows as off
  while Search Only in is on, and clicking it restarts the same search from the top.
- sorted-cut-ignores-folders-on-top (sev 1): by the plan's choice a sort chooses before folders are put on top; the
  wording fix adds the sentence to the manual.
- #4 above (selection moved without asking for its preview) and #6 above.

## Outcome (20 Nov, when the branch was finished)

Four of the six fix-ups were built, each by an implementer and then a reviewer, like every other change:

| Fix-up | Commit | Changelog |
| --- | --- | --- |
| fix-own-small-faults (#1 above, the dead keys after Go to Folder, Relevance chosen again) | ffb02de | none: the faults were in changes no release has had |
| fix-filter-reruns-and-fresh-rows (#2 and #3) | 5e68c44 | Fixed, for the filter half |
| fix-here-folder-spelling | a085120 | Fixed |
| fix-rename-press-selection | 4efa254 | Fixed |

Two were set aside before they were started, at the owner's request, and are the first entries of later.md:
fix-wording-after-later-changes (#5 above: a few sentences in the Search Syntax dialog, the manual and the README
that later changes made inexact) and fix-folder-search-own-name-pool (a search of one large folder for a word that
is in the folder's own name can miss matches).

Everything under "Left for later" above is still open, and so are the finds each reviewer listed under "Found, not
fixed" (final-pass-notes.md has them by change).
