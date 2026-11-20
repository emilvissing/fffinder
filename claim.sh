#!/usr/bin/env bash
# Take an improvement for your lane, so no two lanes work on the same one.
#
#   /home/user/polish-tools/claim.sh <improvement id> <your lane's branch>
#
# Prints one line:
#   YOURS          your lane holds it (newly, or from an earlier attempt): go on
#   DONE           it is already on the trunk
#   CLOSED         it was tried and given up (see closed/<id>.json)
#   TAKEN <lane>   another lane has it
#   WAIT <id>      it builds on <id>, which another lane is working on right now
set -euo pipefail
id=${1:?improvement id}; lane=${2:?lane branch}
root=/home/user/polish-tools
mkdir -p "$root/claims"
if [ -f "$root/done/$id.json" ]; then echo DONE; exit 0; fi
if [ -f "$root/closed/$id.json" ]; then echo CLOSED; exit 0; fi
if [ ! -f "$root/briefs/$id.json" ]; then echo "NO BRIEF: $root/briefs/$id.json"; exit 1; fi
owner=$(cat "$root/claims/$id" 2>/dev/null || true)
if [ -n "$owner" ] && [ "$owner" != "$lane" ]; then echo "TAKEN $owner"; exit 0; fi
if [ -z "$owner" ]; then
  # Something this one builds on may still be in another lane's hands.
  for dep in $(python3 -c 'import json,sys; print(" ".join(json.load(open(sys.argv[1])).get(sys.argv[2], [])))' "$root/deps.json" "$id"); do
    [ -f "$root/done/$dep.json" ] && continue
    [ -f "$root/closed/$dep.json" ] && continue
    dep_owner=$(cat "$root/claims/$dep" 2>/dev/null || true)
    if [ -n "$dep_owner" ] && [ "$dep_owner" != "$lane" ]; then echo "WAIT $dep"; exit 0; fi
  done
  ( set -o noclobber; echo "$lane" > "$root/claims/$id" ) 2>/dev/null || true
  owner=$(cat "$root/claims/$id" 2>/dev/null || true)
fi
if [ "$owner" = "$lane" ]; then echo YOURS; else echo "TAKEN ${owner:-unknown}"; fi
