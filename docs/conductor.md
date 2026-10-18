# Conductor

Conductor adds one coordinating conversation to web, iOS, and Android beside **List** and **PR board**. It has three views:

- **Conversation** leads with the latest answer. Session results use the actual glyph and current title, with bold key facts, outcome colors, and report times. Earlier updates stay collapsed. Current requests appear as parallel progress cards.
- **Sessions** groups requests by project and actual session. The session's recap, diff counts, and activity help identify it. Requests have separate active, answered, and failed filters. Online/offline describes the connection, independently of the request outcome.
- **Archive** searches saved questions, answers, and progress, grouped primarily by project and session. Each result includes the original question and complete answer, with copy and open-session actions. Outcome filters and pagination help narrow a search.

All three clients use fixed-width text, spaced paragraphs, and readable tables. Session IDs remain internal. A session's current title and mark refresh without changing its historical answer.

The Worker, desktop wrapper, and native clients must all use a version with Conductor support before the feature can work together. Deploying the Worker updates the web client and API; desktop packages and native apps require separate releases or local builds.

## Start the desktop host

Build the desktop with `cargo build`, then run:

```sh
./target/debug/crabigator conductor
```

The first implementation uses your installed, signed-in **Claude Code** CLI as the coordinator. It runs a full local model/tool loop, with tools limited to the account's session directory, dispatch, progress, and replies. Existing project sessions may use **Claude Code or Codex**. Restart their wrappers with this build so they advertise support for Conductor input. Other assistant platforms are rejected explicitly.

The host uses the cloud origin selected by `crabigator cloud`. It must already be paired to the same account as the clients. One desktop holds the host lease at a time. Keep its process running; Ctrl-C stops coordination while project sessions continue. Messages sent while it is offline remain available when it returns.

For isolated development, `CRABIGATOR_CONFIG_DIR=/absolute/path` selects a separate configuration and device identity without changing your normal Crabigator settings. The local integration check used `http://localhost:8791` and disposable project folders with fictional store values.

## Ask naturally

- “Ask NewsBlur for its iOS and Android store status.”
- “Check versions and time in review for NewsBlur, Turn Touch, and Crabigator.”
- “Ask all sessions what is blocking them.”
- “Stop tracking the old release-status request.”

The model reads the active session directory and relevant session context, then chooses recipients. It can ask a clarification when ownership is ambiguous. A status request is read-only; project assistants retain their own tools and permissions.

Busy Codex sessions receive a queued follow-up using Tab; Claude Code receives a pasted follow-up through its composer. Conductor never sends Escape or an interrupt. A focused question, approval, unsupported screen, or detected draft prevents dispatch and produces an explicit error. Open that session to resolve it, then ask again.

## What the track states mean

| State | Meaning |
|---|---|
| Sending / forwarded | The server sent the request and awaits the desktop acknowledgment. |
| Delivered | The desktop submitted the follow-up. The assistant has not yet acknowledged starting it. |
| Working / waiting | The assigned assistant explicitly reported progress or a blocker. |
| Answered | The assigned assistant explicitly reported the answer for this task ID. |
| Failed | Delivery was rejected or the assistant reported failure. |
| Delivery unknown | Delivery may have happened, but acknowledgment is missing. Inspect the session before requesting another attempt. |
| Cancelled | Conductor stopped tracking the request. The project session keeps running. |

A recap, an idle terminal, and a completed unrelated turn never become a task answer. Each dispatched prompt includes an account-scoped report command. The assistant uses it to report progress and the final answer; the Worker verifies both the desktop and the assigned session. Recent session activity is displayed separately because it may include earlier work.

Messages and task dispatches have idempotency keys. A reconnect or repeated tool call does not replay terminal input. The model context retains recent requests and unfinished work. A separate, account-scoped SQLite archive preserves older questions and reports; users can ask Conductor to stop tracking abandoned requests. The UI polls while open, and native polling pauses in the background.

## Attributed replies

The coordinator's `reply` tool accepts a short lead answer and optional `highlights`, each with a `session_id`, concise Markdown `text`, and optional source `task_id`. The server supplies the real identity and verifies that each referenced session belongs to the account. A source task provides the outcome and report time; an unrelated recap never supplies a task outcome. Highlight text is limited to 1,000 characters and replies to 12 session highlights.

A structured reply counts as the final answer for that model turn, so the CLI's automatic final text does not duplicate it. Existing plain-text replies remain available. Archive search covers highlight text as well as the underlying reports. The archive starts with the history still available when this version first runs; it cannot recover records already trimmed by an older version.

## Verification

Worker integration checks cover account isolation, duplicate sends, delayed acknowledgments, session ownership changes, host completion retries, attention prompts, and bounded history. Native UI tests cover track expansion, draft preservation, and retrying the same message after an ambiguous response. Desktop checks exercise focused composers, draft rejection, and Codex's busy/idle submission behavior.

Runtime checks use disposable sessions and explicitly fictional app versions. They do not establish real App Store or Google Play status. The earlier interactive design remains in `prototypes/conductor/conductor-tracks.html`; the broader terminal multiplexer layout in that proposal is future work. The implemented control surfaces in this change are web, iOS, and Android.

Verified locally on October 17, 2026:

- The full Worker suite passed, including Conductor's four integration tests, and TypeScript checking passed.
- The iOS simulator passed the native suite and the Conductor view-switching, draft, and send test. Android passed its build, unit checks, and Conductor instrumentation test on a connected Samsung phone.
- A local Worker and real desktop coordinator sent requests to disposable Claude Code and Codex sessions. Both returned task-specific progress and final reports. Separate busy-session checks preserved ongoing work while accepting follow-ups. The restarted coordinator also answered from its existing conversation.
- The three focused Rust input tests passed. The full Rust suite had one unrelated PR-board layout failure, reproduced on untouched HEAD: `title_takes_the_slack_link_room_and_the_diff_hugs_the_activity`.

These checks do not establish production availability or native clients connected to a deployed Conductor service.

The redesigned view additionally passed archive retention/search and current-title indexing checks, structured reply identity and duplicate-final checks, the iOS navigation/search/draft UI test, and the Android archive/search/retry UI test on R5CT601C8DA. Fresh captures are under `target/conductor-evidence/` with `redesign` in their filenames. The local browser coordinator produced the attributed fixture reply from existing reports without dispatching new tasks.
