import XCTest

final class ModuleTests: XCTestCase {
    func testModulesExpandIndependentlyAndReturnSpaceToTheTerminal() {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_MODULES"] = "1"
        app.launch()
        let recap = app.buttons["module-recap"], prs = app.buttons["module-prs"]
        XCTAssertTrue(recap.waitForExistence(timeout: 10))
        XCTAssertEqual(recap.value as? String, "Collapsed")
        XCTAssertEqual(prs.value as? String, "Collapsed")
        let terminal = app.textViews["terminal-output"]
        let collapsedHeight = terminal.frame.height
        XCTAssertTrue(recap.staticTexts["7m ago"].exists)
        XCTAssertTrue(prs.staticTexts["3h ago"].exists)
        XCTAssertTrue(app.buttons["module-commits"].staticTexts["2h ago"].exists)
        capture("Modules collapsed")
        recap.tap()
        XCTAssertEqual(recap.value as? String, "Expanded")
        XCTAssertEqual(prs.value as? String, "Collapsed")
        XCTAssertTrue(app.staticTexts["Translated the app into 21 languages."].exists)
        recap.tap()
        prs.tap()
        XCTAssertEqual(prs.value as? String, "Expanded")
        XCTAssertTrue(app.links.matching(NSPredicate(format: "label CONTAINS %@", "turntouch-ios#15")).firstMatch.exists)
        capture("Pull requests expanded")
        XCTAssertLessThan(terminal.frame.height, collapsedHeight)
        prs.tap()
        waitForHeight(terminal, collapsedHeight)
        app.buttons["module-git"].tap()
        XCTAssertTrue(app.staticTexts["Settings/LanguagePicker.swift"].exists)
        capture("Git files expanded")
        app.buttons["module-git"].tap()
        app.buttons["module-changes"].tap()
        XCTAssertTrue(app.staticTexts["LanguagePicker"].exists)
        app.buttons["module-changes"].tap()
        app.buttons["module-commits"].tap()
        XCTAssertTrue(app.staticTexts["abc1234"].exists)
        app.buttons["module-commits"].tap()
        waitForHeight(terminal, collapsedHeight)
        let summary = app.buttons["session-summary"]
        XCTAssertTrue(summary.exists)
        XCTAssertTrue(summary.label.contains("Complete"))
        XCTAssertTrue(summary.label.contains("Thinking time: 2h 32m"))
        let summaryY = summary.frame.midY
        XCTAssertFalse(app.buttons["Hide session modules"].exists, "The toolbar button was replaced by the summary bar")
        summary.tap()
        XCTAssertFalse(recap.isHittable)
        XCTAssertTrue(summary.isHittable)
        XCTAssertEqual(summary.value as? String, "Collapsed")
        XCTAssertEqual(summary.frame.midY, summaryY, accuracy: 2)
        XCTAssertGreaterThan(terminal.frame.height, collapsedHeight)
        capture("Persistent summary with drawer closed")
        summary.tap()
        XCTAssertTrue(recap.isHittable)
        XCTAssertEqual(summary.value as? String, "Expanded")
        XCTAssertEqual(summary.frame.midY, summaryY, accuracy: 2)
        waitForHeight(terminal, collapsedHeight)
        // Folding the drawer preserves the sections the reader opened.
        recap.tap()
        summary.tap()
        summary.tap()
        XCTAssertEqual(recap.value as? String, "Expanded")
        XCTAssertTrue(app.staticTexts["Translated the app into 21 languages."].exists)
        capture("Summary bar and restored recap")
    }
    private func waitForHeight(_ element: XCUIElement, _ height: CGFloat) {
        let settled = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            abs(element.frame.height - height) < 2
        }, object: element)
        XCTAssertEqual(XCTWaiter.wait(for: [settled], timeout: 4), .completed)
    }
    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
