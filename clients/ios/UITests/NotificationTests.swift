import XCTest

/// Opt-in checks of how iOS presents a prompt notification. The app posts a
/// local notification through the same code the service extension uses.
final class NotificationTests: XCTestCase {
    private let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")

    func testPhoneSendAndRemoteResolution() throws {
        guard let id = ProcessInfo.processInfo.environment["CRABIGATOR_SEND_TEST_SESSION"] else {
            throw XCTSkip("Requires a dedicated live test session; sends a read-only question request.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.buttons["PR board"].waitForExistence(timeout: 25) || app.buttons["Back to list"].exists)
        if app.buttons["Back to list"].exists { app.buttons["Back to list"].tap() }
        app.buttons["Search"].tap()
        let search = app.textFields["Search sessions"]
        search.tap(); search.typeText("notification test")
        let row = app.buttons["Session " + id]
        XCTAssertTrue(row.waitForExistence(timeout: 15)); row.tap()
        let field = app.textViews["Message"]
        XCTAssertTrue(field.waitForExistence(timeout: 15)); field.tap()
        field.typeText("Ask me one more question with AskUserQuestion: Is the phone keyboard working? Choices Yes and No. Do not inspect files or run commands.")
        let send = app.keyboards.buttons["Send"]
        XCTAssertTrue(send.exists)
        capture("Phone keyboard Send")
        send.tap()
        XCTAssertEqual(XCTWaiter.wait(for: [XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == ''"), object: field)], timeout: 10), .completed)
        let choice = app.buttons["Prompt option 1"]
        XCTAssertTrue(choice.waitForExistence(timeout: 60))
        capture("Waiting for laptop answer")
        // The test driver answers this dedicated session through tmux.
        XCTAssertEqual(XCTWaiter.wait(for: [XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: choice)], timeout: 30), .completed)
        capture("Question resolved in app")
    }

    func testOnlyTheGraphicShowsChoices() throws {
        let labels = (0..<16).map { "Pick \(UnicodeScalar(65 + $0)!)" }
        try expandFixture(question(labels))
        // The view's rows share these labels; menu buttons are the ones that are not rows.
        let shown = labels.filter { springboard.buttons.matching(NSPredicate(format: "label == %@ AND NOT (identifier BEGINSWITH 'Choice ')", $0)).firstMatch.exists }
        note("Action buttons shown: \(shown.count) of \(labels.count): \(shown.joined(separator: ", "))\n\n" + springboard.debugDescription)
        XCTAssertTrue(shown.isEmpty, "The expanded card must not repeat its choices as system actions")
        XCTAssertTrue(springboard.buttons["Choice Pick P"].exists, "The expanded view still lists every choice")
    }

    func testMultiSelectViewTicksBoxesWithoutAnswering() throws {
        try expandFixture(question(["Cheese", "Pepperoni", "Mushroom", "Olives"], multi: true))
        XCTAssertEqual(springboard.buttons.matching(NSPredicate(format: "label == 'Cheese'")).count, 1, "One tap would only tick a box, so only the row has this label")
        XCTAssertTrue(springboard.buttons["Open session"].exists, "The card keeps a way to open the session")
        springboard.buttons["Choice Cheese"].tap()
        springboard.buttons["Choice Olives"].tap()
        XCTAssertTrue(springboard.buttons["Submit choices"].exists)
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
        if env["CRABIGATOR_TEST_EXPECT_RESOLVED"] == "1" {
            XCTAssertTrue(springboard.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'Choice '")).firstMatch.exists)
        }
        if let seconds = env["CRABIGATOR_TEST_PAUSE"].flatMap(Double.init) {
            // Time to change the prompt from the terminal, so the answer below is stale.
            Thread.sleep(forTimeInterval: seconds)
            capture("Before answering")
        }
        if let text = env["CRABIGATOR_TEST_REPLY"] {
            let field = springboard.textViews["Your answer"]
            XCTAssertTrue(field.waitForExistence(timeout: 5))
            field.tap()
            field.typeText(text)
            springboard.keyboards.buttons["Send"].tap()
            Thread.sleep(forTimeInterval: 3)
            capture("After replying")
        }
        if let label = env["CRABIGATOR_TEST_TAP_BUTTON"] {
            springboard.buttons[label].firstMatch.tap()
            Thread.sleep(forTimeInterval: 3)
            capture("After tapping \(label)")
        }
        for label in env["CRABIGATOR_TEST_TAP_CHOICE"]?.components(separatedBy: "|") ?? [] {
            // Choices separated by "|" answer one question after another in the same view.
            let row = springboard.buttons["Choice " + label].firstMatch
            XCTAssertTrue(row.waitForExistence(timeout: 10), "\(label) must be a choice in the view")
            Thread.sleep(forTimeInterval: 1) // A tap while the view scrolls to the next question only stops the scroll.
            row.tap()
            Thread.sleep(forTimeInterval: 1)
            capture("After choosing \(label)")
        }
        if env["CRABIGATOR_TEST_SUBMIT"] == "1" {
            springboard.buttons["Submit choices"].tap()
            Thread.sleep(forTimeInterval: 1)
            capture("After submitting")
        }
        if env["CRABIGATOR_TEST_EXPECT_RESOLVED"] == "1" {
            let closed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: springboard.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'Choice '")).firstMatch)
            XCTAssertEqual(XCTWaiter.wait(for: [closed], timeout: 15), .completed, "An answer from the laptop must close the card too")
            capture("Resolved from laptop")
        } else if env["CRABIGATOR_TEST_EXPECT_STALE"] == "1" {
            // The prompt changed after the view opened: nothing may be sent.
            let refused = springboard.staticTexts.matching(NSPredicate(format: "label IN %@", ["The question changed, so nothing was sent.", "This request was answered or has ended."])).firstMatch
            XCTAssertTrue(refused.waitForExistence(timeout: 10))
            capture("Stale answer refused")
        } else if env["CRABIGATOR_TEST_TAP_CHOICE"] != nil || env["CRABIGATOR_TEST_REPLY"] != nil {
            let closed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: springboard.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'Choice '")).firstMatch)
            XCTAssertEqual(XCTWaiter.wait(for: [closed], timeout: 15), .completed, "The view closes once the prompt is done")
            capture("Closed")
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
