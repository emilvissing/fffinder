export const meta = {
  name: 'fffinder-branch-review',
  description: 'Whole-branch review of search-ui-polish: one reviewer per lens reads the combined changes, then a skeptic tries to refute every finding against the code',
  phases: [
    { title: 'Find', detail: 'a reviewer reads the whole branch through one lens for faults that single-change reviews miss' },
    { title: 'Refute', detail: 'a second reviewer tries to disprove each finding against the code' },
  ],
}

const SNAP = args.snapshot
const TIP = args.tip
const LENSES = args.lenses

const FIND_SCHEMA = {
  type: 'object',
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'a short kebab-case name for the finding' },
          title: { type: 'string' },
          scenario: { type: 'string', description: 'exact steps or inputs, what happens, what should happen' },
          code_path: { type: 'string', description: 'the path through the code, with file:line at the snapshot' },
          tests_gap: { type: 'string', description: 'why the existing tests do not catch it' },
          improvements: { type: 'array', items: { type: 'string' }, description: 'ids of the improvements involved' },
          severity: { type: 'integer', description: '1 cosmetic .. 5 data loss, crash, or dead keyboard in a common flow' },
          confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
          fix: { type: 'string', description: 'the smallest fix you would make' },
          test_idea: { type: 'string', description: 'a test that would fail today' },
        },
        required: ['id', 'title', 'scenario', 'code_path', 'tests_gap', 'improvements', 'severity', 'confidence', 'fix', 'test_idea'],
      },
    },
    read: { type: 'string', description: 'what you read' },
    not_read: { type: 'string', description: 'what you did not get to' },
  },
  required: ['findings', 'read', 'not_read'],
}

const VERDICT_SCHEMA = {
  type: 'object',
  properties: {
    verdicts: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          verdict: { type: 'string', enum: ['confirmed', 'refuted', 'uncertain'] },
          evidence: { type: 'string', description: 'the lines that prove or prevent it; for uncertain, what would settle it' },
          corrected_scenario: { type: 'string', description: 'the scenario as it really is, if the finder was imprecise; else empty' },
          severity: { type: 'integer' },
          already_known: { type: 'boolean', description: 'recorded in a done record or final-pass-notes.md as known and left' },
        },
        required: ['id', 'verdict', 'evidence', 'corrected_scenario', 'severity', 'already_known'],
      },
    },
    notes: { type: 'string' },
  },
  required: ['verdicts', 'notes'],
}

const COMMON = `fffinder is a Finder-style file browser written in Rust: a GPUI Kit window (src/) on top of a search engine crate (crates/fffinder-core, which wraps the fff-search crate), with a Nushell module (nu/fff.nu), a control server (src/server) and a terminal mode (feature "term", src/term).

The branch search-ui-polish holds 35 commits on top of upstream's 66c919b: one screenshot-tool commit and 34 improvements to search and to the UI, one commit each. See them with \`git -C ${SNAP} log --oneline 66c919b..HEAD\`. Each improvement was written by one agent and then reviewed, fixed and landed by another, against the branch as it stood at that moment, and each has tests that pass. What nobody has done is look at them together.

Where to read:
- The code, read-only: ${SNAP} (a snapshot at ${TIP}). Use git there (log, show, diff, blame, grep) and rg.
- A record per improvement: /home/user/polish-tools/done/<id>.json, with summary, behaviour_changes, left_out, tests, review_fixes and notes. The notes end with "Found, not fixed" and often "Doubts that remain".
- /home/user/polish-tools/final-pass-notes.md: what is already known and deliberately left. Do not report those again unless you can show they are worse than recorded.
- Sources of the dependencies: ~/.cargo/registry/src/*/ (gpui-base-0.8.1, gpui-component-0.8.1, gpui-pre-0.3.10, fff-search-0.11.0).

Rules, all binding:
- Read only. Do not edit anything in ${SNAP}. Do not run cargo or any other build or test: the disk cannot hold another build, and other agents are building.
- Do not touch /home/user/fffinder, /home/user/fffinder-lane-* or /home/user/fast/fffinder-lane-*: other agents are working there, and their trees are ahead of or behind the snapshot.
- Small scripts that only read are fine. Scratch files go in a folder of your own under /tmp/claude-0/-home-user-fffinder/77fc8bed-98cd-53c5-aa96-589871b38301/scratchpad/, which you delete at the end.`

const findPrompt = lens => `You are reviewing a whole branch through one lens: ${lens.title}.

${COMMON}

Your lens. ${lens.focus}

What counts as a finding: a concrete way the combined branch misbehaves for a user or for a developer. A wrong result, lost input, a keyboard that goes dead, state that is stale or contradicts itself, a crash (an index out of bounds, a slice inside a character, an unwrap on nothing), a hang or steady background work, a message or the manual saying something the code does not do, a test that cannot fail or depends on timing, something that breaks only on macOS or Windows. Look hardest where two changes meet: one sets a flag the other never clears, one's guard assumes rows the other now keeps, two listeners answer one event, one's test pins what the other changed. A fault inside a single change counts too if its review missed it. Style, naming, refactors and "could be cleaner" do not count.

How to work. Start from the list of commits and the records of the improvements in your lens, then read the functions they changed whole, as they are in the snapshot, and follow callers and state across changes. Ask of each piece of state: who sets it, who clears it, and which newer change bypasses one of the two. Ask of each key and each click: which handlers see it now, in what order.

For each finding give the exact steps or inputs, what happens and what should happen; the path through the code with file:line; why the existing tests do not catch it; the improvements involved; a severity from 1 to 5 (5: data loss, a crash, or a dead keyboard in a common flow; 3: visibly wrong behaviour in an ordinary flow; 1: cosmetic); your confidence; the smallest fix; and a test that would fail today.

Be exact and honest. A second reviewer will try to refute every finding against the code, and a finding you cannot trace to lines is worth nothing. Few findings, or none, is a fine answer if that is what the code says. Aim for the ones that matter: at most twelve, most severe first. Say what you read and what you did not get to.`

const refutePrompt = (lens, found) => `You are the SKEPTIC in a whole-branch review. Another reviewer read the branch through one lens (${lens.title}) and reported the findings below. Assume each one is wrong until the code proves it.

${COMMON}

For each finding: open the files in the snapshot and follow the path it claims, line by line. Look for the guard, the reset or the early return the finder missed. Look for a test that already pins the behaviour (tests/*.rs and the unit tests in src and crates). Check whether a done record or final-pass-notes.md already lists it as known and left on purpose.

Give each a verdict:
- confirmed: you traced the path yourself and it misbehaves. Quote the lines. If the finder's scenario was imprecise, give the scenario as it really is.
- refuted: say exactly which lines prevent it.
- uncertain: say what would settle it, as a concrete test to run.
Rate the severity again by the same scale (5: data loss, a crash, or a dead keyboard in a common flow; 3: visibly wrong behaviour in an ordinary flow; 1: cosmetic). Do not confirm out of politeness, and do not refute what you have not traced.

The findings:
${JSON.stringify(found.findings, null, 1)}`

const results = await pipeline(
  LENSES,
  lens => agent(findPrompt(lens), { label: `find:${lens.key}`, phase: 'Find', schema: FIND_SCHEMA }),
  (found, lens) => {
    if (!found || !found.findings || found.findings.length === 0) {
      return { lens: lens.key, found, verdicts: null }
    }
    return agent(refutePrompt(lens, found), { label: `refute:${lens.key}`, phase: 'Refute', schema: VERDICT_SCHEMA })
      .then(v => ({ lens: lens.key, found, verdicts: v }))
  },
)

const all = results.filter(Boolean)
const count = v => all.reduce((n, r) => n + ((r.verdicts && r.verdicts.verdicts) || []).filter(x => x.verdict === v).length, 0)
log(`findings ${all.reduce((n, r) => n + ((r.found && r.found.findings) || []).length, 0)}: confirmed ${count('confirmed')}, uncertain ${count('uncertain')}, refuted ${count('refuted')}`)
return all
