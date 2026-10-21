import XCTest
import UIKit

/// Opt-in live checks. Pair this simulator first; these never send terminal input.
final class ReadingTests: XCTestCase {
    private func descendants(of snapshot: XCUIElementSnapshot) -> [XCUIElementSnapshot] {
        snapshot.children.flatMap { [$0] + descendants(of: $0) }
    }
    func testLiveSessionAppearsUnderExpectedAgeBand() throws {
        let env = ProcessInfo.processInfo.environment
        guard env["CRABIGATOR_LIVE_UI_TESTS"] == "1",
              let id = env["CRABIGATOR_TEST_SESSION_ID"],
              let query = env["CRABIGATOR_TEST_SESSION_QUERY"],
              let band = env["CRABIGATOR_TEST_AGE_BAND"] else {
            throw XCTSkip("Requires a paired device, a session, search text and its expected age band.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        let mode = app.buttons["PR board"]
        XCTAssertTrue(mode.waitForExistence(timeout: 25))
        if !(mode.value as? String ?? "").hasPrefix("Selected") { mode.tap() }
        app.buttons["Search"].tap()
        let search = app.textFields["Search sessions"]
        XCTAssertTrue(search.waitForExistence(timeout: 5), app.debugDescription)
        search.tap(); search.typeText(query)
        let row = app.buttons["Session " + id]
        XCTAssertTrue(row.waitForExistence(timeout: 20))
        let heading = app.staticTexts[band]
        XCTAssertTrue(heading.waitForExistence(timeout: 5))
        let snapshot = try app.snapshot()
        let visibleRow = try XCTUnwrap(descendants(of: snapshot).first { $0.identifier == "Session " + id })
        let bands = ["Last hour", "1–3 hours", "3–6 hours", "6–9 hours", "9–12 hours", "12–24 hours", "Older"]
        let preceding = descendants(of: snapshot).filter {
            $0.elementType == .staticText && bands.contains($0.label) && $0.frame.maxY <= visibleRow.frame.minY
        }.max { $0.frame.maxY < $1.frame.maxY }
        XCTAssertEqual(preceding?.label, band, "The row must belong to the expected band, not a later section")
        capture("Waiting session in its prompt age band")
    }
    func testTerminalLinksOpenFromHistoryAndLiveScreen() {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_LINKS"] = "1"
        app.launch()
        for _ in 0..<2 {
            for (label, url) in [("History PR", "https://example.com/history"), ("Codex docs", "https://example.com/docs"), ("https://example.com/plain", "https://example.com/plain")] {
                let link = app.links[label]
                XCTAssertTrue(link.waitForExistence(timeout: 10))
                link.tap()
                let opened = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label == %@", url), object: app.staticTexts["opened-terminal-link"])
                XCTAssertEqual(XCTWaiter.wait(for: [opened], timeout: 3), .completed)
            }
            app.buttons["Toggle wrapping"].tap()
        }
        capture("Tappable terminal links")
    }
    func testCachedTerminalLoadsWithoutAWorkingWebSocket() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired device and a server with terminal snapshots.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_STALL_TERMINAL_SOCKET"] = "1"
        app.launch()
        XCTAssertTrue(app.buttons["PR board"].waitForExistence(timeout: 25) || app.buttons["Back to list"].exists)
        if app.buttons["Back to list"].exists { app.buttons["Back to list"].tap() }
        if let query = ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SESSION_QUERY"] {
            app.buttons["Search"].tap()
            let search = app.textFields["Search sessions"]
            XCTAssertTrue(search.waitForExistence(timeout: 5))
            search.tap(); search.typeText(query)
        }
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 20))
        let id = ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SESSION_ID"]
        let row = try XCTUnwrap(id.map { app.buttons["Session " + $0] } ?? rows.allElementsBoundByIndex.first { $0.isHittable })
        let start = Date()
        row.tap()
        let terminal = app.textViews["terminal-output"]
        let loaded = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            terminal.exists && !(terminal.value as? String ?? "").isEmpty
        }, object: terminal)
        XCTAssertEqual(XCTWaiter.wait(for: [loaded], timeout: 8), .completed,
                       "The cached terminal must load even when the WebSocket cannot deliver a frame")
        print("Cached terminal visible after \(Date().timeIntervalSince(start)) seconds without WebSocket output")
        capture("Terminal recovered over HTTP without WebSocket output")
    }
    func testCompactPRStatusStaysOnOneLine() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired device with visible PRs.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        let mode = app.buttons["PR board"]
        XCTAssertTrue(mode.waitForExistence(timeout: 25))
        if !(mode.value as? String ?? "").hasPrefix("Selected") { mode.tap() }
        if mode.value as? String == "Selected, full width" { mode.tap() }
        let states = app.staticTexts.matching(NSPredicate(format: "label IN %@", ["Open", "Merged", "Closed", "Draft", "Pending"]))
        XCTAssertTrue(states.firstMatch.waitForExistence(timeout: 25))
        let snapshot = try app.snapshot()
        let rows = descendants(of: snapshot).filter { $0.elementType == .button && $0.identifier.hasPrefix("Session ") }
        var checkedStates = 0, checkedDiffs = 0
        for row in rows {
            let texts = descendants(of: row).filter { $0.elementType == .staticText }
            let checks = texts.filter { $0.label == "Checks passed" || $0.label == "CI status unavailable" || $0.label.hasSuffix(" checks failed") || $0.label.hasSuffix(" checks pending") }
            let states = texts.filter { ["Open", "Merged", "Closed", "Draft", "Pending"].contains($0.label) }
            let diffs = texts.filter { $0.label.hasPrefix("+") || $0.label.hasPrefix("−") }
            for text in states + diffs {
                guard text.frame.midY > 100 && text.frame.midY < snapshot.frame.maxY - 90 else { continue }
                let ci = try XCTUnwrap(checks.first { abs($0.frame.midY - text.frame.midY) < 4 }, "CI must share the PR state and diff line")
                XCTAssertGreaterThanOrEqual(text.frame.minX, row.frame.minX)
                XCTAssertGreaterThan(text.frame.width, 0)
                XCTAssertGreaterThan(ci.frame.minX, text.frame.maxX)
                XCTAssertLessThanOrEqual(ci.frame.maxX, row.frame.maxX)
                if ["Open", "Merged", "Closed", "Draft", "Pending"].contains(text.label) { checkedStates += 1 } else { checkedDiffs += 1 }
            }
        }
        XCTAssertGreaterThan(checkedStates, 0, "Requires session grouping with visible PRs")
        XCTAssertGreaterThan(checkedDiffs, 0, "Requires visible PR diff counts")
        capture("Compact PR state diff and CI share one line")
    }

    func testFloatingHeaderConsumesGaps() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_LIVE_UI_TESTS"] == "1" else {
            throw XCTSkip("Requires an already paired tablet.")
        }
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .landscapeRight
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 25))
        for name in ["List", "PR board"] {
            let mode = app.buttons[name]
            if !(mode.value as? String ?? "").hasPrefix("Selected") { tapCenter(mode, in: app) }
            if mode.value as? String != "Selected, full width" { tapCenter(mode, in: app) }
            capture("Fullscreen \(name) before gap tap")
            let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
            let first = try XCTUnwrap(rows.allElementsBoundByIndex.first { $0.isHittable })
            let row = app.buttons[first.identifier]
            let search = app.buttons["Search"], logo = app.images["Crabigator"]
            app.coordinate(withNormalizedOffset: .zero).withOffset(CGVector(dx: (logo.frame.maxX + search.frame.minX) / 2, dy: search.frame.midY)).tap()
            XCTAssertTrue(row.exists, "A tap between header controls must not collapse the device")
            tapCenter(app.buttons["Settings"], in: app)
            XCTAssertTrue(app.buttons["Activity cutoff"].waitForExistence(timeout: 5))
            capture("Settings from floating \(name) header")
            app.coordinate(withNormalizedOffset: CGVector(dx: 0.03, dy: 0.9)).tap()
        }
    }

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
            let style = app.buttons["Style"].firstMatch
            revealInSettings(style, in: app)
            tapCenter(style, in: app)
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
            let machine = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Machine ")).firstMatch
            let before = machine.label
            let logo = app.images["Crabigator"]
            if logo.exists {
                app.coordinate(withNormalizedOffset: .zero).withOffset(CGVector(dx: (logo.frame.maxX + search.frame.minX) / 2, dy: search.frame.midY)).tap()
                XCTAssertEqual(machine.label, before)
            }
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
        XCUIDevice.shared.orientation = .landscapeRight
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
        func texts(in snapshot: XCUIElementSnapshot) -> [XCUIElementSnapshot] {
            snapshot.children.flatMap { child in
                child.elementType == .staticText ? [child] : texts(in: child)
            }
        }
        for _ in 0..<3 {
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
            // Check all visible session rows and PR text, not only the selected title.
            for candidate in rows.allElementsBoundByIndex {
                let snapshot = try candidate.snapshot()
                let bounds = snapshot.frame
                guard bounds.maxY > 100 && bounds.minY < app.frame.maxY - 90 else { continue }
                XCTAssertGreaterThanOrEqual(bounds.minX, app.frame.minX, snapshot.label)
                XCTAssertLessThanOrEqual(bounds.maxX, app.frame.minX + 361, snapshot.label)
                for text in texts(in: snapshot) {
                    let textBounds = text.frame
                    guard textBounds.width > 0 else { continue }
                    XCTAssertGreaterThanOrEqual(textBounds.minX, bounds.minX, text.label)
                    XCTAssertLessThanOrEqual(textBounds.maxX, bounds.maxX, text.label)
                }
            }
            mode.tap()
            XCTAssertEqual(mode.value as? String, "Selected, full width")
            XCTAssertNotEqual(row.value as? String, "Selected")
            capture("PR status columns return beside titles in fullscreen")
            var checkedStatuses = 0
            let appBounds = app.frame
            for candidate in rows.allElementsBoundByIndex {
                let snapshot = try candidate.snapshot()
                let candidateBounds = snapshot.frame
                guard candidateBounds.maxY > 100 && candidateBounds.minY < appBounds.maxY - 90 else { continue }
                let rowTexts = texts(in: snapshot)
                for status in rowTexts where ["Open", "Merged", "Closed", "Draft", "Pending"].contains(status.label) {
                    let bounds = status.frame
                    guard bounds.midY > 100 && bounds.midY < appBounds.maxY - 90 else { continue }
                    checkedStatuses += 1
                    XCTAssertGreaterThan(bounds.minX, appBounds.midX, "PR state/diff must return to the right-hand columns")
                    XCTAssertTrue(rowTexts.contains { text in
                        text.label.hasPrefix("#") && abs(text.frame.midY - bounds.midY) < 16
                    }, "PR status must share its title's line in fullscreen")
                }
            }
            XCTAssertGreaterThan(checkedStatuses, 0, "This live check requires session grouping with visible PRs")
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
        capture("Board before settings")
        tapCenter(app.buttons["Settings"], in: app)
        let cutoff = app.buttons["Activity cutoff"]
        XCTAssertTrue(cutoff.waitForExistence(timeout: 5))
        tapCenter(cutoff, in: app, horizontal: 0.85)
        capture("Activity choices")
        let hour = app.buttons["Last hour"]
        XCTAssertTrue(hour.waitForExistence(timeout: 5))
        tapCenter(hour, in: app)
        capture("Board activity cutoff")
        app.terminate(); app.launch()
        capture("Board before settings")
        tapCenter(app.buttons["Settings"], in: app)
        XCTAssertTrue(cutoff.waitForExistence(timeout: 5))
        XCTAssertTrue(cutoff.label.contains("Last hour") || (cutoff.value as? String) == "Last hour")
        tapCenter(cutoff, in: app, horizontal: 0.85)
        let day = app.buttons["Last 24 hours"]
        XCTAssertTrue(day.waitForExistence(timeout: 5))
        tapCenter(day, in: app)
        // Native menus can open upward after relaunch; retry if the first tap only dismisses it.
        if !cutoff.label.contains("Last 24 hours") {
            tapCenter(cutoff, in: app, horizontal: 0.85); tapCenter(day, in: app)
        }
        XCTAssertTrue(cutoff.label.contains("Last 24 hours"))
        func reveal(_ button: XCUIElement) { revealInSettings(button, in: app) }
        let watch = app.buttons["Watch PR"]
        reveal(watch)
        tapCenter(watch, in: app)
        let target = app.textFields["Watch PR target"]
        XCTAssertTrue(target.waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Watch PR"].isEnabled)
        target.tap(); target.typeText("owner/repo#12")
        XCTAssertTrue(app.buttons["Watch PR"].isEnabled)
        capture("Watch PR form")
        app.terminate(); app.launch()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 15))
        capture("Board before settings")
        tapCenter(app.buttons["Settings"], in: app)
        let newSession = app.descendants(matching: .any).matching(identifier: "Settings form").firstMatch.buttons["New session"]
        reveal(newSession)
        tapCenter(newSession, in: app)
        XCTAssertTrue(app.buttons["New session project"].waitForExistence(timeout: 5))
        app.buttons["New session project"].tap()
        app.buttons["Other folder…"].tap()
        XCTAssertTrue(app.textFields["Folder"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Start session"].exists)
        capture("New session desktop and folder")
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.03, dy: 0.9)).tap()
    }
    private func revealInSettings(_ button: XCUIElement, in app: XCUIApplication) {
        let form = app.descendants(matching: .any).matching(identifier: "Settings form").firstMatch
        XCTAssertTrue(form.waitForExistence(timeout: 5))
        for _ in 0..<6 {
            let frame = form.frame
            let top = frame.minY + 90, bottom = frame.maxY - 40
            if button.exists && button.frame.midY > top && button.frame.midY < bottom { return }
            let down = button.exists && button.frame.midY <= top
            let origin = app.coordinate(withNormalizedOffset: .zero)
            let from = origin.withOffset(CGVector(dx: frame.midX, dy: frame.minY + frame.height * (down ? 0.4 : 0.7)))
            let to = origin.withOffset(CGVector(dx: frame.midX, dy: frame.minY + frame.height * (down ? 0.7 : 0.4)))
            from.press(forDuration: 0.05, thenDragTo: to)
        }
    }
    private func tapCenter(_ element: XCUIElement, in app: XCUIApplication, horizontal: CGFloat = 0.5) {
        let frame = element.frame
        app.coordinate(withNormalizedOffset: .zero).withOffset(CGVector(dx: frame.minX + frame.width * horizontal - app.frame.minX, dy: frame.midY - app.frame.minY)).tap()
    }
    private func capture(_ name: String) {
        // The app's longest transition is 360 ms. Capture its settled geometry.
        Thread.sleep(forTimeInterval: 0.5)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
