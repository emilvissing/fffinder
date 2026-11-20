#!/usr/bin/env python3
"""Build a fix brief from whole-branch review findings.
mkfix.py <brief id> <title> <value> <effort> <area> <finding id>[,<finding id>...] [--note TEXT] [--unreleased]"""
import json, glob, sys
root = '/home/user/polish-tools'
args = sys.argv[1:]
note = ''; unreleased = False
if '--unreleased' in args:
    unreleased = True; args.remove('--unreleased')
if '--note' in args:
    i = args.index('--note'); note = args[i + 1]; del args[i:i + 2]
bid, title, value, effort, area, fids = args[:6]
finds, verds = {}, {}
for p in glob.glob(f'{root}/review/*.json'):
    for r in json.load(open(p)):
        if not isinstance(r, dict): continue
        for f in r.get('findings', []): finds[f['id']] = f
        for v in r.get('verdicts', []): verds[v['id']] = v
parts_problem, parts_fix, parts_test, parts_gap, imps = [], [], [], [], []
for fid in fids.split(','):
    f = finds[fid]; v = verds.get(fid, {})
    head = f"[{fid}] {f['title']}."
    parts_problem.append(f"{head} The finder's scenario: {f['scenario']} The code path: {f['code_path']} "
                         + (f"The second reviewer, who tried to refute it and confirmed it, corrects the scenario: {v['corrected_scenario']} " if v.get('corrected_scenario') else '')
                         + (f"Their evidence: {v['evidence']}" if v.get('evidence') else ''))
    parts_fix.append(f"[{fid}] {f['fix']}")
    parts_test.append(f"[{fid}] {f['test_idea']}")
    parts_gap.append(f"[{fid}] {f['tests_gap']}")
    imps += f.get('improvements', [])
imps = sorted(set(imps))
changelog_rule = ("This fault is in a change that is itself unreleased, so it gets no entry in CHANGELOG.md. In the commit message write the paragraph as `Changelog: none (fixes a fault in an unreleased change).` instead of `Changelog (Fixed): ...`, and in the done record set changelog_kind to \"Fixed\" and begin `changelog` with \"(No entry: a fault in an unreleased change.)\" followed by one sentence on what was wrong. "
                  if unreleased else
                  "This fault was in the app before this branch, at least in part: write a normal `Changelog (Fixed):` paragraph for what a user of the last release would notice. ")
b = {
 'id': bid, 'title': title, 'area': area, 'kind': 'bug',
 'problem': 'Found by the review of the whole branch (20 Nov), in which one reviewer read all the improvements together through one lens and a second reviewer tried to refute each finding against the code. Nothing was run in that review: every finding was traced by reading. ' + ' '.join(parts_problem),
 'proposal': 'Reproduce each fault first with a test that fails on the trunk. Then fix it. The finder\'s suggested fixes, as starting points and not checked plans: ' + ' '.join(parts_fix),
 'files': [], 'original_test_plan': 'The finder\'s test ideas: ' + ' '.join(parts_test) + ' Why the existing tests miss it: ' + ' '.join(parts_gap),
 'original_risks': 'This is a fix late in the branch: keep it to the fault. Every test on the trunk has to keep passing, and the improvements involved (' + ', '.join(imps) + ') have tests that pin the behaviour around the fault: read their done records in /home/user/polish-tools/done/ before changing the functions they changed.',
 'raised_by': ['whole-branch review'],
 'verifier': {'verdict': 'confirmed by reading', 'claim_check': 'A second reviewer traced the same path and confirmed it; neither ran anything. Reproduce before changing anything; if a test shows the fault is not there, write the closed record with the evidence and return skipped.',
              'implementation_notes': 'See the proposal.', 'test_plan': 'See original_test_plan.', 'touches': [], 'risks': 'See original_risks.',
              'user_value': int(value), 'effort': effort, 'worth_doing': True, 'already_handled': False},
 'orchestrator_notes': changelog_rule + 'The full findings, with the second reviewer\'s evidence line by line, are in /home/user/polish-tools/review/*.md and *.json (search for the finding ids in brackets). The review read the branch at 7e8df53; other changes have landed since, so read the functions as they are now. ' + note,
}
json.dump(b, open(f'{root}/briefs/{bid}.json', 'w'), indent=1, ensure_ascii=False)
print('wrote', bid, '| findings:', fids, '| problem chars:', len(b['problem']))
