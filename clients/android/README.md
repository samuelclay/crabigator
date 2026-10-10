# Crabigator for Android

Native Kotlin and Jetpack Compose client. The session list opens first. The icon toolbar switches between the board and session list, filters live or ended work, searches, and opens an animated Settings popover. Selecting a session opens its live terminal. The Reply control opens the composer and terminal keys. At 720 dp and wider, the board stays beside the session.

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

Enable notifications in the app's Settings. A data-only FCM message carries a session ID. The app authenticates and fetches the latest prompt before displaying or cancelling a notification. This makes delayed and out-of-order deliveries converge on current state. WorkManager handles short background work, retries refreshes, and reconciles shown notifications when the app resumes.

Choices and direct replies use the same terminal input translator as in-app questions. Each action includes the server's prompt revision. The session Durable Object rejects stale or duplicate replies and immediately signals other devices after accepting an answer. Replies are never automatically retried after an ambiguous network failure. Multi-select and multi-page prompts continue in subsequent notifications; opening a notification always offers the complete native question controls.

Android and FCM may delay background work under battery restrictions or while offline. Notification clearing is best effort during those periods; reopening the app reconciles current state. Notifications hide question text on a locked screen by default and require unlock for actions. Force-stopping the app prevents delivery until it is opened again.

## Terminal

The server supplies complete row-formatted ANSI snapshots. `TerminalText` converts foreground/background colors, bold, underline, inverse, and horizontal spacing into native text spans. Native text wraps to the detail pane by default; Style can switch to horizontal scrolling to preserve the desktop columns. The pin control follows new output; scrolling up releases it. Transcript view uses the normalized conversation log, cached for late joiners by the Worker. Empty output has an explicit empty state. Viewing a session sends a five-second foreground heartbeat so the desktop keeps its screen current; no foreground service or background screen polling is required.

## Appearance

The detail pane’s top-right Style menu contains text size, line spacing, wrapping, terminal/transcript content, terminal height, widget visibility, list columns, project grouping, and project order. Settings contains list position/density, visible statistics, notifications, account identities, pairing another device, MCP, and unpairing. Choices are saved on this device. Menus are anchored to their toolbar icons, fade and scale into view, and scroll within the available phone or tablet space.

## Verification

Unit tests cover terminal color/spacing and question input translation. Worker tests cover prompt revisions, simultaneous replies, hibernation, auth/account isolation, and FCM token lifecycle. Device checks should cover phone and tablet layouts, rotation, Reply/IME, terminal selection, option and free-text replies, background notification delivery, desktop resolution, and an old notification racing a newer question.

The first builds are direct debug installs, not a Play Store release. Release signing, store listing, and store submission are separate work.
