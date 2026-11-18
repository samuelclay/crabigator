#if DEBUG
import SwiftUI

/// Exercises the real navigation and terminal layout without a desktop or saved credentials.
/// `CRABIGATOR_TEST_SAMPLE=1` loads a small roster with pull requests, git status and context.
/// `CRABIGATOR_TEST_MODE=prs` starts on the PR board, `CRABIGATOR_TEST_ROWS=git,changes` turns
/// row items on, and `CRABIGATOR_TEST_OPEN` opens `view`, `account`, `session` or `session-view`.
struct AdaptiveLayoutFixture: View {
    @ObservedObject var model: SessionModel
    var body: some View {
        RootView(model: model)
            .onAppear {
                let env = ProcessInfo.processInfo.environment, sample = env["CRABIGATOR_TEST_SAMPLE"] == "1" ? Self.sampleRoster() : nil
                model.close()
                model.paired = true
                model.navigation.conductor = false
                model.navigation.expanded = true
                if let mode = env["CRABIGATOR_TEST_MODE"] { model.navigation.listMode = mode != "prs" }
                model.preferences = Preferences()
                for key in (env["CRABIGATOR_TEST_ROWS"] ?? "").split(separator: ",").map(String.init) { for set in Preferences.Shown.allCases where set.items.contains(key) && !model.preferences.shows(key, in: set) { model.preferences.toggle(key, in: set) } }
                model.sessions = sample?.sessions ?? (env["CRABIGATOR_TEST_COMPUTERS"] == "1" ? Self.twoComputers() : ["Add dark mode", "Improve search"].enumerated().map { index, title in
                    Session(["session_id": "layout-\(index)", "title": title, "repo_owner": "example", "repo_name": "website", "cwd": "/sample/website", "device_name": "Demo computer", "state": "complete", "platform": "codex", "started_at": Int(Date().timeIntervalSince1970), "prompts_changed_at": Int(Date().timeIntervalSince1970), "active": true])
                })
                model.prs = sample?.prs ?? []
                model.rosterStatus = .ready
                model.ordering.update(model.sessions, prs: model.prs, prefs: model.preferences)
                if ["session", "session-view"].contains(env["CRABIGATOR_TEST_OPEN"] ?? ""), let first = model.sessions.first { model.select(first, order: model.sessions) }
            }
            .onChange(of: model.selectionID) { _, id in
                guard let id else { return }
                model.stream.screen = "Sample session · Layout audit\n\n› Check the adaptive terminal layout.\n\n" + (1...35).map { "  ✓ Sample check \($0) passed" }.joined(separator: "\n") + "\n\nReady for review."
                model.stream.historyLoaded = true
                model.stream.screenLoaded = true
                model.stream.connected = true
                model.stream.desktopStatusKnown = true
                if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SAMPLE"] == "1", let session = model.sessions.first(where: { $0.id == id }) { model.stream.details = Self.sampleDetails(session) }
            }
    }
    /// Seven sessions on two computers: working, waiting, finished and ended, with pull
    /// requests in each state, uncommitted work, and context from nearly empty to nearly full.
    private static func sampleRoster() -> (sessions: [Session], prs: [PullRequest]) {
        let now = Int(Date().timeIntervalSince1970)
        func session(_ id: String, _ title: String, _ machine: String, _ repo: String, _ branch: String, _ state: String, minutes: Int, mark: (String, String, String), recap: String,
                     uncommitted: Int = 0, additions: Int = 0, deletions: Int = 0, context: Int? = nil, active: Bool = true) -> JSON {
            var stats: JSON = ["work_seconds": minutes * 60 + 5400, "thinking_seconds": minutes * 14 + 1380, "tools": 40 + minutes, "compressions": minutes / 90, "prompts": 6 + minutes / 20, "completions": 5 + minutes / 20]
            if let context { stats["context_used"] = context; stats["context_limit"] = 1_000_000 }
            return ["session_id": id, "title": title, "device_name": machine, "repo_owner": "example", "repo_name": repo, "cwd": "/sample/" + repo, "branch": branch, "state": state, "platform": "claude", "active": active,
                    "session_mark": ["glyph": mark.0, "fg_hex": mark.1, "bg_hex": mark.2], "started_at": now - minutes * 60 - 5400, "ended_at": active ? 0 : now - minutes * 60 + 60,
                    "prompts_changed_at": now - minutes * 60, "completions_changed_at": now - minutes * 60 + 60, "uncommitted": uncommitted, "additions": additions, "deletions": deletions, "stats": stats,
                    "recap": ["headline": recap, "generated_at": (now - minutes * 60 + 60) * 1000]]
        }
        let rows: [JSON] = [
            session("sample-live", "Live updates for mobile", "Studio Mac", "mobile", "mobile-realtime", "thinking", minutes: 2, mark: ("◆", "#AF87FF", "#2A2440"), recap: "Stories now arrive live, with a pill for the new ones.", uncommitted: 3, additions: 212, deletions: 48, context: 186_000),
            session("sample-settings", "Redesign the settings screen", "Studio Mac", "app", "settings-redesign", "question", minutes: 25, mark: ("◈", "#FFAA77", "#352B25"), recap: "Mapped every setting and drew four ways to arrange them.", uncommitted: 12, additions: 1480, deletions: 960, context: 742_000),
            session("sample-sync", "Fix the flaky sync test", "Laptop", "mobile", "fix-sync-test", "complete", minutes: 80, mark: ("ᛝ", "#5FD7FF", "#17333D"), recap: "The test waited on a timer; it now waits on the sync itself.", context: 64_000),
            session("sample-dark", "Add dark mode", "Laptop", "website", "dark-mode", "permission", minutes: 190, mark: ("¬‿¬", "#AFFFD7", "#243B35"), recap: "Dark colours are in place; the charts still need a pass.", uncommitted: 1, additions: 18, deletions: 2, context: 884_000),
            session("sample-search", "Improve search", "Studio Mac", "website", "main", "complete", minutes: 310, mark: ("⌬⌬", "#FFD7AF", "#443524"), recap: "Search now matches titles first and ignores accents."),
            session("sample-deps", "Upgrade dependencies", "Laptop", "app", "upgrade-deps", "complete", minutes: 500, mark: ("✦", "#FFD75F", "#3B3420"), recap: "Every package is current and the tests pass.", context: 301_000),
            session("sample-notes", "Write the release notes", "Studio Mac", "app", "main", "complete", minutes: 640, mark: ("❖", "#FF87D7", "#3D2437"), recap: "Drafted notes for the next release.", active: false)]
        func pr(_ repo: String, _ number: Int, _ title: String, _ session: Int, _ values: JSON) -> JSON {
            ["owner": "example", "repo": repo, "number": number, "sessions": [rows[session]],
             "pr": values.merging(["title": title, "url": "https://github.com/example/\(repo)/pull/\(number)", "primary": true, "branch": rows[session].text("branch"), "updated_at": rows[session].int("prompts_changed_at") * 1000]) { _, new in new }]
        }
        let prs: [JSON] = [
            pr("mobile", 2217, "Live updates and a new stories pill", 0, ["state": "OPEN", "additions": 9036, "deletions": 117, "checks_total": 4, "checks_pending": 1, "mergeable": "MERGEABLE"]),
            pr("app", 311, "Redesign settings around the screen each control changes", 1, ["state": "OPEN", "is_draft": true, "additions": 1480, "deletions": 960, "checks_total": 3, "mergeable": "MERGEABLE", "unresolved_comments": 2]),
            pr("mobile", 2209, "Wait on the sync in the flaky test", 2, ["state": "MERGED", "additions": 24, "deletions": 31, "checks_total": 4]),
            pr("website", 87, "Add dark mode", 3, ["state": "OPEN", "additions": 640, "deletions": 212, "checks_total": 5, "checks_failed": 1, "mergeable": "CONFLICTING", "review_decision": "CHANGES_REQUESTED"]),
            pr("app", 305, "Upgrade dependencies", 5, ["state": "OPEN", "additions": 402, "deletions": 398, "checks_total": 3, "mergeable": "MERGEABLE", "review_decision": "APPROVED"])]
        return (rows.map(Session.init), prs.map(PullRequest.init))
    }
    /// What a desktop would stream for the selected sample session: stats, git and its drawer.
    private static func sampleDetails(_ session: Session) -> [String: JSON] {
        let now = Int(Date().timeIntervalSince1970)
        var stats: JSON = ["work_seconds": session.workSeconds ?? 0, "thinking_seconds": session.thinkingSeconds ?? 0, "prompts": 14, "completions": 13, "prompts_changed_at": session.promptAt, "completions_changed_at": session.completionAt, "tools": session.tools ?? 0, "compressions": session.compactions ?? 0]
        if let context = session.context { stats["context_used"] = context.used; stats["context_limit"] = context.limit }
        return ["stats": stats,
                "recap": ["latest": ["headline": session.recap, "generated_at": session.recapAt * 1000, "bullets": ["Sample data: nothing here came from a computer."]]],
                "git": ["branch": session.branch, "files": [["path": "Views/SettingsMenu.swift", "status": "M", "additions": session.additions, "deletions": session.deletions]]],
                "changes": ["by_language": [["changes": [["name": "BoardFilterBar", "kind": "struct", "additions": session.additions, "deletions": session.deletions]]]]],
                "commit_history": ["history": [["short_hash": "abc1234", "subject": "Add the filter bar", "timestamp": now - 7200]]]]
    }
    /// Forty sessions that alternate between two computers, newest first. Grouped by
    /// computer, the second computer starts below the screen; combined, the rows interleave.
    private static func twoComputers() -> [Session] {
        let now = Int(Date().timeIntervalSince1970)
        return (0..<40).map { index in
            let first = index % 2 == 0
            return Session(["session_id": "\(first ? "first" : "second")-\(index / 2)", "title": "Sample task \(index + 1)", "repo_owner": "example", "repo_name": "website", "cwd": "/sample/website",
                            "device_name": first ? "First computer" : "Second computer", "state": "complete", "platform": "codex", "active": true,
                            "started_at": now - 7200, "prompts_changed_at": now - index * 60, "completions_changed_at": now - index * 60,
                            "recap": ["headline": "Sample recap \(index + 1)", "generated_at": (now - index * 60) * 1000]])
        }
    }
}

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
        let session: JSON = ["session_id": "news-session", "title": "Review the NewsBlur release", "repo_owner": "fixture", "repo_name": "NewsBlur", "cwd": "/projects/NewsBlur", "device_name": "Fixture computer", "state": "thinking", "started_at": Int(Date().timeIntervalSince1970), "active": true,
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
