#!/usr/bin/env python3
"""Collect the "Changelog (Kind): ..." paragraphs from the trunk's commits.

    mkchangelog.py [<base>] [<tip>]      # default: 66c919b..search-ui-polish

Prints the entries grouped as Added / Changed / Fixed, oldest first, as
CHANGELOG.md bullets, and lists commits that carry no entry.
"""
import re, subprocess, sys
base = sys.argv[1] if len(sys.argv) > 1 else '66c919b'
tip = sys.argv[2] if len(sys.argv) > 2 else 'search-ui-polish'
log = subprocess.run(['git', '-C', '/home/user/fffinder', 'log', '--reverse', '--format=%H%x00%s%x00%B%x01', f'{base}..{tip}'],
                     capture_output=True, text=True, check=True).stdout
groups = {'Added': [], 'Changed': [], 'Fixed': []}
missing = []
for rec in log.split('\x01'):
    rec = rec.strip('\n')
    if not rec:
        continue
    sha, subject, body = rec.split('\x00', 2)
    m = re.search(r'^Changelog \((Added|Changed|Fixed)\):\s*(.*?)(?=^\s*$|^Co-Authored-By:|^Claude-Session:|\Z)', body, re.S | re.M)
    if not m:
        missing.append((sha[:9], subject))
        continue
    text = ' '.join(line.strip() for line in m.group(2).strip().splitlines())
    groups[m.group(1)].append((sha[:9], subject, text))
for kind in ('Added', 'Changed', 'Fixed'):
    if not groups[kind]:
        continue
    print(f'### {kind}\n')
    for sha, subject, text in groups[kind]:
        print(f'- {text}\n')
print('<!-- commits without a changelog entry: ' + ('; '.join(f'{s} {t}' for s, t in missing) or 'none') + ' -->')
print(f'<!-- entries: ' + ', '.join(f'{k} {len(v)}' for k, v in groups.items()) + ' -->')
