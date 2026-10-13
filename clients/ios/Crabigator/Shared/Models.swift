import Foundation

typealias JSON = [String: Any]
extension Dictionary where Key == String, Value == Any {
    func text(_ key: String, _ fallback: String = "") -> String { self[key] as? String ?? fallback }
    func int(_ key: String, _ fallback: Int = 0) -> Int { (self[key] as? NSNumber)?.intValue ?? fallback }
    func bool(_ key: String, _ fallback: Bool = false) -> Bool { self[key] as? Bool ?? fallback }
    func object(_ key: String) -> JSON { self[key] as? JSON ?? [:] }
    func array(_ key: String) -> [JSON] { self[key] as? [JSON] ?? [] }
}

struct Session: Identifiable, Equatable {
    var id, title, repo, machine, state, glyph, color, background, recap, platform, branch, cwd: String
    var active: Bool
    var startedAt, endedAt, lastSeenAt, promptAt, completionAt: Int
    var workSeconds, thinkingSeconds, tools, compactions: Int?
    var attention: Bool { active && ["question", "permission"].contains(state) }
    var activityAt: Int { max(promptAt, completionAt, startedAt) }
    init(_ j: JSON) {
        id = j.text("session_id", j.text("id"))
        title = j.text("title"); if title.isEmpty { title = j.text("dir_name", "Session") }
        repo = [j.text("repo_owner"), j.text("repo_name")].filter { !$0.isEmpty }.joined(separator: "/")
        machine = j.text("device_name"); if machine.isEmpty { machine = "Unknown machine" }
        state = j.text("state", "ready"); active = j.bool("active", true)
        let mark = j.object("session_mark")
        glyph = mark.text("glyph", "◈"); color = mark.text("fg_hex", "#FFAA77"); background = mark.text("bg_hex", "#352B25")
        recap = j.object("recap").text("headline"); platform = j.text("platform"); branch = j.text("branch"); cwd = j.text("cwd")
        startedAt = j.int("started_at"); endedAt = j.int("ended_at"); lastSeenAt = j.int("last_seen_at")
        promptAt = j.int("prompts_changed_at"); completionAt = j.int("completions_changed_at")
        let stats = j.object("stats")
        workSeconds = (stats["work_seconds"] as? Int).map { max(0, $0) }
        thinkingSeconds = (stats["thinking_seconds"] as? Int).map { max(0, $0) }
        tools = stats["tools"] as? Int; compactions = stats["compressions"] as? Int
    }
    func boardActivity(_ now: Int, prs: [PullRequest]) -> Int {
        if active && ["thinking", "question", "permission"].contains(state) { return now }
        return promptAt > 0 ? promptAt : (prs.map { max($0.updatedAt, $0.closedAt, $0.mentionedAt) }.max() ?? 0)
    }
    func duration(_ now: Int) -> Int? {
        if !active, let workSeconds { return workSeconds }
        let end = active ? now : (endedAt > 0 ? endedAt : lastSeenAt)
        return startedAt > 0 && end > 0 ? max(0, end - startedAt) : workSeconds
    }
    func matches(_ query: String) -> Bool { "\(title) \(repo) \(branch) \(machine)".localizedCaseInsensitiveContains(query) || query.isEmpty }
}

struct PullRequest: Identifiable, Equatable {
    var id, title, repo, state, checks, url, branch, mergeable, mergeState, review: String
    var number, checksFailed, checksPending, mentionedAt, updatedAt, closedAt: Int
    var additions, deletions: Int?
    var sessions: [Session]
    var draft, primary, watched, reviewDismissed: Bool
    var unresolvedComments: Int
    init(_ j: JSON) {
        let p = j["pr"] as? JSON ?? j
        repo = "\(j.text("owner"))/\(j.text("repo"))"; number = j.int("number"); id = "\(repo)#\(number)"
        title = p.text("title", "Pull request"); state = p.text("state", "OPEN"); url = p.text("url"); branch = p.text("branch")
        checksFailed = p.int("checks_failed"); checksPending = p.int("checks_pending")
        checks = checksFailed > 0 ? "✗\(checksFailed) CI" : checksPending > 0 ? "●\(checksPending) CI" : p.int("checks_total") > 0 ? "✓ CI" : ""
        mergeable = p.text("mergeable"); mergeState = p.text("merge_state_status"); review = p.text("review_decision")
        additions = p["additions"] as? Int; deletions = p["deletions"] as? Int
        unresolvedComments = max(0, p.int("unresolved_comments")); reviewDismissed = p.bool("review_dismissed")
        draft = p.bool("is_draft"); primary = p.bool("primary"); watched = p.bool("watched")
        mentionedAt = p.int("last_mentioned_at") / 1000; updatedAt = p.int("updated_at") / 1000; if updatedAt == 0 && watched { updatedAt = j.int("updated_at") }; closedAt = p.int("closed_at") / 1000
        var seen = Set<String>()
        sessions = (j.array("sessions") + j.array("touching")).map(Session.init).filter { seen.insert($0.id).inserted }
    }
    var attentionRank: Int {
        if state.isEmpty { return 2 }; if state == "MERGED" { return 6 }; if state != "OPEN" { return 7 }
        if mergeable == "CONFLICTING" || checksFailed > 0 { return 0 }; if review == "CHANGES_REQUESTED" { return 1 }
        if draft { return 2 }; if checksPending > 0 { return 3 }; return review != "APPROVED" ? 4 : 5
    }
    func matches(_ query: String) -> Bool { query.isEmpty || "\(title) \(repo) \(number)".localizedCaseInsensitiveContains(query) }
}

func durationText(_ seconds: Int?) -> String {
    guard let seconds else { return "—" }; let s = max(0, seconds)
    if s < 60 { return "\(s)s" }; if s < 3600 { return "\(s / 60)m" }
    if s < 86400 { return "\(s / 3600)h \(s % 3600 / 60)m" }; return "\(s / 86400)d \(s % 86400 / 3600)h"
}
func ageText(_ timestamp: Int, now: Int) -> String {
    guard timestamp > 0 else { return "—" }; let s = max(0, now - timestamp)
    if s < 60 { return "now" }; if s < 3600 { return "\(s / 60)m ago" }; if s < 86400 { return "\(s / 3600)h ago" }; return "\(s / 86400)d ago"
}

struct NavigationState: Codable, Equatable {
    var listMode = true
    var expanded = true
    var selectedID: String?
    static func load() -> Self {
        UserDefaults.standard.data(forKey: "navigation").flatMap { try? JSONDecoder().decode(Self.self, from: $0) } ?? Self()
    }
    func save() {
        if let data = try? JSONEncoder().encode(self) { UserDefaults.standard.set(data, forKey: "navigation") }
    }
}

struct Preferences: Codable, Equatable {
    // Optional storage lets older saved preferences keep their existing values.
    private var boardFontSize: Int?
    private var savedBoardAgeHours: Int?
    private var savedBoardComputer: String?
    private var savedBoardRecaps: Bool?
    private var savedBoardGrouping: String?
    var boardAgeHours: Int {
        get { savedBoardAgeHours ?? 24 }
        set { savedBoardAgeHours = newValue }
    }
    var boardComputer: String {
        get { savedBoardComputer ?? "" }
        set { savedBoardComputer = newValue }
    }
    var boardRecaps: Bool {
        get { savedBoardRecaps ?? false }
        set { savedBoardRecaps = newValue }
    }
    var boardGrouping: String {
        get { savedBoardGrouping ?? "sessions" }
        set { savedBoardGrouping = newValue }
    }
    var boardTextSize: Int {
        get { boardFontSize ?? 14 }
        set { boardFontSize = newValue }
    }
    var fontSize = 13, lineSpacing = 145, columns = 0, terminalHeight = 0
    var wrap = true, widgets = false, sidebarRight = false, showEnded = false
    var grouping = "all", order = "recent", density = "comfortable"
    var hidden: Set<String> = ["tools", "compactions"]
    func includesActivity(_ timestamp: Int, now: Int) -> Bool {
        guard boardAgeHours > 0 else { return true }
        let cutoff = boardAgeHours == 24 ? BoardAge.day : BoardAge.age(boardAgeHours * 3600 - 1)
        return BoardAge.age(max(0, now - timestamp)).rawValue <= cutoff.rawValue
    }
    func includesSession(_ session: Session, prs: [PullRequest], now: Int) -> Bool {
        (showEnded || session.active) && (boardComputer.isEmpty || boardComputer == session.machine)
            && includesActivity(session.boardActivity(now, prs: prs), now: now)
    }
    func visible(_ key: String) -> Bool { !hidden.contains(key) }
    mutating func toggle(_ key: String) { if !hidden.insert(key).inserted { hidden.remove(key) } }
    static func load() -> Self { UserDefaults.standard.data(forKey: "appearance").flatMap { try? JSONDecoder().decode(Self.self, from: $0) } ?? Self() }
    func save() { if let data = try? JSONEncoder().encode(self) { UserDefaults.standard.set(data, forKey: "appearance") } }
}

enum BoardActions {
    static func watchTarget(_ text: String) -> JSON? {
        let value = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let pattern = #"^(?:https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/pull/(\d+)(?:[/?#].*)?|([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)#(\d+))$"#
        guard let match = try? NSRegularExpression(pattern: pattern).firstMatch(in: value, range: NSRange(value.startIndex..., in: value)) else { return nil }
        let offset = match.range(at: 1).location == NSNotFound ? 4 : 1
        func part(_ index: Int) -> String { Range(match.range(at: index), in: value).map { String(value[$0]) } ?? "" }
        let owner = part(offset), repo = part(offset + 1)
        guard let number = Int(part(offset + 2)), number > 0 else { return nil }
        return ["owner": owner, "repo": repo, "number": number, "url": "https://github.com/\(owner)/\(repo)/pull/\(number)"]
    }
}
