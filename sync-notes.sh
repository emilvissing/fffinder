#!/usr/bin/env bash
# Copy the notes that are worth keeping into the notes branch's worktree (first argument).
set -euo pipefail
R=/home/user/polish-tools; T=${1:?worktree of search-ui-polish-notes}
python3 $R/mklater.py $R/later.md >/dev/null
for d in briefs done queue parked report; do
  rm -rf "$T/$d"; mkdir -p "$T/$d"; cp -a "$R/$d/." "$T/$d/"
done
rm -rf "$T/queue/before-cutoff" "$T/report/__pycache__"
for f in later.md GUIDE.md final-pass-notes.md deps.json RESUME.md verdicts.json orchestrator-notes.json \
         check.sh shot.sh mkdemo.sh claim.sh next.sh xcheck.sh xcheck-if-due.sh watch3.sh prune-lane.sh prune-probes.sh \
         seed-lane.sh convert-lane.sh mkchangelog.py mkbriefs.py mklater.py digest.py show.py sync-notes.sh impl-workflow-2.js; do
  if [ -f "$R/$f" ]; then cp "$R/$f" "$T/$f"; fi
done
cp /root/.claude/projects/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/workflows/scripts/fffinder-branch-review-wf_4cb04a02-c47.js "$T/branch-review-workflow.js" 2>/dev/null || true
