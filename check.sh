#!/usr/bin/env bash
# Format check, clippy, and every test, for the worktree you run it in.
#
#   /home/user/polish-tools/check.sh
#
# Prints CHECK OK when everything passes. Three tests fail in this container
# only because it runs as root, which is never refused a file; they are listed
# as known and don't count.
set -uo pipefail
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
logs=target/polish-check; mkdir -p "$logs"
bad=0
if ! cargo fmt --all --check > "$logs/fmt.log" 2>&1; then
  echo "FMT FAILED: run cargo fmt --all"; bad=1
fi
if ! cargo clippy --workspace --all-targets -- -D warnings > "$logs/clippy.log" 2>&1; then
  echo "CLIPPY FAILED:"; grep -E "^(error|warning)" -A14 "$logs/clippy.log" | head -120; bad=1
fi
cargo test --workspace --no-fail-fast > "$logs/test.log" 2>&1
if grep -qE "^error(\[E[0-9]+\])?: " "$logs/test.log" && grep -qE "could not compile|aborting due to" "$logs/test.log"; then
  echo "TESTS DID NOT COMPILE:"; grep -E "^error" -A14 "$logs/test.log" | head -120; bad=1
fi
known="a_folder_that_cant_be_read_shows_a_lock
a_change_that_could_not_be_written_is_written_with_the_next
config::tests::a_bad_file_that_cant_be_moved_aside_is_not_replaced"
failed=$(grep -E "^test .* \.\.\. FAILED$" "$logs/test.log" | sed -E 's/^test (.*) \.\.\. FAILED$/\1/' | sort -u)
unexpected=$(comm -23 <(printf '%s\n' "$failed" | sed '/^$/d' | sort -u) <(printf '%s\n' "$known" | sort -u))
passed=$(grep -E "^test result:" "$logs/test.log" | sed -E 's/.* ([0-9]+) passed.*/\1/' | paste -sd+ | bc)
binaries=$(grep -cE "^test result:" "$logs/test.log")
echo "tests: ${passed:-0} passed in $binaries test binaries; known root-only failures seen: $(comm -12 <(printf '%s\n' "$failed" | sed '/^$/d' | sort -u) <(printf '%s\n' "$known" | sort -u) | wc -l)/3"
if [ "$binaries" -lt 14 ]; then echo "ONLY $binaries TEST BINARIES RAN (expected 14): see $logs/test.log"; bad=1; fi
if [ -n "$unexpected" ]; then
  echo "UNEXPECTED TEST FAILURES:"; printf '%s\n' "$unexpected"
  echo "--- details (full log: $logs/test.log) ---"
  grep -E "panicked at|assertion|left:|right:|^---- " "$logs/test.log" | head -60
  bad=1
fi
[ "$bad" = 0 ] && echo "CHECK OK" || { echo "CHECK FAILED"; exit 1; }
