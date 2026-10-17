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
        XCTAssertEqual(terminal.frame.height, collapsedHeight, accuracy: 2)
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
        XCTAssertEqual(terminal.frame.height, collapsedHeight, accuracy: 2)
        app.buttons["Hide session modules"].tap()
        XCTAssertFalse(recap.exists)
        XCTAssertGreaterThan(terminal.frame.height, collapsedHeight)
        app.buttons["Show session modules"].tap()
        XCTAssertTrue(recap.exists)
        XCTAssertEqual(terminal.frame.height, collapsedHeight, accuracy: 2)
    }
    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
