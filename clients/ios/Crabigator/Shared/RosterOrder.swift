import Foundation

enum BoardAge: Int, CaseIterable {
    case hour, three, six, nine, twelve, day, older
    var label: String { ["Last hour", "1–3 hours", "3–6 hours", "6–9 hours", "9–12 hours", "12–24 hours", "Older"][rawValue] }
    var palette: Int { [51, 45, 39, 33, 27, 25, 245][rawValue] }
    static func age(_ seconds: Int) -> Self {
        for (i, limit) in [3600, 10800, 21600, 32400, 43200, 86401].enumerated() where seconds < limit { return Self(rawValue: i)! }
        return .older
    }
}
struct RosterSection: Identifiable {
    var machine: String
    var project: String
    var age: BoardAge?
    var sessions: [Session]
    var id: String { "\(machine.count):\(machine)|\(age?.rawValue ?? -1)|\(project)" }
}

/// Positions change every ten minutes or when a genuinely new session arrives.
/// Filtering never changes the saved order. Machines retain their place even after disappearing.
final class RosterOrder {
    private(set) var machines: [String] = []
    private var seen = Set<String>()
    private var refreshedAt: TimeInterval?
    private var preferences = ""
    private(set) var list: [RosterSection] = []
    private(set) var board: [RosterSection] = []
    private(set) var watched: [PullRequest] = []
    private(set) var prsBySession: [String: [PullRequest]] = [:]

    func update(_ roster: [Session], prs: [PullRequest], prefs: Preferences, now: Int = Int(Date().timeIntervalSince1970), elapsed: TimeInterval = ProcessInfo.processInfo.systemUptime) {
        let grouped = Dictionary(grouping: roster, by: \.machine)
        let ranked = grouped.keys.sorted { a, b in
            let aa = grouped[a]!.map(\.activityAt).max() ?? 0, bb = grouped[b]!.map(\.activityAt).max() ?? 0
            return aa == bb ? a < b : aa > bb
        }
        machines += ranked.filter { !machines.contains($0) }
        let ids = Set(roster.map(\.id)), pref = prefs.grouping + prefs.order
        let reset = refreshedAt == nil || elapsed - (refreshedAt ?? 0) >= 600 || pref != preferences || !ids.subtracting(seen).isEmpty
        seen.formUnion(ids)
        let sorted = roster.sorted { a, b in
            if a.attention != b.attention { return a.attention }; return a.activityAt == b.activityAt ? a.id < b.id : a.activityAt > b.activityAt
        }
        var nextList: [RosterSection] = []
        for machine in machines {
            let rows = sorted.filter { $0.machine == machine }
            if prefs.grouping != "project" {
                if !rows.isEmpty { nextList.append(RosterSection(machine: machine, project: "", sessions: rows)) }
            } else {
                let projects = Dictionary(grouping: rows, by: { $0.repo.isEmpty ? "Sessions" : $0.repo })
                let keys = projects.keys.sorted { a, b in
                    prefs.order == "alpha" ? a.localizedCaseInsensitiveCompare(b) == .orderedAscending : (projects[a]!.map(\.activityAt).max() ?? 0) > (projects[b]!.map(\.activityAt).max() ?? 0)
                }
                for project in keys { nextList.append(RosterSection(machine: machine, project: project, sessions: projects[project]!)) }
            }
        }
        var bySession: [String: [PullRequest]] = [:]
        for pr in prs { for session in pr.sessions { bySession[session.id, default: []].append(pr) } }
        for id in bySession.keys {
            bySession[id]!.sort { a, b in
                if a.attentionRank != b.attentionRank { return a.attentionRank < b.attentionRank }
                if a.primary != b.primary { return a.primary }; return a.mentionedAt > b.mentionedAt
            }
            if !reset { bySession[id] = retainRows(prsBySession[id] ?? [], bySession[id]!) }
        }
        prsBySession = bySession
        func activity(_ s: Session) -> Int {
            if s.active && ["thinking", "question", "permission"].contains(s.state) { return now }
            return s.promptAt > 0 ? s.promptAt : (bySession[s.id]?.map { max($0.updatedAt, $0.closedAt, $0.mentionedAt) }.max() ?? 0)
        }
        var nextBoard: [RosterSection] = []
        for machine in machines {
            let rows = roster.filter { $0.machine == machine }.sorted { activity($0) > activity($1) }
            for age in BoardAge.allCases {
                let band = rows.filter { BoardAge.age(max(0, now - activity($0))) == age }
                var projects: [String] = []
                for s in band {
                    let repo = bySession[s.id]?.first?.repo ?? (s.repo.isEmpty ? "Sessions" : s.repo)
                    if !projects.contains(repo) { projects.append(repo) }
                }
                for project in projects {
                    nextBoard.append(RosterSection(machine: machine, project: project, age: age, sessions: band.filter {
                        (bySession[$0.id]?.first?.repo ?? ($0.repo.isEmpty ? "Sessions" : $0.repo)) == project
                    }))
                }
            }
        }
        list = reset ? nextList : retainSections(list, nextList)
        board = reset ? nextBoard : retainSections(board, nextBoard)
        let watches = prs.filter { $0.sessions.isEmpty }
        watched = reset ? watches : retainRows(watched, watches)
        if reset { refreshedAt = elapsed; preferences = pref }
    }
    private func retainSections(_ old: [RosterSection], _ current: [RosterSection]) -> [RosterSection] {
        var remaining = Dictionary(current.flatMap(\.sessions).map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        var result = old.map { section -> RosterSection in
            var copy = section; copy.sessions = section.sessions.compactMap { remaining.removeValue(forKey: $0.id) }; return copy
        }
        for section in current {
            let added = section.sessions.compactMap { remaining.removeValue(forKey: $0.id) }
            if let index = result.firstIndex(where: { $0.id == section.id }) { result[index].sessions += added }
            else { var copy = section; copy.sessions = added; result.append(copy) }
        }
        return machines.flatMap { machine in result.filter { $0.machine == machine && !$0.sessions.isEmpty } }
    }
    private func retainRows<T: Identifiable>(_ old: [T], _ current: [T]) -> [T] where T.ID: Hashable {
        var remaining = Dictionary(current.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        return old.compactMap { remaining.removeValue(forKey: $0.id) } + current.compactMap { remaining.removeValue(forKey: $0.id) }
    }
}
