#!/usr/bin/env bash
# Find the improvement your lane should work on now, and take it for the lane.
#
#   /home/user/polish-tools/next.sh <your lane's branch> [--not id,id,...]
#
# Prints one line: the improvement's id, or NONE when nothing is left that this
# lane can take. Ids after --not are ones this lane has already finished with
# in this run. An improvement an earlier, interrupted attempt in this lane took
# and never finished comes first.
set -euo pipefail
lane=${1:?lane branch}; shift || true
skip=","
if [ "${1:-}" = "--not" ]; then skip=",${2:-},"; fi
root=/home/user/polish-tools
open() { [ ! -f "$root/done/$1.json" ] && [ ! -f "$root/closed/$1.json" ]; }
for f in "$root"/claims/*; do
  [ -f "$f" ] || continue
  id=$(basename "$f")
  case "$skip" in *",$id,"*) continue;; esac
  if [ "$(cat "$f")" = "$lane" ] && open "$id"; then echo "$id"; exit 0; fi
done
waiting=""
for id in $(cat "$root/queue/$lane.txt" "$root/queue/all.txt" 2>/dev/null | sed 's/#.*//' | awk 'NF && !seen[$1]++ {print $1}'); do
  case "$skip" in *",$id,"*) continue;; esac
  open "$id" || continue
  [ -f "$root/briefs/$id.json" ] || continue
  answer=$("$root/claim.sh" "$id" "$lane")
  case "$answer" in
    YOURS) echo "$id"; exit 0;;
    WAIT*) waiting="$waiting $id";;
  esac
done
if [ -n "$waiting" ]; then echo "NONE (still waiting on another lane:$waiting)"; else echo NONE; fi
