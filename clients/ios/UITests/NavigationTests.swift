import XCTest
import UIKit

final class NavigationTests: XCTestCase {
    func testLayoutRetainsSessionAndDraftAcrossRotation() throws {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_LAYOUT"] = "1"
        XCUIDevice.shared.orientation = .portrait
        defer { XCUIDevice.shared.orientation = .portrait }
        app.launch()
        let sessions = app.buttons.matching(identifier: "Session layout-0")
        XCTAssertTrue(sessions.firstMatch.waitForExistence(timeout: 10))
        try XCTUnwrap(sessions.allElementsBoundByIndex.first { $0.isHittable }).tap()
        let terminal = app.textViews["terminal-output"]
        XCTAssertTrue(terminal.waitForExistence(timeout: 5))
        let selected = app.staticTexts.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Selected session ")).firstMatch
        XCTAssertTrue(selected.waitForExistence(timeout: 5))
        let selectedID = selected.identifier
        let message = app.textViews["Message"]
        XCTAssertTrue(message.waitForExistence(timeout: 5))
        message.tap()
        message.typeText("Keep this draft through resizing")
        capture("Adaptive layout portrait keyboard")
        for orientation in [UIDeviceOrientation.landscapeLeft, .portrait, .landscapeRight, .portrait] {
            let oldFrame = terminal.frame
            XCUIDevice.shared.orientation = orientation
            let resized = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
                terminal.exists && terminal.frame != oldFrame
            }, object: nil)
            XCTAssertEqual(XCTWaiter.wait(for: [resized], timeout: 10), .completed)
            let draft = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", "Keep this draft through resizing"), object: message)
            XCTAssertEqual(XCTWaiter.wait(for: [draft], timeout: 5), .completed)
            XCTAssertTrue(app.staticTexts[selectedID].waitForExistence(timeout: 5))
            XCTAssertTrue(app.buttons["Style"].isHittable, "The trailing control must remain tappable after resizing")
            XCTAssertTrue(app.buttons["Terminal keys"].isHittable)
            XCTAssertTrue(terminal.exists)
            capture("Adaptive layout \(orientation.rawValue) keyboard")
        }
        app.buttons["Terminal keys"].tap()
        XCTAssertTrue(app.buttons["Esc"].waitForExistence(timeout: 5))
        capture("Adaptive terminal keyboard")
    }

    func testFullPageSwipeWhileLoading() throws {
        let app = try openSession(holdLoading: true)
        XCTAssertTrue(app.activityIndicators["session-loading"].waitForExistence(timeout: 5))
        try checkCancelledAndCompletedSwipe(app)
    }

    func testFullPageSwipeWithTerminalOutput() throws {
        let app = try openSession(holdLoading: false)
        let terminal = app.textViews["terminal-output"]
        XCTAssertTrue(terminal.waitForExistence(timeout: 20))
        let loaded = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            !(terminal.value as? String ?? "").isEmpty
        }, object: terminal)
        XCTAssertEqual(XCTWaiter.wait(for: [loaded], timeout: 25), .completed)
        try checkCancelledAndCompletedSwipe(app)
    }

    private func openSession(holdLoading: Bool) throws -> XCUIApplication {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1", UIDevice.current.userInterfaceIdiom == .phone else {
            throw XCTSkip("These checks need a paired phone and CRABIGATOR_LIVE_UI_TESTS=1.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        if holdLoading { app.launchEnvironment["CRABIGATOR_TEST_HOLD_SESSION_LOADING"] = "1" }
        app.launch()
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 25))
        let row = try XCTUnwrap(rows.allElementsBoundByIndex.first { $0.isHittable })
        row.tap()
        XCTAssertTrue(app.buttons["Back to list"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Back to list"].isHittable)
        return app
    }

    private func checkCancelledAndCompletedSwipe(_ app: XCUIApplication) throws {
        // Start far from the edge. Holding before release removes fling velocity.
        let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0.2, dy: 0.4))
        start.press(forDuration: 0.05, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.36, dy: 0.4)), withVelocity: 150, thenHoldForDuration: 0.5)
        waitUntilHittable(app.buttons["Back to list"], "Cancelling must retain the session")
        XCTAssertFalse(app.buttons["List"].exists, "Cancelling must keep the board covered")
        capture("Cancelled swipe retains session")
        start.press(forDuration: 0.05, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.78, dy: 0.4)), withVelocity: 200, thenHoldForDuration: 5)
        XCTAssertTrue(app.buttons["List"].waitForExistence(timeout: 5))
        waitUntilHittable(app.buttons["List"])
        XCTAssertFalse(app.buttons["Back to list"].exists)
        capture("Completed swipe restores board")
        // Reopening and the back button must use the same navigation stack.
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        try XCTUnwrap(rows.allElementsBoundByIndex.first { $0.isHittable }).tap()
        // A real device may receive a banner over the Back button during the test.
        let banner = XCUIApplication(bundleIdentifier: "com.apple.springboard").descendants(matching: .any).matching(identifier: "NotificationShortLookView").firstMatch
        if banner.exists {
            let gone = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: banner)
            XCTAssertEqual(XCTWaiter.wait(for: [gone], timeout: 15), .completed)
        }
        waitUntilHittable(app.buttons["Back to list"])
        app.buttons["Back to list"].tap()
        waitUntilHittable(app.buttons["List"])
    }

    private func waitUntilHittable(_ element: XCUIElement, _ message: String = "") {
        let ready = XCTNSPredicateExpectation(predicate: NSPredicate(format: "hittable == true"), object: element)
        XCTAssertEqual(XCTWaiter.wait(for: [ready], timeout: 5), .completed, message)
    }

    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
