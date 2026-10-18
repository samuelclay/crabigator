# Conductor: one conversation across your sessions

Historical design proposal and interactive mockup. The selected name is now **Conductor**. The working web, iOS, Android, and desktop host implementation is documented in [docs/conductor.md](../../docs/conductor.md).

The HTML below remains a standalone simulation. Nothing here sends messages, queries stores, calls an LLM, or changes the shipping app. All release values in the mockup are fictional. No deployment is needed.

## Product shape

Run `crabigator conduct` to open a persistent coordinating conversation. This is a proposed command, not an implemented one. Use your preferred assistant CLI for its reasoning and tools. Existing project sessions keep their conversations, credentials, tools, and responsibilities.

Tell it: “Check the iOS and Android release status for NewsBlur, Turn Touch, and Crabigator.” It identifies the three responsible sessions, sends each a scoped question, watches their work, asks follow-up questions where evidence is missing, and presents one answer with six app rows. Cooking Blog stays outside this request.

The coordinator can route to one, several, or all sessions. Each user request creates a run. Each participating session gets a track within that run. A session can participate in successive runs without being restarted or cloned.

Working name: **Conductor**. Alternatives: Bridge, Switchboard, Dispatch, Helm, Ensemble, Chorus, Foreman, Harbormaster, Overlook. The selected name is Conductor; the implemented host command is `crabigator conductor`.

The user requested more naming ideas. Second set: Rally, Relay, Switchyard, Quarterdeck, Lookout, Roundtable, Crew, Clutch, Reef, Tidepool. Strongest new directions: Rally for bringing sessions together, Relay for carrying requests and answers, Switchyard for parallel tracks, and Clutch for a more distinctive Crabigator name. Conductor was selected for implementation.

## Try the sketch

Open `prototypes/conductor/conductor-tracks.html` in a browser, or use the inline preview in the conversation. Select a request and choose **Run example**, then **Next moment** to advance it. **Expand** focuses a track, **Session screen** switches to its simulated terminal, and **Consumer app** resolves the sample question without advancing the other tracks. Result rows expose sample source and check-time details.

The inline preview also offers working-name, layout, and detail choices through its design controls. This is a browser sketch of the terminal layout, not a working terminal multiplexer or LLM router.

Verified in the local browser: one-, three-, and four-session scenarios; immediate follow-up display; independent question handling; partial app results; focus and screen views; six-row completion; and a 360px layout with no horizontal overflow. JavaScript syntax was checked. Actual assistant dispatch and native terminal rendering remain unimplemented.

## The interface

Keep the coordinator's real assistant CLI across the full terminal width in the upper portion. Reserve the lower portion for Crabigator's parallel tracks. The assistant's own prompt remains in its upper pane, so ordinary typing always addresses the coordinator.

At a large terminal size, show two-by-two track panes. Each has the existing session identity chip, project name, task state, and timestamped activity. At wide sizes, three active tracks can sit in a row. At narrower sizes, use horizontal rows with one expanded track. Keep selection stable as activity arrives. Bring questions to an attention strip instead of moving panes under the user's cursor.

Each track has three levels:

1. **Overview:** current action, last meaningful update, elapsed time, and the results already available.
2. **Activity:** ordered assistant updates, tool calls/results, questions, errors, and final answer for this request. Preserve the full visible activity trail; summarize repetitive output only in overview.
3. **Session:** the captured terminal screen, with a deliberate fullscreen attach action to type directly into the original session. Returning restores the coordinator layout. Attaching does not restart the child or change its original terminal size.

“All intermediate steps” means observable assistant commentary and tool activity. There is no dependency on private reasoning. Distinguish exact session text from coordinator summaries. A result or summary can open its originating event and source link.

The mockup supports one/several/all routing examples, step-through progress, an app-listing question that blocks only one track, incremental results, track focus, activity/screen views, and two layout alternatives through the host's design controls. Screen text is simulated, not a real terminal capture.

Proposed terminal keys: normal text goes to the coordinator; a configurable prefix enters track navigation; arrows select, Enter expands, `f` attaches, and the existing attach escape returns. Do not globally consume letters, Tab, or arrows that the assistant needs. Resolve the prefix alongside Flow's existing Ctrl-] binding during implementation.

Keep the small roster of nonparticipating sessions visible as a quiet line. For large rosters, show a count that expands. Beyond six participants, keep all statuses in compact rows and expand selected tracks; do not shrink every terminal into unreadable tiles.

## Routing and responsibilities

The LLM chooses recipients. Rust validates and executes the chosen route. Keyword matching is a hint, not the router.

Give the model a compact directory with stable session IDs, repository/worktree, title, machine, assistant, current state, latest verified activity, and explicit responsibilities. A responsibility might be “NewsBlur: iOS and Android release owner.” Store roles at workspace/account scope, with a live session binding. If that binding ends, retain the responsibility but do not silently bind an arbitrary replacement.

The current local roster has multiple NewsBlur and Turn Touch sessions, plus this planning session and another Crabigator session. A repository name alone is insufficient. Exclude the coordinator itself from automatic routing. Exclude other coordinators unless the user deliberately requests delegation, and reject cycles.

Use exact names, roles, recent conversation references, and capabilities to resolve requests. “Ask them again” uses the recipients from the referenced run. “All apps” selects app-owning roles; “all sessions” selects the current managed roster. Freeze the recipient set when dispatching; newly started sessions do not join an existing run automatically.

For a confident route, show “Checking NewsBlur, Turn Touch, and Crabigator” and proceed. Let the user expand the reason and exact message. Ask one focused question when two plausible sessions own the same job, or the desired action is ambiguous. Do not ask for confirmation on every fan-out.

## A real coordinating tool loop

The coordinator runs a full assistant conversation with tools. Start by integrating an existing CLI and a local MCP bridge, rather than building another chat client or requiring a separate model API key. Validate provider-specific tool registration and background-result delivery in the first technical spike; do not assume the CLIs share those protocols.

Proposed tools:

| Tool | Purpose |
| --- | --- |
| `list_targets` | Return roles, identities, availability, and compact context. |
| `dispatch` | Create one run with a batch of targeted tasks; return track IDs promptly. |
| `read_events` | Read events after a cursor; support a bounded wait and partial results. |
| `read_result` | Return a track's attributed answer, evidence, gaps, and freshness. |
| `reply_to_question` | Answer the exact task/question revision, subject to authority. |
| `follow_up` | Ask a specific track for missing or conflicting information. |
| `cancel_pending` | Remove tasks that have not started. |

The model routes, compares evidence, and writes the answer. A deterministic Rust controller handles queues, delivery, deduplication, event storage, and reconnects. The UI subscribes directly to this controller, so tool progress can keep moving while the coordinator is waiting or writing. Do not ask the model to narrate every tool line.

Initially, the coordinator can remain in its tool loop using bounded event waits. If a user sends a follow-up or it ends its turn early, retain the run; later completion needs an adapter-supported notification or a queued coordinator follow-up. This is an explicit part of the spike, not something an MCP notification alone guarantees.

Each project session receives a self-contained instruction: original request, its own responsibility, relevant context, allowed action, expected answer, and a task ID. Do not forward every project's transcript to every other session. Child output is evidence, not authority to add recipients or perform unrelated actions.

## Reliable dispatch and completion

Existing input transport is useful but does not provide a task protocol. Add an envelope containing `run_id`, `task_id`, `attempt_id`, source/target session IDs, conversation generation, instruction, permitted action, queue policy, and creation time.

Persist run/task records before delivery in a local SQLite journal outside `/tmp`. Start on one machine. Later, add a cloud run owner and the same contract across machines; use an ownership lease so two coordinators do not dispatch the same pending task.

Track lifecycle:

`queued → delivered → started → working → answered | failed | cancelled`

Also track `waiting_for_user`, `offline`, and `delivery_unknown` without confusing them with successful completion. “Delivered” means the owning desktop acknowledged the envelope. “Started” means the assistant consumed the corresponding prompt. “Answered” requires an answer attributed to that task. A server's `{ok: true}`, an idle state, or a recap alone cannot establish that.

The desktop owns a per-session inbox and deduplicates task IDs. Delivery must check the local prompt state atomically with input injection. Do not type into a permission menu, question form, or partially entered user draft. The user selected this default: deliver a follow-up immediately without interrupting current work. Use the provider’s supported pending-message mechanism, verify that it accepted the follow-up, and show “Follow-up delivered · current turn continues” until the matching new turn starts. Do not send Escape or Ctrl-C. A generic “complete” flag is not sufficient proof of an empty prompt. If the provider has no safe follow-up mechanism, retain the request in the desktop inbox and explicitly report that delivery is waiting; never pretend local queueing was delivery.

For existing sessions without this protocol, offer observation and manual attach. Never silently downgrade a reliable dispatch to blind text injection. The first technical spike may use one dedicated instrumented child session; it must demonstrate prompt consumption before claiming existing sessions are supported.

Correlate task IDs with provider turn IDs and transcript cursors. An explicit result tool is strongest where available; an adapter can also identify the final answer of the consumed task's turn. In the latter case, mark attribution uncertainty if concurrent user steering prevents a clean association. A textual task marker is a fallback aid, not a guarantee. Do not ask the LLM to guess which recent final answer was ours.

Retries reuse the task's idempotency key. If the connection drops after input injection but before acknowledgment, reconcile with the desktop journal and transcript before retrying. There is no atomic transaction spanning a journal write and PTY input; preserve `delivery_unknown` when this cannot be resolved. Never claim exactly-once execution from a socket ACK.

Cancellation distinguishes removing a queued task, asking a running session to stop, and confirming it stopped. Detaching or closing the coordinator does not kill project sessions. On reopening, replay the journal and reconcile live state. Resume monitoring after compaction, disconnect, and coordinator restart.

## Events and the parallel display

Normalize observable provider output into events with `event_id`, `sequence`, `run_id`, `task_id`, `session_id`, `turn_id`, `timestamp`, `kind`, `content`, and a source cursor. Kinds include accepted, started, assistant update, tool started/finished, question, result, error, and disconnect. Persist ordered events before rendering them; deduplicate when reconnecting.

Reuse platform transcript parsing to build a structured event path before formatting into ANSI. Today the parsers mostly return display text; the new event stream should not try to parse that text back into structure. Some providers write transcript records only after an output chunk or tool finishes: update the screen independently and label activity freshness honestly.

Show the last few events in each pane, with an expandable complete trail. Keep large tool output behind the originating event and enforce a retention limit with a visible truncation marker. Let the coordinator read bounded event batches and final answers rather than every full transcript on every poll.

Keep two sources separate: transcript events supply task activity; `screen.txt` and actual styled `terminal_history.log` supply terminal views. A transcript is not terminal scrollback.

## App-store status as the first useful run

Send a read-only check to the three app owners, each covering iOS and Android. Ask them to verify store state using their own existing access, not infer it from the repository version.

Each app row needs: application identifier, platform, public version, candidate version/build, store or release track, exact store state, submission time, review-start time if known, last check time, source link/reference, and blockers. Preserve account and track distinctions where there are multiple entries.

Report “time since submission” separately from “time in review.” If review-start time is unavailable, say unknown; never relabel submission age as review duration. Keep uploaded, submitted, in review, approved, and publicly available separate. For phased/staged releases, include rollout information when available.

Return useful partial results as they arrive. If one session needs access or must resolve an ambiguous app listing, show the verified rows from the others plus “waiting for access” for the missing apps. The coordinator checks for missing platforms, stale timestamps, and conflicting versions, then asks only the affected owner a follow-up. Finishing all tasks with some unknown answers is a partial result, not a fully verified report.

A parent request to check status authorizes reading, not submitting or releasing apps. The coordinator relays authority and unresolved questions without widening it. Children retain their own permission controls. A session may answer “I do not own this app”; do not assume every project necessarily has both store listings.

## Existing building blocks verified in this checkout

| Code | Reuse and limitation |
| --- | --- |
| `src/prs_board.rs` | Local/cloud roster, session identity, quick look, fullscreen attach. Extract shared session access as needed; keep the conductor state machine separate. |
| `src/attach.rs` | Local socket sends key/paste frames. Add a separate task envelope/inbox path rather than treating these frames as acknowledged jobs. |
| `src/app.rs` | Owns input injection and PTY loop. Suitable place for prompt-safe delivery and task acknowledgment. |
| `src/platforms/*/transcript.rs` | Existing provider readers. Add attributed structured events ahead of ANSI formatting. |
| `src/capture.rs` | Screens and separate transcript/terminal history sources. |
| `workers/crabigator-api/src/mcp/tools.ts` | Discovery, snapshots, scrollback, input, attention waiting. Input sends text plus Enter; attention waiting only watches questions/permissions. |
| `workers/crabigator-api/src/mcp/events.ts` | State, prompt, recap notifications. These do not form a full task-linked activity stream. |
| `workers/crabigator-api/src/durable-objects/SessionDO.ts` | Existing input forwarding and prompt revision checks. Its response confirms forwarding, not that a matching assistant turn ran. |

The existing terminal architecture can support a full-width coordinator PTY above a rendered track area. Do not forward several child PTY escape streams into adjacent panes: the vertical scroll region does not isolate horizontal cursor movements. Render captured child screens into bounded regions; fullscreen attach can reuse the existing path. Avoid changing the original sessions' sizes merely to fit thumbnails.

## Build sequence and acceptance gates

1. **Interface prototype (this change):** simulated routing, independent tracks, progress, questions, partial and final results. Choose naming and preferred layout. Busy policy is immediate follow-up without interruption. No production code changes.
2. **One local real round trip:** one coordinator CLI, local MCP bridge, one instrumented project session, durable task IDs, acknowledged prompt consumption, attributed events/final answer. Prove immediate follow-up delivery during a busy turn without stopping it, and prove the unrelated current turn cannot complete the new task. No cloud changes.
3. **Three-session store check:** send one batch, receive independent answers, handle one question without blocking the other tracks, synthesize six sourced rows with honest unknowns. Only this gate demonstrates real orchestration.
4. **Resilience and direct interaction:** busy sessions, drafts, duplicate submission, input/ACK disconnect, interleaved direct user input, compaction, restart, offline sessions, cancellation, large output, and narrow terminal sizes. Prove no duplicate commands and no accidental prompt answers.
5. **Cross-machine support:** carry the same task/event contract through existing cloud auth and session transport; persist ownership and resumable cursors. Verify two devices and reconnects. Deployment is a separate future decision.

Do not build a general scheduler, spawn new sessions automatically, or add mobile parity before the first read-only three-session run works. A scheduled fleet check can come later using the same run model.

## Open decisions

- Final name and command.
- Busy behavior is decided: deliver an immediate follow-up without interrupting. Verify provider support and clearly report any delivery fallback.
- How users assign and correct session responsibilities; suggested default is infer once and remember explicit corrections.
- Which assistant CLI to prove first. Supporting every provider should follow a verified single-provider round trip.
- Whether waiting runs remain active when the coordinator UI closes. Suggested default: retain tasks and let child sessions continue, then resume monitoring on reopen.
