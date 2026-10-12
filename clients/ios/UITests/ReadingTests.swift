import XCTest
import UIKit

/// Opt-in live checks. Pair this simulator first; these never send terminal input.
final class ReadingTests: XCTestCase {
    func testReadLiveSessionAndReturnToBoard() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Set CRABIGATOR_LIVE_UI_TESTS=1 and pair this simulator before running live checks.")
        }
        continueAfterFailure = false
        if UIDevice.current.userInterfaceIdiom == .pad { XCUIDevice.shared.orientation = .landscapeLeft }
        defer { XCUIDevice.shared.orientation = .portrait }
        let app = XCUIApplication()
        app.launch()
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 25))
        capture("Live list")
        app.buttons["PR board"].tap()
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 5))
        capture("Live PR board")
        let sessionID = ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SESSION_ID"]
        let matches = sessionID.map { app.buttons.matching(identifier: "Session " + $0) } ?? rows
        let row = try XCTUnwrap(matches.allElementsBoundByIndex.first { $0.isHittable })
        row.tap()
        let terminal = app.textViews["terminal-output"]
        XCTAssertTrue(terminal.waitForExistence(timeout: 20))
        let history = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in !(terminal.value as? String ?? "").isEmpty }, object: terminal)
        XCTAssertEqual(XCTWaiter.wait(for: [history], timeout: 25), .completed, "The session must load terminal output")
        XCTAssertFalse(app.keyboards.firstMatch.exists)
        capture("Live terminal")
        terminal.tap()
        XCTAssertFalse(app.keyboards.firstMatch.exists, "Reading the terminal must not open a keyboard")
        terminal.swipeDown()
        XCTAssertTrue(app.buttons["Follow latest output"].waitForExistence(timeout: 5), "Choose a session with enough output to scroll away from the bottom")
        capture("Reading scrollback")
        app.buttons["Follow latest output"].tap()
        XCTAssertTrue(app.buttons["Stop following output"].waitForExistence(timeout: 5))
        app.buttons["Style"].tap()
        XCTAssertTrue(app.steppers.firstMatch.waitForExistence(timeout: 5))
        capture("Style menu")
        // Close a native popover by tapping its source again.
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.02, dy: 0.6)).tap()
        app.buttons["Terminal keys"].tap()
        XCTAssertTrue(app.buttons["QWERTY"].waitForExistence(timeout: 5))
        app.buttons["QWERTY"].tap()
        XCTAssertTrue(app.buttons["q"].waitForExistence(timeout: 5))
        capture("Terminal keyboard")
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.02, dy: 0.25)).tap()
        let close = app.buttons["Back to list"].exists ? app.buttons["Back to list"] : app.buttons["Close session"]
        if !close.isHittable { app.swipeDown() }
        close.tap()
        XCTAssertTrue(app.buttons["List"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["List"].isHittable)
        if app.frame.width >= 720 {
            app.buttons["List"].tap()
            XCTAssertEqual(app.buttons["List"].value as? String, "Selected, sidebar")
            app.buttons["List"].tap()
            XCTAssertEqual(app.buttons["List"].value as? String, "Selected, full width")
        }
        capture("Returned to board")
    }
    private func capture(_ name: String) {
        // The app's longest transition is 360 ms. Capture its settled geometry.
        Thread.sleep(forTimeInterval: 0.5)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
