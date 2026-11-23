import XCTest

final class DemoTests: XCTestCase {
    func testSampleChoiceAndMessage() {
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_DEMO"] = "1"
        app.launch()
        XCTAssertTrue(app.staticTexts["Crabigator demo"].waitForExistence(timeout: 10))
        capture("demo-sessions", app)
        app.buttons["PR board"].tap()
        XCTAssertTrue(app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "PR #23")).firstMatch.exists)
        capture("demo-pr-board", app)
        app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "PR #23")).firstMatch.tap()
        XCTAssertTrue(app.buttons["Dark"].waitForExistence(timeout: 5))
        capture("demo-question", app)
        let input = app.descendants(matching: .any)["demo-message"]
        input.tap()
        XCTAssertTrue(app.buttons["demo-hide-keyboard"].waitForExistence(timeout: 5))
        app.buttons["demo-hide-keyboard"].tap()
        XCTAssertTrue(app.buttons["Dark"].waitForExistence(timeout: 5))
        app.buttons["Dark"].tap()
        XCTAssertFalse(app.buttons["Dark"].exists)
        XCTAssertTrue(input.waitForExistence(timeout: 5))
        input.tap()
        input.typeText("Show the tests")
        XCTAssertTrue(app.buttons["Send"].isHittable)
        capture("demo-keyboard", app)
        app.buttons["Send"].tap()
        let reply = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value CONTAINS %@", "your message stayed on this device"), object: app.textViews["terminal-output"])
        XCTAssertEqual(XCTWaiter.wait(for: [reply], timeout: 5), .completed)
        capture("demo-reply", app)
        let previous = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "Previous prompt")).firstMatch
        XCTAssertTrue(previous.label.contains("3 of 3, Show the tests"))
        previous.tap()
        XCTAssertTrue(previous.label.contains("2 of 3, Dark"))
        previous.tap()
        let next = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "Next prompt")).firstMatch
        XCTAssertTrue(next.label.contains("3 of 3, Show the tests"))
        next.tap()
        XCTAssertFalse(next.exists)
        app.buttons["Back"].tap()
        XCTAssertTrue(app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "PR #24")).firstMatch.waitForExistence(timeout: 5))
    }

    private func capture(_ name: String, _ app: XCUIApplication) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
