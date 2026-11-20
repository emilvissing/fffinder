# State after the cut-off (00:30 UTC, 20 Nov)

The user chose: build all nine items rated 4 of 5, plus anything already started (tab-drop-and-close), then the final pass; save the rest for later on a
separate branch. Two usage-limit resets are left to them.

## Lanes (single-lane queue workflows, script fffinder-polish-queue-wf_a453dfef-918.js)

| lane | task | on (00:45) |
| --- | --- | --- |
| b | wyj7ehftn | stray-double-click-opens-file (in review), then next.sh |
| c | wll6wrw95 | new-item-in-hidden-row |
| d | w9m9jkino (relaunched 00:31, max_turns 3; run wf_5fa16ccf-4a6) | near-miss-beside-scattered |
| e | wpyd16dif | thumbnail-memory |
| f | wki1hybum | column-pane-selection-by-path (in review), then next.sh |
| g | wvmcoud34 | stale-previews-and-cancel (in review), then next.sh |

Unclaimed within the cut-off: accented-names-search, stale-rows-on-place-change, split-active-pane.
queue/all.txt and queue/lane-*.txt hold only these; queue/later.txt holds the 35 saved items;
queue/before-cutoff/ has the old files. Lane d's partial tab-drop-and-close is parked/ (patch) and a git stash.
When next.sh prints NONE a lane's workflow ends: that is expected now. If an item is still unclaimed
and a lane is idle, relaunch that lane with max_turns 2.

## Whole-branch review (started 00:40 on snapshot /home/user/fast/review-snap at f5562db)

Script: /root/.claude/projects/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/workflows/scripts/fffinder-branch-review-wf_4cb04a02-c47.js
- w8ng2chs0 (run wf_4cb04a02-c47): search-flow, focus-keys
- wlxca0s45 (run wf_bec92998-404): engine, platforms-cli
- w6yhzkqml (run wf_4d3336c3-ba4): previews-work, docs-tests
Each returns [{lens, found, verdicts}]. Next: collect confirmed and uncertain findings, prove and fix them in
freed lanes (GUIDE discipline: failing test first), then a light delta review of what landed after f5562db.

## Notes branch

search-ui-polish-notes on the fork (first push d0effb3, 00:50). Worktree:
/tmp/claude-0/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/scratchpad/notes-wt
Update: ./sync-notes.sh <worktree>, git add -A, commit, git push origin search-ui-polish-notes. Push again at the end.

## Final pass, in order

1. Fix what the whole-branch review confirmed. 2. CHANGELOG.md [Unreleased] with mkchangelog.py, entries
reconciled (the docs-tests lens lists the ones later commits made untrue; final-pass-notes.md has two
wording fixes). 3. Linux: clippy with Rust 1.100.0 --all-targets, --features term clippy and tests, nu-check
of nu/fff.nu, assets/actions.nu, scripts/*.nu, full check.sh on the trunk. 4. xcheck. 5. Push. 6. Unfreeze
/home/user/fffinder (chattr -R -i, remove the target guard file), check out search-ui-polish there, remove
lane worktrees and branches. 7. Final notes-branch push, page update, report (task #6).

## 00:37 UTC: the user asked that whatever was already being worked on gets to finish

tab-drop-and-close (the item lane d was stopped on) is back in scope: first unclaimed item in lanes b, f, g, c, e
and in queue/all.txt; its brief's orchestrator_notes point at the parked patch. Ten items within the cut-off.
Do not stop work that has started again without asking.

## 00:58 UTC, 20 Nov: the container restarted at 00:55 and everything was relaunched

Lanes (queue script, max_turns 2): b w6xhcrcvl (accented-names-search), c wxtzvm11y (new-item-in-hidden-row),
d wvfkveefb (near-miss-beside-scattered), e w14bvaz76 (thumbnail-memory, committed 07e4912, needs review),
f wpzm1olpq (stale-rows-on-place-change), g we8en35gz (tab-drop-and-close).
Unclaimed within the cut-off: column-row-menu-wrong-item (value 5, S, new at 00:51), split-active-pane.
Whole-branch review restarted on the snapshot at 7e8df53 (38 commits): wiqagmtlx (search-flow, focus-keys),
wxue4l604 (engine, platforms-cli), wwf86x48n (previews-work, docs-tests). The first attempt (w8ng2chs0, wlxca0s45,
w6yhzkqml) was lost with the restart before any finder returned.
Watcher: bdjfpaeyj. The xcheck cache in /dev/shm is gone: the next xcheck is a cold run of about 11 minutes.
Eleven items within the cut-off now: the nine rated 4, tab-drop-and-close (already started; the user asked that
it finish), and column-row-menu-wrong-item (can trash the wrong file).

## 04:06 UTC, 20 Nov: relaunched after the Fable usage limit

The limit was hit at about 01:08 UTC, ten minutes after the 00:57 relaunch; every agent failed and nothing ran
until the user reset the limit at about 04:03 (their second reset this session; ONE reset is left to them).
The container had restarted once more in between (xcheck cache in /dev/shm is gone).
Lanes (queue script, max_turns 2): b whom6gd9c (accented-names-search, 1 dirty file), c we56ar0pu
(new-item-in-hidden-row, 9 dirty), d wnlcqzxlu (near-miss-beside-scattered, 1 dirty), e wjd479ivb
(thumbnail-memory, committed d01f598, needs review), f wvsuzgfd6 (stale-rows-on-place-change, clean),
g wj560hxhb (tab-drop-and-close, 1 dirty; the parked patch is in parked/).
Unclaimed within the cut-off: column-row-menu-wrong-item, split-active-pane.
Whole-branch review on the snapshot at 7e8df53: we2ar8ix5 (search-flow, focus-keys), w16nb8qdo (engine,
platforms-cli), w2vbvvyx2 (previews-work, docs-tests). Watcher: bjp8ftnh6.
One allowance bought about 15 landings plus partial work, roughly 35 to 40M tokens. What is left (8 items,
the review, its fixes, the final checks) should take about 25M. Be frugal: no new briefs unless a find can lose
or damage the user's files.

## 04:10 UTC: the user asked to park the two queued items as well

column-row-menu-wrong-item and split-active-pane are in queue/later.txt, out of every lane file and of all.txt.
Within the cut-off now: only the six items in flight. When a lane finishes its item, next.sh prints NONE and its
workflow ends; that is expected. Do not start anything new.

## 06:20 UTC, 20 Nov: container restarted a third time (06:18); lanes relaunched

Trunk = origin = ddefe96, 41 improvements. Whole-branch review is finished (review/*.md, final-fixes.md,
changelog-fixes.md); assemble-changelog.py is written and dry-run; Linux clippy with Rust 1.100.0 (plain and term)
and nu-check were clean at 1acaff3; xcheck OK at 1acaff3 (its cache in /dev/shm is gone again).
Lanes (queue script, max_turns 2): b wwfxuwpcx (accented-names-search, committed 57fc7b9, in review),
c w3me1adqw (fix-rename-press-selection, committed 179e49a, in review), d wofkke8ul (near-miss-beside-scattered,
committed 8c3fe72, in review), e wxl223x0l (fix-filter-reruns-and-fresh-rows, committed dcf3cf4, in review),
f wg53tc542 (fix-here-folder-spelling, being written), g wlvwkcbah (fix-own-small-faults, committed 8e5d97c,
in review). Unclaimed: fix-wording-after-later-changes, fix-folder-search-own-name-pool (waits for both engine
items). Watcher: bw4qhjz6k. Reviewers' pushes to origin are sometimes refused: after each landing compare
`git ls-remote origin search-ui-polish` with the trunk and push from the main session.

## 06:25 UTC: the user asked to move the remaining queued items to the later batch

fix-wording-after-later-changes and fix-folder-search-own-name-pool are in queue/later.txt. Nothing is queued now:
only what the lanes hold is finished (accented-names-search, near-miss-beside-scattered, fix-rename-press-selection,
fix-filter-reruns-and-fresh-rows, fix-own-small-faults, fix-here-folder-spelling). Do not start anything new.
What the two parked fix-ups leave on the branch, for the report: a few sentences in the Search Syntax dialog, the
manual and the README that later changes made inexact (every word must match; "Return searches now"; "Kept up to
date" for a contents search; F5 rows), and that searching one large folder for a word in the folder's own name can
miss matches (worse since scoped-search-special-folder-names for folders with a space or special character in
their path, between about 4,000 and 20,000 items).
