#!/usr/bin/env bash
# Run the Windows and macOS compile checks when the trunk has moved enough
# since the last run (default: 3 commits). Keeps the last result in xcheck-last.txt.
set -uo pipefail
root=/home/user/polish-tools
need=${1:-3}
tip=$(git -C /home/user/fffinder rev-parse search-ui-polish)
last=$(awk 'NR==1{print $1}' "$root/xcheck-last.txt" 2>/dev/null || true)
if [ -n "$last" ]; then
  ahead=$(git -C /home/user/fffinder rev-list --count "$last..$tip" 2>/dev/null || echo 99)
  if [ "$ahead" -lt "$need" ]; then echo "xcheck not due: $ahead new commits since ${last:0:9}"; exit 0; fi
fi
# The cache lives in memory, to spare the disk; a restart of the container empties it.
mkdir -p /dev/shm/xcheck-target
out=$(cd /home/user/fast/xcheck && XCHECK_MIN_FREE_GB=4 "$root/xcheck.sh" "$tip" 2>&1)
verdict=$(printf '%s\n' "$out" | grep -E "^XCHECK (OK|FAILED)" | tail -1)
if printf '%s' "$verdict" | grep -q "^XCHECK OK"; then
  # Linux as it ships: the library and the program without the features that
  # only the tests' dependencies turn on, with the toolchain CI uses.
  host=$(cd /home/user/fast/xcheck && env -u RUSTC_WRAPPER RUSTUP_TOOLCHAIN=1.100.0 CARGO_TARGET_DIR=/home/user/fast/xcheck/target \
    cargo check -j2 --locked --workspace 2>&1)
  if [ $? -ne 0 ] || printf '%s\n' "$host" | grep -qE "^(warning|error)"; then
    printf '%s\n' "$host" | grep -vE "^\s*(Compiling|Checking|Finished|Downloading|Downloaded)" | tail -60
    echo "XCHECK PROBLEM (Linux as shipped) at ${tip:0:9} (last good: ${last:0:9})"
    exit 0
  fi
  echo "$tip OK $(date +%H:%M)" > "$root/xcheck-last.txt"
  echo "XCHECK OK: Windows and macOS (check, clippy, term, as shipped) and Linux as shipped, at ${tip:0:9}"
else
  printf '%s\n' "$out" | tail -80
  echo "XCHECK PROBLEM at ${tip:0:9} (last good: ${last:0:9})"
fi
