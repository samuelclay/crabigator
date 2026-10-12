# Android commits reviewed for the iOS port

Chronological history. The current Android sources determine the final behavior when later commits revise earlier choices.

commit 28573eaf3412dc8c46f4e27a8eb84a3ef3c685f7
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Fri Oct 9 20:23:40 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Fri Oct 9 20:23:40 2026 -0700

    Add the native Android app with session views and notification replies

 clients/android/.gitignore                         |   6 +
 clients/android/README.md                          |  44 ++++
 clients/android/app/build.gradle.kts               |  42 ++++
 clients/android/app/src/main/AndroidManifest.xml   |  11 +
 .../app/src/main/java/com/crabigator/app/Api.kt    | 114 ++++++++++
 .../main/java/com/crabigator/app/MainActivity.kt   | 241 +++++++++++++++++++++
 .../main/java/com/crabigator/app/Notifications.kt  | 165 ++++++++++++++
 .../main/java/com/crabigator/app/PromptActions.kt  |  75 +++++++
 .../main/java/com/crabigator/app/SessionModel.kt   | 146 +++++++++++++
 .../main/java/com/crabigator/app/SessionStats.kt   |  60 +++++
 .../main/java/com/crabigator/app/TerminalText.kt   |  81 +++++++
 .../android/app/src/main/res/drawable/ic_back.xml  |   3 +
 .../android/app/src/main/res/drawable/ic_board.xml |   3 +
 .../android/app/src/main/res/drawable/ic_check.xml |   3 +
 .../android/app/src/main/res/drawable/ic_close.xml |   3 +
 .../app/src/main/res/drawable/ic_crabigator.xml    |   4 +
 .../app/src/main/res/drawable/ic_external.xml      |   3 +
 .../app/src/main/res/drawable/ic_history.xml       |   3 +
 .../android/app/src/main/res/drawable/ic_list.xml  |   3 +
 .../android/app/src/main/res/drawable/ic_live.xml  |   3 +
 .../android/app/src/main/res/drawable/ic_reply.xml |   3 +
 .../app/src/main/res/drawable/ic_search.xml        |   3 +
 .../android/app/src/main/res/drawable/ic_send.xml  |   3 +
 .../app/src/main/res/drawable/ic_settings.xml      |   3 +
 .../app/src/main/res/drawable/ic_terminal.xml      |   3 +
 .../app/src/main/res/drawable/ic_transcript.xml    |   3 +
 clients/android/app/src/main/res/values/styles.xml |   1 +
 .../app/src/main/res/xml/data_extraction_rules.xml |   4 +
 .../java/com/crabigator/app/PromptActionsTest.kt   |  67 ++++++
 .../java/com/crabigator/app/SessionStatsTest.kt    |  38 ++++
 clients/android/build.gradle.kts                   |   5 +
 clients/android/gradle.properties                  |   3 +
 clients/android/gradle/wrapper/gradle-wrapper.jar  | Bin 0 -> 59203 bytes
 .../gradle/wrapper/gradle-wrapper.properties       |   6 +
 clients/android/gradlew                            | 185 ++++++++++++++++
 clients/android/settings.gradle.kts                |   4 +
 36 files changed, 1344 insertions(+)

commit 3430d222831917247ce36f3ff67b2ea47a4cdbcd
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sat Oct 10 11:29:10 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sat Oct 10 11:29:10 2026 -0700

    Add native style menus and improve Android session reading

 clients/android/README.md                          |   8 +-
 .../main/java/com/crabigator/app/MainActivity.kt   | 134 +++++++-------
 .../src/main/java/com/crabigator/app/MenuPanel.kt  | 198 +++++++++++++++++++++
 .../main/java/com/crabigator/app/Notifications.kt  |   1 +
 .../main/java/com/crabigator/app/SessionDetail.kt  | 137 ++++++++++++++
 .../main/java/com/crabigator/app/SessionModel.kt   |  22 ++-
 .../main/java/com/crabigator/app/SessionStats.kt   |   3 +
 .../main/java/com/crabigator/app/TerminalText.kt   |  14 ++
 .../main/java/com/crabigator/app/UiPreferences.kt  |  43 +++++
 .../android/app/src/main/res/drawable/ic_pin.xml   |   1 +
 .../android/app/src/main/res/drawable/ic_style.xml |   1 +
 .../java/com/crabigator/app/PromptActionsTest.kt   |   7 +
 12 files changed, 497 insertions(+), 72 deletions(-)

commit 12e10e95045d687b5a1d410dca36ca4635ebbad1
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sat Oct 10 12:07:39 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sat Oct 10 12:07:39 2026 -0700

    Add cancellable voice transcription for Android

 clients/android/app/src/main/AndroidManifest.xml   |  2 +
 .../app/src/main/java/com/crabigator/app/Api.kt    | 34 +++++++++++----
 .../main/java/com/crabigator/app/VoiceRecorder.kt  | 48 ++++++++++++++++++++++
 3 files changed, 75 insertions(+), 9 deletions(-)

commit 480bd421e3ec93264d39d3c28d9cf46420bfea1d
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sat Oct 10 12:07:39 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sat Oct 10 12:07:39 2026 -0700

    Unify Android session output and add native message controls

 clients/android/README.md                          |  12 +-
 .../main/java/com/crabigator/app/MainActivity.kt   |  78 ++++++----
 .../src/main/java/com/crabigator/app/MenuPanel.kt  |   3 +-
 .../java/com/crabigator/app/SessionComposer.kt     | 169 +++++++++++++++++++++
 .../main/java/com/crabigator/app/SessionDetail.kt  |  54 +++----
 .../main/java/com/crabigator/app/SessionStatus.kt  |  73 +++++++++
 .../java/com/crabigator/app/TerminalKeyboard.kt    |  97 ++++++++++++
 .../main/java/com/crabigator/app/TerminalKeys.kt   |  25 +++
 .../main/java/com/crabigator/app/UiPreferences.kt  |   5 +-
 .../android/app/src/main/res/drawable/ic_edit.xml  |   1 +
 .../app/src/main/res/drawable/ic_keyboard.xml      |   1 +
 .../app/src/main/res/drawable/ic_microphone.xml    |   1 +
 .../java/com/crabigator/app/TerminalKeysTest.kt    |  18 +++
 13 files changed, 471 insertions(+), 66 deletions(-)

commit 8ae6299ed00e4a3c88840a7c00952ae601eb42fc
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sat Oct 10 12:37:05 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sat Oct 10 12:37:05 2026 -0700

    Preserve dim terminal text on Android

 .../app/src/main/java/com/crabigator/app/TerminalText.kt      |  9 +++++----
 .../app/src/test/java/com/crabigator/app/PromptActionsTest.kt | 11 +++++++++++
 2 files changed, 16 insertions(+), 4 deletions(-)

commit abff75b14400d914e18082aa795e363870301002
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sat Oct 10 12:37:05 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sat Oct 10 12:37:05 2026 -0700

    Animate closing sessions and shorten phone popovers

 .../src/main/java/com/crabigator/app/MainActivity.kt    | 17 +++++++++++++----
 .../app/src/main/java/com/crabigator/app/MenuPanel.kt   |  5 ++++-
 .../src/main/java/com/crabigator/app/SessionDetail.kt   |  4 ++--
 3 files changed, 19 insertions(+), 7 deletions(-)

commit 37d4e8acc29c9dabd70197276105438dcc1e6053
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sat Oct 10 12:37:13 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sat Oct 10 12:37:13 2026 -0700

    Show Send only for messages and match the input height

 .../java/com/crabigator/app/SessionComposer.kt     | 25 +++++++++++++++++-----
 1 file changed, 20 insertions(+), 5 deletions(-)

commit f10f815159f6bbc1bb61f712222a280d62123965
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sat Oct 10 12:37:13 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sat Oct 10 12:37:13 2026 -0700

    Arrange terminal letter keys in QWERTY rows

 .../src/main/java/com/crabigator/app/TerminalKeyboard.kt  | 15 +++++++++++----
 1 file changed, 11 insertions(+), 4 deletions(-)

commit 5a9141c553b1eb6b6fb16da6d9abef42a1ce7ff4
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sat Oct 10 16:45:38 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sat Oct 10 16:45:38 2026 -0700

    Add interactive swipes and paging between Android sessions

 clients/android/README.md                          |  4 +-
 .../main/java/com/crabigator/app/MainActivity.kt   | 72 ++++++++++++++++------
 .../java/com/crabigator/app/SessionComposer.kt     |  7 ++-
 .../main/java/com/crabigator/app/SessionDetail.kt  |  6 +-
 .../main/java/com/crabigator/app/SessionModel.kt   |  9 +--
 .../main/java/com/crabigator/app/SessionPages.kt   | 49 +++++++++++++++
 .../main/java/com/crabigator/app/SessionSwipe.kt   | 65 +++++++++++++++++++
 7 files changed, 182 insertions(+), 30 deletions(-)

commit 630f5ea022487989a525727de52f22aeaab555d4
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 09:42:52 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 09:42:52 2026 -0700

    Keep Android notifications silent unless sound is enabled

 clients/android/README.md                                        | 2 +-
 .../android/app/src/main/java/com/crabigator/app/MenuPanel.kt    | 9 +++++++--
 .../app/src/main/java/com/crabigator/app/Notifications.kt        | 9 +++++++--
 3 files changed, 15 insertions(+), 5 deletions(-)

commit 40782b6109923cd7a45836a29995c6ca88f6fc78
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 10:25:44 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 10:25:44 2026 -0700

    Render terminal status arrows correctly on Android

 clients/android/README.md                          |   2 +-
 .../main/assets/licenses/NotoSansSymbols2-OFL.txt  |  93 +++++++++++++++++++++
 .../main/java/com/crabigator/app/TerminalText.kt   |  11 ++-
 .../app/src/main/res/font/noto_sans_symbols2.ttf   | Bin 0 -> 6356 bytes
 .../java/com/crabigator/app/PromptActionsTest.kt   |  10 +++
 5 files changed, 113 insertions(+), 3 deletions(-)

commit b663c49335fd2f39a42d5169c18246b7c68877e3
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 10:38:24 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 10:38:24 2026 -0700

    Keep Android terminal text still while status symbols animate

 clients/android/README.md                          |   4 +-
 clients/android/app/build.gradle.kts               |   3 ++
 .../java/com/crabigator/app/TerminalCellTest.kt    |  49 +++++++++++++++++++++
 .../main/java/com/crabigator/app/SessionDetail.kt  |   9 +++-
 .../main/java/com/crabigator/app/TerminalText.kt   |   9 ++--
 .../app/src/main/res/font/noto_sans_symbols2.ttf   | Bin 6356 -> 7296 bytes
 .../java/com/crabigator/app/PromptActionsTest.kt   |   2 +-
 clients/android/scripts/terminal-symbols.py        |  49 +++++++++++++++++++++
 8 files changed, 117 insertions(+), 8 deletions(-)

commit 2a95bb7bcb0ac7a2aab70334f7dfe8ee131bb979
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 20:40:14 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 20:40:14 2026 -0700

    Keep Android scrolling responsive during terminal updates

    Parse terminal colors off the UI thread and lay out only nearby text blocks. Keep list sorting and session clocks from rebuilding the whole board during screen updates.

    Preserve reading position while new output arrives, and keep the pin control and horizontal scrolling working.

 .../java/com/crabigator/app/TerminalScrollTest.kt  |  80 +++++++++++++++
 .../main/java/com/crabigator/app/MainActivity.kt   |  36 ++++---
 .../main/java/com/crabigator/app/SessionDetail.kt  |  34 +-----
 .../java/com/crabigator/app/TerminalViewport.kt    | 114 +++++++++++++++++++++
 .../java/com/crabigator/app/TerminalBlocksTest.kt  |  21 ++++
 5 files changed, 235 insertions(+), 50 deletions(-)

commit 3157feb4d75340023fb153511c0aa1a4a751f66b
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 20:40:30 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 20:40:30 2026 -0700

    Add repeatable Android scroll profiling

    Provide an optimized local build with Compose tracing and a foreground scroll capture script. Document how to compare the same workload without clearing pairing or changing device settings.

 clients/android/README.md                          | 23 +++++-
 clients/android/app/build.gradle.kts               | 11 +++
 .../android/app/src/profile/AndroidManifest.xml    |  5 ++
 clients/android/scripts/profile-scroll.py          | 82 ++++++++++++++++++++++
 clients/android/scripts/scroll-summary.sql         | 19 +++++
 5 files changed, 139 insertions(+), 1 deletion(-)

commit 7f8b05780f8853b5e6dcb2c1450bf2e157c6aab0
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 20:41:19 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 20:41:19 2026 -0700

    Report when slow gestures outlast a performance trace

 clients/android/README.md                 | 2 +-
 clients/android/scripts/profile-scroll.py | 4 ++++
 2 files changed, 5 insertions(+), 1 deletion(-)

commit 34d6107c3e2a7b4ceaabb3c72ab4db2fdc7155b8
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 21:04:54 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 21:04:54 2026 -0700

    Align Android session cards and simplify their timing

    Give session identity and status icons separate fixed-width columns. Align the title and timing beside them, with the computer name inline in gray.

    Show prompt and completion recency without counts, and retain accessible status names.

 .../main/java/com/crabigator/app/MainActivity.kt   | 56 ++++++++++++----------
 .../src/main/java/com/crabigator/app/MenuPanel.kt  |  2 +-
 .../main/java/com/crabigator/app/SessionStats.kt   |  5 --
 .../main/java/com/crabigator/app/SessionStatus.kt  | 20 ++++----
 .../java/com/crabigator/app/SessionStatsTest.kt    |  5 +-
 5 files changed, 44 insertions(+), 44 deletions(-)

commit f5debf080740c88532c544738411ac32145cf128
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 21:31:35 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 21:31:46 2026 -0700

    Group Android sessions by machine and simplify session cards

    Keep machine folders in a stable order with animated collapse controls and session counts. Group the PR board by machine too, and keep the tablet detail pane in place when it closes.

    Keep timing on one line with recency at the right edge, and highlight questions and permission requests.

 clients/android/README.md                          |   4 +
 .../java/com/crabigator/app/SessionTimelineTest.kt |  77 +++++++++++
 .../main/java/com/crabigator/app/MainActivity.kt   | 153 ++++++++++++++-------
 .../src/main/java/com/crabigator/app/MenuPanel.kt  |   2 +-
 .../main/java/com/crabigator/app/SessionGroups.kt  |  40 ++++++
 .../main/java/com/crabigator/app/SessionStatus.kt  |  16 ++-
 .../java/com/crabigator/app/SessionTimeline.kt     |  94 +++++++++++++
 .../app/src/main/res/drawable/ic_chevron_down.xml  |   3 +
 .../app/src/main/res/drawable/ic_machine.xml       |   3 +
 .../java/com/crabigator/app/SessionGroupsTest.kt   |  46 +++++++
 10 files changed, 386 insertions(+), 52 deletions(-)

commit abc96cc23ecece05d2063f5c5e8952224d2419b4
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 21:34:03 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 21:34:03 2026 -0700

    Show loading progress before Android sessions arrive

    Distinguish a pending request from an empty account or a connection failure. Show animated dots while loading, Retry after a failure, and pairing controls for a confirmed empty list.

    Keep pairing busy state separate from refreshes and discard responses from the previous connection.

 clients/android/README.md                          |  2 +
 .../com/crabigator/app/RosterPlaceholderTest.kt    | 35 +++++++++++
 .../main/java/com/crabigator/app/MainActivity.kt   | 10 ++--
 .../java/com/crabigator/app/RosterPlaceholder.kt   | 68 ++++++++++++++++++++++
 .../main/java/com/crabigator/app/SessionModel.kt   | 37 +++++++++---
 5 files changed, 141 insertions(+), 11 deletions(-)

commit ecb91ecd0630fcd1bd1a65e127101f72deb6f846
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 21:34:57 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 21:34:57 2026 -0700

    Keep ago visible on Android completion times

    Show ago on both recency values when they fit. Drop it from the prompt first and shorten durations before hiding timing details.

 clients/android/README.md                                |  2 +-
 .../java/com/crabigator/app/SessionTimelineTest.kt       |  6 +++++-
 .../src/main/java/com/crabigator/app/SessionTimeline.kt  | 16 +++++++++-------
 3 files changed, 15 insertions(+), 9 deletions(-)

commit e742319b0ada0091a98eff4b109f7dc28d4034bd
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 21:39:07 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 21:39:07 2026 -0700

    Match Android recency colors to terminal stats

 .../src/main/java/com/crabigator/app/SessionStatus.kt  |  1 +
 .../main/java/com/crabigator/app/SessionTimeline.kt    | 18 ++++++++++++------
 2 files changed, 13 insertions(+), 6 deletions(-)

commit ad376494fd80cf3bba27b3f532b28a76770a6d3a
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 21:39:07 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 21:39:07 2026 -0700

    Clarify follow-output controls and separate toolbar buttons

 clients/android/app/src/main/java/com/crabigator/app/MainActivity.kt    | 2 +-
 clients/android/app/src/main/java/com/crabigator/app/SessionDetail.kt   | 2 +-
 .../app/src/main/res/drawable/{ic_pin.xml => ic_follow_output.xml}      | 2 +-
 3 files changed, 3 insertions(+), 3 deletions(-)

commit 526f0979bdf57deb85033c1b5582b9e2901787ad
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 21:43:02 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 21:43:02 2026 -0700

    Respect native toolbar button bounds

 clients/android/app/src/main/java/com/crabigator/app/MainActivity.kt | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)

commit e4febe189138f3b2a76c6df1c883d130b90b1a2d
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 21:58:12 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 21:58:12 2026 -0700

    Give Android a dense PR board and expandable session views

    Group session rows by machine, activity age, and repository, with pull request details beneath each session. Match the terminal board's ordering.

    Move view selection into a floating segmented footer. Switching modes opens the full view; tapping the selected segment toggles the sidebar, with its action shown inside the button. Preserve scroll positions and stop hidden terminal input. Keep question highlighting on the badge.

 clients/android/README.md                          |   6 +-
 .../app/src/main/java/com/crabigator/app/Api.kt    |  12 +-
 .../java/com/crabigator/app/BoardModeFooter.kt     |  39 +++++
 .../main/java/com/crabigator/app/MainActivity.kt   | 134 +++++++----------
 .../src/main/java/com/crabigator/app/MenuPanel.kt  |   1 +
 .../src/main/java/com/crabigator/app/PrBoard.kt    | 163 +++++++++++++++++++++
 .../main/java/com/crabigator/app/PrBoardGroups.kt  |  64 ++++++++
 .../main/java/com/crabigator/app/SessionGroups.kt  |  14 --
 .../main/java/com/crabigator/app/SessionModel.kt   |   9 +-
 .../main/java/com/crabigator/app/SessionPages.kt   |   3 +-
 .../main/java/com/crabigator/app/UiPreferences.kt  |   8 +-
 .../app/src/main/res/drawable/ic_collapse.xml      |   1 +
 .../app/src/main/res/drawable/ic_expand.xml        |   1 +
 .../java/com/crabigator/app/PrBoardGroupsTest.kt   |  46 ++++++
 .../java/com/crabigator/app/SessionGroupsTest.kt   |  12 --
 15 files changed, 391 insertions(+), 122 deletions(-)

commit 35ca0b0f8db88ba274d13ddd51256dc3494d742f
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 22:13:29 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 22:13:29 2026 -0700

    Keep the tablet sidebar open when switching views

 clients/android/app/src/main/java/com/crabigator/app/MainActivity.kt | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)

commit 77192d96869c860e905bf8692014cd73ba9503bb
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 22:14:42 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 22:14:42 2026 -0700

    Align PR board rows and open their sessions on tap

    Place explicit PR links beside the scroll control in the session header. Keep PR titles under the session title, align columns at the top, and group status and diff details consistently.

 .../com/crabigator/app/PrBoardInteractionTest.kt   | 79 ++++++++++++++++++
 .../src/main/java/com/crabigator/app/PrBoard.kt    | 94 +++++++++++++---------
 .../main/java/com/crabigator/app/SessionDetail.kt  | 50 +++++++++---
 3 files changed, 174 insertions(+), 49 deletions(-)

commit 1c7cba9c4888d69cf408a4450d4a759ac7418d42
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 22:17:08 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 22:17:08 2026 -0700

    Add space around the header PR buttons

 .../android/app/src/main/java/com/crabigator/app/SessionDetail.kt   | 6 +++---
 1 file changed, 3 insertions(+), 3 deletions(-)

commit 88d2a45efdb3ea8f05690d199ee9154a7e69b36e
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 22:28:00 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 22:28:00 2026 -0700

    Keep session and PR board positions stable for ten minutes

    Continue showing live row contents while holding session, repository, age-band, and PR ordering. Append new arrivals immediately, remove missing rows, and apply search and filters without resetting the saved order.

 .../main/java/com/crabigator/app/MainActivity.kt   | 13 ++--
 .../src/main/java/com/crabigator/app/PrBoard.kt    | 10 +--
 .../main/java/com/crabigator/app/RosterOrder.kt    | 73 +++++++++++++++++++
 .../java/com/crabigator/app/RosterOrderTest.kt     | 82 ++++++++++++++++++++++
 4 files changed, 166 insertions(+), 12 deletions(-)

commit 916018b9d3eb52d840ad3411906ec3ecce5c8385
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 22:30:16 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 22:30:16 2026 -0700

    Keep returning machines in their original position

 .../src/main/java/com/crabigator/app/RosterOrder.kt    |  5 +++--
 .../test/java/com/crabigator/app/RosterOrderTest.kt    | 18 ++++++++++++++++++
 2 files changed, 21 insertions(+), 2 deletions(-)

commit ef4eb27e1d1d3814eceefc89f258ecd67fc48645
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 22:52:11 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 22:52:11 2026 -0700

    Keep Android terminal history steady while output changes

    Join saved and live rows without extra space. Use stable block snapshots and preserve the reading position as the live screen grows.

 .../java/com/crabigator/app/TerminalScrollTest.kt  | 22 +++++++++++++---
 .../java/com/crabigator/app/TerminalViewport.kt    | 30 ++++++++++++++--------
 2 files changed, 38 insertions(+), 14 deletions(-)

commit a38d0560607daa2fd3e8eec53a9492e617cdc3f0
Author:     Samuel Clay <samuel@ofbrooklyn.com>
AuthorDate: Sun Oct 11 23:00:53 2026 -0700
Commit:     Samuel Clay <samuel@ofbrooklyn.com>
CommitDate: Sun Oct 11 23:00:53 2026 -0700

    Reorder Android sessions immediately when a new session arrives

    Restart the ten-minute ordering timer for new session IDs. Keep existing activity and reconnects from moving rows before that timer expires.

 .../main/java/com/crabigator/app/RosterOrder.kt    |  7 ++-
 .../java/com/crabigator/app/RosterOrderTest.kt     | 51 +++++++++++++++++++---
 2 files changed, 50 insertions(+), 8 deletions(-)
