import XCTest
import UIKit

/// Opt-in live checks. Pair this simulator first; these never send terminal input.
final class ReadingTests: XCTestCase {
    func testRestoresBoardModeAndSelectedSessionAfterRelaunch() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired device.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        // A prior run may have left a phone on its restored terminal page.
        XCTAssertTrue(app.buttons["PR board"].waitForExistence(timeout: 25) || app.buttons["Back to list"].exists)
        if app.buttons["Back to list"].exists { app.buttons["Back to list"].tap() }
        if !(app.buttons["PR board"].value as? String ?? "").hasPrefix("Selected") { app.buttons["PR board"].tap() }
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 25))
        let requestedID = ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SESSION_ID"]
        let row = try XCTUnwrap(requestedID.map { app.buttons["Session " + $0] } ?? rows.allElementsBoundByIndex.first { $0.isHittable })
        XCTAssertTrue(row.isHittable)
        let id = String(row.identifier.dropFirst("Session ".count))
        row.tap()
        XCTAssertTrue(app.staticTexts["Selected session " + id].waitForExistence(timeout: 5))
        let terminal = app.textViews["terminal-output"]
        let loaded = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            terminal.exists && !(terminal.value as? String ?? "").isEmpty
        }, object: terminal)
        XCTAssertEqual(XCTWaiter.wait(for: [loaded], timeout: 25), .completed)
        app.terminate(); app.launch()
        XCTAssertTrue(app.staticTexts["Selected session " + id].waitForExistence(timeout: 25))
        capture("Restored selected session")
        if app.frame.width >= 720 {
            XCTAssertEqual(app.buttons["PR board"].value as? String, "Selected, sidebar")
            app.buttons["PR board"].tap()
            app.terminate(); app.launch()
            XCTAssertTrue(row.waitForExistence(timeout: 25))
            XCTAssertEqual(app.buttons["PR board"].value as? String, "Selected, full width")
            XCTAssertEqual(row.value as? String, "Selected")
            app.buttons["PR board"].tap()
            app.buttons["Close session"].tap()
        } else {
            app.buttons["Back to list"].tap()
            XCTAssertEqual(app.buttons["PR board"].value as? String, "Selected")
        }
        app.buttons["List"].tap()
        app.terminate(); app.launch()
        XCTAssertTrue(app.buttons["List"].waitForExistence(timeout: 25))
        XCTAssertTrue((app.buttons["List"].value as? String ?? "").hasPrefix("Selected"))
        XCTAssertFalse(app.staticTexts["Selected session " + id].exists, "Closing must clear the saved selection")
        capture("Restored List with no selected session")
    }

    func testSlowPromptRequestDoesNotDelayTerminalOutput() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired device and CRABIGATOR_LIVE_UI_TESTS=1.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_SLOW_PROMPT"] = "1"
        app.launch()
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 25), "Pair this device before running this live check")
        let row = try XCTUnwrap(rows.allElementsBoundByIndex.first { $0.isHittable })
        let start = Date()
        row.tap()
        let terminal = app.textViews["terminal-output"]
        let loaded = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            terminal.exists && !(terminal.value as? String ?? "").isEmpty
        }, object: terminal)
        XCTAssertEqual(XCTWaiter.wait(for: [loaded], timeout: 5), .completed,
                       "Cached terminal output must arrive while the 15-second prompt request is pending")
        print("Terminal visible after \(Date().timeIntervalSince(start)) seconds including the tap and page transition")
        capture("Terminal loaded while prompt request is pending")
    }

    func testSlowTerminalDoesNotBecomeEmptyAfterEightSeconds() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired device and CRABIGATOR_LIVE_UI_TESTS=1.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_HOLD_SESSION_LOADING"] = "1"
        app.launch()
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 25), "Pair this device before running this live check")
        try XCTUnwrap(rows.allElementsBoundByIndex.first { $0.isHittable }).tap()
        XCTAssertTrue(app.activityIndicators["session-loading"].waitForExistence(timeout: 5))
        // Wait beyond the old empty-state timeout while the stream is deliberately held.
        XCTAssertTrue(app.staticTexts["Waiting for terminal output…"].waitForExistence(timeout: 12))
        XCTAssertFalse(app.staticTexts["No terminal output"].exists)
        XCTAssertTrue(app.buttons["Reconnect"].isHittable)
        capture("Slow terminal remains loading")
    }

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
    func testBoardAndTerminalTextSizesStayIndependent() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired simulator.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 20))
        app.buttons["Settings"].tap()
        app.buttons["Style"].tap()
        let board = app.steppers["Board text size"], terminal = app.steppers["Terminal text size"]
        XCTAssertTrue(board.waitForExistence(timeout: 5))
        let oldBoard = board.label, oldTerminal = terminal.label
        for _ in 0..<4 { board.buttons["Board text size-Increment"].tap() }
        let largerBoard = board.label
        XCTAssertNotEqual(largerBoard, oldBoard)
        XCTAssertEqual(terminal.label, oldTerminal)
        capture("Independent text size controls")
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.02, dy: 0.7)).tap()
        capture("Larger list text")
        app.buttons["PR board"].tap()
        capture("Larger PR board text")
        app.terminate(); app.launch()
        app.buttons["Settings"].tap(); app.buttons["Style"].tap()
        XCTAssertEqual(board.label, largerBoard)
        XCTAssertEqual(terminal.label, oldTerminal)
        for _ in 0..<4 { board.buttons["Board text size-Decrement"].tap() }
        XCTAssertEqual(board.label, oldBoard)
    }
    private func capture(_ name: String) {
        // The app's longest transition is 360 ms. Capture its settled geometry.
        Thread.sleep(forTimeInterval: 0.5)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
