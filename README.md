# search-ui-polish: notes and backlog

This branch holds no code. It keeps the working notes behind the `search-ui-polish` branch of this fork:
what was built, how each change was checked, and the improvements that were written up and saved for later.

## Where things stand

- `search-ui-polish` is the product branch: improvements to search and to the UI on top of upstream's `66c919b`,
  one commit each. Every commit's message ends with its changelog entry.
- Work stopped at a cut-off: everything rated 4 of 5 for value to a user was built, and the rest was saved here.
- Nothing on `search-ui-polish` has been linked or run on macOS or Windows. It is compiled and linted for both
  (check and clippy, with and without the `term` feature), and the tests run on Linux.
- No pull request was opened.

## What is here

| Path | What it is |
| --- | --- |
| `later.md` | The backlog: every improvement that is written up and not built, highest value first, with the problem, a proposal, tests and risks. Start here. |
| `briefs/<id>.json` | The full write-up of every improvement, built or not, with the verifier's plan and notes that later reviews added. |
| `done/<id>.json` | The reviewer's record of each improvement that landed: what changed for a user, what was left out, the tests, what the review fixed, what it found and did not fix, and the doubts that remain. |
| `final-pass-notes.md` | Running notes: things only the owner can check or decide, and finds that did not get a write-up. |
| `GUIDE.md` | The rules every implementer and reviewer followed: house style, the test requirement, pictures, documentation, the commit format. |
| `queue/`, `deps.json` | The order of work and which improvement builds on which. `queue/later.txt` is the saved list. |
| `parked/` | Partial work that was stopped at the cut-off, as patches against `search-ui-polish`. |
| `report/` | The generator of the progress page and the curated list of things for the owner (`owner.json`). |
| `*.sh`, `*.py`, `*.js` | The tools: `check.sh` (format, clippy, the whole test suite), `shot.sh` (a picture of the real window, headless), `xcheck.sh` (compile checks for Windows and macOS from Linux), the queue and its claims, and the workflow scripts. |

Not here: the screenshots (about 400 MB), the demo folder the pictures were taken in (`mkdemo.sh` makes it again),
and build output.

## How each improvement was made

1. A review of the whole repository listed candidates. A second reviewer checked each against the code and wrote a
   plan (`briefs/`, `verdicts.json`).
2. An implementer took one improvement in its own git worktree, wrote the change with tests and documentation, and
   committed it there.
3. A different reviewer assumed the change had a bug it had not found yet: showed that the new tests fail without the
   change, read the changed functions whole, looked at the window in pictures, fixed what it found, rebased onto the
   branch, ran `check.sh` again, and only then put the commit on `search-ui-polish` and wrote `done/<id>.json`.
4. Every few landings the branch was compiled for Windows and macOS.

## Picking up the backlog

For one improvement: read its section in `later.md` and `briefs/<id>.json`, then `GUIDE.md`. Reproduce the problem
first; several write-ups say plainly that nobody has seen the fault in the window yet. A change needs a test that
fails without it, and the reviewers' notes in `done/` name the tests and functions that later work has to respect:
`grep -l '<id>' done/*.json` finds every record that mentions an improvement.

To run the same pipeline again in a Claude Code session, give it this branch and `search-ui-polish`, and point it at
`GUIDE.md`, `RESUME.md` and `impl-workflow-2.js`. The scripts hold absolute paths of the machine the work ran on
(`/home/user/polish-tools`, `/home/user/fffinder-lane-*`); they need the same layout or a search and replace.
