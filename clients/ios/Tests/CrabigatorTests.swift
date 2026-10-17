import XCTest
import UIKit
import SwiftUI
import UserNotifications
@testable import Crabigator

final class CrabigatorTests: XCTestCase {
    func testModulesKeepTheLastRecapAndStructuredDiffsWhileRefreshing() throws {
        var prefs = Preferences()
        XCTAssertTrue(prefs.widgets)
        prefs.widgets = false
        XCTAssertFalse(try JSONDecoder().decode(Preferences.self, from: JSONEncoder().encode(prefs)).widgets)
        let selected = Session(["id": "modules", "state": "thinking", "recap": ["headline": "Roster recap"]])
        let data = SessionModuleData(session: selected, details: [
            "recap": ["status": "waiting"],
            "recap_history": ["history": [["headline": "Previous turn", "bullets": ["Keep this detail"]]]],
            "git": ["branch": "main", "files": [["path": "Session.swift", "status": "M", "additions": 12, "deletions": 3]]],
            "stats": ["prompts": 3, "tools": 9]])
        XCTAssertEqual(data.recap, "Previous turn")
        XCTAssertEqual(data.bullets, ["Keep this detail"])
        XCTAssertEqual(data.files.first?.name, "Session.swift")
        XCTAssertEqual(data.files.first?.additions, 12)
        XCTAssertEqual(data.stats.int("tools"), 9)
        XCTAssertEqual(SessionModuleData(session: selected, details: [:]).recap, "Roster recap")
    }
    func session(_ id: String, prompt: Int, machine: String = "Mac", state: String = "complete") -> Session {
        Session(["session_id": id, "title": id, "device_name": machine, "prompts_changed_at": prompt, "state": state, "active": true])
    }
    @MainActor func testFollowRequestedDuringCoastingRunsWhenIdleEvenWithoutNewOutput() {
        let terminal = TerminalViewport(history: "", screen: "", preferences: Preferences(), pinned: .constant(true))
        let coordinator = terminal.makeCoordinator()
        let view = TerminalScrollView(frame: CGRect(x: 0, y: 0, width: 320, height: 400))
        coordinator.view = view
        view.text.text = Array(repeating: "A history row", count: 200).joined(separator: "\n")
        view.layoutIfNeeded()
        view.text.layoutIfNeeded()
        XCTAssertGreaterThan(view.text.contentSize.height, view.text.bounds.height)
        view.text.setContentOffset(.zero, animated: false)
        // No new source revision: ending a coast must still honor the follow button.
        coordinator.scrollViewDidEndDecelerating(view.text)
        XCTAssertGreaterThan(view.text.contentOffset.y, 0)
        XCTAssertEqual(view.text.contentOffset.y, view.text.contentSize.height - view.text.bounds.height, accuracy: 1)
    }
    func testAddingBoardTextSizePreservesSavedTerminalSettings() throws {
        let legacy = Data(#"{"fontSize":19,"lineSpacing":170,"columns":2,"terminalHeight":350,"wrap":false,"widgets":true,"sidebarRight":true,"showEnded":true,"grouping":"project","order":"alpha","density":"compact","hidden":["tools"]}"#.utf8)
        var prefs = try JSONDecoder().decode(Preferences.self, from: legacy)
        XCTAssertEqual(prefs.boardTextSize, 14)
        XCTAssertEqual(prefs.fontSize, 19)
        XCTAssertEqual(prefs.lineSpacing, 170)
        XCTAssertTrue(prefs.sidebarRight)
        prefs.boardTextSize = 20
        let restored = try JSONDecoder().decode(Preferences.self, from: JSONEncoder().encode(prefs))
        XCTAssertEqual(restored, prefs)
        XCTAssertEqual(restored.boardTextSize, 20)
        XCTAssertEqual(restored.fontSize, 19)
    }
    func testOrderingKeepsLiveContentButOnlyMovesOnNewSessionsOrTenMinutes() {
        let order = RosterOrder(), prefs = Preferences()
        let a = session("a", prompt: 100), b = session("b", prompt: 90)
        order.update([a, b], prs: [], prefs: prefs, now: 100, elapsed: 0)
        order.update([a, session("b", prompt: 200)], prs: [], prefs: prefs, now: 200, elapsed: 20)
        XCTAssertEqual(order.list.flatMap(\.sessions).map(\.id), ["a", "b"])
        XCTAssertEqual(order.list.flatMap(\.sessions)[1].promptAt, 200)
        order.update([a, session("b", prompt: 200), session("c", prompt: 300)], prs: [], prefs: prefs, now: 300, elapsed: 30)
        XCTAssertEqual(order.list.flatMap(\.sessions).map(\.id), ["c", "b", "a"])
        order.update([session("a", prompt: 400), b, session("c", prompt: 300)], prs: [], prefs: prefs, now: 400, elapsed: 629)
        XCTAssertEqual(order.list.flatMap(\.sessions).map(\.id), ["c", "b", "a"])
        order.update([session("a", prompt: 400), b, session("c", prompt: 300)], prs: [], prefs: prefs, now: 400, elapsed: 630)
        XCTAssertEqual(order.list.flatMap(\.sessions).map(\.id), ["a", "c", "b"])
    }
    func testMachinesKeepTheirOriginalPlaceAfterDisappearing() {
        let order = RosterOrder(), prefs = Preferences()
        order.update([session("a", prompt: 30, machine: "First"), session("b", prompt: 20, machine: "Second")], prs: [], prefs: prefs, elapsed: 0)
        order.update([session("b", prompt: 40, machine: "Second")], prs: [], prefs: prefs, elapsed: 10)
        order.update([session("b", prompt: 40, machine: "Second"), session("a", prompt: 50, machine: "First")], prs: [], prefs: prefs, elapsed: 20)
        XCTAssertEqual(order.machines, ["First", "Second"])
        XCTAssertEqual(order.list.map(\.machine), ["First", "Second"])
    }
    func testCompletedAgeUsesPromptNotCompletionWhileWorkingStaysRecent() {
        let order = RosterOrder()
        var old = session("old", prompt: 100); old.completionAt = 10000
        order.update([old, session("working", prompt: 100, state: "thinking")], prs: [], prefs: Preferences(), now: 10000, elapsed: 0)
        XCTAssertEqual(order.board.first { $0.sessions.contains { $0.id == "old" } }?.age, .three)
        XCTAssertEqual(order.board.first { $0.sessions.contains { $0.id == "working" } }?.age, .hour)
    }
    func testNewPromptMovesIntoLastHourWithoutReorderingOtherRows() {
        let order = RosterOrder(), prefs = Preferences(), now = 100_000
        let recent = session("recent", prompt: now - 10)
        let otherRecent = session("other-recent", prompt: now - 20)
        let target = session("portal", prompt: now - 7200)
        var old = session("old", prompt: now - 8000)
        order.update([recent, otherRecent, target, old], prs: [], prefs: prefs, now: now, elapsed: 0)
        XCTAssertEqual(order.board.first { $0.sessions.contains { $0.id == target.id } }?.age, .three)

        // A quick response may already be complete when the next roster update arrives.
        let prompted = session("portal", prompt: now + 1)
        old.completionAt = now + 1
        order.update([prompted, old, otherRecent, recent], prs: [], prefs: prefs, now: now + 1, elapsed: 1)
        XCTAssertEqual(order.board.map(\.age), [.hour, .three])
        XCTAssertEqual(order.board.first?.sessions.map(\.id), ["recent", "other-recent", "portal"])
        XCTAssertEqual(order.board.last?.sessions.map(\.id), ["old"])
        XCTAssertEqual(order.list.flatMap(\.sessions).map(\.id), ["recent", "other-recent", "portal", "old"])

        order.update([prompted, old, otherRecent, recent], prs: [], prefs: prefs, now: now + 601, elapsed: 601)
        XCTAssertEqual(order.board.first?.sessions.first?.id, "portal")
    }
    func testWorkingSessionMovesUpBeforeItsPromptTimestampArrives() {
        let order = RosterOrder(), prefs = Preferences(), now = 100_000
        var target = session("portal", prompt: now - 7200)
        order.update([target], prs: [], prefs: prefs, now: now, elapsed: 0)
        target.state = "thinking"
        order.update([target], prs: [], prefs: prefs, now: now + 1, elapsed: 1)
        XCTAssertEqual(order.board.map(\.age), [.hour])
        target.state = "complete"; target.promptAt = now + 1
        order.update([target], prs: [], prefs: prefs, now: now + 3601, elapsed: 2)
        XCTAssertEqual(order.board.map(\.age), [.three])
    }
    func testBoardFiltersMatchCliAgesAndKeepWorkingSessions() throws {
        var prefs = Preferences()
        XCTAssertEqual(prefs.boardAgeHours, 24)
        let now = 200_000
        var old = session("old", prompt: now - 7200)
        old.completionAt = now
        prefs.boardAgeHours = 1
        XCTAssertFalse(prefs.includesSession(old, prs: [], now: now))
        old.state = "thinking"
        XCTAssertTrue(prefs.includesSession(old, prs: [], now: now))
        old.state = "complete"; prefs.boardAgeHours = 3
        XCTAssertTrue(prefs.includesSession(old, prs: [], now: now))
        prefs.boardComputer = "Another Mac"
        XCTAssertFalse(prefs.includesSession(old, prs: [], now: now))
        prefs.boardComputer = ""; prefs.boardAgeHours = 0; prefs.boardRecaps = true; prefs.boardGrouping = "prs"
        XCTAssertTrue(prefs.includesSession(session("unknown", prompt: 0), prs: [], now: now))
        XCTAssertEqual(try JSONDecoder().decode(Preferences.self, from: JSONEncoder().encode(prefs)), prefs)
        for hours in [1, 3, 6, 9, 12, 24] {
            prefs.boardAgeHours = hours
            XCTAssertTrue(prefs.includesActivity(now - hours * 3600 + 1, now: now))
            XCTAssertFalse(prefs.includesActivity(now - hours * 3600 - 1, now: now))
        }
    }
    func testGroupingByPRCombinesSessionsAcrossTimeBands() {
        let now = 200_000, first = session("first", prompt: 199_990), second = session("second", prompt: 192_800)
        let pr = PullRequest(["owner": "owner", "repo": "repo", "number": 1, "pr": ["title": "Shared PR", "primary": true]])
        let source = [RosterSection(machine: "Mac", project: "repo", age: .hour, sessions: [first]), RosterSection(machine: "Mac", project: "repo", age: .three, sessions: [second])]
        var prefs = Preferences(); prefs.boardAgeHours = 1
        let grouped = groupBoardByPullRequest(source, prsBySession: [first.id: [pr], second.id: [pr]], now: now) { prefs.includesActivity($0, now: now) }
        XCTAssertEqual(grouped.count, 1)
        XCTAssertEqual(grouped.first?.age, .hour)
        XCTAssertEqual(grouped.first?.pullRequests.first?.sessions.map(\.id), ["first", "second"])
        XCTAssertTrue(grouped.first?.sessions.isEmpty == true)
    }
    func testPRGroupsUseTheirOwnEventsOnlyWhenNoSessionHasPrompted() {
        let now = 200_000, never = session("never", prompt: 0)
        var primary = PullRequest(["owner": "owner", "repo": "repo", "number": 1, "pr": ["primary": true]])
        primary.updatedAt = now - 7200
        var secondary = PullRequest(["owner": "owner", "repo": "repo", "number": 2])
        secondary.updatedAt = now
        var prefs = Preferences(); prefs.boardAgeHours = 1
        let source = [RosterSection(machine: "Mac", project: "repo", age: .hour, sessions: [never])]
        XCTAssertTrue(groupBoardByPullRequest(source, prsBySession: [never.id: [primary, secondary]], now: now) { prefs.includesActivity($0, now: now) }.isEmpty)
        primary.updatedAt = now
        XCTAssertEqual(groupBoardByPullRequest(source, prsBySession: [never.id: [primary]], now: now).first?.age, .hour)
        let oldPrompt = [RosterSection(machine: "Mac", project: "repo", age: .three, sessions: [session("never", prompt: now - 7200)])]
        XCTAssertTrue(groupBoardByPullRequest(oldPrompt, prsBySession: [never.id: [primary]], now: now) { prefs.includesActivity($0, now: now) }.isEmpty)
    }
    func testMergedSecondaryPRsHideOnlyForTheirSecondarySession() {
        let merged = PullRequest(["owner": "owner", "repo": "repo", "number": 1,
            "pr": ["state": "MERGED", "primary": true],
            "sessions": [["session_id": "owner", "pr_primary": true]],
            "touching": [["session_id": "reader", "pr_primary": false]]])
        XCTAssertTrue(merged.visibleOnBoard(for: "owner"))
        XCTAssertFalse(merged.visibleOnBoard(for: "reader"))
        var open = merged; open.state = "OPEN"
        XCTAssertTrue(open.visibleOnBoard(for: "reader"))
        var watched = merged; watched.watched = true
        XCTAssertTrue(watched.visibleOnBoard(for: "reader"))
        let secondary = PullRequest(["pr": ["state": "MERGED", "primary": false]])
        XCTAssertFalse(secondary.visibleOnBoard(for: "reader"))
        let source = [RosterSection(machine: "Mac", project: "owner/repo", age: .hour,
            sessions: [session("owner", prompt: 100), session("reader", prompt: 100)])]
        let groups = groupBoardByPullRequest(source, prsBySession: ["owner": [merged], "reader": [merged]], now: 100)
        XCTAssertEqual(groups.flatMap(\.pullRequests).flatMap(\.sessions).map(\.id), ["owner"])
        XCTAssertEqual(groups.flatMap(\.sessions).map(\.id), ["reader"])
    }
    func testWatchTargetsAndNewWatchRecency() {
        XCTAssertEqual(BoardActions.watchTarget("owner/repo#12")?.int("number"), 12)
        XCTAssertEqual(BoardActions.watchTarget("https://github.com/owner/repo/pull/12/files")?.text("repo"), "repo")
        XCTAssertNil(BoardActions.watchTarget("https://other.test/owner/repo/pull/12"))
        XCTAssertNil(BoardActions.watchTarget("owner/repo#0"))
        let watched = PullRequest(["owner": "owner", "repo": "repo", "number": 12, "updated_at": 100, "pr": ["watched": true]])
        XCTAssertEqual(watched.updatedAt, 100)
    }
    func testPromptTranslationForCheckboxesCustomAnswersAndGrok() throws {
        let q: JSON = ["question": "Choose", "multi_select": true, "allows_other": true, "options": [["label": "A"], ["label": "B"]]]
        let prompt: JSON = ["prompt_type": "question", "questions": [q], "cursor_row": 1]
        XCTAssertEqual(try PromptActions.action(prompt, option: 1).body.array("steps").first?.text("text"), "2")
        let custom = try PromptActions.action(prompt, text: "Elsewhere").body.array("steps")
        XCTAssertEqual(custom.map { $0.text("key", $0.text("text", $0.text("type"))) }, ["down", "down", "Elsewhere", "delay", "down", "enter"])
        var grok = prompt; grok["ui"] = "grok_card"
        XCTAssertEqual(try PromptActions.action(grok, option: 1).body.array("steps").map { $0.text("key") }, ["down", "space"])
        var single = q; single["multi_select"] = false; grok["questions"] = [single]
        XCTAssertEqual(try PromptActions.action(grok, text: "Custom").body.array("steps").last?.text("key"), "enter")
        XCTAssertThrowsError(try PromptActions.action(prompt, option: 9))
        XCTAssertThrowsError(try PromptActions.action(["prompt_type": "text", "state": "permission"], text: "yes"))
    }
    func testCustomAnswerReplacesTextAlreadyOnTheRow() throws {
        let q: JSON = ["question": "Which crust?", "allows_other": true, "options": [["label": "Thin"], ["label": "Thick"], ["label": "Stuffed"]]]
        let prompt: JSON = ["prompt_type": "question", "questions": [q], "current_question": 0, "cursor_row": 2]
        func steps(_ p: JSON) throws -> [String] { try PromptActions.action(p, text: "extra crispy").body.array("steps").map { $0.text("key", $0.text("text", $0.text("type"))) } }
        XCTAssertEqual(try steps(prompt), ["4", "extra crispy", "delay", "enter"])
        // Pressing 4 would send "extra" at once, so walk there and delete it from its end.
        var typed = prompt; typed["custom_text"] = "extra"
        let replace = ["\u{5}"] + Array(repeating: "backspace", count: 5) + ["extra crispy", "delay", "enter"]
        XCTAssertEqual(try steps(typed), ["down", "down"] + replace)
        typed["cursor_row"] = 4
        XCTAssertEqual(try steps(typed), replace)
    }
    func testReviewAndPermissionNeverUseGenericFreeText() throws {
        let review: JSON = ["prompt_type": "question", "review": [:]]
        XCTAssertEqual(try PromptActions.action(review, option: 0).body.array("steps").first?.text("text"), "1")
        let permission: JSON = ["prompt_type": "permission", "options": [["label": "Allow", "value": "y"]]]
        XCTAssertEqual(try PromptActions.action(permission, option: 0).body.text("text"), "y")
        XCTAssertFalse(PromptActions.allowsText(permission))
    }
    func testTerminalModifiersMatchDesktop() {
        XCTAssertEqual(terminalKey("←", shift: true), "\u{1b}[1;2D")
        XCTAssertEqual(terminalKey("↑", shift: true, control: true, alt: true), "\u{1b}[1;8A")
        XCTAssertEqual(terminalKey("⌫", alt: true), "\u{1b}\u{7f}")
        XCTAssertEqual(terminalKey("Enter", shift: true), "\u{1b}[13;2u")
        XCTAssertEqual(terminalKey("c", control: true), "\u{3}")
        XCTAssertEqual(terminalKey("Tab", shift: true), "\u{1b}[Z")
    }
    func testPairingRejectsCredentialAndPathInjection() throws {
        XCTAssertEqual(try API.origin(" https://example.com/ "), "https://example.com")
        for origin in ["http://example.com", "https://user:secret@example.com", "https://example.com/path", "https://example.com?secret=1", "https://example.com#fragment"] { XCTAssertThrowsError(try API.origin(origin)) }
        XCTAssertEqual(API.pairingCode("abcdefghi"), "ABC-DEF-GHI")
    }
    func testTerminalStylesSpacingAndTrim() {
        let text = TerminalText.parse("\u{1b}[2mDim\u{1b}[1mBold\u{1b}[22mPlain\u{1b}[38;5;208mOrange\u{1b}[0m\tX   \nY\u{1b}[4CZ", size: 13, spacing: 1.45, wrap: true)
        XCTAssertEqual(text.string, "DimBoldPlainOrange      X\nY    Z")
        XCTAssertEqual((text.attribute(.foregroundColor, at: 0, effectiveRange: nil) as? UIColor)?.cgColor.alpha, 0.5)
        XCTAssertEqual((text.attribute(.foregroundColor, at: 3, effectiveRange: nil) as? UIColor)?.cgColor.alpha, 1)
        XCTAssertEqual(text.attribute(.foregroundColor, at: 12, effectiveRange: nil) as? UIColor, TerminalText.palette(208))
    }
    func testSpinnerFramesHaveStableAdvancesAndLineHeight() {
        XCTAssertNotNil(UIFont(name: "NotoSansSymbols2-Regular", size: 13), "The bundled symbol font must be registered")
        for size: CGFloat in [10, 13, 20] {
            let font = UIFont.monospacedSystemFont(ofSize: size, weight: .regular)
            let expected = (" " as NSString).size(withAttributes: [.font: font]).width
            var heights: [CGFloat] = []
            for symbol in ["·", "✢", "✳", "✶", "✻", "✽", "⏵"] {
                let parsed = TerminalText.parse(symbol + " Bash", size: size, spacing: 1.45, wrap: true)
                let attributes = parsed.attributes(at: 0, effectiveRange: nil)
                let advance = (symbol as NSString).size(withAttributes: [.font: attributes[.font]!]).width + (attributes[.kern] as? CGFloat ?? 0)
                XCTAssertEqual(advance, expected, accuracy: 0.01)
                let paragraph = attributes[.paragraphStyle] as! NSParagraphStyle
                heights.append(paragraph.maximumLineHeight)
                XCTAssertEqual(paragraph.minimumLineHeight, paragraph.maximumLineHeight)
            }
            XCTAssertEqual(Set(heights).count, 1)
        }
    }
    func testNotificationsUseRevisionSilentDefaultAndGuardPermissionReplies() {
        Credentials.defaults.removeObject(forKey: "notificationSound")
        let content = NotificationContent.content(["title": "Session", "state": "permission", "prompt_revision": 42], id: "session")
        XCTAssertNil(content.sound)
        XCTAssertEqual(content.userInfo["revision"] as? Int, 42)
        XCTAssertEqual(content.categoryIdentifier, "CRABIGATOR_PROMPT_0_0")
        XCTAssertEqual(content.threadIdentifier, "session")
    }
    func testNotificationBodyListsEveryChoiceAfterTheQuestion() {
        let labels = ["Thin", "Thick", "Stuffed", "Deep dish", "Gluten free", "Cauliflower"]
        let snapshot: JSON = ["prompt_revision": 3, "prompt": ["prompt_type": "question", "questions": [["question": "Which pizza crust?", "options": labels.map { ["label": $0] }]]]]
        XCTAssertEqual(NotificationContent.content(snapshot, id: "s").body, "Which pizza crust?\n" + labels.joined(separator: " · "))
    }
    func testChoiceButtonsUseLabelsAndAnswerOnlyTheirOwnPrompt() throws {
        let labels = (1...12).map { "Option \($0)" }
        let prompt: JSON = ["prompt_type": "question", "questions": [["question": "Pick", "options": labels.map { ["label": $0] }, "allows_other": true]]]
        let actions = NotificationContent.actions(prompt, id: "session", revision: 7)
        XCTAssertEqual(actions.map(\.title), Array(labels.prefix(NotificationContent.actionLimit - 1)) + ["Reply"])
        for (index, action) in actions.dropLast().enumerated() {
            XCTAssertEqual(NotificationContent.choice(action.identifier, id: "session"), .init(option: index, revision: 7))
            XCTAssertNil(NotificationContent.choice(action.identifier, id: "other"), "A reused category must not answer another session")
        }
        XCTAssertEqual(NotificationContent.choice(try XCTUnwrap(actions.last).identifier, id: "session"), .init(option: nil, revision: 7))
        XCTAssertEqual(NotificationContent.choice("option_2", id: "session"), .init(option: 2, revision: nil))
        XCTAssertNil(NotificationContent.choice(UNNotificationDefaultActionIdentifier, id: "session"))
        var multi = prompt; multi["questions"] = [["question": "Pick", "options": [["label": "A"]], "multi_select": true, "allows_other": false]]
        // One tap would only tick a box, and iOS only expands a notification with a regular button.
        XCTAssertEqual(NotificationContent.actions(multi, id: "session", revision: 7).map(\.title), ["Open session"])
        multi["questions"] = [["question": "Pick", "options": [["label": "A"]], "multi_select": true, "allows_other": true]]
        XCTAssertEqual(NotificationContent.actions(multi, id: "session", revision: 7).map(\.title), ["Reply", "Open session"])
    }
    func testCategorySlotsAreNeverSharedBetweenSessions() {
        var claims: [Int: NotificationContent.Claim] = [:]
        let a = NotificationContent.claim("a", shown: [:], claims: claims, now: 100); claims = a.claims
        let b = NotificationContent.claim("b", shown: [:], claims: claims, now: 101); claims = b.claims
        XCTAssertNotEqual(a.slot, b.slot, "A slot claimed but not yet shown stays reserved")
        XCTAssertEqual(NotificationContent.claim("a", shown: [b.slot!: "b"], claims: claims, now: 102).slot, a.slot)
        let shown = Dictionary(uniqueKeysWithValues: (0..<NotificationContent.slotCount).map { ($0, "busy\($0)") })
        XCTAssertNil(NotificationContent.claim("c", shown: shown, claims: [:], now: 103).slot)
        XCTAssertEqual(NotificationContent.claim("c", shown: [:], claims: claims, now: 100 + NotificationContent.claimLifetime + 2).slot, 0, "Unshown claims expire")
    }
    @MainActor func testPhoneReturnSendsOnceAndPastedLinesStayIntact() {
        let view = ReturnTextView()
        var sends = 0
        view.onReturn = { sends += 1 }
        XCTAssertEqual(view.returnKeyType, .send)
        view.insertText("Hello")
        view.insertText("\n")
        XCTAssertEqual(sends, 1)
        XCTAssertEqual(view.text, "Hello")
        view.insertText(" first\nsecond")
        XCTAssertEqual(view.text, "Hello first\nsecond")
        XCTAssertEqual(sends, 1)
    }
    func testContentExtensionListsEveryPromptCategory() throws {
        let plugin = try XCTUnwrap(Bundle.main.builtInPlugInsURL?.appendingPathComponent("NotificationContentExtension.appex"))
        let info = try XCTUnwrap(Bundle(url: plugin)?.infoDictionary?["NSExtension"] as? [String: Any])
        let categories = (info["NSExtensionAttributes"] as? [String: Any])?["UNNotificationExtensionCategory"] as? [String]
        XCTAssertEqual(Set(categories ?? []), Set(NotificationContent.extensionCategories))
    }
    func testMultiSelectSubmitTicksEveryChangedBoxThenSubmits() throws {
        let q: JSON = ["question": "Toppings", "multi_select": true, "options": [["label": "A"], ["label": "B"], ["label": "C"]]]
        let prompt: JSON = ["prompt_type": "question", "questions": [q], "cursor_row": 1, "checked": [2]]
        let steps = try PromptActions.action(prompt, submit: true, toggling: [0, 1]).body.array("steps")
        XCTAssertEqual(steps.map { $0.text("key", $0.text("text")) }, ["1", "2", "down", "down", "down", "down", "enter"])
        var grok = prompt; grok["ui"] = "grok_card"
        XCTAssertEqual(try PromptActions.action(grok, submit: true, toggling: [2]).body.array("steps").map { $0.text("key") }, ["down", "down", "space", "enter"])
        var single = q; single["multi_select"] = false
        XCTAssertThrowsError(try PromptActions.action(["prompt_type": "question", "questions": [single]], submit: true, toggling: [0]))
    }
}
