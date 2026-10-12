import XCTest

/// Only target an isolated test session: this sends keys to its terminal, never a prompt.
final class KeyboardTests: XCTestCase {
    func testOpenPendingCodexQuestionFromPopover() throws {
        guard let sessionID = ProcessInfo.processInfo.environment["CRABIGATOR_KEY_TEST_SESSION_ID"] else {
            throw XCTSkip("Set CRABIGATOR_KEY_TEST_SESSION_ID to an isolated terminal test session.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        let row = app.buttons.matching(identifier: "Session " + sessionID).firstMatch
        XCTAssertTrue(row.waitForExistence(timeout: 25))
        row.tap()
        waitForTerminal("to answer", in: app)
        openKeyboard(app)
        app.buttons["Reset modifiers"].tap()
        app.buttons["Shift"].tap()
        XCTAssertEqual(app.buttons["Shift"].value as? String, "On")
        app.buttons["←"].tap()
        dismissKeyboard(app)
        waitForTerminal("1. Blue", in: app)

        // Return to the composer, keeping the question unanswered.
        openKeyboard(app)
        app.buttons["Reset modifiers"].tap()
        app.buttons["Shift"].tap()
        app.buttons["→"].tap()
        dismissKeyboard(app)
        waitForTerminal("to answer", in: app)

        openKeyboard(app)
        app.buttons["Questions"].tap()
        dismissKeyboard(app)
        waitForTerminal("1. Blue", in: app)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = "Codex question opened"; attachment.lifetime = .keepAlways; add(attachment)
    }

    private func openKeyboard(_ app: XCUIApplication) {
        app.buttons["Terminal keys"].tap()
        let left = app.buttons["←"]
        XCTAssertTrue(left.waitForExistence(timeout: 5))
        let ready = XCTNSPredicateExpectation(predicate: NSPredicate(format: "enabled == true"), object: left)
        XCTAssertEqual(XCTWaiter.wait(for: [ready], timeout: 20), .completed)
    }
    private func dismissKeyboard(_ app: XCUIApplication) { app.coordinate(withNormalizedOffset: CGVector(dx: 0.01, dy: 0.2)).tap() }
    private func waitForTerminal(_ text: String, in app: XCUIApplication) {
        let terminal = app.textViews["terminal-output"]
        let expected = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            // Ignore older question screens retained in terminal scrollback.
            let latest = (terminal.value as? String ?? "").components(separatedBy: "Queued follow-up inputs").last ?? ""
            return latest.contains(text)
        }, object: terminal)
        XCTAssertEqual(XCTWaiter.wait(for: [expected], timeout: 20), .completed, "Expected Codex terminal to show: \(text)")
    }
}
