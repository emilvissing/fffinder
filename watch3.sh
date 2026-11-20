#!/usr/bin/env bash
# Wait until something worth looking at happens, then say what. Samples the
# machine every 15 s into watch.log meanwhile, and keeps the disk from filling:
# when little is left, it deletes the incremental-compilation cache of a lane
# that is not building (that lane's next build is slower, nothing else changes).
#   exits on: a new done or closed record, disk under 2.5 GB, memory under 1.2 GB, or after $1 seconds
root=/home/user/polish-tools
limit=${1:-6600}
lanes="/home/user/fffinder-lane-b /home/user/fffinder-lane-c /home/user/fast/fffinder-lane-d /home/user/fast/fffinder-lane-e /home/user/fast/fffinder-lane-f /home/user/fast/fffinder-lane-g"
busy() {
  local d=$1 p
  for p in $(pgrep -x 'cargo|rustc|clippy-driver|ld.lld|rust-lld|cc|screenshot|rustfmt|cargo-clippy|cargo-fmt' 2>/dev/null); do
    if { tr '\0' ' ' < /proc/$p/cmdline; } 2>/dev/null | grep -q -- "$d/" || [ "$(readlink /proc/$p/cwd 2>/dev/null)" = "$d" ]; then return 0; fi
  done
  pgrep -f -- "$d/target/debug/deps/" >/dev/null 2>&1 && return 0
  return 1
}
relieve() {  # free space in the quiet lane whose incremental cache is largest
  local best="" size=0 d s
  for d in $lanes; do
    [ -d "$d/target/debug/incremental" ] || continue
    busy "$d" && continue
    s=$(du -sm "$d/target/debug/incremental" 2>/dev/null | cut -f1)
    if [ "${s:-0}" -gt "$size" ]; then size=$s; best=$d; fi
  done
  [ -z "$best" ] && return 1
  busy "$best" && return 1
  rm -rf "$best/target/debug/incremental"
  echo "$(date +%H:%M:%S) janitor: removed ${size}M of incremental cache in $best" >> "$root/watch.log"
  echo "$(date +%H:%M:%S) $best ${size}M" >> "$root/janitor.log"
}
before=$(ls "$root/done" "$root/closed" 2>/dev/null | sort | tr '\n' ' ')
start=$(date +%s); peak_rss=0; min_mem=999999; min_disk=999999; last_relief=0; last_probe_prune=0
while :; do
  now=$(date +%s)
  mem=$(awk '/MemAvailable/ {print int($2/1024)}' /proc/meminfo)
  disk=$(df -m --output=avail /home/user | tail -1 | tr -d ' ')
  load=$(cut -d' ' -f1 /proc/loadavg)
  rss=$(ps -eo rss,comm | awk '$2 ~ /rustc|cargo|clippy|lld|ld$|finder|screenshot|basics|controls|columns|file_ops|server|settings/ {s+=$1} END {print int(s/1024)}')
  n=$(pgrep -c rustc || true)
  echo "$(date +%H:%M:%S) mem_avail=${mem}M disk=${disk}M load=$load build_rss=${rss}M rustc=$n" >> "$root/watch.log"
  [ "$rss" -gt "$peak_rss" ] && peak_rss=$rss
  [ "$mem" -lt "$min_mem" ] && min_mem=$mem
  [ "$disk" -lt "$min_disk" ] && min_disk=$disk
  if [ "$disk" -lt 5000 ] && [ $((now - last_probe_prune)) -ge 600 ]; then "$root/prune-probes.sh" >> "$root/watch.log" 2>&1; last_probe_prune=$now; disk=$(df -m --output=avail /home/user | tail -1 | tr -d " "); fi
  if [ "$disk" -lt 4500 ] && [ $((now - last_relief)) -ge 60 ]; then relieve && last_relief=$now; fi
  after=$(ls "$root/done" "$root/closed" 2>/dev/null | sort | tr '\n' ' ')
  why=""
  [ "$after" != "$before" ] && why="records changed"
  [ "$disk" -lt 2500 ] && why="DISK LOW: ${disk}M"
  [ "$mem" -lt 1200 ] && why="MEMORY LOW: ${mem}M"
  [ $((now - start)) -ge "$limit" ] && why="time limit"
  if [ -n "$why" ]; then
    echo "WATCH: $why"
    echo "new: $(comm -13 <(echo "$before" | tr ' ' '\n' | sort) <(echo "$after" | tr ' ' '\n' | sort) | tr '\n' ' ')"
    echo "peaks since start: build_rss=${peak_rss}M min_mem_avail=${min_mem}M min_disk=${min_disk}M"
    echo "done: $(ls "$root/done" | wc -l)  closed: $(ls "$root/closed" | wc -l)  claims: $(for f in "$root"/claims/*; do i=$(basename "$f"); [ -f "$root/done/$i.json" ] || [ -f "$root/closed/$i.json" ] || echo -n "$i=$(cat "$f") "; done)"
    [ -f "$root/janitor.log" ] && echo "janitor runs so far: $(wc -l < "$root/janitor.log")"
    git -C /home/user/fffinder log --oneline -4 search-ui-polish
    df -h /home/user | tail -1
    exit 0
  fi
  sleep 15
done
