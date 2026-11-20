#!/usr/bin/env bash
# Delete what a lane's build cache inherited from the lane it was copied from:
# the project's own binaries and libraries, built for the other lane's path.
# Only files older than the lane's seed marker go, so everything the lane has
# built itself since stays. Run it when the lane is not building.
#
#   prune-lane.sh <lane dir>
set -uo pipefail
lane=$1; t="$lane/target"; mark="$t/.small"
[ -f "$mark" ] || { echo "no seed marker in $t"; exit 1; }
names="fffinder|libfffinder|fffinder_core|libfffinder_core"
for f in "$lane"/tests/*.rs "$lane"/examples/*.rs; do names="$names|$(basename "$f" .rs)"; done
before=$(du -sm "$t" | cut -f1)
find "$t/debug/deps" "$t/debug/examples" -maxdepth 1 -type f ! -newer "$mark" 2>/dev/null \
  | grep -E "/($names)-[0-9a-f]{16}(\.|$)" | xargs -r rm -f
find "$t/debug/deps" "$t/debug/examples" -maxdepth 1 -type f ! -newer "$mark" 2>/dev/null \
  | grep -E "/($names)(\.d)?$" | xargs -r rm -f
find "$t/debug/incremental" -mindepth 1 -maxdepth 1 -type d ! -newer "$mark" 2>/dev/null | xargs -r rm -rf
after=$(du -sm "$t" | cut -f1)
echo "pruned $lane: ${before}M -> ${after}M"
