export const meta = {
  name: 'fffinder-changelog-check',
  description: 'Check the assembled changelog of search-ui-polish against the code at the tip: one reader looks for sentences that later commits made untrue, then a skeptic tries to refute each correction',
  phases: [
    { title: 'Find', detail: 'a reader checks the assembled [Unreleased] section against the code and the later commits' },
    { title: 'Refute', detail: 'a skeptic tries to disprove each proposed correction against the code' },
  ],
}

const SNAP = args.snapshot
const TIP = args.tip
const TEXT = args.text
const PENDING = args.pending

const FIND_SCHEMA = {
  type: 'object',
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'a short kebab-case name' },
          entry: { type: 'string', description: 'the first eight words of the changelog entry the sentence is in, and the heading it is under (Added, Changed, Fixed)' },
          sentence: { type: 'string', description: 'the sentence exactly as it stands in the text, whole' },
          kind: { type: 'string', enum: ['untrue-at-tip', 'contradiction', 'duplicate', 'wrong-name', 'never-shipped', 'unclear'] },
          problem: { type: 'string', description: 'what is not so, and what is true at the tip' },
          evidence: { type: 'string', description: 'file:line in the snapshot, and the commit that changed it if a later commit did' },
          corrected: { type: 'string', description: 'the sentence as it should read, changing as little as possible; "REMOVE" if it should go' },
          severity: { type: 'integer', description: '3: a reader would expect behaviour the app does not have; 2: imprecise in a way that matters to someone trying it; 1: wording' },
          confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
        },
        required: ['id', 'entry', 'sentence', 'kind', 'problem', 'evidence', 'corrected', 'severity', 'confidence'],
      },
    },
    read: { type: 'string', description: 'what you checked, entry by entry in a few words' },
    not_read: { type: 'string', description: 'which entries or claims you did not get to check against the code' },
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
          evidence: { type: 'string', description: 'the lines that prove the sentence wrong, or the lines that show it is right as it stands' },
          corrected: { type: 'string', description: 'for a confirmed finding, the sentence as it should read (the finder\'s, or yours if theirs is itself inexact); else empty' },
        },
        required: ['id', 'verdict', 'evidence', 'corrected'],
      },
    },
    notes: { type: 'string' },
  },
  required: ['verdicts', 'notes'],
}

const COMMON = `fffinder is a Finder-style file browser written in Rust: a GPUI Kit window (src/) on top of a search engine crate (crates/fffinder-core, which wraps the fff-search crate), with a Nushell module (nu/fff.nu), a control server (src/server) and a terminal mode (feature "term", src/term).

The branch search-ui-polish holds ${args.commits} commits on top of upstream's 66c919b: one screenshot-tool commit and ${args.commits - 1} improvements to search and to the UI, one commit each (\`git -C ${SNAP} log --oneline 66c919b..HEAD\`). Each commit message ends with a paragraph "Changelog (Added|Changed|Fixed): ...", which was true when that commit landed. A script has now put those paragraphs into CHANGELOG.md's [Unreleased] section, after the three entries upstream already had there. That text is about to be committed as the branch's changelog. Nobody has read it as one text since nine commits ago.

Where to read:
- The assembled section by itself: ${TEXT}. The same text is in ${SNAP}/CHANGELOG.md (uncommitted there).
- The code, read-only: ${SNAP}, a snapshot at ${TIP}, the branch's tip. Use git there (log, show, diff, blame, grep) and rg. docs/manual.md and README.md are the user documentation.
- /home/user/polish-tools/changelog-fixes.md: eight corrections an earlier review found; they are already applied to the text. Do not report them again.
- A record per improvement: /home/user/polish-tools/done/<id>.json (summary, behaviour_changes, left_out, review_fixes, notes). The commit of each is in its "commit" field.
- One more commit is still under review and will be added to the branch after you finish: ${PENDING} (a patch with its message). Its Changelog paragraph will be added to the text as one more Fixed entry. It changes when near misses of a misspelled word are listed.
- /home/user/polish-tools/GUIDE.md has the house style for wording, under its section on the changelog and documentation.

The nine commits that landed after the text was last read as a whole, oldest first: e2e1672 (new-item-in-hidden-row), ac62eaf (tab-drop-and-close), 1acaff3 (thumbnail-memory), ddefe96 (stale-rows-on-place-change), ffb02de (fix-own-small-faults; it has no entry, because it mends faults in changes that no release has had), 5e68c44 (fix-filter-reruns-and-fresh-rows), a085120 (fix-here-folder-spelling), 4efa254 (fix-rename-press-selection), 362de8f (accented-names-search).

Rules, all binding:
- Read only. Do not edit anything in ${SNAP} or anywhere under /home/user/fffinder*, /home/user/fast or /home/user/polish-tools. Do not run cargo or any other build or test.
- Small scripts that only read are fine. Scratch files go in a folder of your own under <scratch>/, which you delete at the end.`

const findPrompt = `You are checking a changelog before it is committed.

${COMMON}

What to find: every sentence of the assembled section that is not true of the app as it is at the tip, or that a reader would be misled by. In order of importance:

1. Sentences that a later commit made untrue. Go through the nine later commits and the pending one, one at a time: read the diff and the record, list what each changed for a user, and then look through the WHOLE text, every entry, for a sentence that describes the behaviour as it was before that commit. Examples of what to look for: an entry that says near misses are listed only when nothing else matches (the pending commit changes when they are listed); an entry that says a word with an accent or in another alphabet is treated specially by the typo check (362de8f removed that); an entry about the buttons under the file area's message, the Sort menu or Go to Folder whose behaviour ffb02de mended; an entry about filters, Recents or results following saved files (5e68c44); an entry about --here, Search Only in or a folder's path (a085120); an entry about what is selected after a rename (4efa254); an entry that says what a tab, a drop or a thumbnail does (ac62eaf, 1acaff3, ddefe96, e2e1672).
2. The entries of the nine later commits themselves: check each claim in them against the code at the tip, since the commits after them may have changed it, and since two of them were rebased over each other.
3. The text read as one text: two entries that contradict each other; the same change told twice; an entry under Fixed that describes a fault in something this same section adds or changes (a fault no release ever had is not a fix a reader can see; it should be folded into the entry that adds the thing, or go); a key, a menu item, a button label or a setting that the code names otherwise (grep for each quoted label and each key in src/ and docs/manual.md); an example that would not give what the entry says.

How to work. Check claims against the code, not against the commit messages: the messages are where the text came from. For each finding quote the sentence whole, say what is true at the tip with file:line, and give the sentence as it should read, changing as little as possible and keeping the entry's voice (plain sentences, \`code\` for what is typed, **bold** for labels in the window). If a sentence should simply go, say REMOVE.

Be exact. A second reader will try to refute every finding against the code, and a finding without lines is worth nothing. A wording you merely would have chosen differently is not a finding. None is a fine answer for an entry that is true. Say entry by entry what you checked, and which entries or claims you did not get to check against the code.`

const refutePrompt = found => `You are the SKEPTIC. Another reader checked a changelog against the code and proposes the corrections below. Assume each sentence is right as it stands until the code proves it wrong.

${COMMON}

For each finding: read the sentence in its entry in ${TEXT}, then open the code in the snapshot and check what the app does at the tip. A sentence is wrong only if the app does not do what it says, or if it contradicts another entry of the same text, or if it names a key, label or setting the code names otherwise. Check the proposed sentence as hard as the original: a correction that is itself inexact is worse than none.

Verdicts:
- confirmed: the sentence is wrong as it stands; quote the lines; give the sentence as it should read (the finder's, or your own if theirs is inexact), changing as little as possible.
- refuted: the sentence is right as it stands; quote the lines that show it.
- uncertain: say what would settle it.
Do not confirm out of politeness, and do not refute what you have not traced.

The findings:
${JSON.stringify(found.findings, null, 1)}`

phase('Find')
const found = await agent(findPrompt, { label: 'find:changelog', phase: 'Find', schema: FIND_SCHEMA })
if (!found || !found.findings || found.findings.length === 0) {
  log('no findings')
  return { found, verdicts: null }
}
phase('Refute')
const verdicts = await agent(refutePrompt(found), { label: 'refute:changelog', phase: 'Refute', schema: VERDICT_SCHEMA })
const n = v => ((verdicts && verdicts.verdicts) || []).filter(x => x.verdict === v).length
log(`findings ${found.findings.length}: confirmed ${n('confirmed')}, uncertain ${n('uncertain')}, refuted ${n('refuted')}`)
return { found, verdicts }
