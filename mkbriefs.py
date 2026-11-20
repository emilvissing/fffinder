#!/usr/bin/env python3
"""Write a brief for every verified candidate that has none yet, and refresh verdicts.json.

Reads the review workflow's journal for the verifiers' results. Paths that
point into the frozen main checkout are rewritten relative to the repository.
"""
import json, os, sys
ROOT = '/home/user/polish-tools'
J = '/root/.claude/projects/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/subagents/workflows/wf_18cbedd6-a2e/journal.jsonl'

def rel(v):
    if isinstance(v, str):
        return v.replace('/home/user/fffinder/', '')
    if isinstance(v, list):
        return [rel(x) for x in v]
    if isinstance(v, dict):
        return {k: rel(x) for k, x in v.items()}
    return v

key2label, res = {}, {}
for line in open(J):
    r = json.loads(line)
    if r.get('type') == 'started':
        key2label[r['key']] = r.get('label')
    elif r.get('type') == 'result':
        res[key2label.get(r.get('key'))] = r.get('result')

merged = json.load(open(f'{ROOT}/merged.json'))['merged']
verdicts = json.load(open(f'{ROOT}/verdicts.json'))
notes = json.load(open(f'{ROOT}/orchestrator-notes.json')) if os.path.exists(f'{ROOT}/orchestrator-notes.json') else {}
wrote = []
for c in merged:
    cid = c['id']
    v = res.get('verify:' + cid)
    if not isinstance(v, dict):
        continue
    verdicts[cid] = v
    path = f'{ROOT}/briefs/{cid}.json'
    if os.path.exists(path):
        continue
    if not v.get('worth_doing') or v.get('already_handled') or v.get('verdict') == 'refuted':
        print('not briefed:', cid, v.get('verdict'), 'worth', v.get('worth_doing'), 'handled', v.get('already_handled'))
        continue
    brief = {
        'id': cid, 'title': c.get('title'), 'area': c.get('area'), 'kind': c.get('kind'),
        'problem': c.get('problem'), 'proposal': c.get('proposal'), 'files': c.get('files'),
        'original_test_plan': c.get('test_plan'), 'original_risks': c.get('risks'),
        'raised_by': c.get('sources'),
        'verifier': {k: v.get(k) for k in ['verdict', 'claim_check', 'implementation_notes', 'test_plan', 'touches', 'risks', 'user_value', 'effort', 'worth_doing', 'already_handled']},
    }
    if cid in notes:
        brief['orchestrator_notes'] = notes[cid]
    json.dump(rel(brief), open(path, 'w'), indent=1, ensure_ascii=False)
    wrote.append(cid)
json.dump(verdicts, open(f'{ROOT}/verdicts.json', 'w'), indent=1, ensure_ascii=False)
print('verdicts:', len(verdicts), 'briefs written now:', wrote)
