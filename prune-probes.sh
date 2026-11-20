#!/usr/bin/env bash
# Delete the binaries of throwaway tests (tests/zz_*.rs and the like) whose
# source file is gone from the lane. Safe while a lane builds: a probe that is
# in use has its source file, and is left alone.
set -uo pipefail
total=0
for d in /home/user/fffinder-lane-b /home/user/fffinder-lane-c /home/user/fast/fffinder-lane-d /home/user/fast/fffinder-lane-e /home/user/fast/fffinder-lane-f /home/user/fast/fffinder-lane-g; do
  [ -d "$d/target/debug/deps" ] || continue
  known="$(ls "$d"/tests/*.rs "$d"/examples/*.rs 2>/dev/null | xargs -r -n1 basename | sed 's/\.rs$//' | tr '\n' '|')fffinder|fffinder_core|server|nu_module"
  for f in "$d"/target/debug/deps/zz*-????????????????* "$d"/target/debug/deps/*probe*-????????????????*; do
    [ -f "$f" ] || continue
    base=$(basename "$f" | sed -E 's/-[0-9a-f]{16}.*$//')
    echo "$base" | grep -qxE "($known)" && continue
    s=$(du -sm "$f" | cut -f1); total=$((total + s))
    rm -f "$f"
  done
  for i in "$d"/target/debug/incremental/zz* "$d"/target/debug/incremental/*probe*; do
    [ -d "$i" ] || continue
    base=$(basename "$i" | sed -E 's/-[0-9a-z]{10,}$//')
    echo "$base" | grep -qxE "($known)" && continue
    s=$(du -sm "$i" | cut -f1); total=$((total + s))
    rm -rf "$i"
  done
done
echo "prune-probes: freed about ${total}M"
