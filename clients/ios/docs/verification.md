# iOS verification — October 12, 2026

The native app is installed, paired and running on the physical ClayPad Air and ClayPhone SE 3, as well as the iPhone 17e and iPad Air simulators. Live Apple notifications are not configured yet.

On October 12, Xcode automatic signing succeeded with the NewsBlur team (`HR7P97SD72`). The signed Debug build and notification extension installed on both physical devices. Each device claimed its own pairing code, and screenshots confirmed that the live session roster loaded. This verifies installation, launch, pairing and roster access; it does not establish full device interaction or push-delivery coverage.

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
- Provide an APNs signing key for the NewsBlur team, apply migration 0023, configure the Worker secret and deploy the reviewed notification changes. None of those production operations has been performed.
- Verify actual APNs delivery, sound off/on, lock-screen choices and replies, stale-revision rejection, cross-device resolution and revoked pairing on real devices.
- Verify microphone permission, recording, cancellation, transcription and background interruption on a physical device.

The app is left paired and running on ClayPad Air and ClayPhone SE 3. Production release and Apple push deployment remain pending.
