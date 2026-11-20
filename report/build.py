#!/usr/bin/env python3
"""Build the progress page for the search and UI polish branch.

Reads the pipeline's own records (done/, briefs/, claims/, queue/, owner.json,
xcheck-last.txt, the trunk's git log) and writes index.html and data.json into
the output folder given as the first argument.
"""
import datetime
import glob
import html
import json
import os
import re
import subprocess
import sys

ROOT = "/home/user/polish-tools"
REPO = "/home/user/fffinder"
BASE = "66c919b"
TRUNK = "search-ui-polish"
out = sys.argv[1]
os.makedirs(out, exist_ok=True)


def git(*args):
    return subprocess.run(["git", "-C", REPO, *args], capture_output=True, text=True).stdout


def esc(s):
    return html.escape(str(s), quote=True)


def inline(s):
    """Escaped text with `code` and **strong** turned into elements."""
    s = esc(s)
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", s)
    return s


def first_sentences(text, limit=230):
    """The start of a changelog entry, cut at a sentence end."""
    parts = re.split(r"(?<=[.!?])\s+", text.strip())
    got = ""
    for p in parts:
        if got and len(got) + len(p) + 1 > limit:
            break
        got = (got + " " + p).strip()
    return got or text[:limit]


# ----- the trunk --------------------------------------------------------------
commits = {}
order = []
for line in git("log", "--reverse", "--format=%H%x09%ct%x09%s", f"{BASE}..{TRUNK}").splitlines():
    h, ct, subject = line.split("\t", 2)
    commits[h] = {"time": int(ct), "subject": subject}
    order.append(h)
tip = git("rev-parse", TRUNK).strip()

# ----- landed -----------------------------------------------------------------
HEADINGS = r"(For (the )?briefs still to come|Found, not fixed|Housekeeping|Scratch files|Known seams|How it was checked|The three ways|Checked\.|Probes|Pictures|With the new trunk)"


def doubts_of(notes):
    m = re.search(r"(Doubts that remain[^\n]*\n?(?:.*?))(?:\n\s*\n(?=" + HEADINGS + r")|\Z)", notes, re.S)
    if not m:
        return ""
    text = m.group(1).strip()
    text = re.sub(r"^Doubts that remain[^.:\n]*[.:]\s*", "", text)
    return text[:2600]


def passed_of(notes):
    m = re.findall(r"(\d{3,4}) passed in (\d+) test binaries", notes)
    return int(m[-1][0]) if m else None


landed = []
details = {}
tests_now = None
for p in glob.glob(f"{ROOT}/done/*.json"):
    d = json.load(open(p))
    h = d.get("commit", "")
    c = commits.get(h)
    when = c["time"] if c else int(os.path.getmtime(p))
    notes = d.get("notes", "") or ""
    as_list = lambda v: v if isinstance(v, list) else ([v] if v else [])
    landed.append({
        "id": d["id"],
        "commit": h[:7],
        "subject": c["subject"] if c else d["id"],
        "kind": d.get("changelog_kind", "Changed"),
        "lead": first_sentences(d.get("changelog", "")),
        "tests": len(as_list(d.get("tests"))),
        "pictures": len(as_list(d.get("screenshots"))),
        "time": when,
        "seq": order.index(h) if h in order else 10**6,
        "passed": passed_of(notes),
    })
    details[d["id"]] = {
        "changelog": d.get("changelog", ""),
        "behaviour": as_list(d.get("behaviour_changes")),
        "left_out": as_list(d.get("left_out")),
        "tests": as_list(d.get("tests")),
        "review_fixes": as_list(d.get("review_fixes")),
        "doubts": doubts_of(notes),
    }
landed.sort(key=lambda r: (r["seq"], r["time"]))
for r in landed:
    if r["passed"]:
        tests_now = r["passed"]

# ----- still to come -----------------------------------------------------------
done_ids = {r["id"] for r in landed}
closed_ids = {os.path.basename(p)[:-5] for p in glob.glob(f"{ROOT}/closed/*.json")}
claims = {}
for p in glob.glob(f"{ROOT}/claims/*"):
    claims[os.path.basename(p)] = open(p).read().strip()
queue_pos = {}
for lane in "bcdefg":
    try:
        lines = open(f"{ROOT}/queue/lane-{lane}.txt").read().splitlines()
    except OSError:
        continue
    for n, line in enumerate(lines):
        i = line.split("#")[0].strip()
        if i and i not in queue_pos:
            queue_pos[i] = (lane, n)
coming = []
for p in glob.glob(f"{ROOT}/briefs/*.json"):
    b = json.load(open(p))
    i = b["id"]
    if i in done_ids or i in closed_ids:
        continue
    v = b.get("verifier", {})
    lane = claims.get(i, "")
    coming.append({
        "id": i,
        "title": b.get("title", i),
        "area": b.get("area", ""),
        "kind": b.get("kind", ""),
        "value": v.get("user_value") or 0,
        "effort": v.get("effort") or "",
        "state": "flight" if lane else "queued",
        "lane": lane.replace("lane-", "").upper() if lane else "",
        "from_review": any(str(r).startswith("review of") for r in b.get("raised_by", [])),
        "pos": queue_pos.get(i, ("z", 999)),
    })
coming.sort(key=lambda r: (r["state"] != "flight", -r["value"], r["pos"]))
# After the cut-off, only what queue/all.txt names is still being built; the rest is saved for later.
try:
    within = {l.split("#")[0].strip() for l in open(f"{ROOT}/queue/all.txt") if l.split("#")[0].strip()}
except OSError:
    within = None
if within is not None and os.path.exists(f"{ROOT}/queue/later.txt"):
    later = [r for r in coming if r["id"] not in within and r["state"] != "flight"]
    coming = [r for r in coming if r["id"] in within or r["state"] == "flight"]
else:
    later = []
later.sort(key=lambda r: (-r["value"], r["id"]))

# ----- checks -------------------------------------------------------------------
xc = {"commit": "", "time": ""}
try:
    parts = open(f"{ROOT}/xcheck-last.txt").read().split()
    xc = {"commit": parts[0][:7], "time": parts[2] if len(parts) > 2 else ""}
except OSError:
    pass
xc_behind = 0
if xc["commit"]:
    try:
        xc_behind = int(git("rev-list", "--count", f"{xc['commit']}..{TRUNK}").strip() or 0)
    except ValueError:
        xc_behind = 0

owner = json.load(open(f"{ROOT}/report/owner.json"))
# When the branch is finished, report/final.json says how the final pass went.
final = None
try:
    final = json.load(open(f"{ROOT}/report/final.json"))
except OSError:
    pass
landed_ids = done_ids
owner = [o for o in owner if not o["id"] or o["id"] in landed_ids]

now = datetime.datetime.now(datetime.timezone.utc)
stamp = now.strftime("%-d %b %Y, %H:%M UTC")
n_landed, n_flight = len(landed), sum(1 for r in coming if r["state"] == "flight")
n_queued = len(coming) - n_flight
total = n_landed + len(coming)
kinds = {k: sum(1 for r in landed if r["kind"] == k) for k in ("Added", "Changed", "Fixed")}

json.dump({"generated": stamp, "details": details}, open(os.path.join(out, "data.json"), "w"), ensure_ascii=False)

# ----- the page -----------------------------------------------------------------
CATS = [
    ("mac", "Check on a Mac", "Nothing here could run on macOS. Each of these takes a few seconds to try."),
    ("changed", "Works differently now", "Behaviour you may notice, because it changed on purpose."),
    ("call", "Judgment calls", "Choices made along the way that you may want to reverse."),
    ("limit", "Known limits", "Found and left alone, or left for later."),
    ("upstream", "Found in existing code", "Not part of this branch, and not changed."),
]


def owner_block():
    parts = []
    seen_keys = {}
    for key, title, sub in CATS:
        items = [o for o in owner if o["cat"] == key]
        if not items:
            continue
        rows = []
        for n, o in enumerate(items):
            ref = f'<a class="ref" href="#i-{esc(o["id"])}" data-jump="{esc(o["id"])}">{esc(o["id"])}</a>' if o["id"] else ""
            if key == "mac":
                seen_keys[o["id"]] = seen_keys.get(o["id"], 0) + 1
                cid = f"mac-{o['id'] or 'tabs'}" + (f"-{seen_keys[o['id']]}" if seen_keys[o["id"]] > 1 else "")
                rows.append(
                    f'<li class="check"><input type="checkbox" id="{esc(cid)}" data-key="{esc(cid)}">'
                    f'<label for="{esc(cid)}">{inline(o["text"])}</label>{ref}</li>'
                )
            else:
                rows.append(f"<li><span>{inline(o['text'])}</span>{ref}</li>")
        parts.append(
            f'<section class="group" data-cat="{key}"><h3>{esc(title)} <span class="count">{len(items)}</span></h3>'
            f'<p class="sub">{esc(sub)}</p><ul class="notes">{"".join(rows)}</ul></section>'
        )
    return "".join(parts)


def landed_rows():
    rows = []
    for n, r in enumerate(reversed(landed)):
        t = datetime.datetime.fromtimestamp(r["time"], datetime.timezone.utc).strftime("%-d %b %H:%M")
        rows.append(
            f'<li class="item" id="i-{esc(r["id"])}" data-id="{esc(r["id"])}" data-kind="{esc(r["kind"])}">'
            f'<button class="row" aria-expanded="false" aria-controls="d-{esc(r["id"])}">'
            f'<span class="tri" aria-hidden="true"></span>'
            f'<span class="name"><span class="subject">{esc(r["subject"][0].upper() + r["subject"][1:])}</span>'
            f'<span class="lead">{inline(r["lead"])}</span></span>'
            f'<span class="kind k-{esc(r["kind"].lower())}">{esc(r["kind"])}</span>'
            f'<span class="num" title="tests added or changed">{r["tests"]} tests</span>'
            f'<span class="hash">{esc(r["commit"])}</span>'
            f'<span class="when">{esc(t)}</span>'
            f"</button>"
            f'<div class="detail" id="d-{esc(r["id"])}" hidden></div></li>'
        )
    return "".join(rows)


def coming_rows(items=None):
    rows = []
    for r in (coming if items is None else items):
        dots = "".join('<i class="on"></i>' if n < r["value"] else "<i></i>" for n in range(5))
        state = (
            f'<span class="state flight">Lane {esc(r["lane"])}</span>' if r["state"] == "flight" else f'<span class="state">{"Saved" if items is not None else "Queued"}</span>'
        )
        tag = '<span class="tag">found in review</span>' if r["from_review"] else ""
        rows.append(
            f'<li class="todo" data-state="{r["state"]}">'
            f'<span class="name"><span class="subject">{esc(r["title"])}</span>{tag}</span>'
            f'<span class="value" title="value to a user, {r["value"]} of 5">{dots}</span>'
            f'<span class="effort" title="size of the change">{esc(r["effort"])}</span>'
            f"{state}</li>"
        )
    return "".join(rows)


def later_block():
    if not later:
        return ""
    return (
        '<section><h2>Saved for later</h2>'
        f'<p class="lede">{len(later)} improvements that are written up and not built. Each has its problem, a proposal, tests and risks in '
        '<code>later.md</code> on the <a href="https://github.com/emilvissing/fffinder/tree/search-ui-polish-notes">search-ui-polish-notes</a> branch of your fork.</p>'
        f'<details class="later"><summary>Show all {len(later)}</summary><ul class="list">{coming_rows(later)}</ul></details></section>'
    )


pct = round(100 * n_landed / total) if total else 0


def coming_block():
    return f"""  <section>
    <h2>{'Finishing now' if later else 'Still to come'}</h2>
    <p class="lede">{'The cut-off is everything rated four of five. These are the last items before the final pass: a review of the whole branch, the changelog, and the last checks.' if later else 'What the lanes are on now, then the queue, highest value first.'} Dots show value to a user out of five; S, M and L are the size of the change.</p>
    <ul class="list">{coming_rows() or '<li class="empty">Nothing is left to build. The final pass is next.</li>'}</ul>
  </section>

"""


def final_block():
    items = "".join(f"<li>{inline_md(x)}</li>" for x in final["done"])
    return (
        '  <section>\n    <h2>Final pass</h2>\n'
        f'    <p class="lede">{inline_md(final["lede"])}</p>\n'
        f'    <ul class="checks">{items}</ul>\n  </section>\n'
    )


def inline_md(text):
    """Escape, then `code` and **bold**."""
    out = esc(text)
    out = re.sub(r"`([^`]+)`", r"<code>\1</code>", out)
    return re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", out)

xc_line = (
    f'Compiles and lints for Windows and macOS at <code>{esc(xc["commit"])}</code>'
    + (f", {xc_behind} landing{'s' if xc_behind != 1 else ''} ago" if xc_behind else ", the current tip")
    if xc["commit"]
    else "No cross-platform compile check yet"
)

page = f"""<title>fffinder Polish Log</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&family=Space+Grotesk:wght@500;600&display=swap">
<style>
/* Layout: one column like a file browser's window. A path bar on top, a list view with
   disclosure triangles for what landed, and a status bar that counts the items. */
:root {{
  --bg: #f3f5f8; --surface: #ffffff; --stripe: #f7f8fb; --ink: #1a2230; --muted: #5b6678;
  --line: #dde2ea; --accent: #2360d6; --sel: #e2ecfe; --focus: #2360d6;
  --added: #166b43; --added-bg: #e1f3e8; --changed: #7d5200; --changed-bg: #faeed0;
  --fixed: #5b36b0; --fixed-bg: #ebe5fa; --live: #0b6f8a; --live-bg: #dcf1f6;
  --display: "Space Grotesk", "Inter", system-ui, sans-serif;
  --text: "Inter", system-ui, -apple-system, "Segoe UI", sans-serif;
  --mono: "JetBrains Mono", ui-monospace, "SF Mono", Menlo, monospace;
}}
@media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{
  --bg: #13161b; --surface: #1a1e25; --stripe: #1e232b; --ink: #e6e9ef; --muted: #98a3b5;
  --line: #2b313b; --accent: #7aa7ff; --sel: #22324f; --focus: #7aa7ff;
  --added: #73d6a3; --added-bg: #16301f; --changed: #efc368; --changed-bg: #352a0d;
  --fixed: #bba4f6; --fixed-bg: #2a2147; --live: #79d3ea; --live-bg: #11313a; color-scheme: dark }} }}
:root[data-theme="dark"] {{
  --bg: #13161b; --surface: #1a1e25; --stripe: #1e232b; --ink: #e6e9ef; --muted: #98a3b5;
  --line: #2b313b; --accent: #7aa7ff; --sel: #22324f; --focus: #7aa7ff;
  --added: #73d6a3; --added-bg: #16301f; --changed: #efc368; --changed-bg: #352a0d;
  --fixed: #bba4f6; --fixed-bg: #2a2147; --live: #79d3ea; --live-bg: #11313a; color-scheme: dark }}
body {{ background: var(--bg); color: var(--ink); font: 15px/1.5 var(--text); }}
.wrap {{ max-width: 980px; margin: 0 auto; padding-block: 28px 56px;
  padding-left: max(16px, env(safe-area-inset-left, 0px)); padding-right: max(16px, env(safe-area-inset-right, 0px));
  display: flex; flex-direction: column; gap: 28px; }}
a {{ color: var(--accent); }}
code {{ font: 0.9em var(--mono); background: var(--stripe); border: 1px solid var(--line); border-radius: 4px; padding: 0 4px; overflow-wrap: anywhere; }}
:focus-visible {{ outline: 2px solid var(--focus); outline-offset: 2px; border-radius: 4px; }}

.path {{ display: flex; flex-wrap: wrap; align-items: center; gap: 6px; font: 500 12.5px var(--mono); color: var(--muted); }}
.path b {{ color: var(--ink); font-weight: 500; }}
.path i {{ font-style: normal; opacity: .6; }}
h1 {{ font: 600 clamp(26px, 5vw, 36px)/1.1 var(--display); letter-spacing: -0.01em; margin: 10px 0 8px; text-wrap: balance; }}
.intro {{ color: var(--muted); max-width: 65ch; margin: 0; }}

.status {{ background: var(--surface); border: 1px solid var(--line); border-radius: 10px; padding: 14px 16px; display: flex; flex-direction: column; gap: 10px; }}
.status .line {{ display: flex; flex-wrap: wrap; gap: 6px 18px; align-items: baseline; font-variant-numeric: tabular-nums; }}
.status .big {{ font: 600 20px var(--display); }}
.status .unit {{ color: var(--muted); }}
.bar {{ height: 6px; border-radius: 3px; background: var(--line); overflow: hidden; display: flex; }}
.bar span {{ display: block; height: 100%; }}
.bar .done {{ background: var(--accent); }}
.bar .fly {{ background: var(--live); opacity: .7; }}
.status .fine {{ color: var(--muted); font-size: 13px; margin: 0; }}

h2 {{ font: 600 20px/1.2 var(--display); margin: 0 0 4px; }}
.lede {{ color: var(--muted); margin: 0 0 14px; max-width: 65ch; }}
h3 {{ font: 600 14px var(--text); margin: 0; display: flex; align-items: center; gap: 8px; }}
.count {{ font: 500 12px var(--mono); color: var(--muted); border: 1px solid var(--line); border-radius: 999px; padding: 0 7px; }}
.sub {{ color: var(--muted); font-size: 13px; margin: 2px 0 8px; }}
.groups {{ display: flex; flex-direction: column; gap: 18px; }}
.group {{ background: var(--surface); border: 1px solid var(--line); border-radius: 10px; padding: 14px 16px; }}
.notes {{ list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }}
.notes li {{ display: flex; flex-wrap: wrap; gap: 4px 12px; align-items: baseline; padding: 8px 0; border-top: 1px solid var(--line); }}
.notes li > span, .notes label {{ flex: 1 1 32ch; min-width: 0; max-width: 72ch; }}
.notes .check {{ align-items: flex-start; }}
.notes input {{ margin: 4px 0 0; width: 16px; height: 16px; accent-color: var(--accent); flex: none; }}
.notes input:checked + label {{ color: var(--muted); text-decoration: line-through; text-decoration-color: var(--line); }}
.ref {{ font: 12px var(--mono); color: var(--muted); text-decoration: none; white-space: nowrap; }}
.ref:hover {{ color: var(--accent); text-decoration: underline; }}

.tools {{ display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 10px; }}
.chip {{ font: 500 13px var(--text); color: var(--ink); background: var(--surface); border: 1px solid var(--line); border-radius: 999px; padding: 4px 12px; cursor: pointer; }}
.chip[aria-pressed="true"] {{ background: var(--sel); border-color: var(--accent); }}
.find {{ flex: 1 1 180px; min-width: 0; font: 14px var(--text); color: var(--ink); background: var(--surface); border: 1px solid var(--line); border-radius: 8px; padding: 6px 10px; }}

.list {{ list-style: none; margin: 0; padding: 0; background: var(--surface); border: 1px solid var(--line); border-radius: 10px; overflow: hidden; }}
.list > li + li {{ border-top: 1px solid var(--line); }}
.list > li:nth-child(even) {{ background: var(--stripe); }}
.row {{ width: 100%; display: grid; grid-template-columns: 14px minmax(0, 1fr) auto auto auto auto; gap: 4px 12px; align-items: center;
  text-align: left; font: inherit; color: inherit; background: none; border: 0; padding: 10px 14px; cursor: pointer; }}
.row:hover {{ background: var(--sel); }}
.tri {{ width: 0; height: 0; border-left: 6px solid var(--muted); border-top: 4px solid transparent; border-bottom: 4px solid transparent; transition: transform .12s; }}
.row[aria-expanded="true"] .tri {{ transform: rotate(90deg); }}
.name {{ min-width: 0; display: flex; flex-direction: column; gap: 2px; }}
.subject {{ font-weight: 600; overflow-wrap: anywhere; }}
.lead {{ color: var(--muted); font-size: 13px; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }}
.row[aria-expanded="true"] .lead {{ display: none; }}
.kind {{ font: 600 11px var(--text); letter-spacing: .04em; text-transform: uppercase; border-radius: 4px; padding: 2px 7px; }}
.k-added {{ color: var(--added); background: var(--added-bg); }}
.k-changed {{ color: var(--changed); background: var(--changed-bg); }}
.k-fixed {{ color: var(--fixed); background: var(--fixed-bg); }}
.num, .hash, .when {{ font: 12px var(--mono); color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }}
.detail {{ padding: 4px 14px 16px 40px; display: flex; flex-direction: column; gap: 12px; }}
.detail p {{ margin: 0; max-width: 70ch; }}
.detail h4 {{ font: 600 12px var(--text); letter-spacing: .05em; text-transform: uppercase; color: var(--muted); margin: 0 0 4px; }}
.detail ul {{ margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 6px; max-width: 74ch; }}
.detail li {{ overflow-wrap: anywhere; }}
.detail details > summary {{ cursor: pointer; font: 600 12px var(--text); letter-spacing: .05em; text-transform: uppercase; color: var(--muted); }}
.detail details[open] > summary {{ margin-bottom: 6px; }}
.empty {{ padding: 18px 14px; color: var(--muted); }}

.todo {{ display: grid; grid-template-columns: minmax(0, 1fr) auto auto auto; gap: 4px 12px; align-items: center; padding: 9px 14px; }}
.todo .subject {{ font-weight: 500; }}
.tag {{ font: 11px var(--mono); color: var(--muted); }}
.value {{ display: inline-flex; gap: 3px; }}
.value i {{ width: 6px; height: 6px; border-radius: 50%; background: var(--line); }}
.value i.on {{ background: var(--accent); }}
.effort {{ font: 12px var(--mono); color: var(--muted); width: 1.5em; text-align: center; }}
.state {{ font: 500 12px var(--text); color: var(--muted); min-width: 4.5em; text-align: right; }}
.state.flight {{ color: var(--live); background: var(--live-bg); border-radius: 4px; padding: 1px 7px; text-align: center; }}

.later > summary {{ cursor: pointer; font: 500 14px var(--text); color: var(--accent); margin-bottom: 10px; }}
.checks {{ margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 6px; max-width: 72ch; }}
.foot {{ display: flex; flex-wrap: wrap; gap: 4px 16px; justify-content: space-between; font: 12px var(--mono); color: var(--muted); border-top: 1px solid var(--line); padding-top: 12px; }}

@media (max-width: 640px) {{
  .row {{ grid-template-columns: 14px minmax(0, 1fr) auto; }}
  .row .num, .row .hash, .row .when {{ display: none; }}
  .detail {{ padding-left: 14px; }}
  .todo {{ grid-template-columns: minmax(0, 1fr) auto; }}
  .todo .value, .todo .effort {{ display: none; }}
}}
@media (prefers-reduced-motion: reduce) {{ .tri {{ transition: none; }} }}
</style>

<div class="wrap">
  <header>
    <div class="path"><span>emilvissing</span><i>›</i><span>fffinder</span><i>›</i><b>{esc(TRUNK)}</b></div>
    <h1>Search and UI polish</h1>
    <p class="intro">Every improvement on the branch is one commit, written by one agent and then attacked, fixed and landed by a second. {esc(final["intro"]) if final else "This page follows the branch as it grows."}</p>
  </header>

  <section class="status" aria-label="Where things stand">
    <div class="line">
      <span><span class="big">{n_landed}</span> <span class="unit">landed</span></span>
      <span><span class="big">{n_flight}</span> <span class="unit">being worked on</span></span>
      <span><span class="big">{n_queued}</span> <span class="unit">queued</span></span>
      {f'<span><span class="big">{len(later)}</span> <span class="unit">saved for later</span></span>' if later else ''}
    </div>
    <div class="bar" role="img" aria-label="{n_landed} of {total} landed, {n_flight} in progress">
      <span class="done" style="width:{100 * n_landed / total if total else 0:.2f}%"></span><span class="fly" style="width:{100 * n_flight / total if total else 0:.2f}%"></span>
    </div>
    <p class="fine">{kinds['Fixed']} fixed, {kinds['Changed']} changed, {kinds['Added']} added. {f"{final['tests']} tests pass on Linux at the final commit. " if final and final.get("tests") else (f"{tests_now} tests pass on Linux. " if tests_now else "")}{xc_line}. Nothing has been linked or run on macOS or Windows.</p>
  </section>

  <section>
    <h2>For you</h2>
    <p class="lede">The short list that needs a person: things only a Mac can show, behaviour that changed on purpose, and choices you may want to undo.</p>
    <div class="groups">{owner_block()}</div>
  </section>

  <section>
    <h2>Landed</h2>
    <p class="lede">Newest first. Open a row for the changelog entry, every behaviour change, what was left out, and the doubts the reviewer still has.</p>
    <div class="tools">
      <button class="chip" data-filter="all" aria-pressed="true">All {n_landed}</button>
      <button class="chip" data-filter="Fixed" aria-pressed="false">Fixed {kinds['Fixed']}</button>
      <button class="chip" data-filter="Changed" aria-pressed="false">Changed {kinds['Changed']}</button>
      <button class="chip" data-filter="Added" aria-pressed="false">Added {kinds['Added']}</button>
      <input class="find" id="find" type="search" placeholder="Filter by word" aria-label="Filter landed improvements by word">
    </div>
    <ul class="list" id="landed">{landed_rows()}</ul>
    <p class="empty" id="none" hidden>Nothing landed matches that.</p>
  </section>

{final_block() if final else coming_block()}
{later_block()}

  <section>
    <h2>How each one was checked</h2>
    <ul class="checks">
      <li>New tests are shown to fail without the change, by the reviewer and not the author.</li>
      <li>The whole test suite, formatting and clippy pass on the commit that lands, after it is rebased onto the branch.</li>
      <li>Anything visible is looked at in pictures of the real window, before and after, in light and dark.</li>
      <li>Every few landings the branch is compiled and linted for Windows and macOS, with and without the terminal feature. That is a compile check only.</li>
      <li>No pull request has been opened. {"The branch and its changelog are on your fork." if final else "The branch is pushed to your fork after every landing."}</li>
    </ul>
  </section>

  <footer class="foot"><span>{total} items, {n_landed} landed, {total - n_landed} to go{f', {len(later)} saved for later' if later else ''}</span><span>tip {esc(tip[:7])} · updated {esc(stamp)}</span></footer>
</div>

<script>
(function () {{
  var details = null, pending = [];
  function esc(s) {{ return String(s).replace(/[&<>"]/g, function (c) {{ return {{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}}[c]; }}); }}
  function inline(s) {{ return esc(s).replace(/`([^`]+)`/g, '<code>$1</code>').replace(/\\*\\*([^*]+)\\*\\*/g, '<strong>$1</strong>'); }}
  function list(title, items, open) {{
    if (!items || !items.length) return '';
    var body = '<ul>' + items.map(function (x) {{ return '<li>' + inline(x) + '</li>'; }}).join('') + '</ul>';
    return open ? '<div><h4>' + title + '</h4>' + body + '</div>'
                : '<details><summary>' + title + ' (' + items.length + ')</summary>' + body + '</details>';
  }}
  function fill(box, id) {{
    var d = details && details[id];
    if (!d) {{ box.innerHTML = '<p class="sub">' + (details ? 'No details recorded for this one.' : 'Loading the details…') + '</p>'; return; }}
    box.innerHTML = '<p>' + inline(d.changelog) + '</p>' +
      list('What changes for a user', d.behaviour, true) +
      (d.doubts ? '<div><h4>Doubts the reviewer still has</h4><p>' + inline(d.doubts) + '</p></div>' : '') +
      list('Left out', d.left_out, false) +
      list('Fixed in review', d.review_fixes, false) +
      list('Tests', d.tests, false);
  }}
  function toggle(btn, want) {{
    var li = btn.closest('.item'), box = li.querySelector('.detail');
    var open = want === undefined ? btn.getAttribute('aria-expanded') !== 'true' : want;
    btn.setAttribute('aria-expanded', open ? 'true' : 'false');
    box.hidden = !open;
    if (open) fill(box, li.dataset.id);
  }}
  document.querySelectorAll('#landed .row').forEach(function (btn) {{
    btn.addEventListener('click', function () {{ toggle(btn); }});
  }});
  fetch('data.json').then(function (r) {{ return r.json(); }}).then(function (j) {{
    details = j.details;
    document.querySelectorAll('#landed .row[aria-expanded="true"]').forEach(function (btn) {{
      fill(btn.closest('.item').querySelector('.detail'), btn.closest('.item').dataset.id);
    }});
  }}).catch(function () {{
    details = {{}};
  }});

  var kind = 'all', word = '';
  function apply() {{
    var shown = 0;
    document.querySelectorAll('#landed .item').forEach(function (li) {{
      var ok = (kind === 'all' || li.dataset.kind === kind) && (!word || li.textContent.toLowerCase().indexOf(word) >= 0);
      li.hidden = !ok; if (ok) shown++;
    }});
    document.getElementById('none').hidden = shown > 0;
  }}
  document.querySelectorAll('.chip').forEach(function (chip) {{
    chip.addEventListener('click', function () {{
      kind = chip.dataset.filter;
      document.querySelectorAll('.chip').forEach(function (c) {{ c.setAttribute('aria-pressed', c === chip ? 'true' : 'false'); }});
      apply();
    }});
  }});
  document.getElementById('find').addEventListener('input', function (e) {{ word = e.target.value.trim().toLowerCase(); apply(); }});

  document.querySelectorAll('[data-jump]').forEach(function (a) {{
    a.addEventListener('click', function (e) {{
      var li = document.getElementById('i-' + a.dataset.jump);
      if (!li) return;
      e.preventDefault();
      kind = 'all'; word = ''; document.getElementById('find').value = '';
      document.querySelectorAll('.chip').forEach(function (c) {{ c.setAttribute('aria-pressed', c.dataset.filter === 'all' ? 'true' : 'false'); }});
      apply();
      toggle(li.querySelector('.row'), true);
      li.scrollIntoView({{ block: 'start', behavior: 'smooth' }});
      li.querySelector('.row').focus({{ preventScroll: true }});
    }});
  }});

  document.querySelectorAll('.check input').forEach(function (box) {{
    var key = 'polish-log:' + box.dataset.key;
    try {{ box.checked = localStorage.getItem(key) === '1'; }} catch (e) {{}}
    box.addEventListener('change', function () {{
      try {{ localStorage.setItem(key, box.checked ? '1' : '0'); }} catch (e) {{}}
    }});
  }});
}})();
</script>
"""
open(os.path.join(out, "index.html"), "w").write(page)
print(f"built: {n_landed} landed, {n_flight} in flight, {n_queued} queued; page {len(page)//1024} KB, data {os.path.getsize(os.path.join(out, 'data.json'))//1024} KB")
