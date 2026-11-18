# Crabigator for Android

Native Kotlin and Jetpack Compose client. The session list opens first. A floating segmented footer switches between List and PR board. On tablets, switching modes opens the full-width view; tapping the selected segment toggles between full width and a sidebar. Selecting a session narrows the browser beside its terminal. Each mode retains its scroll position. With Columns set to Auto, the List lays out as many columns as fit; narrow panes keep one column. On phones, selecting a session slides its terminal in from the right. Drag right anywhere on the terminal to reveal the browser; release a short drag to return, or swipe farther or flick to close. Selecting another row pages the terminal in list order. Only the selected session has a live connection; outgoing pages retain their last screen until hidden. Expanding the browser cancels hidden voice recording and clears keyboard focus while preserving the draft.

Both modes use collapsible sticky machine headers with session counts. Machines start in order of their newest session and keep their positions while the screen is open; new machines append. List mode can group projects within each machine. The PR board follows the CLI's dense session layout: machine, activity-age band, repository, then session rows with PRs underneath. Work in flight stays in Last hour; settled rows follow their last prompt, with PR event timestamps as a fallback. Repositories follow their freshest row. PR subrows show branch, change counts, state, checks, and merge status. Shared PRs appear beneath each contributing session; watches without sessions remain in Tracked PRs. The filter bar's Sessions chip adds ended sessions; updates are always live.

In the View panel, **Group by** switches between a section for each computer and one combined list in order of recency. The List can also group by project, alone or inside each computer. The computer choice applies to both views and is saved on this device. A combined view names the computer on each session row. Computer names use the same saved friendly names and colors across clients. Recaps share one blue color.

Session cards use separate identity and status columns, with title and timing aligned beside them. Question and permission badges carry orange and yellow backgrounds; the surrounding cell keeps its normal color. Timing stays on one line: durations at the left, prompt/completion recency at the right. Prompt ages are gold-orange and completion ages deeper orange, matching the CLI. Completion keeps “ago,” and the prompt keeps it when both fit. Durations shorten to their largest unit when space is tight. At very large text sizes, recency takes priority and full durations remain in the accessible description. A row can also show a counts line (tools, compactions, and context as `186k / 1M · 19%`), a git line (`⎇ branch`, then the number of files not committed), and a changes line (`Δ +212 −48`). A line with nothing to show is left out. The terminal's downward-arrow control follows the latest output.

The initial session request shows animated loading dots. Connection failures offer Retry; only a successful empty response shows “No sessions,” the desktop pairing command, and a pairing-code field. Background refreshes keep the last successful list visible.

## Build and install

Use JDK 17 or later and Android SDK 36:

```sh
cd clients/android
./gradlew :app:testDebugUnitTest :app:lintDebug :app:assembleDebug
adb -s DEVICE_SERIAL install -r app/build/outputs/apk/debug/app-debug.apk
```

Run `crabigator pair` on the desktop, then enter its code in the app. Self-hosted servers require HTTPS. Pairing credentials are encrypted with Android Keystore and excluded from backups and device transfers.

## Notifications

Register the Android package `com.crabigator.app` in Firebase. Create the ignored `firebase.properties` file beside `settings.gradle.kts` from the downloaded Android configuration:

```properties
google_app_id=YOUR_MOBILE_SDK_APP_ID
google_api_key=YOUR_ANDROID_API_KEY
gcm_defaultSenderId=YOUR_PROJECT_NUMBER
project_id=YOUR_PROJECT_ID
```

Only the Firebase client configuration belongs in the app. Never put a service-account private key in the Android project or APK. Store its JSON in the Worker secret `FIREBASE_SERVICE_ACCOUNT`. Apply Worker migration `0022_mobile_push.sql` before deploying the mobile API. The app builds without Firebase configuration, but background delivery then remains unavailable.

Enable notifications under Account → Notifications. Notifications are silent by default, including on existing installs. Turn on Sound there to allow the sound chosen in Android's notification settings; system mute and Do Not Disturb still apply. A data-only FCM message carries a session ID. The app authenticates and fetches the latest prompt before displaying or cancelling a notification. This makes delayed and out-of-order deliveries converge on current state. WorkManager handles short background work, retries refreshes, and reconciles shown notifications when the app resumes.

Choices and direct replies use the same terminal input translator as in-app questions. Each action includes the server's prompt revision. The session Durable Object rejects stale or duplicate replies and immediately signals other devices after accepting an answer. Replies are never automatically retried after an ambiguous network failure. Multi-select and multi-page prompts continue in subsequent notifications; opening a notification always offers the complete native question controls.

Android and FCM may delay background work under battery restrictions or while offline. Notification clearing is best effort during those periods; reopening the app reconciles current state. Notifications hide question text on a locked screen by default and require unlock for actions. Force-stopping the app prevents delivery until it is opened again.

## Terminal

The server supplies complete row-formatted ANSI snapshots. `TerminalText` converts foreground/background colors, bold, dim, underline, inverse, and horizontal spacing into native text spans. Terminal status markers such as `⏵⏵` and Claude’s animated bullets and spinners use a single-cell subset of [Noto Sans Symbols 2](https://github.com/google/fonts/tree/main/ofl/notosanssymbols2) so they render on devices whose system font lacks them. All animation frames advance exactly one native monospace cell, so nearby text stays still. Rebuild the subset from the upstream font with `scripts/terminal-symbols.py`. Its SIL Open Font License is included in `app/src/main/assets/licenses/`. Native text wraps to the detail pane by default; the session's View panel can switch to sideways scrolling to preserve the desktop columns. The pin control follows new output; scrolling up releases it. The normalized conversation history sits above the live terminal in the same scrollable view, cached for late joiners by the Worker. ANSI parsing runs off the UI thread. The view lays out small blocks near the viewport instead of drawing the entire transcript, and an independent drawing layer keeps terminal updates from redrawing the session list. Native selection works on loaded blocks; Select all does not include offscreen blocks that have not been composed. Empty output has an explicit empty state. Viewing a session sends a five-second foreground heartbeat so the desktop keeps its screen current; no foreground service or background screen polling is required.

## Input

The composer grows to six lines, then scrolls. Its keyboard popover has common shortcuts, navigation and letter pads, and Shift, Ctrl, and Alt modifiers that stay selected until reset or dismissal. Voice input requests microphone access when tapped, shows a live audio-level waveform, and offers Cancel, Edit, or Send. Edit adds the transcription to the draft; Send submits it. Recordings stop after two minutes or when the app leaves the foreground; either way the recording is transcribed into the draft. The screen stays on while recording. Audio is transcribed through the same authenticated endpoint as the web dashboard, which saves every transcript to the account. Account → Voice transcripts lists them all, and a session's View panel lists that session's, each with Copy and Delete. If transcription fails, the recording is kept so Try again can resend it; otherwise temporary recordings are deleted after use.

## Filters, View and Account

A filter bar sits under the board header in both modes. Its three chips choose the activity window (`1h` to `24h`, or `All`), live sessions or live and ended, and one computer or all. When the bar is too narrow, the chips scroll sideways and **View** stays at the trailing edge. On a full-width tablet board, the toolbar floats in the middle of the same row.

**View** opens the choices for the screen it sits on: the List, the PR board, or a session. It is a sheet under the button on a phone and a popover on a tablet. Choices that change a layout are drawn as tiles (`ViewPanel.kt`).

- **List and PR board**: the same three filters as the chips; Group by; on the PR board, whether each block starts with the session or the pull request; what a row shows; text size and row spacing; List columns; and, on tablets, the session list side. The PR board offers only One list and Computer under Group by, because it always groups by time band and repository. The sample row is the real row view, drawn from a sample session.
- **Session**: terminal text size and line spacing, wrapping or sideways scrolling for long lines, terminal height, whether the drawer under the terminal is open, the widgets in the drawer, and the items in the stats strip. A small drawing of the session screen follows the height and drawer choices.

The List row, the PR board row and the stats strip each save their own set of shown items (`listRow`, `boardRow`, `sessionStats`). Until a set is first changed it follows the choices made on the old Settings screen, so an updated app looks as it did before. Context has one more rule. On a List row and in the stats strip it starts on if its old switch was on, or if every other stat the old screen could switch was on. On a PR board row it starts off. A fresh install shows no context until it is turned on, and a choice made in a View panel sticks.

The person icon opens **Account**: notifications, voice transcripts, sign-in, pairing another device, MCP, and this device. Nothing there changes how a screen looks. The + button offers New session and Watch a pull request. On a tablet with a session open, a header button expands the board or returns to the session.

Session titles, status indicators, and statistics use Crabigator’s terminal palette. Working sessions use the same animated braille throbber. Choices are saved on this device. Panels fade into view and scroll within the available phone or tablet space.

## Verification

Unit tests cover terminal color/spacing, question input translation, the saved view choices (including what an older save shows after an update), and the Group by mapping. `TerminalCellTest` measures native text on a device: the blinking bullet and all six spinner frames must preserve following text positions and row height at three font sizes. Worker tests cover prompt revisions, simultaneous replies, hibernation, auth/account isolation, and FCM token lifecycle. Device checks should cover phone and tablet layouts, rotation, composer/IME, voice edit/cancel/send, terminal selection, option and free-text replies, background notification delivery, desktop resolution, and an old notification racing a newer question.

Debug builds include `PreviewActivity`, which shows the app with a sample roster and sends no request. Use it to check or photograph a screen on an emulator without pairing:

```sh
adb -s EMULATOR_SERIAL shell am start -n com.crabigator.app/.PreviewActivity --ez reset true
```

Add `--es mode board` for the PR board, `--ez session true` to open a session, and `--ez expanded true` to keep a tablet's board at full width. `--ez reset true` returns every saved view choice to its default.

## Store build

`python3 scripts/build-store.py` runs unit tests, release lint, and builds a signed
`app/build/outputs/bundle/release/app-release.aab`. Use JDK 21 and Android SDK 36.
The release build requires the Firebase client settings above and an upload key;
it fails if either is missing. Debug and local profile builds remain independent.

Set `CRABIGATOR_ANDROID_KEYSTORE` to the upload keystore and provide
`CRABIGATOR_ANDROID_STORE_PASSWORD` through your secret manager. The alias is
`crabigator-upload`. On macOS, the script also supports the encrypted keystore at
`~/.crabigator/signing/crabigator-upload.jks` and its password in Keychain under
service `com.crabigator.android-upload`, account `crabigator-upload`. Never commit
the keystore, password, or Firebase client configuration. Keep a secure backup of
the upload key before relying on it for future releases.

The pairing screen includes **Try a demo**, an offline walkthrough with sample
sessions, a PR board, a question, and local sample messages. It does not pair a
device or send messages to a server. Privacy information is available from the
pairing screen and Account at <https://drinkcrabigator.com/privacy>.

## Scroll profiling

Use the `profile` build for performance checks. It is optimized, non-debuggable, and signed with the local debug key, so `adb install -r` preserves the existing pairing. This is a local install, not a store release. Compose tracing is included only in this build.

```bash
./gradlew :app:assembleProfile
adb -s DEVICE install -r app/build/outputs/apk/profile/app-profile.apk
adb -s DEVICE shell cmd package compile -m speed -f com.crabigator.app
```

Open a long-running session on the tablet in landscape and let initial history loading settle. Capture each pane separately, starting with the list at the top and terminal pinned to its bottom:

```bash
python3 scripts/profile-scroll.py --serial DEVICE --label before-list --output /tmp/crabigator-profile --x 250
python3 scripts/profile-scroll.py --serial DEVICE --label before-terminal --output /tmp/crabigator-profile --x 1300 --start-down
```

Coordinates default to the 1920 × 1200 Samsung tablet; adjust `--x`, `--top`, and `--bottom` on other devices. Each run makes eight 700 ms swipes. Frame statistics cover the gestures; Perfetto records a fixed 18-second window, usually including a settling period afterward. Severe stalls can make the gestures outlast that trace; use the recorded frame statistics for the full gesture run in that case. `--start-down` starts toward older output, avoiding idle swipes against the pinned bottom. It aborts if another app takes focus. Keep the session, app compilation mode, pane sizes, and gestures the same when comparing builds. Allow initial history loading to finish before each capture. A session finishing work changes the workload: do not compare an active run with a settled run.

Open the trace in Perfetto or run `trace_processor query -f scripts/scroll-summary.sql TRACE` using [Perfetto’s trace processor](https://perfetto.dev/docs/analysis/trace-processor). Look at UI-thread drawing and measurement, not just recomposition counts. Frame statistics include live updates during the gestures; compare repeated runs rather than treating a single percentage as a universal benchmark.
