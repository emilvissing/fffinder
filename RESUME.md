# If the container restarts

Workflows die with the container; everything on disk survives.

1. Implementation, lanes B and C (fixed lists):
   Workflow({scriptPath: "/root/.claude/projects/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/workflows/scripts/fffinder-implement-polish-wf_c269a0ce-a18.js", resumeFromRunId: "wf_c269a0ce-a18", args: <impl-args-1.json>})
2. Implementation, lanes D and E (shared queue):
   Workflow({scriptPath: "/root/.claude/projects/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/workflows/scripts/fffinder-polish-queue-wf_a453dfef-918.js", resumeFromRunId: "wf_a453dfef-918", args: <impl-args-2.json>})
3. Implementation, lanes F and G (shared queue, same script as 2):
   Workflow({scriptPath: ".../workflows/scripts/fffinder-polish-queue-wf_a453dfef-918.js", resumeFromRunId: "wf_dd2dc3e0-d26", args: <impl-args-3.json>})
4. Verification is finished (all 61 verdicts in verdicts.json). Kept for reference:
   Workflow({scriptPath: ".../workflows/scripts/fffinder-search-ui-review-wf_18cbedd6-a2e.js", resumeFromRunId: "wf_18cbedd6-a2e"})
   then python3 /home/user/polish-tools/mkbriefs.py and append the new ids to queue/all.txt.

State: done/ (integrated), closed/ (given up), claims/ (who has what), queue/ (order), briefs/.
Trunk: branch search-ui-polish, pushed to origin after every integration.

Lanes: B /home/user/fffinder-lane-b, C /home/user/fffinder-lane-c, D-G /home/user/fast/fffinder-lane-{d,e,f,g}.
Small builds for all lanes come from /home/user/.cargo/config.toml and /home/user/fast/.cargo/config.toml (debug = false).
New lane: seed-lane.sh; prune inherited artifacts: prune-lane.sh. Cross-platform checks: xcheck.sh (worktree /home/user/fast/xcheck).
Background watcher: watch.sh (re-arm after each notification). Changelog at the end: mkchangelog.py.

## 16:45 UTC, after a pause and a container restart

All three earlier workflows ended at about 10:10 UTC; the container restarted at 16:41.
Relaunched with the queue script (fffinder-polish-queue-wf_a453dfef-918.js), max_turns 1, to finish the six improvements that were in flight:
  lanes b,c: run wf_a621c71b-14d   lanes d,e: run wf_405cba75-499   lanes f,g: run wf_fcb90ce8-03d
After a restart do not resume these by run id blindly: just launch the queue script again on the lanes; next.sh gives each lane its unfinished improvement first.
The cross-check cache in /dev/shm is lost on a restart (a cold xcheck run takes about 11 minutes).

## Since 21:20 UTC, Nov 19

- Watcher: use `watch3.sh 6900` (background, timeout 7200000) from now on; it is watch2 plus `prune-probes.sh` (deletes binaries of throwaway tests whose source is gone) when disk is under 5 GB.
- After each notification: `python3 digest.py`, read the new done record in full, route "Found, not fixed" (new brief only for clear user-visible bugs of value 3 or more; notes into queued briefs' `orchestrator_notes`; the rest into final-pass-notes.md), then `./xcheck-if-due.sh 3` (or 1 after a change to crates/fffinder-core or platform code).
- New briefs written from reviews in this stretch: table-preview-column-widths (lane e), stale-rows-on-place-change (lane c), tree-arrows-past-smart-folder (lane f), filter-stale-keep-set (lane g), idle-redraw-after-search (lane b), settings-rows-fit (lane d).
- Value-4 items were moved to the front of lanes b, f and g (stray-double-click-opens-file, thumbnail-memory, column-pane-selection-by-path, stale-previews-and-cancel).
- One reviewer's `git push origin search-ui-polish` was refused by the permission system; after each landing check `git ls-remote origin search-ui-polish` against the trunk and push from the main session if behind.
- Progress page for the user (private Artifact, published 22:50 UTC): https://claude.ai/artifact/Y6jfGz5JURJaoqeyKkdYuu
  Rebuild with `report/rebuild.sh` after each landing and publish again with the Artifact tool, file_path
  /tmp/claude-0/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/scratchpad/polish-log/index.html and
  files {"data.json": ".../scratchpad/polish-log/data.json"} (same path keeps the URL; after a restart of the
  container pass `url` instead). The curated "For you" items are report/owner.json: add to it when a done record
  has something only the owner can check or decide. The page is the basis of the final report (task #6).
