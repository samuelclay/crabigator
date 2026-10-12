# iOS verification — October 12, 2026

The native app is installed, paired and running on the physical ClayPad Air and ClayPhone SE 3, as well as the iPhone 17e and iPad Air simulators. Live Apple notifications are not configured yet.

On October 12, Xcode automatic signing succeeded with the NewsBlur team (`HR7P97SD72`). The signed Debug build and notification extension installed on both physical devices. Each device claimed its own pairing code, and screenshots confirmed that the live session roster loaded. This verifies installation, launch, pairing and roster access; it does not establish full device interaction or push-delivery coverage.

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
- Provide an APNs signing key for the NewsBlur team and configure the Worker secret. Migration 0023 and the Worker deployment completed on October 12; both existing Android registrations were preserved and the health check passed. Apple delivery remains unconfigured.
- Verify actual APNs delivery, sound off/on, lock-screen choices and replies, stale-revision rejection, cross-device resolution and revoked pairing on real devices.
- Verify microphone permission, recording, cancellation, transcription and background interruption on a physical device.

The app is left paired and running on ClayPad Air and ClayPhone SE 3. Production app release and Apple push credentials remain pending.
