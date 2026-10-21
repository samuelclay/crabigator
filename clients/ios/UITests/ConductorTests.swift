import XCTest

final class ConductorTests: XCTestCase {
    func testNewSessionHeaderShowsProjectAndAllAssistants() {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_CONDUCTOR"] = "1"
        app.launchEnvironment["CRABIGATOR_TEST_CONDUCTOR_ROSTER"] = "1"
        app.launch()
        XCTAssertTrue(app.staticTexts["Check app store review"].waitForExistence(timeout: 10))
        let create = app.buttons.matching(identifier: "New session").allElementsBoundByIndex.filter(\.isHittable)
        XCTAssertEqual(create.count, 1)
        create[0].tap()
        XCTAssertTrue(app.buttons["New session project"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["/projects/NewsBlur"].exists)
        for assistant in ["Codex", "Claude", "OpenCode", "Grok"] {
            XCTAssertTrue(app.buttons[assistant].exists)
            app.buttons[assistant].tap()
            XCTAssertEqual(app.buttons[assistant].value as? String, "Selected")
        }
        XCTAssertTrue(app.buttons["Start session"].isHittable)
        XCTAssertLessThan(app.buttons["Grok"].frame.maxY, app.buttons["Start session"].frame.minY)
        let capture = XCTAttachment(screenshot: app.screenshot())
        capture.name = "New session header popover"; capture.lifetime = .keepAlways; add(capture)
        app.buttons["New session project"].tap()
        app.buttons["Other folder…"].tap()
        XCTAssertTrue(app.textFields["Folder"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Start session"].isEnabled)
        app.buttons["Done"].tap()
        app.buttons.matching(identifier: "List").allElementsBoundByIndex.first { $0.isHittable }!.tap()
        XCTAssertTrue(app.buttons["New session"].waitForExistence(timeout: 5))
        app.buttons["New session"].tap()
        XCTAssertTrue(app.buttons["New session project"].waitForExistence(timeout: 5))
    }

    func testVoiceCancelAndEditPreserveDraft() {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_VOICE"] = "1"
        app.launch()
        let input = app.textViews["Message"]
        XCTAssertTrue(input.waitForExistence(timeout: 5))
        input.tap(); input.typeText("Keep my draft")
        app.buttons["Record voice"].tap()
        XCTAssertTrue(app.buttons["Cancel recording"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Terminal keys"].exists)
        let recording = XCTAttachment(screenshot: app.screenshot())
        recording.name = "Expanded voice composer"; recording.lifetime = .keepAlways; add(recording)
        app.buttons["Cancel recording"].tap()
        XCTAssertTrue(input.waitForExistence(timeout: 5))
        XCTAssertEqual(input.value as? String, "Keep my draft")
        app.buttons["Record voice"].tap()
        app.buttons["Edit recording"].tap()
        let transcript = NSPredicate(format: "value == %@", "Keep my draft\nSample voice transcript")
        expectation(for: transcript, evaluatedWith: input)
        waitForExpectations(timeout: 5)
        XCTAssertTrue(app.buttons["Terminal keys"].exists)
    }

    func testTracksAndDraftSurviveViewSwitching() {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchEnvironment["CRABIGATOR_TEST_CONDUCTOR"] = "1"
        app.launchEnvironment["CRABIGATOR_TEST_CONDUCTOR_ROSTER"] = "1"
        app.launch()
        XCTAssertTrue(app.staticTexts["Check app store review"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Prepare the mobile release"].exists)
        let overview = XCTAttachment(screenshot: app.screenshot())
        overview.name = "iOS Conductor overview"; overview.lifetime = .keepAlways; add(overview)
        app.buttons["Sessions"].tap()
        XCTAssertTrue(app.staticTexts["Check app store review"].exists)
        XCTAssertTrue(app.staticTexts["0 active · 1 answered · 1 failed"].exists)
        app.buttons["Archive"].tap()
        let search = app.textFields["Search Conductor archive"]
        XCTAssertTrue(search.waitForExistence(timeout: 5))
        search.tap(); search.typeText("unavailable")
        XCTAssertTrue(app.staticTexts["Earlier store check"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["Prepare the mobile release"].exists)
        app.buttons["Conversation"].tap()
        let input = app.textFields["Conductor message"].exists ? app.textFields["Conductor message"] : app.textViews["Conductor message"]
        input.tap(); input.typeText("Keep this draft")
        let list = app.buttons.matching(identifier: "List").allElementsBoundByIndex.filter(\.isHittable)
        XCTAssertEqual(list.count, 1, "Only the visible view should accept navigation taps")
        list[0].tap()
        XCTAssertTrue(app.buttons["Session news-session"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Search"].exists)
        app.buttons["PR board"].tap()
        XCTAssertTrue(app.staticTexts["#42  Prepare the NewsBlur release"].waitForExistence(timeout: 5))
        XCTAssertTrue((app.buttons["PR board"].value as? String ?? "").hasPrefix("Selected"))
        XCTAssertGreaterThan(app.buttons["PR board"].frame.midY, app.frame.height * 0.8)
        XCTAssertFalse(app.staticTexts["Fixture session browser"].exists)
        let board = XCTAttachment(screenshot: app.screenshot())
        board.name = "PR board after leaving Conductor"; board.lifetime = .keepAlways; add(board)
        app.buttons["Conductor"].tap()
        XCTAssertTrue(app.staticTexts["Check app store review"].waitForExistence(timeout: 5))
        XCTAssertEqual(input.value as? String, "Keep this draft")
        app.buttons["Send to Conductor"].tap()
        let sent = XCTAttachment(screenshot: app.screenshot()); sent.name = "After Send"; sent.lifetime = .keepAlways; add(sent)
        XCTAssertTrue(app.staticTexts["Fixture received: Keep this draft"].waitForExistence(timeout: 5), app.debugDescription)
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = "iOS Conductor with two fixture tracks"
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
