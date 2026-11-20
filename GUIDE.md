# Working on the `search-ui-polish` improvements

You are one of several agents improving **fffinder**, a Finder-style file browser written in Rust
(GPUI Kit UI, the `fff-search` engine, Nushell built in). The owner uses it on macOS and wants better
search and a better UI on a branch of their fork called `search-ui-polish`. Each improvement was found
by a reviewer and then checked against the code by a second, skeptical reviewer. Your prompt tells you
which improvement is yours and whether you are its **implementer** or its **reviewer**.

Read this whole guide before you do anything.

## Where things are

| What | Where |
| --- | --- |
| Your worktree (your "lane") | given in your prompt, with its branch (for example `/home/user/fffinder-lane-b`, branch `lane-b`). Several lanes work at the same time. |
| The trunk | the branch `search-ui-polish`. It is checked out nowhere. Lanes add to it only with `git push . HEAD:search-ui-polish`, which refuses anything but a fast-forward. Never force it, never rewrite it. |
| Briefs | `/home/user/polish-tools/briefs/<id>.json` |
| Done records | `/home/user/polish-tools/done/<id>.json`, written by the reviewer when the change is on the trunk |
| Screenshots you keep | `/home/user/polish-tools/shots/<id>-<what>.png` |
| Claims | `/home/user/polish-tools/claims/<id>` names the lane that has taken an improvement. Only `claim.sh` and `next.sh` write them. |
| Closed records | `/home/user/polish-tools/closed/<id>.json`, written when an improvement is given up (skipped, failed, or rejected) |
| Helpers | `/home/user/polish-tools/claim.sh`, `/home/user/polish-tools/next.sh`, `/home/user/polish-tools/check.sh`, `/home/user/polish-tools/shot.sh` |

Rules that keep the lanes from hurting each other:

- **Your shell starts every command in `/home/user/fffinder`, which is the wrong place**, and a `cd`
  does not carry over to your next command. So begin every Bash command with `cd <your lane> &&`, and
  give Read, Edit, Write, Grep and Glob absolute paths under your lane. `/home/user/fffinder` is a
  frozen copy other agents are reading: its files are locked, and cargo fails there with "Not a
  directory" or "Operation not permitted". If you see either message, you are in the wrong directory.
- Work **only inside your lane's worktree**. Never edit, build in, or run git commands that change
  `/home/user/fffinder` or another lane's worktree.
- Paths in a brief are relative to the repository root. Resolve them inside your lane.
- Other agents are working in the other lanes right now, and each adds its change to the trunk
  when it is done. So the trunk moves while you work, often more than once, and line numbers in a brief
  can be stale: find code by name, not by line number. `ls /home/user/polish-tools/done/` and
  `git log search-ui-polish` show what has already landed. If an earlier change already covers part of
  your brief, don't do it again.
- Every lane adds tests and manual text, and two lanes that add at the same spot collide when the
  second one rebases. So put a new test beside the tests it is most related to, **not at the end of
  the file**, and keep documentation edits inside the paragraph or table row they belong to.
- **Scratch files.** Every agent shares one scratchpad folder (the one your instructions name). Make
  a folder of your own in it, named `<lane>-<id>` (for example `lane-c-escape-ends-search`), and keep
  everything you write there: patches, saved copies, logs, test data. Never write, move or delete at
  the scratchpad's top level or in a folder that isn't yours, and never delete by pattern
  (`rm *.patch`) anywhere else: another lane's files are there. Delete your folder when you are done.
- Disk is limited, and a lane's `target/` is about 8 GB. Never run `cargo clean`, never build with `--release` or another `--profile`, never use a
  second target directory, never set `RUSTFLAGS` or change profile settings (any of these builds
  everything a second time and fills the disk for every lane), and delete large scratch files you
  make. `df -h /home/user` shows what is left; if it is under 2 GB, stop building and say so in your
  result.
- The machine has 4 CPUs and 15 GB of memory shared by up to six lanes. Pass `-j2` to every `cargo`
  command you run yourself (the helpers already do). A full check takes 2 to 10 minutes depending on
  what the other lanes are doing, so give those commands a long timeout (`timeout: 900000`).

## The project in one minute

```text
crates/fffinder-core/src   engine.rs (wraps fff-search: search, grep, ranking), config.rs (settings),
                           kind.rs, listing.rs, ops.rs (copy/move), ...
src/finder/                the window: mod.rs (state, search, navigation, keys), toolbar.rs, bars.rs,
                           views.rs + list.rs (the four views), sidebar.rs, folder_tree.rs, dialogs.rs,
                           preview.rs, quick_look.rs, tab_bar.rs, menus.rs, filter.rs, ...
src/actions.rs             every action and key binding
src/workspace.rs           tabs and split view
src/server/, nu/fff.nu     the local socket and the Nushell module that talks to it
src/macos.rs, src/win.rs   platform code that this Linux container does NOT compile
src/term/                  `fffinder --term`, behind the `term` feature, not built by default
tests/*.rs                 headless UI tests that drive the real window (harness: tests/common/mod.rs)
docs/manual.md             documents every view, menu, shortcut and setting
```

Dependency sources, when you need to know what the engine or the UI kit really does:
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` has `fff-search-0.11.0`, `gpui-kit-0.8.1`,
`gpui-base-0.8.1`, `gpui-component-0.8.1`, `gpui-pre-0.3.10`, and the rest.

## House style

- Match the code around you: naming, error handling, how state is updated and `cx.notify()` is called,
  how element IDs are built (tests click elements by ID).
- Comments are plain prose that says **why**, the way the existing ones do. No comments that narrate
  the change ("fix for", "now we", "previously"), no banners, no emoji.
- The smallest change that fully solves the problem. No drive-by refactors, no renames for taste, no
  unrelated cleanups, no new dependencies unless the brief calls for one.
- Nothing slow on the window's thread: no file I/O, no index locks, no work that grows with the folder
  size on every frame.
- User-facing text follows the app's wording (look at neighbouring labels and at `docs/manual.md`).
- **Other platforms.** The app ships for macOS, Linux and Windows, and CI runs clippy with
  `-D warnings` on all three, but only Linux compiles here. So when you change, rename or remove a
  function, type, field, import or signature, `grep` for every use, including code under
  `#[cfg(target_os = "macos")]`, `#[cfg(windows)]`, `#[cfg(not(...))]`, in `src/macos.rs`, `src/win.rs`
  and `src/term/`, and update those uses by hand with extra care. Don't leave something unused on one
  platform (an import or helper only one `cfg` uses must carry the same `cfg`). Keys: follow how
  `src/actions.rs` declares bindings for Cmd on macOS and Ctrl elsewhere.
- **The `term` feature.** If you touched `src/term/` or changed the signature of something it calls
  (`grep -rn <name> src/term`), also run
  `cargo clippy -j2 --workspace --all-targets --features term -- -D warnings`.

## Tests

- Every behaviour change gets a test that fails before the change and passes after it: a unit test
  beside the code (`#[cfg(test)] mod tests`) or a headless UI test in `tests/*.rs`.
- Read the harness (`tests/common/mod.rs`) and a few neighbouring tests first. Test names are plain
  sentences in snake case (`a_failed_step_puts_everything_back`). Tests drive the window through real
  clicks and key presses (`ui.click(id)`, `ui.press("ctrl-f")` via `cmd("f")`, `ui.type_text(..)`),
  then assert on state (`ui.names()`, `ui.selected()`, `ui.read(|f| ..)`, `ui.snapshot(id)`).
- No sleeps and nothing that depends on wall-clock timing; use `ui.settle()` and the harness clock.
- One test: `cargo test -j2 --test controls <name>` or `cargo test -j2 -p fffinder-core <name>`.
- Everything: `/home/user/polish-tools/check.sh` (format check, clippy, all tests). It must print
  `CHECK OK`. Three tests fail here only because the container runs as root; the script knows them.
  If a UI test fails once, run it again alone before you believe it: the other lanes share the CPU.
  Logs are in `target/polish-check/`.

## Looking at the window

`/home/user/polish-tools/shot.sh OUT.png [options]` builds the screenshot example in your lane and
renders the real window to a PNG, which you can open with the Read tool. Use it for anything visible:
look at the result, in `--light` and `--dark` when colours are involved.

```bash
cd <your lane>
/home/user/polish-tools/shot.sh /home/user/polish-tools/shots/<id>-search.png \
    --view list --light --size 1100x700 --do key:ctrl-f --do 'type:tax exemption'
```

- Options: `--view icons|list|columns|gallery`, `--light|--dark`, `--query TEXT`, `--scope-folder`,
  `--select N`, `--quicklook`, `--explore`, `--settings`, `--info`, `--tabs N`, `--split`,
  `--no-sidebar`, `--size WxH`, `--dir PATH`, and `--do STEP` any number of times.
  (`--no-sidebar` does not work: the sidebar stays. Use `--do key:ctrl-alt-s` to hide it. Known,
  and not yours to fix.)
- Steps: `key:<keys>` (`ctrl-f`, `enter`, `escape`, `down`; Linux keys, so Ctrl where the Mac has
  Cmd), `click:<element id>`, `right-click:<element id>` (for its menu), `hover:<element id>` (until
  its tooltip shows), `type:<text>`, `wait:<ms>`. The header of `examples/screenshot.rs` on the trunk
  lists them all; earlier changes have added some.
- There is no committed step that turns the wheel. For a picture of something scrolled,
  `/home/user/polish-tools/shots/table-preview-scroll-add-scroll-steps.py` adds temporary ones to
  your lane's `examples/screenshot.rs` (`scroll:x,y,dy[,dx]`, and `qlscroll:`, `qlkey:`, `qlclick:`
  for Quick Look's window); read it first, and check the example out again before you commit.
- It opens the demo folder `/home/user/polish-tools/demo/Studio` (156 files: folders, text, code, CSV,
  JSON, images, names with spaces and brackets, `README.md` beside `readme.txt`, a `Many` folder with
  120 files), which is also the only search location. Don't change the demo folder; copy it if you
  need a different one and pass `--dir`.
- More settings: `SHOT_SETTINGS='{"sidebar_mode":"tree"}' shot.sh ...` merges into `settings.json`.
- If the example can't reach the state you need, a headless UI test that asserts on the element tree
  is the fallback. Extend `examples/screenshot.rs` only if a picture is essential.

## Documentation

- `docs/manual.md` documents behaviour in detail. Update the sections your change makes wrong or
  incomplete, in its voice. Check its tables (search syntax, keyboard shortcuts, settings).
- `README.md` only where it states the thing you changed.
- The in-app Search Syntax dialog (`src/finder/dialogs.rs`) mirrors the manual's syntax table; keep
  them the same.
- **Do not edit `CHANGELOG.md`.** Every lane would collide in it. The changelog entry goes in the
  commit message instead (below), and the changelog is assembled at the end.

## The commit

One commit per improvement, on your lane branch, with git's default identity (don't set a name or
email). Message format, in the style of the existing history (`git log -5`):

```text
<summary: lowercase, imperative, what changes for the user, at most 72 characters>

<body: what was wrong and what happens now, in plain prose, wrapped at 72>

Changelog (<Added|Changed|Fixed>): <the entry as CHANGELOG.md would have it, written for users,
one paragraph>

Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01CT7TqKfmTYMKX3NEP3oJ5f
```

## If you are the implementer

1. Find out which improvement is yours and take it for your lane, before anything else.
   - **Your prompt names an improvement id:** run
     `/home/user/polish-tools/claim.sh <id> <your lane's branch>` (for example
     `claim.sh escape-ends-search lane-c`). It prints one line:
     - `YOURS`: go on with step 2.
     - `DONE`: it is already on the trunk. Return `already-integrated` and stop.
     - `CLOSED`: it was tried and given up. Return status `skipped` with a summary that starts
       `CLOSED`, and stop.
     - `TAKEN <lane>`: another lane has it. Change nothing, return status `skipped` with a summary
       that starts `TAKEN by <lane>`, and stop.
     - `WAIT <other id>`: it builds on an improvement another lane is working on right now. Change
       nothing, return status `skipped` with a summary that starts `WAIT for <other id>`, and stop.
   - **Your prompt names no id:** run `/home/user/polish-tools/next.sh <your lane's branch>`, adding
     `--not id,id,...` when your prompt lists improvements this lane has finished with. It prints the
     id of the improvement you now own (its brief is `/home/user/polish-tools/briefs/<id>.json`), or a
     line starting `NONE` when nothing is left for this lane: then return status `none-left` and stop.
     Run it once; the id it prints is taken for your lane from that moment.
2. Start from the trunk's tip. Run `git status --short` and `git log --oneline search-ui-polish..HEAD`.
   - Clean and empty: `git merge --ff-only search-ui-polish`.
   - Uncommitted changes or commits that are an earlier, interrupted attempt at **this** improvement:
     keep what is good (`git rebase search-ui-polish` if there are commits) and carry on from it.
     The earlier agent may have been stopped at any point, even after committing or in the middle
     of a review's fixes: read what is there, finish it, and check it as if it were your own.
   - `git status` says a rebase is in progress: an earlier agent was stopped while moving this
     improvement onto a newer trunk. Finish the rebase, resolving so that both sides keep working
     (`git log -p search-ui-polish` for the files in conflict shows what the trunk changed), or
     abort it (`git rebase --abort`) and rebase again yourself.
   - Anything that belongs to a different improvement: park it and start clean
     (`git stash -u` for changes; `git branch -f parked/<its id> HEAD` then
     `git reset --hard search-ui-polish` for commits) and say so in your result.
   - `git branch --list 'parked/<id>'` shows a branch for **your** improvement: an earlier attempt was
     set aside there. Read it (`git log -p search-ui-polish..parked/<id>`) and take over what is good
     (`git cherry-pick`), then delete the branch.
3. Read your brief, and what earlier changes left for it:
   `grep -l '<id>' /home/user/polish-tools/done/*.json`. Reviewers write notes there for the briefs
   their change affects, by id; read every record that names yours, and its commit
   (`git show <commit>`), before you plan. In the brief, `verifier.implementation_notes` is the plan
   of record; where it and the original `proposal` disagree, the verifier wins. `orchestrator_notes`,
   when present, overrides both. Nobody has run this plan yet, so check its claims against the code
   as you go and follow the code when it says otherwise. Record every deviation.
4. Reproduce first. Write the test, or take the "before" screenshot, and see it fail for the reason the
   brief gives. If you can't reproduce the problem and the code shows it isn't real, make no changes,
   write the closed record (below), and return `skipped` with the evidence.
5. Implement. Keep to the brief's scope. If it bundles several things and one turns out to be wrong
   or risky, do the solid parts and list the rest as left out, with reasons.
6. Check every item in `verifier.risks` and the plan's pitfalls against your implementation.
7. For anything visible, take screenshots and look at them.
8. Update the documentation.
9. Run `/home/user/polish-tools/check.sh` until it prints `CHECK OK`.
10. Commit (one commit). Push nothing. Leave the worktree clean.
11. If two honest attempts fail, stop: restore the worktree (`git reset --hard search-ui-polish`,
    `git clean -fd` but never `-x`, which would delete `target/`), write the closed record, and return
    `failed` with what you learned.

**The closed record.** An improvement that ends as `skipped` because the brief was wrong, or as
`failed`, gets `/home/user/polish-tools/closed/<id>.json`:
`{"id", "status" ("skipped" or "failed"), "lane", "why" (the evidence, or what you tried and learned)}`.
It keeps the queue from handing the improvement out again. Don't write one for `TAKEN`, `WAIT`,
`DONE` or `CLOSED`.

## If you are the reviewer

You are a different agent from the implementer, and you own the change from here: you find the
problems, fix them, and put the change on the trunk.

1. `cd` into your lane. If the done record exists, return `integrated` with its commit. Read the
   done records that name this improvement (`grep -l '<id>' /home/user/polish-tools/done/*.json`):
   earlier reviewers may have left a warning the implementer had to heed. If a rebase
   is in progress from an interrupted run, finish or abort it. The lane's HEAD should be the
   improvement's one commit on top of the trunk (`git log --oneline search-ui-polish..HEAD`).
2. Read the brief, then the change: `git diff $(git merge-base HEAD search-ui-polish) HEAD`. Read the
   changed functions whole, in their files, not only the hunks. Try to break it:
   - Does it solve the brief's problem, including each item in `verifier.risks` and each pitfall in the
     plan? Tick them off one by one.
   - Regressions: follow every caller of what changed. Think about the other locations (Recents,
     Trash, search results, Smart Folders), the other views, tabs and split view, the file chooser
     (`--pick`), Quick Look in its own window, empty and loading states, very large folders, and
     the other platforms (see House style).
   - Tests: do the new tests fail without the change? Prove it: save the diff of the non-test files
     (`git diff <base> HEAD -- src crates > <your scratch folder>/change.patch`), `git apply -R` it,
     run the new tests, see them fail, then `git apply` it again and confirm `git status` is clean.
     Do they test behaviour a user would see, and are they free of timing dependence?
   - Fit: house style, comments, no leftovers or unrelated edits, documentation correct and complete,
     user-facing wording, commit message in the format above with a Changelog paragraph that is true.
   - For anything visible, take your own screenshots and look at them.
3. Fix what you find and fold the fixes into the same commit (`git commit --amend`), so the trunk gets
   one commit per improvement. If the change is wrong at its root or does more harm than good and
   can't reasonably be fixed, reject it: `git reset --hard search-ui-polish`, write
   `/home/user/polish-tools/closed/<id>.json` (`{"id", "status": "rejected", "lane", "why"}`), and
   return `rejected` with the reasons.
4. `/home/user/polish-tools/check.sh` must print `CHECK OK`.
5. Integrate. `git rebase search-ui-polish`. If another lane's work conflicts with yours, read its
   commits (`git log -p search-ui-polish` for the files in conflict) and resolve so that every change
   keeps working: keep both sides' tests and both sides' behaviour, never drop the other lane's lines
   to make yours apply. If the rebase brought in new trunk commits, run `check.sh` again, and look
   once more at how your change and the new ones work together when they touch the same code. Then
   `git push . HEAD:search-ui-polish`. If that is refused as not a fast-forward, another lane got
   there first: rebase, check, and push again.
6. `git push origin search-ui-polish`. On a network error retry up to four times, waiting 2, 4, 8 and
   16 seconds. If it still fails, say so in your result and go on; the next integration carries it.
7. Write the done record `/home/user/polish-tools/done/<id>.json`:
   `{"id", "commit" (the trunk commit), "summary", "changelog_kind", "changelog", "behaviour_changes",
   "left_out", "tests", "screenshots", "review_fixes", "notes"}`. In `notes`, name by id every brief
   still to come that your change affects (`ls /home/user/polish-tools/briefs/`), with what its
   implementer has to know: their implementers search the done records for their id. Bugs you found
   that were there before and are outside this brief go in `notes` too, under "Found, not fixed".
8. Leave the lane clean and at the trunk's tip.
