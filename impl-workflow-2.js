export const meta = {
  name: 'fffinder-polish-queue',
  description: 'Work through the shared queue of search and UI improvements in worktree lanes; each change is implemented, reviewed, fixed, and integrated into search-ui-polish',
  phases: [
    { title: 'Implement', detail: 'an implementer takes the next improvement from the queue, in its lane worktree' },
    { title: 'Review', detail: 'a separate reviewer tries to break it, fixes it, and puts it on the trunk' },
  ],
}

const LANES = args.lanes
const MAX_TURNS = args.max_turns || 40

const IMPL_SCHEMA = {
  type: 'object',
  properties: {
    id: { type: 'string', description: 'the improvement id next.sh gave you, or empty when it printed NONE' },
    status: { type: 'string', enum: ['committed', 'skipped', 'failed', 'already-integrated', 'none-left'] },
    commit: { type: 'string', description: 'the lane commit hash, or empty' },
    summary: { type: 'string', description: 'what you changed, in a few sentences' },
    deviations: { type: 'string', description: 'where you departed from the plan, and why' },
    left_out: { type: 'string', description: 'parts of the brief you did not do, and why' },
    behaviour_changes: { type: 'string', description: 'every user-visible change in behaviour' },
    tests_added: { type: 'array', items: { type: 'string' } },
    screenshots: { type: 'array', items: { type: 'string' } },
    check_ok: { type: 'boolean', description: 'check.sh printed CHECK OK on the committed tree' },
    notes_for_reviewer: { type: 'string', description: 'what you are least sure of' },
  },
  required: ['id', 'status', 'commit', 'summary', 'deviations', 'left_out', 'behaviour_changes', 'tests_added', 'check_ok', 'notes_for_reviewer'],
}

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    outcome: { type: 'string', enum: ['integrated', 'rejected', 'failed'] },
    trunk_commit: { type: 'string', description: 'the commit on search-ui-polish, or empty' },
    issues_found: { type: 'array', items: { type: 'string' } },
    fixes_made: { type: 'array', items: { type: 'string' } },
    tests_fail_without_change: { type: 'string', enum: ['yes', 'no', 'partly', 'not-checked'] },
    check_ok: { type: 'boolean' },
    rebased_onto_new_trunk: { type: 'boolean' },
    pushed_origin: { type: 'boolean' },
    behaviour_changes: { type: 'string' },
    notes: { type: 'string', description: 'anything the owner should know, including doubts that remain' },
  },
  required: ['outcome', 'trunk_commit', 'issues_found', 'fixes_made', 'tests_fail_without_change', 'check_ok', 'rebased_onto_new_trunk', 'pushed_origin', 'behaviour_changes', 'notes'],
}

const shell = lane => `Your shell starts every command in /home/user/fffinder, which is a locked copy and the WRONG place. Begin every Bash command with \`cd ${lane.dir} &&\` and give the file tools absolute paths under ${lane.dir}.`

const implPrompt = (lane, turn, tried, again) => `You are the IMPLEMENTER of one improvement to fffinder, a Finder-style file browser written in Rust.

Your lane: ${lane.dir} (branch ${lane.branch}). This is turn ${turn} of the lane.
Guide: /home/user/polish-tools/GUIDE.md
Your improvement: not named here. You take the next one from the shared queue, as step 1 of the guide's section "If you are the implementer" describes for a prompt that names no id. The command is:

    /home/user/polish-tools/next.sh ${lane.branch}${tried.length ? ' --not ' + tried.join(',') : ''}

Run it once. It prints the id of the improvement that is now yours (brief: /home/user/polish-tools/briefs/<id>.json), or a line starting NONE, in which case return status "none-left" with an empty id and stop.${tried.length ? `\nThis lane has already finished with: ${tried.join(', ')}.` : ''}

${shell(lane)}

Read the guide in full first, then take your improvement, read its brief, and follow the guide's section "If you are the implementer" step by step. The guide is binding: the house style, the test requirement, the screenshots for anything visible, the documentation, the commit format, and the rule against editing CHANGELOG.md.

What matters most, in order: the change is correct and does not regress anything; it has a test that fails without it; it reads like the code around it. A smaller change that is certainly right beats a larger one that is probably right. If the brief turns out to be wrong about the code, say so and return "skipped" rather than forcing it.

When you are done, return the structured result, with the improvement's id in it. Be exact and honest: the reviewer will check every claim.${again ? '\n\n(An earlier attempt at this turn ended without a result. Run next.sh exactly as above: it gives you the improvement that attempt took. Then check the state of the lane, as the guide describes, and carry on from whatever it left.)' : ''}`

const reviewPrompt = (id, lane, impl, again) => `You are the REVIEWER of one improvement to fffinder, a Finder-style file browser written in Rust. A different agent implemented it a moment ago in your lane and committed it there. It is not on the trunk yet; putting it there, once it is right, is your job.

Improvement id: ${id}
Your lane: ${lane.dir} (branch ${lane.branch})
Brief: /home/user/polish-tools/briefs/${id}.json
Guide: /home/user/polish-tools/GUIDE.md

${shell(lane)}

The implementer's report follows. Treat it as claims to check, not as facts:
${JSON.stringify(impl, null, 1)}

Read the guide in full first, then the brief, then follow the guide's section "If you are the reviewer" step by step. Your default stance is that the change has a bug you have not found yet: read the changed functions whole, follow the callers, prove the new tests fail without the change, and look at the window yourself for anything visible. Fix what you find and fold it into the commit. Reject only what cannot reasonably be fixed.

Do not stop at "looks fine". Before you integrate, name to yourself the three most likely ways this change could hurt a user, and check each against the code.

When the change is on the trunk and the done record is written, return the structured result.${again ? '\n\n(An earlier attempt at this step ended without a result. Check the state of the lane first, as the guide describes: a rebase may be in progress, or the change may already be on the trunk.)' : ''}`

const results = []

async function runLane(lane) {
  const tried = []
  let empty = 0
  for (let turn = 1; turn <= MAX_TURNS; turn++) {
    let impl = await agent(implPrompt(lane, turn, tried, false), { label: `impl:${lane.key}:${turn}`, phase: 'Implement', schema: IMPL_SCHEMA })
    if (!impl) {
      impl = await agent(implPrompt(lane, turn, tried, true), { label: `impl:${lane.key}:${turn}:again`, phase: 'Implement', schema: IMPL_SCHEMA })
    }
    if (impl && impl.status === 'none-left') {
      log(`${lane.key}: nothing left in the queue for this lane (${impl.summary || ''})`)
      break
    }
    if (!impl || !impl.id) {
      // Two turns in a row without a usable answer: something is wrong with the lane, so stop it.
      empty += 1
      results.push({ id: '', lane: lane.key, impl, review: null })
      if (empty >= 2) {
        log(`${lane.key}: stopped after two turns without a result`)
        break
      }
      continue
    }
    empty = 0
    const id = impl.id
    tried.push(id)
    let review = null
    if (impl.status === 'committed') {
      review = await agent(reviewPrompt(id, lane, impl, false), { label: `review:${id}`, phase: 'Review', schema: REVIEW_SCHEMA })
      if (!review) {
        review = await agent(reviewPrompt(id, lane, impl, true), { label: `review:${id}:again`, phase: 'Review', schema: REVIEW_SCHEMA })
      }
    }
    results.push({ id, lane: lane.key, impl, review })
    const state = review ? review.outcome : impl.status
    log(`${lane.key} ${id}: ${state}${review && review.trunk_commit ? ' ' + review.trunk_commit.slice(0, 9) : ''}`)
  }
}

await parallel(LANES.map(lane => () => runLane(lane)))

const count = s => results.filter(r => (r.review ? r.review.outcome : (r.impl ? r.impl.status : 'no result')) === s).length
log(`integrated ${count('integrated')}, rejected ${count('rejected')}, skipped ${count('skipped')}, failed ${count('failed')}`)
return results
