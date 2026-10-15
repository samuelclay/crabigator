---
name: flow-upstream
description: Review new upstream Flow changes, interview the user about ambiguous porting choices, and port selected changes into Crabigator's Rust scenes with reference-frame verification. Use for recurring Flow updates, upstream triage, or selected scene ports.
---

# Update the Flow port

Crabigator maintains a Rust port, not a vendored TypeScript plugin. Read `src/flow/FORK.md` first: it owns the file map, baseline, intentional differences, JavaScript numeric rules, and port log. Read the repository's `AGENTS.md` and current Rust implementation too; the log can lag the code.

Keep the user's scope: a review asks for recommendations and decisions, a skill-maintenance request updates this workflow, and a port request implements the selected work. Fetching upstream is not a Rust port. Do not automatically merge, push, release, or install either repository as part of reviewing it.

## Discover once, then inspect the relevant code

Run from the Crabigator root:

```sh
python3 .agents/skills/flow-upstream/scripts/inventory.py --flow-dir ../flow --fetch > /tmp/flow-upstream.json
```

The helper verifies `upstream` is `robdmac/flow`, fetches that remote only when requested, and reports exact SHAs, commits, changed paths, prior decisions, and the custom Flow fork's delta. It never changes branches, merges, writes fixtures, or advances the port log. With no decision ledger it uses *Synced to* in `FORK.md`. `--base` and `--head` can narrow a requested review; a narrowed review must not advance the global review cursor past omitted commits. Missing or rewritten history requires resolving the baseline, never guessing a new one.

Read the whole inventory, including changes outside `hooks/`. Inspect the patches for relevant commits, current TypeScript, corresponding Rust, and their tests. Commit titles and paths are discovery hints, not proof a change applies. A sound change can alter a visible animation's timing or consume random numbers. Adapter fixes can reveal an event-handling bug in Crabigator even though their UI plumbing is irrelevant.

Group related commits into changes a person can decide on: one new scene, waiting behavior, companion lifecycle, a rendering fix. Include later fixes and shared dependencies with the feature. Present a compact table: change and upstream commits, what the user would notice in Crabigator, proposed action, dependencies, and uncertainty. Avoid one question per commit.

Use these dispositions:

- **Port**: a relevant fix or an explicitly chosen feature, with the actual Rust gap established.
- **Already present**: equivalent Rust behavior confirmed in code/tests; cite the evidence.
- **Skip**: an established exclusion still applies, or the user declined it; record why.
- **Defer**: useful but outside this batch; record the reason and when to revisit.
- **Needs decision**: product behavior, visual preference, or scope is unclear.

Existing exclusions normally cover sound playback, Claude/pi/opencode plugin installation and panes, and `/flow` settings. They do not exclude shared scene or activity changes just because those shipped in the same commit. Preserve Crabigator's small column, session hue accents, Random claims, saved named scenes, Ctrl+], fade preference, and per-assistant event sources.

## Interview about the uncertain choices

For a requested port, proceed with clear bug fixes within scope. Ask about new scenes, changed defaults, new controls, performance/space tradeoffs, and behavior that conflicts with Crabigator's additions. Explain the observable choice, give a recommendation and 2–3 concrete options, and include a preview when appearance decides it. For example: “Doom3d needs a taller view; add it to the short column, adapt its composition, or defer it?”

Use the available user-input tool (async when possible), or a direct question if none is available. Ask a few related questions at a time. Continue independent analysis and already-authorized changes while waiting; an unanswered question is not acceptance of the recommendation. Reuse prior decisions. Reopen one only if new evidence changes its tradeoff, and say what changed. Do not ask the user to approve every routine translation detail.

## Port against a pinned reference

1. Inspect both repositories' status. Preserve other work; use an isolated Crabigator worktree when needed. Use a detached Flow reference worktree pinned to the selected SHA instead of switching the user's checkout.
2. Treat the maintained custom TypeScript fork and upstream as distinct sources. `FORK.md` records four original custom commits through `1a10bb9`; verify their current descendants rather than assuming upstream absorbed them. Raw upstream lacks small-box layout, ember, palette, and fade changes. For exact Rust fixtures, use a reference revision that includes the relevant retained custom behavior. If constructing one, record its upstream SHA, custom patches and resulting SHA. Never silently substitute raw upstream for the custom reference.
3. Port shared dependencies before scene consumers. Preserve `js::round`, wrapping typed arrays, RNG draw order, and `js::hypot` as documented. Preserve sound-only RNG draws even though sound is not shipped. Introduce new dials/events only when the chosen behavior needs them, and verify what each assistant's actual event stream can supply.
4. For a selective port, generate only the selected scenes' fixtures from the chosen reference:
   ```sh
   make flow-reference FLOW_DIR=/absolute/pinned/flow SCENES=surf,ski
   cargo test flow::
   ```
   The recorder supports comma-separated `SCENES`. Never regenerate every fixture from latest when only some scenes were ported. Shared rendering changes may require expanding the dependency batch; otherwise keep untouched fixtures pinned to their prior source. Never edit expected JSON to make Rust pass. Extend the recorder and Rust replay together when new waiting/agent dials need coverage; existing cases do not exercise every new upstream dial.
5. Run the relevant tests, formatting/build checks, and the repository's required review. Render representative 15–40-column, 4–12-row boxes at idle, busy, day/night, smoke/blue, and the selected waiting/companions behavior. Check hue accents separately from unaccented frame equality. Use the repository's preview/example if available; inspect the actual output. Measure the Rust step/render cost when a change adds substantial work. A TypeScript preview does not prove the Rust column looks right.
6. Separate focused changes into reviewable commits when committing is in scope. Do not install or release unless requested. Report exact reference SHAs, tests, visual checks, unresolved choices, and what remains unported.

## Remember decisions without overstating coverage

Record decisions in `src/flow/upstream-review.json` (schema in [references.md](references.md)); update it only from inspected evidence or an actual user answer. Record unresolved choices as `needs-decision`. Keep deferred/selected-but-unported entries visible on later runs, even if they precede the review cursor.

`reviewed_through` means every non-merge commit in that ancestry range has a disposition, including host-only changes. Merge commits also need inspection for conflict-resolution-only changes. `FORK.md`'s *Synced to* means every applicable change through that commit is ported and verified or explicitly excluded, as the log requires. A deferred applicable feature blocks that coverage claim. Record later selective ports individually without moving *Synced to* past a hole. Per-scene reference SHAs allow fixtures from different batches to remain honest.

Update `FORK.md` with the selected commits, Rust files, deliberate deviations, reference revision(s), and verification. Keep its license. Do not advance either cursor merely because fetch or merge succeeded.
