#!/usr/bin/env bash
# Take a picture of the fffinder window, built from the worktree you run it in.
#
#   /home/user/polish-tools/shot.sh OUT.png [screenshot example arguments...]
#
# Without --dir it opens the demo folder (/home/user/polish-tools/demo/Studio),
# which is also the only search location. Extra settings.json fields can be
# given as JSON in SHOT_SETTINGS, e.g. SHOT_SETTINGS='{"sidebar_mode":"tree"}'.
# Keys in --do steps are Linux ones: ctrl-f, not cmd-f.
set -euo pipefail
out=$1; shift
DEMO=$(/home/user/polish-tools/mkdemo.sh)
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
cargo build -q --example screenshot
cfg=$(mktemp -d /tmp/polish-shot-XXXXXX)
python3 - "$cfg" "$DEMO" <<'PY'
import json, os, sys
cfg, demo = sys.argv[1], sys.argv[2]
settings = {"search_roots": [demo]}
settings.update(json.loads(os.environ.get("SHOT_SETTINGS") or "{}"))
json.dump(settings, open(os.path.join(cfg, "settings.json"), "w"))
PY
args=("$@")
case " ${args[*]-} " in *" --dir "*) ;; *) args+=(--dir "$DEMO");; esac
FFFINDER_CONFIG_DIR=$cfg FFFINDER_DATA_DIR=$cfg/data FFFINDER_SOFTWARE_RENDERING=1 \
  VK_LOADER_DRIVERS_SELECT='*lvp*' LIBGL_ALWAYS_SOFTWARE=1 \
  timeout 180 ./target/debug/examples/screenshot "$out" "${args[@]}" 2> "$cfg/stderr.log" || {
    echo "screenshot failed:"; tail -20 "$cfg/stderr.log"; exit 1; }
rm -rf "$cfg"
