#!/usr/bin/env python3
"""Write later.md: every improvement that is written up but not built, highest value first."""
import glob, json, os, sys
root = '/home/user/polish-tools'
out = sys.argv[1] if len(sys.argv) > 1 else f'{root}/later.md'
done = {os.path.basename(p)[:-5] for p in glob.glob(f'{root}/done/*.json')}
claims = {os.path.basename(p): open(p).read().strip() for p in glob.glob(f'{root}/claims/*')}
# What is being built right now is not for later.
now = {l.split('#')[0].strip() for l in open(f'{root}/queue/all.txt') if l.split('#')[0].strip()}
items = []
for p in glob.glob(f'{root}/briefs/*.json'):
    b = json.load(open(p))
    if b['id'] in done or b['id'] in claims or b['id'] in now:
        continue
    items.append(b)
items.sort(key=lambda b: (-(b['verifier'].get('user_value') or 0), b['id']))
L = ['# Saved for later', '',
     f'{len(items)} improvements that are written up and not built. Each was found by reading the code; those marked',
     '"verified" were checked against the code by a second reviewer before any work started, and those marked',
     '"found in review" were found while another change was being reviewed and have not had that second check.',
     'Highest value first. The full write-up of each, with the verifier\'s plan, is `briefs/<id>.json`.', '']
L += ['| Value | Size | Area | Item | |', '| --- | --- | --- | --- | --- |']
for b in items:
    v = b['verifier']
    src = 'found in review' if any(str(r).startswith('review of') for r in b.get('raised_by', [])) else 'verified'
    L.append(f"| {v.get('user_value')} | {v.get('effort')} | {b.get('area','')} | [{b['title']}](#{b['id']}) | {src} |")
L.append('')
for b in items:
    v = b['verifier']
    L += [f"## {b['id']}", '', f"**{b['title']}**", '',
          f"Value {v.get('user_value')} of 5, size {v.get('effort')}, {b.get('area','')}, {b.get('kind','')}.", '',
          '**Problem.** ' + b.get('problem', '').strip(), '',
          '**Proposal.** ' + b.get('proposal', '').strip(), '']
    notes = (v.get('implementation_notes') or '').strip()
    if notes and not notes.startswith('See the proposal'):
        L += ['**Checked plan.** ' + notes, '']
    tp = (v.get('test_plan') or '').strip()
    if tp.startswith('See original'):
        tp = (b.get('original_test_plan') or '').strip()
    if tp:
        L += ['**Tests.** ' + tp, '']
    rk = (v.get('risks') or '').strip()
    if rk.startswith('See original'):
        rk = (b.get('original_risks') or '').strip()
    if rk:
        L += ['**Risks.** ' + rk, '']
    if b.get('orchestrator_notes'):
        L += ['**Notes from later reviews.** ' + b['orchestrator_notes'].strip(), '']
open(out, 'w').write('\n'.join(L) + '\n')
print(f'{len(items)} items -> {out}')
