# Crabigator for iPhone and iPad

A native SwiftUI app with a read-only UIKit terminal. Requires iOS 17 or later. The Android app's final behavior is the reference; see [the commit-by-commit port log](docs/parity.md).

## Build and run

Open `Crabigator.xcodeproj`, select Crabigator, and choose an iPhone or iPad. The project uses the NewsBlur team (`HR7P97SD72`), application ID `com.newsblur.crabigator`, and shared app group `group.com.newsblur.crabigator`.

```sh
xcodebuild -project clients/ios/Crabigator.xcodeproj -scheme Crabigator \
  -destination 'platform=iOS Simulator,name=iPhone 17e' \
  -derivedDataPath clients/ios/build build
```

If several runtimes have an iPhone 17e, use its simulator ID instead of its name. Install with `xcrun simctl install DEVICE clients/ios/build/Build/Products/Debug-iphonesimulator/Crabigator.app`. Launch with `xcrun simctl launch DEVICE com.newsblur.crabigator`.

Run `crabigator pair` on a paired computer, then enter its code in the app. The Server control supports a compatible HTTPS deployment. Pairing credentials live in a device-only shared Keychain entry; neither credentials nor prompts are included in notification transport payloads.

The checked-in project is ready to open without extra packages. To regenerate it after adding source files, install the Ruby `xcodeproj` gem and run `ruby clients/ios/scripts/generate-project.rb`. The icon comes from Android's vector geometry, rendered by `swift clients/ios/scripts/render-icon.swift`.

## Apple notifications

The app uses APNs directly. Firebase continues to serve Android. Enable Push Notifications for the application ID, register the NotificationService extension ID, and enable the shared app group for both targets. Xcode automatic signing needs the NewsBlur developer account signed in. A development certificate alone cannot create the required profiles.

The Worker requires migration `0023_apple_push.sql` and an `APNS_CREDENTIALS` secret with these fields:

```json
{"team_id":"HR7P97SD72","key_id":"YOUR_APNS_KEY_ID","topic":"com.newsblur.crabigator","private_key":"YOUR_P8_KEY_CONTENTS"}
```

Store the secret through Wrangler's secret input; never put it in this repository. Deploy using the repository's official deployment instructions after reviewing the prepared changes. Migration 0023 preserves existing Android registrations. Tests cover independence between Apple and Firebase provider failures.

The device registers its token, signing environment, and sound preference after pairing. Enable notifications in Settings → Notifications. Sound defaults to off. The notification service extension loads the current prompt using the shared Keychain credential. Choices and free-text replies carry the displayed prompt revision; changed or disconnected prompts cannot receive stale answers. Ambiguous input failures are never retried by the app.

Apple background delivery is best effort. A resolved prompt already in transit can still produce a passive “resolved” notification; foreground/background reconciliation removes stale notifications. The service extension cannot retract an alert before delivery. See Apple's [notification request documentation](https://developer.apple.com/documentation/usernotifications/sending-notification-requests-to-apns) and [service extension documentation](https://developer.apple.com/documentation/usernotifications/modifying-content-in-newly-delivered-notifications).

## Verification

See [the current verification report](docs/verification.md) for results and pending physical-device checks.

```sh
xcodebuild -project clients/ios/Crabigator.xcodeproj -scheme Crabigator \
  -destination 'platform=iOS Simulator,id=DEVICE' \
  -derivedDataPath clients/ios/build -only-testing:CrabigatorTests test
```

The live UI check is opt-in and requires an already paired simulator with a session containing scrollback. It opens List, PR board, a live terminal and Style, verifies read-only terminal taps and follow-output, and returns to the board. On tablets it checks that changing modes preserves the sidebar. It does not send input to a session.

```sh
TEST_RUNNER_CRABIGATOR_TEST_SESSION_ID=SESSION_ID TEST_RUNNER_CRABIGATOR_LIVE_UI_TESTS=1 xcodebuild \
  -project clients/ios/Crabigator.xcodeproj -scheme Crabigator \
  -destination 'platform=iOS Simulator,id=DEVICE' \
  -derivedDataPath clients/ios/build -only-testing:CrabigatorUITests/ReadingTests \
  -resultBundlePath /tmp/crabigator-reading.xcresult test
```

For repeatable native scroll measurements, use `scripts/profile-scroll.sh DEVICE SESSION_ID`. Each XCTest measurement includes its complete gesture sequence; screenshots and measurements are retained in the result bundle. Simulator timing is useful for regressions but does not prove physical-device frame rates.

`docs/user-prompts.local.md` is a private research export excluded from Git. Regenerate it with `python3 clients/ios/scripts/collect-history.py`. It collects this repository's user messages from October 7 onward, excluding subagent copies, system instructions, assistant responses, and tool output. `docs/android-commits.md` records the corresponding Git history.
