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
            XCTAssertTrue(app.buttons["Session view"].isHittable, "The trailing control must remain tappable after resizing")
            XCTAssertTrue(app.buttons["Terminal keys"].isHittable)
            XCTAssertTrue(terminal.exists)
            capture("Adaptive layout \(orientation.rawValue) keyboard")
        }
        app.buttons["Terminal keys"].tap()
        XCTAssertTrue(app.buttons["Esc"].waitForExistence(timeout: 5))
        capture("Adaptive terminal keyboard")
    }

    /// Rows that are already on screen must redraw when a choice in the View panel changes.
    func testVisibleRowsFollowBoardSettings() throws {
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_LAYOUT"] = "1"
        app.launchEnvironment["CRABIGATOR_TEST_COMPUTERS"] = "1"
        app.launch()
        XCTAssertTrue(app.buttons["Board view"].waitForExistence(timeout: 10))
        for mode in ["PR board", "List"] {
            let control = app.buttons[mode]
            if !(control.value as? String ?? "").hasPrefix("Selected") { control.tap() }
            XCTAssertTrue(app.buttons["Machine First computer"].waitForExistence(timeout: 5), "\(mode): a grouped board starts with computer headings")
            checkRows(in: app, mode, "grouped", contain: "computer", false)
            setRecap(on: true, mode, in: app)
            checkRows(in: app, mode, "grouped with recaps", contain: "Sample recap", true)
            setRecap(on: false, mode, in: app)
            checkRows(in: app, mode, "grouped without recaps", contain: "Sample recap", false)

            groupBy("One list", mode, in: app)
            XCTAssertFalse(app.buttons["Machine First computer"].exists, "\(mode): a combined board has no computer headings")
            capture("\(mode) combined")
            checkRows(in: app, mode, "combined", contain: "computer", true)

            setRecap(on: true, mode, in: app)
            capture("\(mode) with recaps")
            checkRows(in: app, mode, "with recaps", contain: "Sample recap", true)
            setRecap(on: false, mode, in: app)
            checkRows(in: app, mode, "without recaps", contain: "Sample recap", false)

            groupBy("Computer", mode, in: app)
            XCTAssertTrue(app.buttons["Machine First computer"].waitForExistence(timeout: 5), "\(mode): grouping again restores computer headings")
            capture("\(mode) grouped again")
            checkRows(in: app, mode, "grouped again", contain: "computer", false)
        }
    }

    /// Every session row on screen must show, or leave out, the given text. A combined board
    /// names each row's own computer: `first-…` rows run on "First computer".
    private func checkRows(in app: XCUIApplication, _ mode: String, _ step: String, contain text: String, _ expected: Bool) {
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session ")).allElementsBoundByIndex.filter(\.isHittable)
        XCTAssertGreaterThan(rows.count, 5, "\(mode), \(step): the board should show several rows")
        let wrong = rows.compactMap { row -> String? in
            let id = row.identifier, label = row.label
            let wanted = text == "computer" ? (id.hasPrefix("Session first") ? "First computer" : "Second computer") : text
            return label.contains(wanted) == expected ? nil : "\(id) [\(label)]"
        }
        XCTAssertTrue(wrong.isEmpty, "\(mode), \(step): \(wrong.count) of \(rows.count) rows \(expected ? "lack" : "still show") \"\(text)\": \(wrong.joined(separator: "; "))")
    }

    /// The List and the PR board each keep their own Recap choice, in their own View panel.
    private func setRecap(on: Bool, _ mode: String, in app: XCUIApplication) {
        changeView(mode, in: app) { panel in
            let chip = app.buttons["Recap"]
            for _ in 0..<6 where !(chip.exists && chip.isHittable) { panel.swipeUp() }
            XCTAssertTrue(chip.isHittable, "\(mode): the Recap chip should scroll into view")
            if chip.isSelected != on { chip.tap() }
            XCTAssertEqual(chip.isSelected, on, "\(mode): Recap should be \(on ? "on" : "off")")
        }
    }

    private func groupBy(_ tile: String, _ mode: String, in app: XCUIApplication) {
        changeView(mode, in: app) { _ in
            let choice = app.buttons[tile]
            XCTAssertTrue(choice.waitForExistence(timeout: 5))
            choice.tap()
            XCTAssertTrue(choice.isSelected, "\(mode): Group by should be \(tile)")
        }
    }

    /// Opens the View panel for the mode on screen, makes one change, and closes it with Done.
    private func changeView(_ mode: String, in app: XCUIApplication, _ change: (XCUIElement) -> Void) {
        app.buttons["Board view"].tap()
        let title = app.staticTexts["View " + mode]
        XCTAssertTrue(title.waitForExistence(timeout: 5), "View should open for the \(mode)")
        change(app.scrollViews["View panel"])
        app.buttons["Done"].tap()
        let closed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: title)
        XCTAssertEqual(XCTWaiter.wait(for: [closed], timeout: 5), .completed)
        Thread.sleep(forTimeInterval: 0.6)
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
