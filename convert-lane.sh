#!/usr/bin/env bash
# Swap a lane's big build cache (debug info) for a copy of a small one, at a
# moment when nothing is building in either lane.
#
#   convert-lane.sh <lane dir> <source lane dir> [<second source lane dir>]
set -uo pipefail
lane=$1; shift
sources=("$@")
busy() {  # any build or test process that belongs to this worktree
  local d=$1 p
  for p in $(pgrep -x 'cargo|rustc|clippy-driver|ld.lld|rust-lld|cc|screenshot|rustfmt|cargo-clippy|cargo-fmt' 2>/dev/null); do
    if { tr '\0' ' ' < /proc/$p/cmdline; } 2>/dev/null | grep -q -- "$d/" || [ "$(readlink /proc/$p/cwd 2>/dev/null)" = "$d" ]; then return 0; fi
  done
  # test binaries run from the lane's target directory
  pgrep -f -- "$d/target/debug/deps/" >/dev/null 2>&1 && return 0
  return 1
}
quiet_for() {  # wait until the worktree has been quiet for $2 seconds, give up after $3
  local d=$1 need=$2 give_up=$3 q=0 t=0
  while [ $q -lt $need ]; do
    if busy "$d"; then q=0; else q=$((q+1)); fi
    sleep 1; t=$((t+1))
    if [ $t -ge $give_up ]; then return 1; fi
  done
  return 0
}
[ -f "$lane/target/.small" ] && { echo "already converted: $lane"; exit 0; }
free=$(df -m --output=avail /home/user | tail -1 | tr -d ' ')
[ "$free" -lt 8500 ] && { echo "not enough free disk for the copy: ${free}M"; exit 1; }
copied=""
for attempt in 1 2 3 4 5 6; do
  for src in "${sources[@]}"; do
    quiet_for "$src" 3 120 || continue
    stamp=$(mktemp); rm -rf "$lane/target.new"
    cp -a "$src/target" "$lane/target.new"
    if busy "$src" || [ -n "$(find "$src/target" -newer "$stamp" -type f -print -quit)" ]; then
      rm -rf "$lane/target.new" "$stamp"; echo "source $src was building during the copy, trying again"; continue
    fi
    rm -f "$stamp"; copied=$src; break
  done
  [ -n "$copied" ] && break
done
[ -z "$copied" ] && { echo "no quiet source found"; exit 1; }
rm -rf "$lane/target.new/polish-check"
if ! quiet_for "$lane" 2 900; then rm -rf "$lane/target.new"; echo "lane never went quiet: $lane"; exit 1; fi
# The project has a .cargo/config.toml of its own. The lane's copy gets the
# profile line, and git is told not to look at that file, until the same
# setting is in /home/user/.cargo/config.toml for every lane.
printf '\n[profile.dev]\ndebug = false\n' >> "$lane/.cargo/config.toml"
git -C "$lane" update-index --skip-worktree .cargo/config.toml
touch "$lane/target.new/.small"
mv "$lane/target" "$lane/target.old" && mv "$lane/target.new" "$lane/target"
[ -d "$lane/target.old/polish-check" ] && cp -a "$lane/target.old/polish-check" "$lane/target/polish-check"
rm -rf "$lane/target.old"
echo "converted $lane from $copied: $(du -sh "$lane/target" | cut -f1)"
df -h /home/user | tail -1
