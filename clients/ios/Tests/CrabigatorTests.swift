import XCTest
import UIKit
@testable import Crabigator

final class CrabigatorTests: XCTestCase {
    func session(_ id: String, prompt: Int, machine: String = "Mac", state: String = "complete") -> Session {
        Session(["session_id": id, "title": id, "device_name": machine, "prompts_changed_at": prompt, "state": state, "active": true])
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
}
