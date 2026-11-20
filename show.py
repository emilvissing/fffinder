#!/usr/bin/env python3
"""Print the parts of a done record the orchestrator needs: show.py <id> [chars]"""
import json, sys, re
d = json.load(open(f'/home/user/polish-tools/done/{sys.argv[1]}.json'))
cap = int(sys.argv[2]) if len(sys.argv) > 2 else 520
print('==', d['id'], d['commit'][:7], '[', d.get('changelog_kind'), ']')
print('CHANGELOG:', d.get('changelog', ''))
for key in ('behaviour_changes', 'left_out', 'review_fixes'):
    v = d.get(key) or []
    if isinstance(v, str): v = [v]
    print(f'\n{key.upper()} ({len(v)}):')
    for x in v: print('  -', x[:cap] + ('…' if len(x) > cap else ''))
n = d.get('notes', '') or ''
m = re.search(r'(\d{3,4}) passed in (\d+) test binaries', n)
print('\nTESTS:', len(d.get('tests') or []), '| passed:', m.group(1) if m else '?', '| pushed:', 'NOT pushed' if re.search(r'NOT pushed|Not pushed to origin', n) else 'ok')
for head in ('Found, not fixed', 'Doubts that remain', 'CHECK ONCE ON A MAC', 'On a Mac check'):
    i = n.find(head)
    if i >= 0:
        j = n.find('\n\n', i)
        print(f'\n[{head}]', n[i:(j if j > 0 else i + 5000)][:5000])
