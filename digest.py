#!/usr/bin/env python3
"""A short reading of done records: what changed, what the reviewer fixed, and what it flagged.

    digest.py <id>...      # or no ids: every record not yet digested (marks them in digested.txt)
"""
import json, os, re, sys
root = '/home/user/polish-tools'
seen_path = f'{root}/digested.txt'
seen = set(open(seen_path).read().split()) if os.path.exists(seen_path) else set()
ids = sys.argv[1:] or sorted(f[:-5] for f in os.listdir(f'{root}/done') if f[:-5] not in seen)
for cid in ids:
    d = json.load(open(f'{root}/done/{cid}.json'))
    notes = d.get('notes') or ''
    if not isinstance(notes, str):
        notes = json.dumps(notes, ensure_ascii=False)
    fixes = d.get('review_fixes') or []
    if isinstance(fixes, str):
        fixes = [fixes]
    print(f"===== {cid}  {str(d.get('commit'))[:9]}  [{d.get('changelog_kind')}]  tests:{len(d.get('tests') or [])}  review fixes:{len(fixes)}")
    print('WHAT:', str(d.get('summary'))[:500].replace('\n', ' '))
    for f in fixes[:4]:
        print('  FIX:', str(f)[:260].replace('\n', ' '))
    for key, width in (('Found, not fixed', 1400), ('doubt for the owner', 700), ('Doubts that remain', 700)):
        m = re.search(key, notes, re.I)
        if m:
            print(f'  [{key}]', notes[m.start():m.start() + width].replace('\n', ' '))
    seen.add(cid)
    print()
open(seen_path, 'w').write('\n'.join(sorted(seen)) + '\n')
for f in sorted(os.listdir(f'{root}/closed')):
    c = json.load(open(f'{root}/closed/{f}'))
    print(f"CLOSED {c.get('id')} [{c.get('status')}] lane {c.get('lane')}: {str(c.get('why'))[:600]}")
