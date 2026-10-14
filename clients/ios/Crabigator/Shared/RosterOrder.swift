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
    var pullRequests: [BoardPRGroup] = []
    var id: String { "\(machine.count):\(machine)|\(age?.rawValue ?? -1)|\(project)" }
}

struct BoardPRGroup: Identifiable {
    var pr: PullRequest
    var sessions: [Session]
    var id: String { pr.id }
}

func groupBoardByPullRequest(_ sections: [RosterSection], prsBySession: [String: [PullRequest]], now: Int,
                            includesActivity: (Int) -> Bool = { _ in true }) -> [RosterSection] {
    var machines: [String] = []
    for section in sections where !machines.contains(section.machine) { machines.append(section.machine) }
    return machines.flatMap { machine -> [RosterSection] in
        let source = sections.filter { $0.machine == machine }
        var groups: [BoardPRGroup] = []
        var represented = Set<String>()
        for session in source.flatMap(\.sessions) {
            for pr in prsBySession[session.id] ?? [] where (pr.primary || pr.watched) && pr.visibleOnBoard(for: session.id) {
                represented.insert(session.id)
                if let index = groups.firstIndex(where: { $0.id == pr.id }) { groups[index].sessions.append(session) }
                else { groups.append(BoardPRGroup(pr: pr, sessions: [session])) }
            }
        }
        var result = source.map { section -> RosterSection in
            var copy = section
            copy.sessions.removeAll { represented.contains($0.id) || !includesActivity($0.boardActivity(now, prs: prsBySession[$0.id] ?? [])) }
            return copy
        }
        for group in groups {
            let prompted = group.sessions.map { session in
                session.active && ["thinking", "question", "permission"].contains(session.state) ? now : session.promptAt
            }.max() ?? 0
            let activity = prompted > 0 ? prompted : max(group.pr.updatedAt, group.pr.closedAt, group.pr.mentionedAt)
            guard includesActivity(activity) else { continue }
            let age = BoardAge.age(max(0, now - activity))
            if let index = result.firstIndex(where: { $0.age == age && $0.project == group.pr.repo }) {
                result[index].pullRequests.append(group)
            } else {
                result.append(RosterSection(machine: machine, project: group.pr.repo, age: age, sessions: [], pullRequests: [group]))
            }
        }
        return BoardAge.allCases.flatMap { age in result.filter { $0.age == age && (!$0.sessions.isEmpty || !$0.pullRequests.isEmpty) } }
    }
}

/// Positions change every ten minutes or when a genuinely new session arrives.
/// Time-band membership stays live, so new activity moves into the correct band immediately.
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
            s.boardActivity(now, prs: bySession[s.id] ?? [])
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
        board = reset ? nextBoard : retainSections(board, nextBoard, liveAges: true)
        let watches = prs.filter { $0.sessions.isEmpty }
        watched = reset ? watches : retainRows(watched, watches)
        if reset { refreshedAt = elapsed; preferences = pref }
    }
    private func retainSections(_ old: [RosterSection], _ current: [RosterSection], liveAges: Bool = false) -> [RosterSection] {
        let ages = Dictionary(current.flatMap { section in section.sessions.map { ($0.id, section.age) } }, uniquingKeysWith: { a, _ in a })
        var remaining = Dictionary(current.flatMap(\.sessions).map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        var result = old.map { section -> RosterSection in
            var copy = section
            copy.sessions = section.sessions.compactMap { session in
                if liveAges && (ages[session.id] ?? nil) != section.age { return nil }
                return remaining.removeValue(forKey: session.id)
            }
            return copy
        }
        for section in current {
            let added = section.sessions.compactMap { remaining.removeValue(forKey: $0.id) }
            if let index = result.firstIndex(where: { $0.id == section.id }) { result[index].sessions += added }
            else { var copy = section; copy.sessions = added; result.append(copy) }
        }
        return machines.flatMap { machine in
            let sections = result.filter { $0.machine == machine && !$0.sessions.isEmpty }
            return liveAges ? BoardAge.allCases.flatMap { age in sections.filter { $0.age == age } } : sections
        }
    }
    private func retainRows<T: Identifiable>(_ old: [T], _ current: [T]) -> [T] where T.ID: Hashable {
        var remaining = Dictionary(current.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        return old.compactMap { remaining.removeValue(forKey: $0.id) } + current.compactMap { remaining.removeValue(forKey: $0.id) }
    }
}
