import XCTest
import UIKit

/// The filter bar, the three View panels, Account and the + menu, on a sample roster
/// (`CRABIGATOR_TEST_LAYOUT` and `CRABIGATOR_TEST_SAMPLE`). No pairing is needed. Each
/// screenshot is kept in the result bundle, named for the phone or the tablet.
final class ViewPanelTests: XCTestCase {
    private var app: XCUIApplication!
    private var tablet: Bool { UIDevice.current.userInterfaceIdiom == .pad }

    func testFilterChipsAndTheListViewPanel() throws {
        launch(mode: "list")
        capture("list")
        // A chip and the panel's ruler edit the same activity window.
        let time = app.buttons["Time filter"]
        XCTAssertEqual(time.label, "24h")
        time.tap()
        XCTAssertTrue(app.buttons["Last 3 hours"].waitForExistence(timeout: 5))
        for choice in ["Last hour", "Last 6 hours", "Last 9 hours", "Last 12 hours", "Last 24 hours", "All ages"] { XCTAssertTrue(app.buttons[choice].exists, choice) }
        capture("list-time-menu")
        app.buttons["Last 3 hours"].tap()
        expect(time, label: "3h")
        XCTAssertFalse(app.buttons["Session sample-search"].exists, "A session last prompted five hours ago is outside three hours")
        var panel = openView("List")
        XCTAssertTrue(app.buttons["Ruler 3h"].isSelected)
        app.buttons["Ruler 24h"].tap()
        XCTAssertTrue(app.buttons["Ruler 24h"].isSelected)
        let ended = app.switches["Ended sessions"]
        XCTAssertEqual(ended.value as? String, "0")
        ended.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).tap()
        XCTAssertEqual(ended.value as? String, "1")
        XCTAssertTrue(app.buttons["Computer All computers"].isSelected)
        XCTAssertTrue(app.buttons["Computer Laptop"].exists && app.buttons["Computer Studio Mac"].exists)
        capture("list-view-1-which-sessions")

        // Group by is one picker over the two older settings.
        for tile in ["One list", "Computer", "Project", "Computer, then project"] { XCTAssertTrue(app.buttons[tile].exists, tile) }
        XCTAssertTrue(app.buttons["Computer"].isSelected)
        XCTAssertFalse(app.buttons["Sort projects A to Z"].exists)
        app.buttons["Computer, then project"].tap()
        XCTAssertTrue(app.buttons["Computer, then project"].isSelected)
        XCTAssertTrue(app.buttons["Sort projects Most recent first"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Sort projects Most recent first"].isSelected)
        reveal(app.buttons["Sort projects A to Z"], in: panel)
        capture("list-view-2-group-by-project")
        app.buttons["Computer"].tap()
        XCTAssertFalse(app.buttons["The pull request"].exists, "The block choice belongs to the PR board")

        // The sample row is the real row: each chip shows what it adds.
        let sample = app.descendants(matching: .any).matching(identifier: "Sample row").firstMatch
        let git = app.buttons["Git status"]
        reveal(app.buttons["Changes"], in: panel)
        XCTAssertFalse(git.isSelected, "Git status starts off")
        XCTAssertFalse(sample.label.contains("not committed"))
        git.tap()
        XCTAssertTrue(git.isSelected)
        XCTAssertTrue(sample.label.contains("3 files not committed"), sample.label)
        for chip in ["Recap", "Session time", "Thinking time", "Last prompt", "Last completion", "Tools", "Compactions", "Context", "Git status", "Changes"] { XCTAssertTrue(app.buttons[chip].exists, chip) }
        // A fresh install hides Tools and Compactions, so Context starts off with them.
        XCTAssertFalse(app.buttons["Context"].isSelected)
        XCTAssertFalse(sample.label.contains("186k / 1M"), sample.label)
        app.buttons["Context"].tap()
        XCTAssertTrue(sample.label.contains("186k / 1M · 19%"), sample.label)
        app.buttons["Changes"].tap()
        XCTAssertTrue(app.staticTexts["The PR board keeps its own set."].exists)
        capture("list-view-3-row-shows")

        reveal(app.buttons["Columns 4"], in: panel)
        XCTAssertEqual(app.staticTexts["Board text size"].label, "14 pt")
        app.buttons["Larger text"].tap()
        XCTAssertEqual(app.staticTexts["Board text size"].label, "15 pt")
        app.buttons["Smaller text"].tap()
        app.buttons["Row spacing Tight"].tap()
        XCTAssertTrue(app.buttons["Row spacing Tight"].isSelected)
        app.buttons["Row spacing Normal"].tap()
        XCTAssertTrue(app.buttons["Columns Auto"].isSelected)
        for count in 1...4 { XCTAssertTrue(app.buttons["Columns \(count)"].exists) }
        if tablet { reveal(app.buttons["Right"], in: panel) }
        XCTAssertEqual(app.buttons["Left"].exists, tablet, "Only a tablet has a session list beside the session")
        capture("list-view-4-size-and-spacing")
        checkTouchTargets(in: panel)
        closeView()

        // The board and the chips follow what the panel changed.
        expect(time, label: "24h")
        XCTAssertEqual(app.buttons["Sessions filter"].label, "Live + ended")
        let row = app.buttons["Session sample-live"]
        XCTAssertTrue(row.waitForExistence(timeout: 5))
        XCTAssertTrue(row.label.contains("3 files not committed"), row.label)
        XCTAssertTrue(row.label.contains("186k / 1M · 19%"), row.label)
        capture("list-with-context-git-changes")

        // The PR board keeps its own row set.
        app.buttons["PR board"].tap()
        panel = openView("PR board")
        reveal(app.buttons["Git status"], in: panel)
        XCTAssertFalse(app.buttons["Git status"].isSelected, "Turning Git status on for the List leaves the PR board alone")
        // Context starts off on the PR board, whatever the List shows. Turned on, it stays on.
        let context = app.buttons["Context"]
        XCTAssertFalse(context.isSelected)
        XCTAssertFalse(row.label.contains("186k / 1M"), row.label)
        reveal(context, in: panel)
        context.tap()
        XCTAssertTrue(context.isSelected)
        closeView()
        XCTAssertTrue(row.label.contains("186k / 1M · 19%"), row.label)
        _ = openView("PR board")
        reveal(context, in: app.scrollViews["View panel"])
        XCTAssertTrue(context.isSelected, "The saved set keeps Context on")
        closeView()
    }

    func testPRBoardViewPanel() throws {
        launch(mode: "prs", rows: "context,git,changes")
        capture("prboard")
        let panel = openView("PR board")
        capture("prboard-view-1-which-sessions")
        // The PR board groups by computer or not at all.
        XCTAssertTrue(app.buttons["One list"].exists && app.buttons["Computer"].isSelected)
        XCTAssertFalse(app.buttons["Project"].exists || app.buttons["Computer, then project"].exists)
        let lead = app.buttons["The pull request"]
        reveal(lead, in: panel)
        XCTAssertTrue(app.buttons["The session"].isSelected)
        capture("prboard-view-2-group-and-block")
        lead.tap()
        XCTAssertTrue(lead.isSelected)
        reveal(app.buttons["Changes"], in: panel)
        XCTAssertTrue(app.buttons["Last prompt"].isSelected && app.buttons["Last completion"].isSelected)
        XCTAssertTrue(app.staticTexts["The List keeps its own set."].exists)
        capture("prboard-view-3-row-shows")
        reveal(app.buttons["Row spacing Relaxed"], in: panel)
        XCTAssertFalse(app.buttons["Columns Auto"].exists, "Columns only affect the List")
        capture("prboard-view-4-size-and-spacing")
        checkTouchTargets(in: panel)
        closeView()
        capture("prboard-by-pull-request")
        reveal(app.buttons["The session"], in: openView("PR board"))
        XCTAssertTrue(app.buttons["The pull request"].isSelected, "The block choice is saved")
        app.buttons["The session"].tap()
        closeView()
    }

    func testSessionViewPanel() throws {
        launch(mode: "list")
        app.buttons["Session sample-live"].tap()
        let view = app.buttons["Session view"]
        XCTAssertTrue(view.waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 0.8)
        capture("session")
        view.tap()
        XCTAssertTrue(app.staticTexts["View Session"].waitForExistence(timeout: 5))
        let panel = app.scrollViews["View panel"]
        capture("session-view-1-terminal-text")
        XCTAssertEqual(app.staticTexts["Terminal text size"].label, "13 pt")
        app.buttons["Larger text"].tap()
        XCTAssertEqual(app.staticTexts["Terminal text size"].label, "14 pt")
        app.buttons["Smaller text"].tap()
        app.buttons["Line spacing Relaxed"].tap()
        XCTAssertTrue(app.buttons["Line spacing Relaxed"].isSelected)
        app.buttons["Line spacing Normal"].tap()
        let sideways = app.buttons["Scroll sideways"]
        reveal(sideways, in: panel)
        XCTAssertTrue(app.buttons["Wrap"].isSelected)
        sideways.tap()
        XCTAssertTrue(sideways.isSelected)
        app.buttons["Wrap"].tap()
        let tucked = app.buttons["Details drawer Tucked away"]
        reveal(tucked, in: panel)
        for height in ["Fill", "Short", "Medium", "Tall", "Tallest"] { XCTAssertTrue(app.buttons["Terminal height " + height].exists, height) }
        app.buttons["Terminal height Medium"].tap()
        XCTAssertTrue(app.buttons["Terminal height Medium"].isSelected)
        capture("session-view-2-terminal-height")
        tucked.tap()
        XCTAssertTrue(tucked.isSelected)
        app.buttons["Details drawer Open"].tap(); app.buttons["Terminal height Fill"].tap()
        let prompts = app.buttons["Prompts"]
        reveal(prompts, in: panel)
        for chip in ["Recap", "Pull requests", "Commits", "Git status", "Changes", "Flow scene", "Session time", "Thinking time", "Prompts", "Completions", "Tools", "Compactions", "Context"] { XCTAssertTrue(app.buttons[chip].exists, chip) }
        XCTAssertTrue(prompts.isSelected, "Prompts and Completions start on")
        XCTAssertFalse(app.buttons["Context"].isSelected, "Context starts off while Tools and Compactions are hidden")
        XCTAssertFalse(app.buttons["Tools"].isSelected)
        capture("session-view-3-drawer-and-stats")
        prompts.tap()
        XCTAssertFalse(prompts.isSelected)
        reveal(app.buttons["Context"], in: panel)
        app.buttons["Context"].tap()
        XCTAssertTrue(app.buttons["Context"].isSelected)
        let transcripts = app.buttons["Voice transcripts for this session"]
        reveal(transcripts, in: panel)
        capture("session-view-4-end")
        checkTouchTargets(in: panel)
        transcripts.tap()
        XCTAssertTrue(app.navigationBars["Voice transcripts"].waitForExistence(timeout: 5))
        app.navigationBars["Voice transcripts"].buttons.firstMatch.tap()
        XCTAssertTrue(app.staticTexts["View Session"].waitForExistence(timeout: 5))
        app.buttons["Done"].tap()
        // The stats strip follows its own set.
        let summary = app.descendants(matching: .any).matching(identifier: "session-summary").firstMatch
        XCTAssertTrue(summary.waitForExistence(timeout: 5))
        XCTAssertFalse(summary.label.contains("Prompts:"), summary.label)
        XCTAssertTrue(summary.label.contains("Completions:"), summary.label)
        XCTAssertTrue(summary.label.contains("Context: 186k / 1M · 19%"), summary.label)
    }

    func testAccountAndTheAddMenu() throws {
        launch(mode: "list")
        app.buttons["Account"].tap()
        let screen = app.scrollViews["Account screen"]
        XCTAssertTrue(screen.waitForExistence(timeout: 5))
        for heading in ["Account", "Alerts and history", "Sign-in", "Devices and agents", "This device", "Connect an agent with MCP"] { XCTAssertTrue(app.staticTexts[heading].exists, heading) }
        for control in ["Notifications", "Voice transcripts", "Pair another device", "Manage sign-in on the web", "Tools and examples"] { XCTAssertTrue(app.buttons[control].exists || app.links[control].exists, control) }
        capture("account")
        let unpair = app.buttons["Unpair this device"]
        reveal(unpair, in: screen)
        XCTAssertTrue(app.links["Privacy policy"].exists || app.buttons["Privacy policy"].exists)
        capture("account-end")
        unpair.tap()
        XCTAssertTrue(app.buttons["Unpair"].waitForExistence(timeout: 5), "Unpairing asks first")
        capture("account-unpair-question")
        // Leave without unpairing: Cancel where the question has one, otherwise a tap outside it.
        if app.buttons["Cancel"].exists { app.buttons["Cancel"].tap() } else { app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.06)).tap() }
        let asked = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.buttons["Unpair"])
        XCTAssertEqual(XCTWaiter.wait(for: [asked], timeout: 5), .completed)
        XCTAssertTrue(screen.exists, "Leaving the question keeps Account open")
        for _ in 0..<4 where !(app.buttons["Notifications"].exists && app.buttons["Notifications"].isHittable) { screen.swipeDown() }
        app.buttons["Notifications"].tap()
        XCTAssertTrue(app.buttons["Enable notifications"].waitForExistence(timeout: 5), "Notifications opens the existing page")
        app.navigationBars.buttons.firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Alerts and history"].waitForExistence(timeout: 5))
        app.buttons["Done"].tap()

        // Watch PR lives in the + menu, with New session.
        let add = app.buttons["Add"]
        XCTAssertTrue(add.waitForExistence(timeout: 5))
        waitUntilHittable(add)
        add.tap()
        XCTAssertTrue(app.buttons["Watch a pull request"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["New session"].exists)
        capture("add-menu")
        app.buttons["Watch a pull request"].tap()
        let target = app.textFields["Watch PR target"]
        XCTAssertTrue(target.waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Watch PR"].isEnabled)
        target.tap(); target.typeText("owner/repo#12")
        XCTAssertTrue(app.buttons["Watch PR"].isEnabled)
        capture("watch-pull-request")
        app.buttons["Done"].tap()

        guard tablet else { return }
        // On a tablet, a header button swaps the full-width board and the session.
        XCTAssertFalse(app.buttons["Return to session"].exists, "Nothing to return to before a session is chosen")
        let row = app.buttons["Session sample-live"]
        waitUntilHittable(row)
        row.tap()
        let expand = app.buttons["Expand List / PR board"]
        XCTAssertTrue(expand.waitForExistence(timeout: 5))
        XCTAssertEqual(app.buttons["List"].value as? String, "Selected, sidebar")
        capture("sidebar-and-session")
        expand.tap()
        XCTAssertTrue(app.buttons["Return to session"].waitForExistence(timeout: 5))
        XCTAssertEqual(app.buttons["List"].value as? String, "Selected, full width")
        Thread.sleep(forTimeInterval: 0.6)
        capture("full-width-with-return")
        app.buttons["Return to session"].tap()
        XCTAssertTrue(expand.waitForExistence(timeout: 5))
        XCTAssertEqual(app.buttons["List"].value as? String, "Selected, sidebar")
    }

    private func launch(mode: String, rows: String = "") {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_LAYOUT"] = "1"
        app.launchEnvironment["CRABIGATOR_TEST_SAMPLE"] = "1"
        app.launchEnvironment["CRABIGATOR_TEST_MODE"] = mode
        if !rows.isEmpty { app.launchEnvironment["CRABIGATOR_TEST_ROWS"] = rows }
        app.launch()
        XCTAssertTrue(app.buttons["Board view"].waitForExistence(timeout: 10))
    }

    private func openView(_ scope: String) -> XCUIElement {
        let button = app.buttons["Board view"]
        waitUntilHittable(button)
        button.tap()
        XCTAssertTrue(app.staticTexts["View " + scope].waitForExistence(timeout: 5), "View should open for the \(scope)")
        XCTAssertTrue(button.isSelected, "The View button is filled while its panel is open")
        return app.scrollViews["View panel"]
    }

    private func closeView() {
        let panel = app.scrollViews["View panel"]
        app.buttons["Done"].tap()
        let closed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: panel)
        XCTAssertEqual(XCTWaiter.wait(for: [closed], timeout: 5), .completed)
        Thread.sleep(forTimeInterval: 0.6)
    }

    /// Scrolls the panel until the control is wholly inside it.
    private func reveal(_ element: XCUIElement, in panel: XCUIElement) {
        for _ in 0..<10 {
            let frame = panel.frame
            if element.exists && element.isHittable && element.frame.minY >= frame.minY && element.frame.maxY <= frame.maxY - 24 { return }
            let below = !element.exists || element.frame.midY > frame.midY
            let from = panel.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: below ? 0.7 : 0.35))
            let to = panel.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: below ? 0.35 : 0.7))
            from.press(forDuration: 0.05, thenDragTo: to, withVelocity: .slow, thenHoldForDuration: 0.3)
        }
        XCTFail("\(element) did not scroll into view")
    }

    /// Every control wholly on screen in the panel is at least 44 points each way.
    private func checkTouchTargets(in panel: XCUIElement) {
        let visible = panel.frame.insetBy(dx: 0, dy: 1)
        for button in panel.buttons.allElementsBoundByIndex where button.isHittable && visible.contains(button.frame) {
            XCTAssertGreaterThanOrEqual(button.frame.height, 43.5, "\(button.label) is too short to tap")
            XCTAssertGreaterThanOrEqual(button.frame.width, 43.5, "\(button.label) is too narrow to tap")
        }
    }

    private func expect(_ element: XCUIElement, label: String) {
        let changed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label == %@", label), object: element)
        XCTAssertEqual(XCTWaiter.wait(for: [changed], timeout: 5), .completed, "\(element.label) should read \(label)")
    }

    private func waitUntilHittable(_ element: XCUIElement) {
        let ready = XCTNSPredicateExpectation(predicate: NSPredicate(format: "hittable == true"), object: element)
        XCTAssertEqual(XCTWaiter.wait(for: [ready], timeout: 5), .completed, "\(element) should be on screen")
    }

    private func capture(_ name: String) {
        // The app's longest transition is 360 ms. Capture its settled geometry.
        Thread.sleep(forTimeInterval: 0.5)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = (tablet ? "ipad-" : "iphone-") + name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
