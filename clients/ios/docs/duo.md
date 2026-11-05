# iPhone Duo layout

The app uses the window's horizontal size class to switch between a single session and a sidebar with a terminal. It uses the space SwiftUI offers after safe-area insets, including the Duo camera and status area.

When the inner display has an active vertical fold, both columns stop at the fold's margins. Either sidebar position follows those margins. On a flat display, the sidebar takes up to 360 points and leaves the remaining width for the terminal. Expanded-board controls and search stay on the trailing side of an active fold; continuous scrolling content can span it.

In tabletop posture, the board and session toolbars move to the bottom. Settings and Style popovers can shrink to fit the available space. Closing and reopening the phone retains the selected session and draft; keyboard focus may close when the navigation layout changes.

## Verification — November 5, 2026

Built on claybook-tavus with Xcode 27.2 (27B52) and the iOS 27.2 SDK for version 1.0, build 3.

- All 46 unit tests passed, including asymmetric fold geometry and both sidebar positions.
- The rotation UI test passed on an iPhone 18 Pro and an iPad Pro simulator. It checks selection, draft retention, accessible controls, and the terminal keyboard.
- Manual checks on the iPhone Duo simulator (iOS 27.1, 24A94425) covered the outer display, flat inner display, partially folded book and tabletop postures, rotation, split keyboard, closing and reopening with a draft, expanded board, and Style popover placement.
- The automated Duo run timed out waiting for an orientation change; its screenshot attachments were black. Duo posture verification used Device Hub directly instead.
- These are simulator checks with sample sessions, not physical Duo or live network verification. The layout fixture is DEBUG-only and should run on a disposable simulator because it resets preferences.

The fixture uses `CRABIGATOR_TEST_LAYOUT=1`. Run the automated check with `-only-testing:CrabigatorUITests/NavigationTests/testLayoutRetainsSessionAndDraftAcrossRotation`. Use Device Hub's Closed, Partially Open, Open, and rotation controls for posture checks.

Apple references: [Prepare your app for iPhone Duo](https://developer.apple.com/videos/play/tech-talks/111461/) and [Strike a pose](https://developer.apple.com/videos/play/tech-talks/111463/).
