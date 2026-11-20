#!/usr/bin/env python3
"""Summarise a whole-branch review workflow's journal: review-digest.py <run id> [detail]"""
import json, sys, os
run = sys.argv[1]
detail = len(sys.argv) > 2
j = f'/root/.claude/projects/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/subagents/workflows/{run}/journal.jsonl'
finds, verds = {}, {}
for line in open(j):
    try:
        e = json.loads(line)
    except Exception:
        continue
    if e.get('type') != 'result':
        continue
    r = e.get('result') or e.get('value') or {}
    if isinstance(r, str):
        try: r = json.loads(r)
        except Exception: continue
    if 'findings' in r:
        for f in r['findings']: finds[f['id']] = f
    if 'verdicts' in r:
        for v in r['verdicts']: verds[v['id']] = v
out = []
for i, f in finds.items():
    v = verds.get(i, {})
    out.append((v.get('verdict', 'no verdict'), -(v.get('severity') or f.get('severity') or 0), i, f, v))
out.sort(key=lambda x: ({'confirmed': 0, 'uncertain': 1, 'no verdict': 2, 'refuted': 3}.get(x[0], 4), x[1]))
for verdict, sev, i, f, v in out:
    print(f"[{verdict:9s}] sev {f.get('severity')}->{v.get('severity','?')} known={v.get('already_known','?')} {i}: {f['title'][:150]}")
    if detail and verdict in ('confirmed', 'uncertain'):
        print('    improvements:', ', '.join(f.get('improvements', [])))
        print('    scenario:', (v.get('corrected_scenario') or f['scenario'])[:900])
        print('    evidence:', v.get('evidence', '')[:700])
        print('    fix:', f.get('fix', '')[:500])
        print('    test:', f.get('test_idea', '')[:400])
        print()
print(len(finds), 'findings;', sum(1 for o in out if o[0] == 'confirmed'), 'confirmed,', sum(1 for o in out if o[0] == 'uncertain'), 'uncertain,', sum(1 for o in out if o[0] == 'refuted'), 'refuted')
