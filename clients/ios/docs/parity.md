# Android → iOS port log

Reference: Android at `a38d056`, including 30 commits from October 7–11, 2026. The local prompt export contains 67 user messages. Current source and the last change to each behavior take precedence over earlier requests.

## Final decisions

- List is the default. List and PR board keep separate scroll positions. The floating selected segment expands or collapses the tablet browser; changing modes keeps a selected session's sidebar open.
- Machine order survives temporary disappearance. Row positions stay fixed for ten minutes, except genuinely new session IDs cause an immediate reorder. Labels and status continue updating.
- Machine headers are sticky. List cards use a fixed identity column, a narrow status column, and a flexible title/timing column. Completion age retains “ago”; prompt and completion use distinct oranges. Tool and compaction counts start hidden.
- PR board is dense, grouped by machine, age and repository, with each session's PRs underneath. Tapping its session block opens the session; header PR buttons open GitHub. iPad full-width columns share alignment.
- Terminal content combines actual terminal history and the current screen. It does not substitute the assistant's normalized transcript. ANSI parsing runs off the main thread; text layout and selection remain native. Output updates do not publish changes to the board.
- Terminal taps are read-only. The later bottom composer replaces the original explicit Reply entry point. Drafts remain per session; recording stops when hidden or backgrounded. The keyboard menu exposes modifiers, shortcuts and QWERTY letters.
- Notifications cover both questions and permissions, start silent, and guard every prompt action by revision. Apple uses a notification service extension and APNs, rather than Android services or FCM.

## Commit ledger

| Android commit | Final behavior carried to iOS | Implementation |
| --- | --- | --- |
| `28573ea` | Pairing, session roster, terminal, widgets, prompt actions, notification replies and tablet support. Original PR-first/Reply layout superseded below. | `Shared/API.swift`, `App/SessionModel.swift`, `Views`, `NotificationService`, Worker Apple provider |
| `3430d22` | Native style/settings menus, readable output, account/invite/MCP controls. | `Views/SettingsMenu.swift`, `Terminal/TerminalText.swift` |
| `12e10e9` | Cancellable two-minute voice capture, waveform, edit/send, draft preservation. | `App/VoiceRecorder.swift`, `Views/SessionComposer.swift` |
| `480bd42` | One continuous output viewport and native bottom composer. | `Terminal/TerminalViewport.swift`, `Views/SessionComposer.swift` |
| `8ae6299` | ANSI dim and intensity reset. | `Terminal/TerminalText.swift` |
| `abff75b` | Animated session dismissal and compact phone popovers. | `App/CrabigatorApp.swift`, `Views/SessionDetail.swift`, `Views/SettingsMenu.swift` |
| `37d4e8a` | Send appears only for nonblank text, with matching control height. | `Views/SessionComposer.swift` |
| `f10f815` | QWERTY rows and persistent modifier switches. | `Views/SessionComposer.swift`, `Shared/PromptActions.swift` |
| `5a9141c` | Phone navigation keeps the board visible under interactive swipes, including while loading. Tablet session paging retains frozen outgoing content. | `App/PhoneNavigation.swift`, `Terminal/TerminalViewport.swift`, `App/CrabigatorApp.swift` |
| `630f5ea` | Silent notifications by default, optional sound. | `App/Notifications.swift`, `Shared/NotificationContent.swift`, Worker Apple provider |
| `40782b6` | Status-arrow glyph coverage. | Bundled Noto Symbols font and `Terminal/TerminalText.swift` |
| `b663c49` | Fixed symbol advances and line height while spinners change. | `Terminal/TerminalText.swift`, unit tests |
| `2a95bb7` | Off-main parsing, coalesced updates, native viewport, separate stream observation. | `Terminal/TerminalViewport.swift`, `App/SessionModel.swift` |
| `3157feb` | Repeatable scroll profiling adapted to XCTest native metrics. | `UITests/ScrollPerformanceTests.swift`, `scripts/profile-scroll.sh` |
| `7f8b057` | Measurements encompass whole gestures; no truncated trace is reported as complete. | XCTest measurement boundaries and result bundle |
| `34d6107` | Aligned session cards with simpler timing. | `Views/BoardView.swift`, `Views/Theme.swift` |
| `f5debf0` | Machine groups and compact session cards. | `Shared/RosterOrder.swift`, `Views/BoardView.swift` |
| `abc96cc` | Distinct loading, empty and failed roster states, bounded terminal loading indicator. | `App/SessionModel.swift`, `Views/BoardView.swift`, `Views/SessionDetail.swift` |
| `ecb91ec` | Completion suffix stays visible. | `Views/Theme.swift` |
| `e742319` | Terminal palette recency colors. | `Views/Theme.swift` |
| `ad37649` | Clear follow-output control and distinct toolbar buttons. | `Views/SessionDetail.swift` |
| `526f097` | Native toolbar hit areas. | `Views/Theme.swift` |
| `e4febe1` | Dense PR board, floating mode switch, full-width tablet browser. | `Views/BoardView.swift`, `App/CrabigatorApp.swift`, `Shared/RosterOrder.swift` |
| `35ca0b0` | Mode changes retain the tablet sidebar. | `Views/BoardView.swift`, live UI test |
| `77192d9` | Top-aligned shared board columns; session blocks open sessions. | `Views/BoardView.swift` |
| `1c7cba9` | Extra space around header PR links. | `Views/SessionDetail.swift` |
| `88d2a45` | Ten-minute position retention. | `Shared/RosterOrder.swift`, unit tests |
| `916018b` | Returning machines retain their original position. | `Shared/RosterOrder.swift`, unit tests |
| `ef4eb27` | Preserve scrollback position, selection and history while replacing the live screen. | `Terminal/TerminalViewport.swift` |
| `a38d056` | A new session immediately recomputes positions and resets the ten-minute clock. | `Shared/RosterOrder.swift`, unit tests |

## Verification boundaries

See [the verification report](verification.md) for exact completed checks and remaining device work.

Phone and iPad simulator pairing and live roster rendering have been exercised. Focused unit tests and Worker tests pass. Live UI tests cover navigation and reading; result bundles contain screenshots. Physical ClayPad and ClayPhone signing, installation, launch and pairing passed on October 12. Real APNs delivery remains pending its signing key and backend setup. Migration 0023 and the notification Worker were deployed on October 12; Apple delivery still needs its signing credential. Microphone/transcription and lock-screen direct reply still require physical-device checks. Do not treat a simulator build or synthetic push as proof of real APNs delivery.
