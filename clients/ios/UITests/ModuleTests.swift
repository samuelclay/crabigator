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
    }
    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
}
