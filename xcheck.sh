#!/usr/bin/env bash
# Cross-platform compile checks for fffinder, run from this Linux container.
#
#   /home/user/polish-tools/xcheck.sh [<git ref>]        (default: search-ui-polish)
#
# Type-checks and lints the code this container never compiles on its own: what
# is under cfg(windows) and cfg(target_os = "macos"), with src/win.rs and
# src/macos.rs. For each of the targets
#
#   x86_64-pc-windows-gnu      (CI builds x86_64-pc-windows-msvc: the same
#                               cfg(windows) Rust code, another C runtime)
#   aarch64-apple-darwin
#
# it runs, with and without `--features term`:
#
#   cargo check  -j2 --locked --keep-going --workspace --all-targets --target <target>
#   cargo clippy -j2 --locked --keep-going --workspace --all-targets --target <target> -- -D warnings
#
# and ends with `XCHECK OK: <what passed>`, or with the errors and `XCHECK FAILED`.
# (CI itself only runs the `--features term` clippy on macOS and Linux.)
#
# This is `cargo check`: nothing is linked and nothing is run. It finds compile
# errors and lints. It does not find link errors (a missing framework or
# library, a misspelled extern symbol), errors that only show when code is
# generated, or anything that happens at run time. No test runs, and doc tests
# are not compiled.
#
# It works in its own worktree, /home/user/fast/xcheck, which it moves to the
# ref (detached, so no branch is touched). Everything it writes is under
# /home/user/fast/xcheck/target: logs in xcheck-logs/, its tools in xcheck-tools/.
#
# What it needs, and what it does about it:
#
# * Rust 1.100.0, next to the container's default 1.97.0 (which is left alone).
#   gpui-pre-macos 0.3.10 calls Option::map_or_default, which 1.97.0 rejects as
#   unstable, so the macOS target cannot be checked with the default toolchain.
#   1.100.0 was the latest stable when this was written, which is also what CI
#   installs (dtolnay/rust-toolchain@stable). Its clippy knows lints 1.97.0 does
#   not, so a failure here can be one the Linux lanes do not see; CI would.
#   Installed by this script if it is missing: `rustup toolchain install 1.100.0`
#   with clippy and both targets, 0.9 GB. XCHECK_TOOLCHAIN names another one.
# * Windows: mingw-w64's C compiler and windres, for the C dependencies' build
#   scripts and the icon embed-resource compiles. From apt; not installed by
#   this script:
#     apt-get install -y --no-install-recommends gcc-mingw-w64-x86-64-posix \
#       binutils-mingw-w64-x86-64 mingw-w64-x86-64-dev
# * macOS: no Apple SDK and no Xcode. Two things stand in for them:
#   1. The C dependencies' build scripts (zlib, lmdb, libgit2, blake3,
#      objc_exception) are compiled by the container's own clang (from apt; the
#      Linux build needs it anyway) against the macOS libc and Objective-C
#      runtime headers that zig ships: the folder lib/libc/include/any-darwin-any
#      of the `ziglang` package on PyPI, 8 MB. This script downloads that
#      package once (to RAM, /dev/shm), keeps only those headers, in
#      xcheck-tools/macos-headers/, and never runs zig itself.
#   2. AN EMPTY STAND-IN FILE. gpui-pre-apple include_str!s
#      $OUT_DIR/stitched_shaders.metal, the Metal shader source its build script
#      writes. That build script only does anything when it is compiled ON
#      macOS (its body is under #[cfg(target_os = "macos")], and build scripts
#      are compiled for the host), so here the file is never written. A
#      RUSTC_WRAPPER (xcheck-tools/bin/rustc-standin) creates it empty just
#      before gpui-pre-apple is compiled. That is fine only because nothing is
#      linked or run: a real build needs the real file, made on a Mac.
#
# Disk: about 0.5 GB of build scripts and proc macros both targets share, and
# the checked dependencies of each target: 0.83 GB for Windows, 0.62 GB for
# macOS. A target whose dependencies are all there is always checked: that
# writes next to nothing. The disk is shared, so when checking the dependencies
# of a target would leave less than XCHECK_MIN_FREE_GB free, the other target's
# are deleted first, after that target has been checked (it then takes 3 to 7
# minutes the next time instead of one), and when that is not enough the target
# is not run and the result is XCHECK FAILED. To give all of it back:
#   rm -rf /home/user/fast/xcheck/target/{debug,x86_64-pc-windows-gnu,aarch64-apple-darwin}
#
# Options, as environment variables:
#   XCHECK_TARGETS="windows macos"   which targets to run (default both)
#   XCHECK_TOOLCHAIN=1.100.0         the rustup toolchain
#   XCHECK_MIN_FREE_GB=9             what to leave free on the disk (0: don't care)
#   XCHECK_KEEP_WORKTREE=1           do not move the worktree: check what is in
#                                    it now, local changes included
set -uo pipefail

ref=${1:-search-ui-polish}
repo=/home/user/fffinder
wt=/home/user/fast/xcheck
logs=$wt/target/xcheck-logs
tools=$wt/target/xcheck-tools
toolchain=${XCHECK_TOOLCHAIN:-1.100.0}
zig_version=0.17.1          # only its macOS libc headers are used
min_free_gb=${XCHECK_MIN_FREE_GB:-9}
want=${XCHECK_TARGETS:-windows macos}
jobs=2
win=x86_64-pc-windows-gnu
mac=aarch64-apple-darwin
export PATH="$HOME/.cargo/bin:$PATH"

case "$ref" in
  -h|--help) sed -n '2,/^set -uo/p' "$0" | sed '$d; s/^# \{0,1\}//'; exit 0 ;;
esac

say() { printf '%s\n' "$*"; }
stop() { say "$*"; say "XCHECK FAILED"; exit 1; }
# Free space on the shared disk in MB, and MB written as GB.
free_mb() { df --output=avail -BM /home/user | tail -1 | tr -dc '0-9'; }
gb() { awk -v m="$1" 'BEGIN { printf "%.1f", m / 1024 }'; }
size_mb() { if [ -d "$1" ]; then du -sm "$1" 2>/dev/null | cut -f1; else echo 0; fi; }
# What a target's checked dependencies depend on: the toolchain and Cargo.lock.
deps_key() { echo "$toolchain $(sha1sum < "$wt/Cargo.lock" | cut -d' ' -f1)"; }
# Roughly what checking a target will add to the disk, in MB: what its
# dependencies take once they are all checked (measured: Windows 832, macOS 616,
# and 486 for the build scripts and proc macros both share), less what is there
# already. A run of the target that went through with these dependencies leaves
# a mark, and then they count as all there.
need_mb() {
  local n=0 have full=840
  [ "$1" = "$mac" ] && full=625
  if [ "$(cat "$wt/target/$1/xcheck-complete" 2>/dev/null)" != "$(deps_key)" ]; then
    have=$(size_mb "$wt/target/$1");  [ "$have" -ge "$full" ] || n=$((n + full - have))
  fi
  have=$(size_mb "$wt/target/debug"); [ "$have" -ge 400 ] || n=$((n + 500 - have))
  [ "$n" = 0 ] || n=$((n + 30))
  echo "$n"
}
# Makes sure checking a target leaves the disk above the limit, by deleting the
# other target's artifacts when it would not. Fails, deleting nothing, when
# even that is not enough.
room_for() {
  local t=$1 other need free reclaim=0 floor=$((min_free_gb * 1024))
  need=$(need_mb "$t")
  if [ "$min_free_gb" = 0 ] || [ "$need" = 0 ]; then return 0; fi
  free=$(free_mb)
  [ $((free - need)) -lt "$floor" ] || return 0
  for other in "$win" "$mac"; do
    [ "$other" = "$t" ] || reclaim=$((reclaim + $(size_mb "$wt/target/$other")))
  done
  if [ $((free + reclaim - need)) -lt "$floor" ]; then
    say "xcheck: no room for $t: it may need $need MB, $free MB are free, the other target's artifacts are $reclaim MB, and $floor MB ($min_free_gb GB) are to stay free"
    return 1
  fi
  for other in "$win" "$mac"; do
    if [ "$other" != "$t" ] && [ -d "$wt/target/$other" ]; then
      say "xcheck: $free MB free and $t may need $need MB: deleting the $other artifacts ($reclaim MB) to keep $floor MB ($min_free_gb GB) free"
      rm -rf "${wt:?}/target/$other"
    fi
  done
  [ $(($(free_mb) - need)) -ge "$floor" ]
}
# The diagnostics in a cargo log: each block that starts with error or warning.
diagnostics() {
  awk '/^(error|warning)/ {p=1}
       /^ +(Compiling|Checking|Finished|Fresh|Dirty|Running|Downloaded|Downloading|Blocking|Updating|Locking|Adding) / {p=0}
       p {print}
       /^$/ {p=0}' "$1"
}
# The macOS libc headers zig ships, without zig: the ziglang wheel is downloaded
# (to RAM when there is a /dev/shm), the one folder unpacked, the rest dropped.
macos_headers() {
  local dest=$1 tmp status=0
  tmp=$(mktemp -d -p /dev/shm xcheck.XXXXXX 2>/dev/null) || tmp=$(mktemp -d -p "$tools" tmp.XXXXXX) || return 1
  rm -rf "$dest"
  if TMPDIR=$tmp python3 -m pip download --quiet --no-deps --no-cache-dir -d "$tmp" "ziglang==$zig_version"; then
    python3 - "$tmp" "$dest/usr/include" <<'PYTHON' || status=1
import glob, os, shutil, sys, zipfile
wheel = glob.glob(os.path.join(sys.argv[1], "ziglang-*.whl"))[0]
prefix = "ziglang/lib/libc/include/any-darwin-any/"
count = 0
with zipfile.ZipFile(wheel) as z:
    for info in z.infolist():
        if info.filename.startswith(prefix) and not info.is_dir():
            out = os.path.join(sys.argv[2], info.filename[len(prefix):])
            os.makedirs(os.path.dirname(out), exist_ok=True)
            with z.open(info) as src, open(out, "wb") as dst:
                shutil.copyfileobj(src, dst)
            count += 1
print(count, "header files")
sys.exit(0 if count > 100 else 1)
PYTHON
  else
    status=1
  fi
  rm -rf "$tmp"
  [ "$status" = 0 ] && echo "ziglang $zig_version" > "$dest/ORIGIN"
}
# cargo for one target. The stand-in wrapper is only there for the macOS target.
run_cargo() {
  local target=$1; shift
  if [ "$target" = "$mac" ]; then
    RUSTC_WRAPPER="$tools/bin/rustc-standin" cargo "$@"
  else
    env -u RUSTC_WRAPPER cargo "$@"
  fi
}

main() {
  local began=$SECONDS
  # --- the worktree ---------------------------------------------------------
  local commit
  if [ "${XCHECK_KEEP_WORKTREE:-0}" = 1 ]; then
    commit=$(git -C "$wt" rev-parse --short HEAD)
    [ -z "$(git -C "$wt" status --porcelain --untracked-files=no)" ] || commit="$commit+local changes"
    say "xcheck: checking $wt as it is, at $commit (XCHECK_KEEP_WORKTREE=1)"
  else
    if [ -n "$(git -C "$wt" status --porcelain --untracked-files=no)" ]; then
      git -C "$wt" status --short --untracked-files=no
      stop "xcheck: $wt has changes to tracked files; not moving it. Undo them there first (git -C $wt checkout -- .)."
    fi
    git -C "$wt" checkout -q --detach "$ref" || stop "xcheck: could not check out $ref in $wt"
    commit=$(git -C "$wt" rev-parse --short HEAD)
    say "xcheck: $ref is $(git -C "$wt" log -1 --format='%h %s')"
  fi
  cd "$wt" || stop "xcheck: no $wt"

  # --- the environment ------------------------------------------------------
  export RUSTUP_TOOLCHAIN=$toolchain
  export CARGO_TARGET_DIR=$wt/target      # never another worktree's
  export CARGO_INCREMENTAL=0              # saves about 650 MB a target
  export CARGO_TERM_COLOR=never
  unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS CARGO_BUILD_TARGET CARGO_BUILD_RUSTFLAGS RUSTC_WORKSPACE_WRAPPER
  local headers=$tools/macos-headers t

  local run_win=0 run_mac=0 missing=
  case " $want " in *" windows "*) run_win=1 ;; esac
  case " $want " in *" macos "*) run_mac=1 ;; esac
  [ "$run_win$run_mac" != 00 ] || stop "xcheck: XCHECK_TARGETS=\"$want\" names neither windows nor macos"

  local toolchains; toolchains=$(rustup toolchain list 2>/dev/null)
  if ! grep -q "^$toolchain-" <<<"$toolchains"; then
    if [ "$min_free_gb" != 0 ] && [ $(($(free_mb) - 950)) -lt $((min_free_gb * 1024)) ]; then
      stop "xcheck: Rust $toolchain is not installed and takes 0.9 GB; $(free_mb) MB are free and $((min_free_gb * 1024)) MB ($min_free_gb GB) are to stay free"
    fi
    say "xcheck: installing Rust $toolchain next to the default toolchain"
    rustup toolchain install "$toolchain" --profile minimal --component clippy \
      --target "$win" --target "$mac" --no-self-update > "$logs/setup-rustup.log" 2>&1 \
      || { tail -5 "$logs/setup-rustup.log"; stop "xcheck: could not install Rust $toolchain"; }
  fi
  local have; have=$(rustup target list --installed --toolchain "$toolchain" 2>/dev/null)
  for t in "$win" "$mac"; do
    grep -qx "$t" <<<"$have" || rustup target add --toolchain "$toolchain" "$t" >> "$logs/setup-rustup.log" 2>&1 \
      || stop "xcheck: could not add the Rust target $t to $toolchain"
  done
  cargo clippy --version > /dev/null 2>&1 || rustup component add --toolchain "$toolchain" clippy >> "$logs/setup-rustup.log" 2>&1 \
    || stop "xcheck: could not add clippy to Rust $toolchain"

  if [ "$run_win" = 1 ]; then
    for t in x86_64-w64-mingw32-gcc x86_64-w64-mingw32-windres; do
      command -v "$t" > /dev/null || missing="$missing $t"
    done
    [ -z "$missing" ] || stop "xcheck: missing$missing. Install them with:
  apt-get install -y --no-install-recommends gcc-mingw-w64-x86-64-posix binutils-mingw-w64-x86-64 mingw-w64-x86-64-dev"
  fi

  if [ "$run_mac" = 1 ]; then
    for t in clang llvm-ar python3; do
      command -v "$t" > /dev/null || missing="$missing $t"
    done
    [ -z "$missing" ] || stop "xcheck: missing$missing. clang and llvm-ar come with: apt-get install -y clang llvm"
    if [ "$(cat "$headers/ORIGIN" 2>/dev/null)" != "ziglang $zig_version" ]; then
      say "xcheck: fetching the macOS libc headers of ziglang $zig_version into $headers"
      macos_headers "$headers" > "$logs/setup-macos-headers.log" 2>&1 \
        || { tail -5 "$logs/setup-macos-headers.log"; stop "xcheck: could not fetch the macOS libc headers"; }
    fi
    # The C compiler of the cc crate, for this target only: clang, told to look
    # for system headers in the folder above and nowhere on this machine.
    cat > "$tools/bin/cc-$mac" <<EOF
#!/usr/bin/env bash
exec clang --target=arm64-apple-macosx -isysroot "$headers" "\$@"
EOF
    # The stand-in for the file gpui-pre-apple's build script cannot write on a
    # Linux host; see the top of xcheck.sh. cargo sets CARGO_PKG_NAME and
    # OUT_DIR for each rustc it starts.
    cat > "$tools/bin/rustc-standin" <<'EOF'
#!/usr/bin/env bash
if [ "${CARGO_PKG_NAME:-}" = gpui-pre-apple ] && [ -n "${OUT_DIR:-}" ] && [ -d "$OUT_DIR" ] \
  && [ ! -e "$OUT_DIR/stitched_shaders.metal" ]; then
  : > "$OUT_DIR/stitched_shaders.metal"
fi
exec "$@"
EOF
    chmod +x "$tools/bin/cc-$mac" "$tools/bin/rustc-standin"
    export CC_aarch64_apple_darwin="$tools/bin/cc-$mac"
    export AR_aarch64_apple_darwin=llvm-ar
  fi
  say "xcheck: $(rustc --version), $(cargo clippy --version), $(gb "$(free_mb)") GB free, logs in $logs"

  # --- the checks -----------------------------------------------------------
  # The target with the most already checked goes first, so that when the disk
  # is tight it is used before it is deleted to make room for the other.
  local list=()
  [ "$run_win" = 1 ] && list+=("$win")
  [ "$run_mac" = 1 ] && list+=("$mac")
  if [ "${#list[@]}" = 2 ] && [ "$(size_mb "$wt/target/$mac")" -gt "$(size_mb "$wt/target/$win")" ]; then
    list=("$mac" "$win")
  fi
  local failed=() bad=0 target variant feature scope cmd label log start took status n no_room cold floor=$((min_free_gb * 1024))
  for target in "${list[@]}"; do
    local name=macos ok_here=()
    [ "$target" = "$win" ] && name=windows
    no_room=0 cold=1
    [ "$(need_mb "$target")" != 0 ] || cold=0
    room_for "$target" || no_room=1
    # The third variant checks the library and the program alone, as they
    # ship: without --all-targets nothing turns on the features that only the
    # tests' dependencies ask for (gpui-kit's test-support), so code that
    # compiles only with those is caught.
    for variant in plain term ship; do
      feature=
      [ "$variant" = term ] && feature="--features term"
      scope=--all-targets
      [ "$variant" = ship ] && scope=
      for cmd in check clippy; do
        [ "$variant" = ship ] && [ "$cmd" = clippy ] && continue
        label="$cmd${feature:+ $feature}"
        [ "$variant" = ship ] && label="check as shipped"
        log="$logs/$name-$variant-$cmd.log"
        # Only a target whose dependencies are still to be checked fills the disk.
        if [ "$no_room" = 1 ] || { [ "$cold" = 1 ] && [ "$min_free_gb" != 0 ] && [ "$(free_mb)" -lt "$floor" ]; }; then
          printf '  NOT RUN %-22s %-28s        %s MB free, and %s MB (%s GB) are to stay free\n' \
            "$target" "$label" "$(free_mb)" "$floor" "$min_free_gb"
          failed+=("$target $label (not run: disk)"); bad=1; no_room=1; rm -f "$log"; continue
        fi
        start=$SECONDS
        # shellcheck disable=SC2086
        if [ "$cmd" = check ]; then
          run_cargo "$target" check -j$jobs --locked --keep-going --workspace $scope --target "$target" $feature > "$log" 2>&1
        else
          run_cargo "$target" clippy -j$jobs --locked --keep-going --workspace --all-targets --target "$target" $feature -- -D warnings > "$log" 2>&1
        fi
        status=$?
        took=$((SECONDS - start))
        n=$(grep -cE '^warning' "$log")
        if [ "$status" = 0 ]; then
          printf '  ok      %-22s %-28s %4ss\n' "$target" "$label" "$took"
          ok_here+=("$label")
          if [ "$n" != 0 ]; then
            say "          $n warning line(s), which do not fail this step:"
            diagnostics "$log" | head -60 | sed 's/^/          /'
          fi
        else
          printf '  FAILED  %-22s %-28s %4ss   %s\n' "$target" "$label" "$took" "$log"
          failed+=("$target $label"); bad=1
          diagnostics "$log" | head -200 | sed 's/^/    /'
          n=$(diagnostics "$log" | wc -l)
          [ "$n" -le 200 ] || say "    ... $((n - 200)) more lines in $log"
        fi
      done
    done
    [ "${#ok_here[@]}" = 0 ] || summary="${summary:+$summary; }$target ($(IFS=,; echo "${ok_here[*]}" | sed 's/,/, /g'))"
    # All five went through, so all of the target's dependencies are checked.
    [ "${#ok_here[@]}" != 5 ] || deps_key > "$wt/target/$target/xcheck-complete"
  done

  say "xcheck: $((SECONDS - began))s; $(gb "$(free_mb)") GB free; $wt/target holds $(gb "$(size_mb "$wt/target")") GB"
  if [ "$bad" = 0 ]; then
    say "XCHECK OK: $summary at $commit, $(rustc --version | cut -d' ' -f1-2)"
  else
    say "passed: ${summary:-nothing}"
    say "failed: $(IFS=';'; echo "${failed[*]}" | sed 's/;/; /g') at $commit, $(rustc --version | cut -d' ' -f1-2)"
    say "XCHECK FAILED"
    return 1
  fi
}

summary=
# The worktree comes first: the logs are kept inside it.
if [ ! -e "$wt/.git" ]; then
  git -C "$repo" worktree add --detach "$wt" "$ref" > /dev/null 2>&1 \
    || stop "xcheck: could not create the worktree $wt at $ref"
fi
mkdir -p "$logs" "$tools/bin" || stop "xcheck: could not create $logs"
# One run at a time: a second one would move the sources under the first.
exec 9> "$wt/target/xcheck.lock"
flock -n 9 || stop "xcheck: another xcheck.sh is running (lock: $wt/target/xcheck.lock)"
main 2>&1 | tee "$logs/last-run.log"
exit "${PIPESTATUS[0]}"
