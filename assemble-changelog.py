#!/usr/bin/env python3
"""Write the branch's entries into CHANGELOG.md's [Unreleased] section.

    assemble-changelog.py <CHANGELOG.md in a checkout of the trunk> [--check]

Takes each commit's `Changelog (Added|Changed|Fixed):` paragraph (66c919b..search-ui-polish), applies the
corrections of changelog-fixes.md, and puts the bullets after the ones upstream already has under each heading.
With --check it only reports. Every correction must match exactly once, or the script stops.
"""
import re, subprocess, sys
path = sys.argv[1]
check = '--check' in sys.argv
log = subprocess.run(['git', '-C', '/home/user/fffinder', 'log', '--reverse', '--format=%H%x00%s%x00%B%x01', '66c919b..search-ui-polish'],
                     capture_output=True, text=True, check=True).stdout
groups = {'Added': [], 'Changed': [], 'Fixed': []}
skipped = []
for rec in log.split('\x01'):
    rec = rec.strip('\n')
    if not rec:
        continue
    sha, subject, body = rec.split('\x00', 2)
    m = re.search(r'^Changelog \((Added|Changed|Fixed)\):\s*(.*?)(?=^\s*$|^Co-Authored-By:|^Claude-Session:|\Z)', body, re.S | re.M)
    if not m:
        skipped.append(f'{sha[:7]} {subject}')
        continue
    groups[m.group(1)].append([sha[:7], ' '.join(l.strip() for l in m.group(2).strip().splitlines())])

# (commit, old, new): corrections from the whole-branch review. See changelog-fixes.md.
FIXES = [
 ('caceef3', "Each file's preview now starts at its top, and a table that is wider than its space starts at its left edge.",
             "Each file's preview now starts at its top (a contents search result's at its matching line), and a table that is wider than its space starts at its left edge."),
 ('323f173', "A search that comes back runs again with the kind and the Nushell filter chosen now, in relevance order.",
             "A search that comes back runs again with the kind and the Nushell filter chosen now, in relevance order; a Smart Folder's search comes back with its own kind and filter."),
 ('da6a5b9', "Near misses with typos then stayed in the list beside the real matches, so `music` also listed `basic.txt`, and they showed with **Allow typos** off too.",
             "Matches that fff-search only made by leaving letters out then stayed in the list beside the real matches, so `music` also listed `basic.txt`."),
 ('323f173', "Show in Enclosing Folder, the sidebar, the path bar, Go to Folder or Enclosing Folder, keeps the search in the window's history",
             "Show in Enclosing Folder, a folder or place clicked in the sidebar or the path bar, Go to Folder or Enclosing Folder, keeps the search in the window's history"),
 ('8dc26ad', "and when it has quoted words, which it didn't. Quoted words still follow **Letter case in searches**.",
             "and when it has quoted words. Quoted words follow **Letter case in searches**."),
 ('06d84e9', "Its first row now says whether typos are allowed, as **Allow typos** is set; it used to read \"typos allowed in Settings\" either way.",
             "Its first row says whether typos are allowed, as **Allow typos** is set."),
]
problems = []
for sha, old, new in FIXES:
    hit = [e for k in groups for e in groups[k] if e[0] == sha]
    if len(hit) != 1 or hit[0][1].count(old) != 1:
        problems.append(f'{sha}: expected the sentence once, found {hit[0][1].count(old) if hit else "no entry"}: {old[:70]}')
        continue
    hit[0][1] = hit[0][1].replace(old, new)

text = open(path).read()
m = re.search(r'(## \[Unreleased\]\n)(.*?)(?=^## \[)', text, re.S | re.M)
assert m, 'no [Unreleased] section'
body = m.group(2)
have = {}
for kind in ('Added', 'Changed', 'Fixed'):
    s = re.search(rf'### {kind}\n\n(.*?)(?=^### |\Z)', body, re.S | re.M)
    have[kind] = [b.strip() for b in re.split(r'\n(?=- )', s.group(1).strip())] if s and s.group(1).strip() else []
# Upstream's entry on quoted words, corrected.
UP_OLD = "Unquoted words still match fuzzily across the whole path, so `taxexemptionc` can find a file whose folders spell it out between them"
UP_NEW = "Unquoted words still match fuzzily along the path inside the search location, or inside the folder when only one is searched, so `taxexemptionc` also finds `tax/exemption/coding/Handler.java`, whose folders spell it out in order"
n = sum(b.count(UP_OLD) for b in have['Added'])
if n != 1:
    problems.append(f"upstream's quoted-words sentence: expected once under Added, found {n}")
have['Added'] = [b.replace(UP_OLD, UP_NEW) for b in have['Added']]
out = ['## [Unreleased]\n']
for kind in ('Added', 'Changed', 'Fixed'):
    bullets = have[kind] + ['- ' + t for _, t in groups[kind]]
    if bullets:
        out.append(f'### {kind}\n')
        out.append('\n'.join(bullets) + '\n')
new_body = '\n'.join(out) + '\n'
for bad in ('off too', "which it didn't", 'typos allowed in Settings'):
    if bad in new_body:
        problems.append(f'still contains: {bad}')
print(f"entries: Added {len(groups['Added'])}, Changed {len(groups['Changed'])}, Fixed {len(groups['Fixed'])}; upstream's own: "
      + ', '.join(f'{k} {len(v)}' for k, v in have.items()))
print('commits without an entry:', '; '.join(skipped) or 'none')
for p in problems:
    print('PROBLEM:', p)
if problems:
    sys.exit(1)
if not check:
    open(path, 'w').write(text[:m.start()] + new_body + text[m.end():])
    print('written:', path)
else:
    print('check only: nothing written;', len(new_body), 'characters')
