import XCTest

final class ScrollPerformanceTests: XCTestCase {
    func testScrollThroughLiveHistory() throws {
        guard ProcessInfo.processInfo.environment["CRABIGATOR_PROFILE_SCROLL"] == "1" else {
            throw XCTSkip("Run scripts/profile-scroll.sh on a paired device with terminal history.")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "Session "))
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 25))
        let sessionID = ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SESSION_ID"]
        let matches = sessionID.map { app.buttons.matching(identifier: "Session " + $0) } ?? rows
        try XCTUnwrap(matches.allElementsBoundByIndex.first { $0.isHittable }).tap()
        let terminal = app.textViews["terminal-output"]
        XCTAssertTrue(terminal.waitForExistence(timeout: 20))
        let history = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in (terminal.value as? String ?? "").count > 4000 }, object: terminal)
        XCTAssertEqual(XCTWaiter.wait(for: [history], timeout: 25), .completed, "Choose a session with enough loaded history to scroll")
        let options = XCTMeasureOptions(); options.iterationCount = 3
        measure(metrics: [XCTClockMetric(), XCTCPUMetric(application: app), XCTMemoryMetric(application: app), XCTOSSignpostMetric.scrollDecelerationMetric], options: options) {
            for _ in 0..<4 { terminal.swipeDown(velocity: .slow) }
            for _ in 0..<4 { terminal.swipeUp(velocity: .slow) }
        }
        let screenshot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        screenshot.name = "After complete scroll sequence"; screenshot.lifetime = .keepAlways; add(screenshot)
    }
}
