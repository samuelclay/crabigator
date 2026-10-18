#if DEBUG
import SwiftUI

@MainActor private final class ConductorFixtureSource {
    private static let news: JSON = ["session_id": "news-session", "project": "NewsBlur", "session_title": "Check app store review", "platform": "Codex", "session_mark": ["glyph": "¬‿¬", "fg_hex": "#AFFFD7", "bg_hex": "#243B35"], "recap": "Checked both store fixtures", "additions": 12, "deletions": 3, "prompts": 4, "session_state": "complete"]
    private static let touch: JSON = ["session_id": "touch-session", "project": "Turn Touch", "session_title": "Prepare the mobile release", "platform": "Claude", "session_mark": ["glyph": "⌬⌬", "fg_hex": "#FFD7AF", "bg_hex": "#443524"]]
    private static func item(_ identity: JSON, _ values: JSON) -> JSON { identity.merging(values) { _, new in new } }
    var snapshot: JSON = [
        "host_online": true, "host_name": "Test desktop",
        "messages": [
            ["id": "question", "role": "user", "text": "Where are the mobile releases?", "at": 1792288800000],
            ["id": "answer", "role": "assistant", "text": "Both apps are waiting on **Apple review**. Android is live. These are fixture results.", "at": 1792288860000,
             "highlights": [item(news, ["text": "iOS **2.0 · In review**\nAndroid **2.1 · Published**\nReview duration: unknown.", "state": "answered", "updated_at": 1792288840000]), item(touch, ["text": "iOS **2.0 · In review**\nAndroid **2.1 · Published**\nReview duration: unknown.", "state": "answered", "updated_at": 1792288850000])]]
        ],
        "tracks": [
            item(news, ["id": "one", "request_id": "question", "title": "App store versions", "state": "answered", "connected": true, "question": "Where are the mobile releases?", "events": [], "result": "Fixture: iOS 2.0 in review. Android 2.1 is live."]),
            item(news, ["id": "retry", "request_id": "old", "title": "Earlier store check", "state": "failed", "connected": true, "events": [], "result": "Desktop was unavailable."]),
            item(touch, ["id": "two", "request_id": "question", "title": "App store versions", "state": "answered", "connected": false, "events": [], "result": "Fixture: Android 2.1 is live."])
        ]
    ]
    func request(_ path: String, _ body: JSON?) async throws -> JSON {
        if path.hasSuffix("/archive") {
            let query = body?.text("query") ?? "", outcome = body?.text("status", "all") ?? "all"
            let entries = snapshot.array("tracks").filter { (outcome == "all" || $0.text("state") == outcome) && (query.isEmpty || $0.text("result").localizedCaseInsensitiveContains(query)) }.map { entry in Self.item(entry, ["kind": "track", "excerpt": entry.text("result")]) }
            return ["entries": entries, "total": entries.count]
        }
        if let body {
            var messages = snapshot.array("messages")
            messages.append(["id": body.text("id"), "role": "user", "text": body.text("text")])
            messages.append(["id": "reply-" + body.text("id"), "role": "assistant", "text": "Fixture received: " + body.text("text")])
            snapshot["messages"] = messages
            return [:]
        }
        return snapshot
    }
}

struct ConductorFixture: View {
    @ObservedObject var model: SessionModel
    @StateObject private var conductor: ConductorModel
    init(model: SessionModel) {
        self.model = model
        let source = ConductorFixtureSource()
        _conductor = StateObject(wrappedValue: ConductorModel(request: source.request))
    }
    var body: some View {
        // Only the coordinator uses sample data. List, PR board and sessions use the real app.
        RootView(model: model, conductor: conductor).onAppear {
            if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_CONDUCTOR_ROSTER"] == "1" { seedTestRoster() }
            model.navigation.conductor = true
        }
    }
    private func seedTestRoster() {
        let session: JSON = ["session_id": "news-session", "title": "Review the NewsBlur release", "repo_owner": "fixture", "repo_name": "NewsBlur", "device_name": "Fixture computer", "state": "thinking", "started_at": Int(Date().timeIntervalSince1970), "active": true,
                             "session_mark": ["glyph": "¬‿¬", "fg_hex": "#AFFFD7", "bg_hex": "#243B35"]]
        model.paired = true; model.close()
        model.sessions = [Session(session)]
        model.prs = [PullRequest(["owner": "fixture", "repo": "NewsBlur", "number": 42, "pr": ["title": "Prepare the NewsBlur release", "state": "OPEN", "primary": true], "sessions": [session]])]
        model.rosterStatus = .ready
        model.ordering.update(model.sessions, prs: model.prs, prefs: model.preferences)
    }
}

struct VoiceComposerFixture: View {
    @StateObject private var model = SessionModel()
    @StateObject private var stream = SessionStream()
    var body: some View {
        VStack {
            Text("Voice transition preview").font(.headline)
            Text("Sample audio · nothing is sent").font(.caption).foregroundStyle(Crab.muted)
            Spacer()
            SessionComposer(stream: stream, model: model, sessionID: "voice-preview", active: true)
        }.padding(.top, 24).background(Crab.ink).onAppear {
            model.selectionID = "voice-preview"; stream.connected = true
        }
    }
}
#endif
