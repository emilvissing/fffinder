#!/usr/bin/env bash
# Make a new lane: a worktree on its own branch at the trunk's tip, with a build
# cache copied from another lane at a moment when that lane is not building.
#
#   seed-lane.sh <new lane dir> <branch> <source lane dir>...
set -uo pipefail
lane=$1; branch=$2; shift 2
sources=("$@")
busy() {
  local d=$1 p
  for p in $(pgrep -x 'cargo|rustc|clippy-driver|ld.lld|rust-lld|cc|screenshot|rustfmt|cargo-clippy|cargo-fmt' 2>/dev/null); do
    if { tr '\0' ' ' < /proc/$p/cmdline; } 2>/dev/null | grep -q -- "$d/" || [ "$(readlink /proc/$p/cwd 2>/dev/null)" = "$d" ]; then return 0; fi
  done
  pgrep -f -- "$d/target/debug/deps/" >/dev/null 2>&1 && return 0
  return 1
}
quiet_for() {
  local d=$1 need=$2 give_up=$3 q=0 t=0
  while [ $q -lt $need ]; do
    if busy "$d"; then q=0; else q=$((q+1)); fi
    sleep 1; t=$((t+1))
    if [ $t -ge $give_up ]; then return 1; fi
  done
  return 0
}
if [ ! -d "$lane" ]; then
  git -C /home/user/fffinder worktree add -q "$lane" -b "$branch" search-ui-polish || { echo "worktree failed"; exit 1; }
fi
[ -f "$lane/target/.small" ] && { echo "already seeded: $lane"; exit 0; }
copied=""
for attempt in 1 2 3 4 5 6 7 8; do
  for src in "${sources[@]}"; do
    quiet_for "$src" 3 120 || continue
    stamp=$(mktemp); rm -rf "$lane/target"
    cp -a "$src/target" "$lane/target"
    if busy "$src" || [ -n "$(find "$src/target" -newer "$stamp" -type f -print -quit)" ]; then
      rm -rf "$lane/target" "$stamp"; echo "source $src was building during the copy, trying again"; continue
    fi
    rm -f "$stamp"; copied=$src; break
  done
  [ -n "$copied" ] && break
done
[ -z "$copied" ] && { echo "no quiet source found"; exit 1; }
rm -rf "$lane/target/polish-check"; touch "$lane/target/.small"
echo "seeded $lane from $copied: $(du -sh "$lane/target" | cut -f1)"
