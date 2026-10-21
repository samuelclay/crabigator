# iOS verification — October 13, 2026

The updated native app is installed and paired on ClayPad Mini. The October 13 checks below verify its live terminal, board navigation and real APNs delivery. Earlier checks on ClayPad Air, ClayPhone SE 3 and simulators remain recorded below.

## Waiting sessions age normally — October 19

The board already refreshes its roster every 12 seconds while foregrounded. Its age calculation treated questions and permission requests as current activity on every refresh, so a session waiting for hours stayed in Last hour. iOS now uses the last prompt for waiting sessions in both session and PR grouping. Only active thinking stays current; waiting sessions keep their attention indicators.

The new regression failed on the previous implementation, then all 31 unit tests passed on ClayPad Mini. It covers a five-hour-old turn moving out of Last hour when it starts waiting, subsequent aging with no new events, both board groupings, and activity filtering. Result: `/tmp/crabigator-waiting-age-after.xcresult` (unit suite passed; its first live check used an incorrect expected age band).

The updated app is installed on Mini. The final live UI check found the affected waiting session beneath the 3–6 hours heading and verified that this was its nearest preceding age heading. Result: `/tmp/crabigator-waiting-age-verified.xcresult`; screenshot under `/tmp/crabigator-waiting-age-verified-shots/`. An intermediate UI attempt missed Search after toggling an already selected board; the check now preserves the selected mode. No session input was sent. This change is limited to iOS.


## Rich prompt notifications — October 15

Checked on ClayPhone SE 3 (iOS 27) with real APNs notifications from throwaway Claude Code and Codex sessions, through opt-in UI tests in `UITests/NotificationTests.swift`. All 25 unit tests passed on the phone.

- The banner shows the question, then every choice on one line, for example "Which pizza crust?" then "Thin · Thick · Stuffed · Deep dish · Cauliflower".
- The service extension's per-prompt category carries the real labels on first delivery: the delivered notification used `CRABIGATOR_PROMPT_SLOT_1` with Thin, Thick, Stuffed, Deep dish and Reply, each tied to its session and revision. Two other sessions' notifications held separate slots at the same time.
- With sixteen choices registered, iOS showed ten buttons. The expanded view still listed all sixteen.
- Tapping a choice in the expanded view sent exactly that answer: Stuffed; Thick, Medium and Submit answers for a two-question prompt; Cheese and Olives with Submit for a multi-select question, then its review page; Deep dish for a five-choice Codex question. The view showed progress, moved to the next question in place, and closed when the session moved on.
- The Thick button and Reply ("Neapolitan please") each reached the session as sent.
- A prompt answered in the terminal while its expanded view was open could not be answered again: the tap sent nothing and the view said the request had ended.

Device findings now handled in the code: the extension must link UserNotificationsUI explicitly; its extension context is empty while the view loads; iOS keeps the first content size it sees; and iOS only expands a notification that has a regular button, so prompts without choice buttons get Open session. Twice in about 25 expansions, iOS showed its plain banner instead of the extension, still with the real-label buttons.

Not verified: answering from the view while the phone is locked, Apple Watch, and the iPad. Failed device tests made Xcode save a 50–280 MB sysdiagnose on the phone each time, which filled it; the runs above use `-collect-test-diagnostics never`.

Outside this change: single-question Claude dialogs reach the phone without the terminal's cursor row, so an answer can pick the wrong row if the cursor was moved in the terminal (a desktop fix is in progress). Three test sessions, each running 45 to 70 minutes, stopped receiving input from the cloud while still sending their prompts; the server accepted answers that never reached the terminal.

## Mobile parity and ClayPad Mini

On October 13, the signed Debug build and notification extension installed on ClayPad Mini (iPad mini A17 Pro, listed by Xcode as “Samuel’s iPad”). Thirteen unit tests and five live UI tests passed across two runs. They covered terminal reading, scrollback, Follow, Style, QWERTY, slow prompt metadata, the loading Reconnect button, fullscreen boards, restart restoration, and a real Apple notification. The delayed-metadata check displayed terminal output in 1.84 seconds while the prompt request was held for 15 seconds.

Both platforms now hide the session highlight when the board fills the screen. The session stays remembered: tapping the selected List or PR board control again returns to it, including after restarting iOS. Explicitly closing the session still clears it. PR board separator lines were removed on both platforms; List already used spacing between repository groups.

Android now keeps drafts per session when switching or closing details, clears a sent draft even if the selection changes before the response, sends Shift+Left from Questions, and distinguishes slow terminal loading from an empty result. iOS gives unused widget space back to the terminal. Its hidden UIKit terminal view had also intercepted Reconnect; the view is now created only when output exists, and the physical Mini test taps Reconnect successfully.

Android passed 28 unit tests, lint, app/test builds, and four instrumentation tests on the Samsung SM-X200 tablet. The tests exercise draft retention, exact Questions shortcut bytes, loading/reconnect states, and the fullscreen round trip in both board modes. The signed-in device state was preserved.

The NewsBlur APNs credential is now configured in the production Worker. The key originally supplied was rejected by Apple; the working key was located in Studio’s NewsBlur secrets repository. Apple accepted a direct alert for Mini with HTTP 200, and XCTest observed and captured its notification on the physical device. The app also reported “Notifications on” after registering with production. This establishes real APNs transport and visible delivery; it does not establish all prompt-action, sound, or cross-device notification behavior.

Results: `/tmp/crabigator-mini-first.xcresult` (13 unit tests and three reading/loading UI tests), `/tmp/crabigator-mini-final.xcresult` (fullscreen restoration and real APNs delivery). Android logs on Studio: `/tmp/crabigator-mobile-parity-final-build.log` and `/tmp/crabigator-mobile-parity-final-tablet.log`. Screenshots are local test artifacts and contain private session content.


### Board spacing and fullscreen toolbar

Style now has separate Tight, Normal and Relaxed spacing for List and PR board. Tight uses one-line titles and smaller row and group gaps; terminal settings are independent. The fullscreen header floats in the center of the machine-name row, with the logo on the left and Search/Settings on the right. Opening a session moves it back into the sidebar header. Search reserves space beneath the floating control.

Android passed its 28 unit tests, lint, app/test builds, and five instrumentation tests on the Samsung tablet, including measured row-height reductions in both modes, toolbar controls, persistence, and unchanged terminal line spacing. The updated APK is installed on the tablet and S22. Final instrumentation log on Studio: `/tmp/crabigator-style-instrument-final.log`.

ClayPad Mini passed 13 unit tests and the physical UI spacing/toolbar test in `/tmp/crabigator-style-mini-final.xcresult`. The UI check measures shorter rows in List and PR board, verifies the centered fullscreen controls and sidebar return, taps Search, checks unchanged terminal spacing, and verifies saved Tight spacing after relaunch. It restores the original spacing afterward. The device remains in its original landscape orientation. Four earlier passing screenshots are exported under `/tmp/crabigator-style-mini7-shots/`.

### Style tabs, animated spacing and aligned columns

Style now has pinned List / PR board and Terminal tabs. Settings opens the board controls; the terminal's Style button opens terminal controls. Board spacing changes animate over 300 milliseconds, with reduced motion respected. The layout setting is labeled List columns.

Prompt and completion ages have separate reserved columns in both views. PR rows reserve independent CI and branch-status columns; missing values show a dash without moving the neighboring column. Android fits status text within its columns at both the smallest and largest board text sizes.

ClayPad Mini passed 13 unit tests and the live spacing, scope-tab and toolbar test in `/tmp/crabigator-style-columns-mini.xcresult`. The device stayed in landscapeRight. Screenshots in `/tmp/crabigator-style-columns-mini-shots/` show both tabs and the List/PR board in fullscreen and the sidebar. The first attempt failed while tapping Search during its animated move; the passing test waits for the control to become hittable.

Android passed 28 unit tests, lint and both APK builds. Ten instrumentation tests passed across focused runs on the Samsung tablet: six mobile parity checks, three PR interactions/column checks, and one timeline check. They measure an intermediate row height during animation, verify independent style controls, compare column positions with missing or changing values, and check that status text does not overflow at minimum/maximum board font sizes. Logs: `/tmp/crabigator-columns-device.log`, `/tmp/crabigator-columns-timeline.log`, and `/tmp/crabigator-columns-size-final.log`. The initial timeline run used outdated text-node queries; the final check uses the controls' accessible labels and geometry.

The latest builds are installed on Mini and both Samsung devices. Real roster screenshots are saved locally as `/tmp/crabigator-columns-mini.png`, `/tmp/crabigator-columns-tablet.png`, and `/tmp/crabigator-style-android-tabs.png`. These private artifacts are not tracked in Git.

### Move active sessions into the current time band

The ten-minute ordering cache also retained a session's old time band after its prompt timestamp changed. Both apps now update band membership on each roster refresh while preserving unrelated row order, repository order and PR order. New prompts and working states move a session into Last hour; ordinary aging also changes bands. The normal ordering refresh still runs on its original schedule.

Fifteen iOS unit tests passed on ClayPad Mini, including prompt changes after a quick completion, working-state changes before their timestamp arrives, ordinary aging, and unchanged unrelated ordering. Result: `/tmp/crabigator-live-age-mini.xcresult`. The updated app's real roster places “Portal Fix and Eval Coverage PRs” in Last hour (`/tmp/crabigator-live-age-mini.png`).

Android passed 30 unit tests, lint, app/test builds, and a physical-tablet UI regression that updates the session while the PR board stays open and measures its move from below 1–3 hours to above that heading. Logs: `/tmp/crabigator-live-age-android.log` and `/tmp/crabigator-live-age-device.log`. The update is installed on Mini and both Samsung devices. Code simplifier review found no further issues.

### Codex activity and Older grouping

The affected desktop pane had no attached transcript, zero prompts/completions/tools, and no activity timestamps. Codex had automatically reopened an older conversation without creating a new rollout or shell snapshot. The desktop now matches its native terminal title to a unique entry in Codex's thread index, checks the repository and ownership, and handles a title that arrives after the first stats poll. Ambiguous names are not guessed.

The parser now recognizes modern task completion and custom-tool events, excludes commentary from completed-turn counts, and retains transcript timestamps when replaying a resumed conversation. Mixed old/new logs keep their prior completion totals. PR-board transcript fallback also recognizes the newer activity records.

The Rust suite passed 673 tests with seven opt-in tests skipped. The additional modern-board activity regression and final discovery regression passed separately. A read-only probe of the actual conversation recovered 21 prompts, five completed turns and 205 tools at the time of capture, with recent activity timestamps and Thinking state (`/tmp/crabigator-live-codex-final.log`). The local Debug binary is rebuilt; existing Crabigator desktop processes need restarting to load it. Native Codex scrolling remains enabled. Clippy completed with two existing warnings in `src/pr/budget.rs` and `src/prs_board.rs` outside these changes.

### PR row layout after collapsing the board

A screenshot from the user's Mini showed the selected second PR-board row extending off the left edge while its terminal remained correct. The row appeared to retain full-width columns inside the sidebar. Session rows now rebuild their column subtree when the width mode changes, and that subtree does not inherit selection/width animations. Selection highlighting and the board's width animation remain outside this subtree.

The live device check switches between two sessions through fullscreen, verifies the selected row and its visible title remain within the sidebar, and repeats in portrait and both landscape orientations. The original screenshot is `/tmp/crabigator-mini-layout-before.png`. Automated attempts on the previous build did not reproduce the intermittent failure, so they are coverage of the transition rather than proof of its exact cause. The updated device result is `/tmp/crabigator-mini-row-final.xcresult`.

At 15:35 on October 13, Mini still showed several wide rows extending off the sidebar's left edge (`/tmp/crabigator-mini-collapse-before.png`). After installing the latest build from `0d97359`, three repeated fullscreen-to-sidebar transitions passed on the physical Mini. The strengthened check validates every visible session row and its PR text, not just the selected title. It stays in landscapeRight. The selected row measured 336 points wide; no clipping reproduced after reinstall. This does not establish that an old build caused the earlier stale layout. Result: `/tmp/crabigator-mini-collapse-current.xcresult`. Standalone PR-group headers and offscreen scrolling are outside this check.

### Refresh PR columns in both width modes

At 16:59, Mini showed the reverse problem: the first session kept its narrow layout after expanding the board, while later rows used the correct wide columns. The lazy stack now rebuilds when the board crosses its wide/narrow threshold. The scroll view remains in place. The fullscreen screenshot after the change shows merge state and diff beside CI on the PR title's line (`/tmp/crabigator-pr-status-now.png`). At this stage, narrow phone/sidebar layouts still wrapped status when the available width could not fit all columns; the follow-up below removes that fallback.

Both apps now place unresolved comments immediately after CI and leave that cell blank at zero. Merged secondary PRs are hidden per session, using the new per-session classification from the Worker. Merged primary and explicitly watched PRs remain visible. Live screenshots on Mini and the Samsung tablet confirm the Safety session no longer shows its merged secondary PRs #1840 and #3280; its merged primary PRs remain.

Twenty iOS unit tests and 35 Android unit tests passed. Android lint, app/test builds and three physical-tablet PR interaction tests passed, including blank comment cells and stable column order. The updated builds are installed on Mini, ClayPhone and both Samsung devices. Android logs: `/tmp/crabigator-pr-status-android-build.log` and `/tmp/crabigator-pr-status-android-device.log`.

The physical Mini UI test passed three fullscreen/sidebar cycles in `/tmp/crabigator-mini-pr-status-verified.xcresult`, with visible sidebar text inside its row and fullscreen PR states on the right beside their titles. Each cycle must check at least one PR status. The test stayed in landscapeRight. An earlier run's test runner was killed without a layout assertion failure; the passing rerun selected a smaller session. The cause of that runner exit is not established. The unit result is `/tmp/crabigator-mini-pr-status-final.xcresult`.

Worker type checking and all 59 tests passed. Production version `de4500d0-db6c-4dd4-96f2-1d9979c35bce` is deployed; health reports `ok`, and an unauthenticated board request returns 401. The code simplifier found no production issues and requested a nonzero count of checked PR statuses in the live layout test; that assertion is now included.

### Keep compact PR status on one line

Phone and sidebar PR rows now keep state, diff, CI, comments, approval and branch status on one line. Both apps reduce the gaps and reserved column widths below 320 points/dp; status text can shrink within its column. Android also centers differently scaled diff values vertically.

ClayPhone's live PR-board test passed, and its saved screenshot shows state/diff/CI together with comments, approval and conflicts. A strengthened check passed on Mini's sidebar: visible states and diff counts must share CI's line, and CI must stay within the owning session row. Results: `/tmp/crabigator-phone-compact-status.xcresult` and `/tmp/crabigator-mini-compact-status-retry.xcresult`. Mini's first attempt failed before running the test because the device connection closed; the retry passed. Code simplifier review found no remaining issues.

Android passed 35 unit tests, lint, both builds and three physical-tablet PR tests. The compact test uses large diff counts and 999 failed checks at the smallest/largest board text sizes, verifies one-line alignment and checks for text overflow. Logs: `/tmp/crabigator-compact-status-android-final.log` and `/tmp/crabigator-compact-status-android-device-final.log`. The update is installed on ClayPhone, Mini and both Samsung devices.

### Board settings, actions and review columns

Both apps expose activity cutoff, live/all sessions, computers, recaps and session/PR grouping. The cutoff defaults to 24 hours. Grouping retains older touching sessions beneath a recent PR after applying the computer/live/search filters. New session selects a desktop, folder and assistant; Watch PR accepts a GitHub URL or owner/repo#number.

The Worker spawn route checks session ownership and sends the request only to the chosen desktop. Worker type checking and 59 tests passed in `/tmp/crabigator-board-actions-worker.log`. Production deployment `6e7a441c-a4cc-4266-bc85-dc8f9f35b1fd` succeeded; an unauthenticated production request returned 401. The live form checks do not submit a spawn or watch request.

The board now has four separate review/status columns: unresolved comments, approval, CI and behind/conflicts. Session glyphs center on the first title line at tight spacing. Prompt/completion times sit directly beside the status throbber. Narrow PR rows put checks on a second line to preserve readable diffs. Floating-header gaps consume taps.

Android passed 34 unit tests, lint and both APK builds. Device checks passed for all three PR interaction/column tests, tight-spacing glyph geometry, floating-header gap taps and timeline layout at large font sizes. The last check also caught and fixed hidden durations when both recency columns were disabled. Logs: `/tmp/crabigator-board-review-final-device.log` and `/tmp/crabigator-timeline-duration-device.log`. The first log has an older timeline assertion failure; the second passes the corrected check and duration fallback. Updated APKs are installed on the Samsung tablet and S22. The real board screenshot is `/tmp/crabigator-board-reviews-tablet.png`.

ClayPad Mini passed 19 unit tests and two live UI tests in `/tmp/crabigator-board-complete-mini.xcresult`. These verify saved cutoff after relaunch, the Watch PR and New session forms, and gap taps followed by working Settings controls in both fullscreen views. The test restores the 24-hour cutoff and keeps the device in landscapeRight. Earlier attempts exposed invalid native-popover hit points and taps beneath the navigation bar; the final test uses visible screen bounds and scrolls controls clear of that bar.

Final accessibility review restored the session state in combined timing labels after moving the throbber. The final Mini build passed its 19 unit tests and repeated header-gap UI check in `/tmp/crabigator-board-accessibility-mini.xcresult`. Android's large-text timeline and accessible state check passed in `/tmp/crabigator-board-accessibility-final-android.log`. The final builds are installed on Mini and both Samsung devices.

### Shared CLI status columns

Session activity and PR metadata now occupy the same right-hand columns, leaving more width for titles. In PR grouping, every session sub-row shows its own state and prompt/completion ages, while the PR header holds GitHub status. Watched PRs without sessions retain an Updated age below the header. Width checks cover narrow rows, workspace diffs, recaps and the repeated identity chip.

The Rust suite passed 675 tests with seven opt-in tests skipped. Clippy, Debug and Release builds passed; Clippy retained two existing warnings outside these changes. A real `prs --once` frame confirms the shared layout. Logs: `/tmp/crabigator-consolidated-full-tests.log`, `/tmp/crabigator-consolidated-clippy.log` and `/tmp/crabigator-consolidated-live-board.txt`. Restart an existing desktop board to load the rebuilt binary.

On October 12, Xcode automatic signing succeeded with the NewsBlur team (`HR7P97SD72`). The signed Debug build and notification extension installed on both physical devices. Each device claimed its own pairing code, and screenshots confirmed that the live session roster loaded. This verifies installation, launch, pairing and roster access; it does not establish full device interaction or push-delivery coverage.

## Restoring the board and selected session

iOS and Android now save the List/PR board choice, selected session ID, and tablet sidebar/full-width choice. The first successful roster refresh resolves the saved ID and reconnects its terminal. Closing a session clears the saved selection; an unavailable saved ID falls back to the board. A delayed iOS deep link cannot replace a newer manual selection.

The iPhone simulator and physical ClayPad passed a terminate/relaunch UI test that restores a selected session and PR board, preserves a full-width tablet board with its selection, then remembers an explicit close and the List choice. The Samsung S22 and tablet passed force-stop/relaunch checks in both modes; the tablet also retained both widths. Twelve iOS unit tests and 28 Android unit tests passed. Updated builds are installed on all four physical devices. The physical iPhone UI runner timed out enabling automation; its installation and launch succeeded separately.

The reported PR #1750 session had a stuck desktop cloud connection. The Worker now reconnects a desktop that fails to answer missing-screen requests for ten seconds and announces when the replacement connection is ready. Fifteen Worker tests passed, including a hibernation/recovery check. That particular desktop process predated the October 9 reader-disconnect fix, so automatic recovery alone did not repair it. At the user's request, it was exited and resumed in the same Ghostty tab with the installed binary and the same Claude conversation UUID. Its replacement cloud session is `f2be6c06-1aa3-48f7-ae2b-2eec06d6129f`. The full streamed screen matched the local captured screen, and the iPad then passed the restoration test using that specific resumed session.

Results: `/tmp/crabigator-restore-sim.xcresult`, `/tmp/crabigator-restore-ipad-navigation.xcresult`, `/tmp/crabigator-restore-resumed-ipad.xcresult`. Android checks and screenshots: `/tmp/crabigator-android-restoration.py` and `/tmp/crab-R5CT601C8DA-*-restored.png`, `/tmp/crab-R8YW810E3FF-*-restored.png`. Worker deployment: `4c9690be-f223-49f9-9d79-d220ceb234dc`.

## Initial terminal loading

The iOS stream previously opened its socket, then awaited the mobile prompt snapshot before reading terminal frames or starting viewer heartbeats. A slow metadata request could therefore hide output already waiting on the socket. Prompt refreshes now run independently, cancel when replaced or disconnected, and discard canceled responses. An eight-second wait now shows “Waiting for terminal output…” with a Reconnect button instead of claiming the terminal is empty.

A live regression check deliberately delays prompt metadata by 15 seconds. It failed before the fix and passed afterward: terminal text appeared in 1.66 seconds on the simulator, 1.81 seconds on ClayPhone, and 1.88 seconds on ClayPad, including the tap and page transition. A second check holds the stream and verifies that the waiting state remains after eight seconds. Both checks passed on each device; twelve unit tests also passed on the simulator and phone. Direct production socket probes received cached screens in about 0.54 seconds and recovered a missing screen from the desktop in 1.28 seconds. The server cache remains in memory and can be lost during hibernation; this change does not add persistent terminal storage.

Results: `/tmp/crabigator-load-before.xcresult`, `/tmp/crabigator-load-after.xcresult`, `/tmp/crabigator-load-ipad.xcresult`, and `/tmp/crabigator-load-phone-final.xcresult`. Live checks require an already paired device and `TEST_RUNNER_CRABIGATOR_LIVE_UI_TESTS=1` when invoked with xcodebuild. They never send terminal input.

## Terminal scrolling momentum

Live frames previously edited TextKit storage and restored the content offset during native deceleration, cutting off momentum. The viewer now keeps the latest source while either scroll view is dragging or coasting, applies it when scrolling settles, and honors a pending Follow action even when no new output arrives.

A repeatable UI test streams ten frames per second into 1,000 history rows. Before the fix, the physical iPhone stopped coasting within 350 milliseconds and moved only 20 points after release. After the fix, it was still coasting at that sample and had moved 325 points. The simulator reproduced the failure and passed after the fix too. Both passing runs also confirmed live output resumes afterward. Twelve unit tests passed, including the pending Follow case. The updated signed app is installed on the phone and iPad; iPad UI automation timed out while enabling automation, so the physical momentum proof is from the phone.

Results: `/tmp/crabigator-momentum-phone-before.xcresult`, `/tmp/crabigator-momentum-phone-after.xcresult`, and `/tmp/crabigator-momentum-final.xcresult`.

## Independent text sizes

Style now saves List / PR board text size separately from Terminal text size. The default board size preserves the existing layout. An iPhone simulator UI check changed the board size, verified the terminal setting stayed unchanged, and confirmed both values survived restarting the app. Eleven unit tests passed, including decoding older preferences without resetting terminal settings. The signed update installed on ClayPad Air and ClayPhone SE 3; the iPad launched, while the phone was locked. The matching Android update passed 28 unit tests and was installed on both Samsung devices. The S22 visual check covered list and PR board text at size 24 while terminal text stayed at 9.

Results: `/tmp/crabigator-independent-text-sizes-final.xcresult`. The UI check restored the simulator's original board size afterward.

## Phone navigation follow-up

Phone pages now use a `UINavigationController` with an interactive slide transition. The board stays underneath the entire detail page. A one-finger rightward pan works on the loading page as well as the terminal; cancelled swipes retain the session and its connection. Text editing, selections and terminal content already scrolled horizontally keep their own gestures. The tablet retains its sidebar and session paging.

Three physical ClayPhone UI tests passed: loading-page swipe/cancellation, loaded-terminal swipe/cancellation, and reading/scrolling/follow-output/style/keyboard/Back navigation. The swipe tests also reopen a session and use the Back button. A device recording showed the board beneath the moving loading page. The iPad simulator reading and sidebar regression check passed. Physical ClayPad automation reached Apple's Touch ID approval prompt, so that additional check was not completed.

Results: `/tmp/crabigator-phone-navigation-final.xcresult`, `/tmp/crabigator-ipad-navigation-final.xcresult`, and `/tmp/crabigator-iphone17e-navigation.xcresult`. Both loading and loaded swipe tests also passed on the iPhone 17e simulator. Phone screenshots are in the ignored `build/verification/phone-navigation/` directory. Earlier attempts exposed a full-height hidden UIKit toolbar intercepting touches, which is now explicitly hidden. Later test interruptions came from a notification banner over Back and an older session no longer supplying output; final tests wait out banners and use a currently streaming session. Code simplifier review found no remaining navigation issues.

## Codex question shortcut

The Questions shortcut now sends Shift+Left (`ESC[1;2D`) and displays that binding. Its old Alt+Up binding no longer matches Codex 0.162.1's queued-question UI. Manual Shift plus Left already sent the correct bytes, verified from ClayPhone by a PTY input recorder in an isolated Codex session.

A live iPhone 17e simulator test opened an asynchronous Codex question with manual Shift+Left, returned to the composer with Shift+Right, then reopened the question with Questions. The input recorder confirmed `ESC[1;2D`, `ESC[1;2C`, `ESC[1;2D`, and the current desktop screen displayed the unanswered options. No test key was sent to an existing user session. The physical phone locked before the pending-question rerun; the first physical byte check and the simulator question-flow check are separate evidence.

The focused key-encoding unit test and live question UI test passed in `/tmp/crabigator-codex-questions-final.xcresult`. Screenshots and the filtered key-byte log are in the ignored `build/verification/codex-questions/` directory. The updated signed device build succeeded. Installing that keyboard update on ClayPhone then failed because its connection timed out; the navigation update is installed, but the Questions shortcut update still needs installation after reconnecting the phone.

## Passed

- Debug simulator build, including the notification service extension.
- Optimized arm64 device build with signing disabled. This is compile proof only; it cannot be installed without provisioning.
- Ten unit tests on each simulator: ordering retention/new arrivals/returning machines, age grouping, prompt action guards, review/permission handling, terminal keys, origin validation, ANSI styling, registered symbol font and fixed cell metrics, silent notification content and revision identity.
- Live UI reading check on both devices: List → PR board → existing session, real terminal/history loading, no keyboard on a terminal tap, scroll-up unpins, follow-output repins, return to the board. iPad additionally checks mode switches retain sidebar width and the selected segment expands it.
- Expanded menu checks on both devices: Style controls appear in native popovers, terminal keyboard switches to QWERTY, and the popovers dismiss normally.
- Worker TypeScript check and full Worker test command: 57 Vitest tests across ten files, plus the ANSI/scrollback Node checks. Apple-specific coverage includes token/environment validation, signed alert/background payloads, silent defaults and provider-failure isolation.
- Code simplifier review, followed by fixes for continuous-render starvation, delayed swipe identity, tracked-PR collapse/count and duplicate notification presentation.
- `git diff --check`.

## Scroll measurement

The optimized iPhone simulator run completed three full eight-gesture measurements. It recorded approximately 76.9 MB average physical memory and 84.9 MB average peak memory. Each sequence took approximately 35.9 seconds, including XCTest accessibility queries and gesture synchronization. These timings are a reproducible baseline, not a frame-rate measurement or proof of physical-device smoothness. The selected existing session was settled; sustained incoming output still needs an instrumented device run.

Result bundles are `/tmp/crabigator-reading-verified.xcresult`, `/tmp/crabigator-ipad-menus.xcresult`, `/tmp/crabigator-phone-menus.xcresult`, and `/tmp/crabigator-scroll-verified.xcresult`. Screenshots and copied logs are under the ignored `clients/ios/build/verification/` directory. They contain private session content and are not included in Git.

## Still required

- Exercise the remaining phone/tablet interactions on the physical devices. Signing and installation are now complete.
- APNs credentials and real transport delivery are verified as of October 13; production prompt dispatch and action flows still need their separate end-to-end checks.
- Verify sound off/on, lock-screen choices and replies, stale-revision rejection, cross-device resolution and revoked pairing on real devices.
- Verify microphone permission, recording, cancellation, transcription and background interruption on a physical device.

The updated app is left paired on ClayPad Mini. A production app release has not been requested.

## Header new-session picker — October 18, 2026

The header + opens the project and assistant picker from List, PR board and Conductor. The iOS simulator UI test `ConductorTests/testNewSessionHeaderShowsProjectAndAllAssistants` passed, including visible assistant choices, a visible Start action, custom-folder validation and navigation back to List. The signed ClayPhone build passed and installed. After unlocking ClayPhone SE 3, the same UI test passed on the physical device, with all four assistants and Start visible together. The app was relaunched without fixture flags afterward.

Galaxy S22 and Tab A8 each passed three instrumentation checks: `NewSessionTest` verifies all three header entry points, correct computer/path/assistant routing through a captured request, recovery when a computer disappears, and retained request state without duplicate submission after reopening; `MobileParityTest#boardSettingsPersistAndExposeActions` verifies the existing Settings entry. Both devices received the debug build. These checks did not launch a real desktop assistant. Logs are under `/tmp/crabigator-new-session-*`; screenshot evidence is under ignored `target/conductor-evidence/`.
