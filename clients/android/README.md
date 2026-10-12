# Crabigator for Android

Native Kotlin and Jetpack Compose client. The session list opens first. The icon toolbar switches between the board and session list, filters live or ended work, searches, and opens an animated Settings popover. Selecting a session slides its live terminal in from the right. Drag right anywhere on the terminal to reveal the list; release a short drag to return, or swipe farther or flick to close. On tablets, selecting another row pages the terminal left or right in list order. Only the selected session has a live connection; outgoing pages keep their last screen until the transition finishes, then are discarded. A growing message composer, microphone, and terminal keyboard stay at the bottom. At 720 dp and wider, the board stays beside the session.

Both views group sessions under sticky machine headers. Tap a header to collapse or expand its sessions; the count remains visible. Machines start in order of their newest session, then keep their positions while the screen is open. New machines append at the bottom. Search and live updates do not reshuffle machines; sessions inside each machine still update. Project folders remain inside their machine, and PRs shared across computers appear once under each contributing machine with only its sessions. Tracked PRs without sessions have a separate folder.

On tablets the list keeps its width when a session opens or closes, leaving an empty detail pane when nothing is selected. Session cards use separate identity and status columns, with title and timing aligned beside them. Questions have an orange badge and card accent; permission requests use yellow. Timing stays on one line: durations sit at the left, prompt/completion recency at the right, without repeated “ago” labels. Durations shorten to their largest unit when space is tight. At very large text sizes, recency takes priority and the full duration values remain in the accessible description.

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

Enable notifications in the app's Settings. Notifications are silent by default, including on existing installs. Turn on Sound under Settings → Notifications to allow the sound chosen in Android's notification settings; system mute and Do Not Disturb still apply. A data-only FCM message carries a session ID. The app authenticates and fetches the latest prompt before displaying or cancelling a notification. This makes delayed and out-of-order deliveries converge on current state. WorkManager handles short background work, retries refreshes, and reconciles shown notifications when the app resumes.

Choices and direct replies use the same terminal input translator as in-app questions. Each action includes the server's prompt revision. The session Durable Object rejects stale or duplicate replies and immediately signals other devices after accepting an answer. Replies are never automatically retried after an ambiguous network failure. Multi-select and multi-page prompts continue in subsequent notifications; opening a notification always offers the complete native question controls.

Android and FCM may delay background work under battery restrictions or while offline. Notification clearing is best effort during those periods; reopening the app reconciles current state. Notifications hide question text on a locked screen by default and require unlock for actions. Force-stopping the app prevents delivery until it is opened again.

## Terminal

The server supplies complete row-formatted ANSI snapshots. `TerminalText` converts foreground/background colors, bold, dim, underline, inverse, and horizontal spacing into native text spans. Terminal status markers such as `⏵⏵` and Claude’s animated bullets and spinners use a single-cell subset of [Noto Sans Symbols 2](https://github.com/google/fonts/tree/main/ofl/notosanssymbols2) so they render on devices whose system font lacks them. All animation frames advance exactly one native monospace cell, so nearby text stays still. Rebuild the subset from the upstream font with `scripts/terminal-symbols.py`. Its SIL Open Font License is included in `app/src/main/assets/licenses/`. Native text wraps to the detail pane by default; Style can switch to horizontal scrolling to preserve the desktop columns. The pin control follows new output; scrolling up releases it. The normalized conversation history sits above the live terminal in the same scrollable view, cached for late joiners by the Worker. ANSI parsing runs off the UI thread. The view lays out small blocks near the viewport instead of drawing the entire transcript, and an independent drawing layer keeps terminal updates from redrawing the session list. Native selection works on loaded blocks; Select all does not include offscreen blocks that have not been composed. Empty output has an explicit empty state. Viewing a session sends a five-second foreground heartbeat so the desktop keeps its screen current; no foreground service or background screen polling is required.

## Input

The composer grows to six lines, then scrolls. Its keyboard popover has common shortcuts, navigation and letter pads, and Shift, Ctrl, and Alt modifiers that stay selected until reset or dismissal. Voice input requests microphone access when tapped, shows a live audio-level waveform, and offers Cancel, Edit, or Send. Edit adds the transcription to the draft; Send submits it. Recordings stop after two minutes or when the app leaves the foreground. Audio is transcribed through the same authenticated endpoint as the web dashboard, and temporary recordings are deleted after use.

## Appearance

The detail pane’s top-right Style menu contains text size, line spacing, wrapping, terminal height, widget visibility, list columns, project grouping, and project order. Settings contains list position/density, visible statistics, notifications, account identities, pairing another device, MCP, and unpairing. Session titles, status indicators, and statistics use Crabigator’s terminal palette. Working sessions use the same animated braille throbber. Choices are saved on this device. Menus are anchored to their toolbar icons, fade and scale into view, and scroll within the available phone or tablet space.

## Verification

Unit tests cover terminal color/spacing and question input translation. `TerminalCellTest` measures native text on a device: the blinking bullet and all six spinner frames must preserve following text positions and row height at three font sizes. Worker tests cover prompt revisions, simultaneous replies, hibernation, auth/account isolation, and FCM token lifecycle. Device checks should cover phone and tablet layouts, rotation, composer/IME, voice edit/cancel/send, terminal selection, option and free-text replies, background notification delivery, desktop resolution, and an old notification racing a newer question.

The first builds are direct debug installs, not a Play Store release. Release signing, store listing, and store submission are separate work.

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
