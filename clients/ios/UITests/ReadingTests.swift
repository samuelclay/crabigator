import XCTest
import UIKit

/// Opt-in live checks. Pair this simulator first; these never send terminal input.
final class ReadingTests: XCTestCase {
    func testBoardSpacingAndFloatingToolbar() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired tablet.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 25))
        let appBounds = app.frame
        if let id = ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SESSION_ID"] {
            let quietSession = app.buttons["Session " + id]
            if quietSession.isHittable { quietSession.tap() }
        }
        func openStyle() {
            app.buttons["Settings"].tap()
            app.popovers.buttons["Style"].tap()
        }
        func dismissStyle() { app.coordinate(withNormalizedOffset: CGVector(dx: 0.05, dy: 0.9)).tap() }
        openStyle()
        let spacing = app.segmentedControls["Board spacing"]
        let original = spacing.buttons.allElementsBoundByIndex.first { $0.isSelected }?.label ?? "Normal"
        let scope = app.segmentedControls["Style scope"]
        XCTAssertTrue(scope.buttons["List / PR board"].isSelected)
        XCTAssertFalse(app.segmentedControls["Terminal line spacing"].exists)
        capture("List and PR board style controls")
        scope.buttons["Terminal"].tap()
        XCTAssertFalse(spacing.exists)
        capture("Terminal style controls")
        let terminal = app.segmentedControls["Terminal line spacing"].buttons.allElementsBoundByIndex.first { $0.isSelected }?.label
        XCTAssertNotNil(terminal)
        dismissStyle()
        for name in ["List", "PR board"] {
            let mode = app.buttons[name]
            if !(mode.value as? String ?? "").hasPrefix("Selected") { mode.tap() }
            let selected = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in (mode.value as? String ?? "").hasPrefix("Selected") }, object: mode)
            XCTAssertEqual(XCTWaiter.wait(for: [selected], timeout: 15), .completed)
            if mode.value as? String != "Selected, full width" { mode.tap() }
            let fullscreen = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in mode.value as? String == "Selected, full width" }, object: mode)
            XCTAssertEqual(XCTWaiter.wait(for: [fullscreen], timeout: 15), .completed)
            let search = app.buttons["Search"]
            let centered = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in abs(search.frame.midX - appBounds.midX) < 100 }, object: search)
            XCTAssertEqual(XCTWaiter.wait(for: [centered], timeout: 20), .completed)
            XCTAssertLessThan(search.frame.minY, appBounds.minY + 90)
            XCTAssertLessThan(abs(search.frame.midX - appBounds.midX), 100)
            let tappable = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in search.isHittable }, object: search)
            XCTAssertEqual(XCTWaiter.wait(for: [tappable], timeout: 15), .completed)
            search.tap()
            let field = app.textFields["Search sessions"]
            XCTAssertTrue(field.isHittable)
            XCTAssertGreaterThanOrEqual(field.frame.minY, search.frame.maxY)
            search.tap()
            let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
            let requested = ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SESSION_ID"]
            let row = try requested.map { app.buttons["Session " + $0] } ?? app.buttons[XCTUnwrap(rows.allElementsBoundByIndex.first { $0.isHittable }).identifier]
            row.tap()
            XCTAssertEqual(mode.value as? String, "Selected, sidebar")
            let sidebar = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in search.frame.maxX <= appBounds.minX + 360 }, object: search)
            XCTAssertEqual(XCTWaiter.wait(for: [sidebar], timeout: 20), .completed)
            openStyle(); spacing.buttons["Normal"].tap(); dismissStyle()
            let normalHeight = row.frame.height
            openStyle(); spacing.buttons["Tight"].tap()
            scope.buttons["Terminal"].tap()
            XCTAssertEqual(app.segmentedControls["Terminal line spacing"].buttons.allElementsBoundByIndex.first { $0.isSelected }?.label, terminal)
            dismissStyle()
            XCTAssertLessThan(row.frame.height, normalHeight)
            capture("Tight \(name) sidebar")
            if name == "List" {
                app.buttons["Style"].tap()
                XCTAssertTrue(scope.buttons["Terminal"].isSelected)
                dismissStyle()
            }
            mode.tap()
            capture("Floating toolbar above tight \(name)")
        }
        app.terminate(); app.launch()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 25))
        openStyle()
        XCTAssertTrue(spacing.buttons["Tight"].isSelected)
        spacing.buttons[original].tap()
        dismissStyle()
    }

    func testPRBoardRowsFitAfterSelectingFromFullscreen() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired tablet.")
        }
        continueAfterFailure = false
        let startingOrientation = XCUIDevice.shared.orientation
        defer { XCUIDevice.shared.orientation = startingOrientation }
        XCUIDevice.shared.orientation = .landscapeLeft
        let app = XCUIApplication()
        app.launch()
        let mode = app.buttons["PR board"]
        XCTAssertTrue(mode.waitForExistence(timeout: 25))
        if !(mode.value as? String ?? "").hasPrefix("Selected") { mode.tap() }
        if mode.value as? String != "Selected, full width" { mode.tap() }
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 25))
        let requestedID = ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SESSION_ID"]
        let visible = rows.allElementsBoundByIndex.filter { $0.isHittable }
        let initial = try XCTUnwrap(requestedID.map { app.buttons["Session " + $0] } ?? visible.dropFirst().first)
        let row = app.buttons[initial.identifier]
        let other = app.buttons[try XCTUnwrap(visible.first { $0.identifier != row.identifier }).identifier]
        for orientation in [UIDeviceOrientation.landscapeLeft, .portrait, .landscapeRight] {
            XCUIDevice.shared.orientation = orientation
            other.tap()
            mode.tap()
            row.tap()
            XCTAssertEqual(mode.value as? String, "Selected, sidebar")
            XCTAssertEqual(row.value as? String, "Selected")
            capture("Selected PR row fits sidebar")
            print("Selected row bounds: \(row.frame), app: \(app.frame)")
            XCTAssertGreaterThanOrEqual(row.frame.minX, app.frame.minX)
            XCTAssertLessThanOrEqual(row.frame.width, 360)
            XCTAssertGreaterThan(row.frame.height, 40)
            let heading = app.staticTexts["Selected session " + String(row.identifier.dropFirst("Session ".count))]
            let title = row.staticTexts[heading.label]
            XCTAssertTrue(title.exists)
            XCTAssertTrue(title.isHittable)
            XCTAssertGreaterThan(title.frame.width, 0)
            XCTAssertGreaterThanOrEqual(title.frame.minX, row.frame.minX)
            XCTAssertLessThanOrEqual(title.frame.maxX, row.frame.maxX)
            mode.tap()
            XCTAssertEqual(mode.value as? String, "Selected, full width")
            XCTAssertNotEqual(row.value as? String, "Selected")
        }
        row.tap()
    }

    func testApplePushDelivery() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_APNS_TEST"] == "1" else {
            throw XCTSkip("Requires a paired physical device and a separate live push trigger.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 25))
        app.buttons["Settings"].tap()
        app.buttons["Notifications"].tap()
        app.buttons["Enable notifications"].tap()
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let allow = springboard.buttons["Allow"]
        if allow.waitForExistence(timeout: 3) { allow.tap() }
        XCTAssertTrue(app.staticTexts["Notifications on"].waitForExistence(timeout: 15))
        XCUIDevice.shared.press(.home)
        print("Ready for live APNs delivery")
        let notification = springboard.staticTexts["Crabigator push delivery check"]
        XCTAssertTrue(notification.waitForExistence(timeout: 60))
        capture("Real APNs notification on ClayPad Mini")
        app.activate()
    }

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
        let initialRow = try XCTUnwrap(requestedID.map { app.buttons["Session " + $0] } ?? rows.allElementsBoundByIndex.first { $0.isHittable })
        let row = app.buttons[initialRow.identifier]
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
            XCTAssertNotEqual(row.value as? String, "Selected")
            XCTAssertFalse(app.staticTexts["Selected session " + id].isHittable)
            capture("Full width PR board remembers session without highlighting it")
            app.buttons["PR board"].tap()
            XCTAssertTrue(app.staticTexts["Selected session " + id].waitForExistence(timeout: 5))
            XCTAssertEqual(row.value as? String, "Selected")
            app.buttons["List"].tap()
            XCTAssertEqual(row.value as? String, "Selected")
            capture("Selected session in List sidebar")
            app.buttons["List"].tap()
            XCTAssertNotEqual(row.value as? String, "Selected")
            XCTAssertFalse(app.staticTexts["Selected session " + id].isHittable)
            capture("Full width List remembers session without highlighting it")
            app.buttons["List"].tap()
            XCTAssertTrue(app.staticTexts["Selected session " + id].waitForExistence(timeout: 5))
            XCTAssertEqual(row.value as? String, "Selected")
            app.buttons["Close session"].tap()
        } else {
            app.buttons["Back to list"].tap()
            XCTAssertEqual(app.buttons["PR board"].value as? String, "Selected")
        }
        if !(app.buttons["List"].value as? String ?? "").hasPrefix("Selected") { app.buttons["List"].tap() }
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
        let reconnect = app.buttons["Reconnect"]
        let ready = XCTNSPredicateExpectation(predicate: NSPredicate(format: "hittable == true"), object: reconnect)
        XCTAssertEqual(XCTWaiter.wait(for: [ready], timeout: 5), .completed)
        reconnect.tap()
        capture("Slow terminal remains loading")
    }

    func testReadLiveSessionAndReturnToBoard() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Set CRABIGATOR_LIVE_UI_TESTS=1 and pair this simulator before running live checks.")
        }
        continueAfterFailure = false
        let startingOrientation = XCUIDevice.shared.orientation
        defer { XCUIDevice.shared.orientation = startingOrientation }
        if UIDevice.current.userInterfaceIdiom == .pad { XCUIDevice.shared.orientation = .landscapeLeft }
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
        let scope = app.segmentedControls["Style scope"]
        let oldBoard = board.label
        scope.buttons["Terminal"].tap()
        let oldTerminal = terminal.label
        scope.buttons["List / PR board"].tap()
        for _ in 0..<4 { board.buttons["Board text size-Increment"].tap() }
        let largerBoard = board.label
        XCTAssertNotEqual(largerBoard, oldBoard)
        scope.buttons["Terminal"].tap()
        XCTAssertEqual(terminal.label, oldTerminal)
        capture("Independent text size controls")
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.02, dy: 0.7)).tap()
        capture("Larger list text")
        app.buttons["PR board"].tap()
        capture("Larger PR board text")
        app.terminate(); app.launch()
        app.buttons["Settings"].tap(); app.buttons["Style"].tap()
        XCTAssertEqual(board.label, largerBoard)
        scope.buttons["Terminal"].tap()
        XCTAssertEqual(terminal.label, oldTerminal)
        scope.buttons["List / PR board"].tap()
        for _ in 0..<4 { board.buttons["Board text size-Decrement"].tap() }
        XCTAssertEqual(board.label, oldBoard)
    }
    func testBoardSettingsAndActionForms() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired tablet.")
        }
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .landscapeRight
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 25))
        app.buttons["Settings"].tap()
        let cutoff = app.buttons["Activity cutoff"]
        XCTAssertTrue(cutoff.waitForExistence(timeout: 5))
        cutoff.tap()
        app.buttons["Last hour"].tap()
        capture("Board activity cutoff")
        app.terminate(); app.launch()
        app.buttons["Settings"].tap()
        XCTAssertTrue(cutoff.waitForExistence(timeout: 5))
        XCTAssertTrue(cutoff.label.contains("Last hour") || (cutoff.value as? String) == "Last hour")
        cutoff.tap()
        let day = app.buttons["Last 24 hours"]
        XCTAssertTrue(day.waitForExistence(timeout: 5))
        day.tap()
        // Native menus can open upward after relaunch; retry if the first tap only dismisses it.
        if !cutoff.label.contains("Last 24 hours") {
            cutoff.tap(); day.tap()
        }
        XCTAssertTrue(cutoff.label.contains("Last 24 hours"))
        func reveal(_ button: XCUIElement) {
            for _ in 0..<4 {
                if button.exists && button.isHittable { return }
                app.coordinate(withNormalizedOffset: CGVector(dx: 0.59, dy: 0.70)).press(forDuration: 0.05, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.59, dy: 0.36)))
            }
        }
        let watch = app.buttons["Watch PR"]
        reveal(watch)
        watch.tap()
        let target = app.textFields["Watch PR target"]
        XCTAssertTrue(target.waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Watch PR"].isEnabled)
        target.tap(); target.typeText("owner/repo#12")
        XCTAssertTrue(app.buttons["Watch PR"].isEnabled)
        capture("Watch PR form")
        app.terminate(); app.launch()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 15))
        app.buttons["Settings"].tap()
        let newSession = app.buttons["New session"]
        reveal(newSession)
        newSession.tap()
        XCTAssertTrue(app.textFields["Folder"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Start session"].exists)
        capture("New session desktop and folder")
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.03, dy: 0.9)).tap()
    }
    private func capture(_ name: String) {
        // The app's longest transition is 360 ms. Capture its settled geometry.
        Thread.sleep(forTimeInterval: 0.5)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
