import XCTest

/// Opt-in checks of how iOS presents a prompt notification. The app posts a
/// local notification through the same code the service extension uses.
final class NotificationTests: XCTestCase {
    private let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")

    func testButtonsUseLabelsUpToTheLimitWhileTheViewListsEveryChoice() throws {
        let labels = (0..<16).map { "Pick \(UnicodeScalar(65 + $0)!)" }
        try expandFixture(question(labels))
        let shown = labels.filter { springboard.buttons[$0].exists }
        note("Action buttons shown: \(shown.count) of \(labels.count): \(shown.joined(separator: ", "))\n\n" + springboard.debugDescription)
        XCTAssertEqual(shown, Array(labels.prefix(10)), "The menu shows the first ten choices, named by their labels")
        XCTAssertTrue(springboard.buttons["Choice Pick P"].exists, "The expanded view still lists every choice")
    }

    func testMultiSelectViewTicksBoxesWithoutAnswering() throws {
        try expandFixture(question(["Cheese", "Pepperoni", "Mushroom", "Olives"], multi: true))
        XCTAssertFalse(springboard.buttons["Cheese"].exists, "One tap would only tick a box, so there is no menu button for it")
        XCTAssertFalse(springboard.buttons["Submit choices"].isEnabled)
        springboard.buttons["Choice Cheese"].tap()
        springboard.buttons["Choice Olives"].tap()
        XCTAssertTrue(springboard.buttons["Submit choices"].isEnabled)
        capture("Two boxes ticked")
    }

    /// Expands a real prompt notification and optionally answers it. With
    /// CRABIGATOR_TEST_NOTIFICATION_SESSION, the app posts that session's current
    /// prompt; otherwise the test waits for the next APNs banner with the title.
    func testLiveNotification() throws {
        let env = ProcessInfo.processInfo.environment
        guard let title = env["CRABIGATOR_LIVE_NOTIFICATION_TITLE"] else { throw XCTSkip("Requires a live session waiting on a question.") }
        continueAfterFailure = false
        if let session = env["CRABIGATOR_TEST_NOTIFICATION_SESSION"] { post(["CRABIGATOR_TEST_NOTIFICATION_SESSION": session]) }
        expand(springboard.descendants(matching: .any).matching(NSPredicate(format: "identifier == 'NotificationShortLookView' AND label CONTAINS %@", title)).firstMatch, wait: 120)
        note(springboard.debugDescription)
        if let seconds = env["CRABIGATOR_TEST_PAUSE"].flatMap(Double.init) {
            // Time to change the prompt from the terminal, so the answer below is stale.
            Thread.sleep(forTimeInterval: seconds)
            capture("Before answering")
        }
        if let text = env["CRABIGATOR_TEST_REPLY"] {
            springboard.buttons["Reply"].firstMatch.tap()
            let field = springboard.textViews["Your answer"].firstMatch
            XCTAssertTrue(field.waitForExistence(timeout: 5))
            field.typeText(text)
            springboard.buttons["Send"].firstMatch.tap()
            Thread.sleep(forTimeInterval: 3)
            capture("After replying")
        }
        if let label = env["CRABIGATOR_TEST_TAP_BUTTON"] {
            springboard.buttons[label].firstMatch.tap()
            Thread.sleep(forTimeInterval: 3)
            capture("After tapping \(label)")
        }
        if let label = env["CRABIGATOR_TEST_TAP_CHOICE"] {
            springboard.buttons["Choice " + label].firstMatch.tap()
            Thread.sleep(forTimeInterval: 1)
            capture("Sending \(label)")
            Thread.sleep(forTimeInterval: 4)
            capture("After choosing \(label)")
        }
    }

    private func question(_ labels: [String], multi: Bool = false) -> [String: Any] {
        ["id": "fixture-\(UUID().uuidString)", "title": "Notification check", "state": "question", "prompt_revision": 1,
         "prompt": ["prompt_type": "question", "current_question": 0,
                    "questions": [["question": "Which one?", "header": "Pick", "options": labels.map { ["label": $0] }, "multi_select": multi, "allows_other": false]]]]
    }
    private func expandFixture(_ snapshot: [String: Any]) throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_NOTIFICATION_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires a device with notifications allowed for Crabigator.")
        }
        continueAfterFailure = false
        post(["CRABIGATOR_TEST_NOTIFICATION": String(decoding: try JSONSerialization.data(withJSONObject: snapshot), as: UTF8.self)])
        expand(springboard.descendants(matching: .any).matching(identifier: "NotificationShortLookView").firstMatch, wait: 15)
    }
    /// The app posts the notification a few seconds after launch; leave it so a banner shows.
    private func post(_ environment: [String: String]) {
        let app = XCUIApplication()
        app.launchEnvironment = environment
        app.launch()
        Thread.sleep(forTimeInterval: 1)
        XCUIDevice.shared.press(.home)
    }
    private func expand(_ banner: XCUIElement, wait: TimeInterval) {
        XCTAssertTrue(banner.waitForExistence(timeout: wait))
        Thread.sleep(forTimeInterval: 0.8)
        capture("Banner")
        banner.press(forDuration: 1.2)
        Thread.sleep(forTimeInterval: 1.5)
        capture("Expanded")
    }
    private func note(_ text: String) {
        let attachment = XCTAttachment(string: text)
        attachment.name = "Findings"; attachment.lifetime = .keepAlways; add(attachment)
    }
    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
