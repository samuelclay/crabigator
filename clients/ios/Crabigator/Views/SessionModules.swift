import SwiftUI

struct ModuleChange {
    var name: String, kind: String
    var additions: Int, deletions: Int
    init(_ json: JSON, nameKey: String, kindKey: String) {
        name = json.text(nameKey); kind = json.text(kindKey)
        additions = json.int("additions"); deletions = json.int("deletions")
    }
}

struct SessionModuleData {
    var recap: String, bullets: [String], recapAt: Int, history: [JSON]
    var branch: String, files: [ModuleChange], changes: [ModuleChange], commits: [JSON]
    var stats: JSON
    init(session: Session, details: [String: JSON]) {
        history = details["recap_history"]?.array("history") ?? []
        let latest = details["recap"]?["latest"] as? JSON ?? history.last ?? [:]
        recap = latest.text("headline", session.recap)
        bullets = latest["bullets"] as? [String] ?? []
        recapAt = latest.int("generated_at")
        let git = details["git"] ?? [:]
        branch = git.text("branch", session.branch)
        files = git.array("files").map { ModuleChange($0, nameKey: "path", kindKey: "status") }
        changes = (details["changes"]?.array("by_language") ?? []).flatMap { $0.array("changes") }.map { ModuleChange($0, nameKey: "name", kindKey: "kind") }
        commits = details["commit_history"].map { Array($0.array("history").reversed()) } ?? git.array("recent_commits")
        stats = details["stats"] ?? [:]
    }
}

struct ModuleContentHeight: PreferenceKey {
    static var defaultValue: CGFloat = 0
    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) { value = max(value, nextValue()) }
}

/// A compact footer. Only the opened sections consume more than a single row.
struct SessionModules: View {
    var session: Session, data: SessionModuleData, prefs: Preferences, prs: [PullRequest]
    var revealed = true
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var expanded: Set<String> = []
    private let line = Color.white.opacity(0.07)
    var body: some View {
        VStack(spacing: 0) {
        ScrollView {
            VStack(spacing: 0) {
                if prefs.visible("recap") && !data.recap.isEmpty { recap }
                if prefs.visible("prs") && !prs.isEmpty { pullRequests }
                if prefs.visible("git") && (!data.branch.isEmpty || !data.files.isEmpty) {
                    section("git", title: "Git", summary: data.branch, suffix: data.files.isEmpty ? "Clean" : "\(data.files.count) file\(data.files.count == 1 ? "" : "s")", color: Crab.mint) {
                        if data.files.isEmpty { detailText("Working tree clean") }
                        else { changeRows(data.files) }
                    }
                }
                if prefs.visible("changes") && !data.changes.isEmpty {
                    section("changes", title: "Changes", summary: "\(data.changes.count) symbol\(data.changes.count == 1 ? "" : "s")", color: Crab.orange) { changeRows(data.changes) }
                }
                if prefs.visible("commits") && !data.commits.isEmpty {
                    section("commits", title: "Commits", summary: data.commits.first?.text("subject") ?? "", suffix: "\(data.commits.count)", color: Crab.muted) {
                        VStack(alignment: .leading, spacing: 8) {
                            ForEach(Array(data.commits.enumerated()), id: \.offset) { _, commit in
                                HStack(alignment: .firstTextBaseline, spacing: 8) {
                                    Text(commit.text("short_hash")).foregroundStyle(Crab.orange)
                                    Text(commit.text("subject")).foregroundStyle(.secondary)
                                }.font(.system(size: 11, design: .monospaced))
                            }
                        }.padding(.horizontal, 12).padding(.bottom, 10)
                    }
                }
            }.fixedSize(horizontal: false, vertical: true)
                .background(GeometryReader { proxy in Color.clear.preference(key: ModuleContentHeight.self, value: proxy.size.height) })
        }
        }.background(Crab.panel).accessibilityIdentifier("Session modules")
    }
    private var recap: some View {
        section("recap", title: "Recap", summary: data.recap, color: Crab.cyan) {
            VStack(alignment: .leading, spacing: 8) {
                Text(data.recap).foregroundStyle(.primary).font(.system(size: 13))
                ForEach(Array(data.bullets.enumerated()), id: \.offset) { _, bullet in
                    HStack(alignment: .top, spacing: 8) { Text("·").foregroundStyle(Crab.cyan); Text(bullet).foregroundStyle(.secondary) }.font(.system(size: 12))
                }
                if data.recapAt > 0 { Text(ageText(data.recapAt, now: Int(Date().timeIntervalSince1970))).font(.system(size: 10)).foregroundStyle(Crab.muted) }
                if data.history.count > 1 {
                    ForEach(Array(data.history.dropLast().reversed().enumerated()), id: \.offset) { _, previous in
                        Divider().overlay(line)
                        Text(previous.text("headline")).font(.system(size: 12)).foregroundStyle(.secondary)
                    }
                }
            }.textSelection(.enabled).padding(.horizontal, 12).padding(.bottom, 12)
        }
    }
    private var pullRequests: some View {
        section("prs", title: "Pull requests", summary: "\(prs.count) PR\(prs.count == 1 ? "" : "s")", suffix: prSummary, color: Crab.purple) {
            VStack(spacing: 6) {
                ForEach(prs) { pr in
                    if let url = URL(string: pr.url), url.scheme == "https" {
                        Link(destination: url) { ModulePRRow(pr: pr) }.buttonStyle(.plain)
                            .accessibilityLabel("Open \(pr.repo)#\(pr.number) on GitHub: \(pr.title), \(pr.state.lowercased())")
                    } else { ModulePRRow(pr: pr) }
                }
            }.padding(.horizontal, 10).padding(.bottom, 10)
        }
    }
    private var prSummary: String {
        let open = prs.filter { $0.state == "OPEN" }.count
        let merged = prs.filter { $0.state == "MERGED" }.count
        return [open > 0 ? "\(open) open" : "", merged > 0 ? "\(merged) merged" : ""].filter { !$0.isEmpty }.joined(separator: " · ")
    }
    private func section<Content: View>(_ key: String, title: String, summary: String, suffix: String = "", color: Color, @ViewBuilder content: () -> Content) -> some View {
        VStack(spacing: 0) {
            Button {
                withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.25)) {
                    if expanded.contains(key) { expanded.remove(key) } else { expanded.insert(key) }
                }
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "chevron.right").font(.system(size: 9, weight: .semibold)).frame(width: 10)
                        .rotationEffect(.degrees(expanded.contains(key) ? 90 : 0))
                    Text(title).font(.system(size: 11, weight: .semibold)).fixedSize()
                    Text(summary).font(.system(size: 12)).foregroundStyle(Crab.muted).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                    if !suffix.isEmpty { Text(suffix).font(.system(size: 10)).foregroundStyle(Crab.muted).lineLimit(1).fixedSize() }
                }.foregroundStyle(color).padding(.horizontal, 10).frame(minHeight: 44).contentShape(Rectangle())
            }.buttonStyle(.plain).accessibilityLabel(title).accessibilityValue(expanded.contains(key) ? "Expanded" : "Collapsed").accessibilityIdentifier("module-" + key)
            if expanded.contains(key) { content().transition(.opacity) }
            Rectangle().fill(line).frame(height: 0.5)
        }.clipped()
            .opacity(revealed ? 1 : 0)
            .scaleEffect(revealed ? 1 : 0.96, anchor: .bottom)
            .offset(y: revealed ? 0 : 14)
            .animation(reduceMotion ? nil : revealed
                ? .spring(response: 0.36, dampingFraction: 0.76).delay(Double(4 - moduleIndex(key)) * 0.035)
                : .easeIn(duration: 0.14).delay(Double(moduleIndex(key)) * 0.016), value: revealed)
    }
    private func moduleIndex(_ key: String) -> Int {
        ["recap", "prs", "git", "changes", "commits"].firstIndex(of: key) ?? 0
    }
    private func detailText(_ text: String) -> some View { Text(text).font(.system(size: 12)).foregroundStyle(Crab.muted).frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 12).padding(.bottom, 10) }
    private func changeRows(_ rows: [ModuleChange]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(row.kind).font(.system(size: 10, design: .monospaced)).foregroundStyle(row.kind == "D" ? Crab.red : Crab.orange)
                    Text(row.name).font(.system(size: 11, design: .monospaced)).foregroundStyle(.secondary).frame(maxWidth: .infinity, alignment: .leading)
                    ModuleDiff(additions: row.additions, deletions: row.deletions)
                }
            }
        }.textSelection(.enabled).padding(.horizontal, 12).padding(.bottom, 10)
    }
}

/// The permanent handle for the drawer. Metrics can still scroll horizontally;
/// a tap anywhere on the bar opens or closes the modules.
struct SessionSummaryBar: View {
    var session: Session, stats: JSON, prefs: Preferences, expanded: Bool
    var toggle: () -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    var body: some View {
        HStack(spacing: 0) {
            metricStrip
            Image(systemName: "chevron.up")
                .font(.system(size: 12, weight: .semibold)).foregroundStyle(Crab.orange)
                .rotationEffect(.degrees(expanded ? 180 : 0))
                .frame(width: 44, height: 44)
                .background(Crab.orange.opacity(expanded ? 0.08 : 0.03))
                .animation(reduceMotion ? nil : .spring(response: 0.32, dampingFraction: 0.75), value: expanded)
                .accessibilityHidden(true)
        }.frame(height: 44).background(Crab.panel)
            .overlay(alignment: .top) { Color.white.opacity(0.09).frame(height: 0.5) }
            .contentShape(Rectangle()).onTapGesture(perform: toggle)
            .accessibilityElement(children: .combine).accessibilityAddTraits(.isButton)
            .accessibilityLabel("Session summary. " + SessionStatus(session: session).label + ". " + metrics.map { $0.label + ": " + $0.value }.joined(separator: ", "))
            .accessibilityValue(expanded ? "Expanded" : "Collapsed")
            .accessibilityHint(expanded ? "Hide session modules" : "Show session modules")
            .accessibilityAction { toggle() }.accessibilityIdentifier("session-summary")
    }
    private var metrics: [(key: String, symbol: String, value: String, label: String, color: Color)] {
        [
            ("sessionTime", "◉", durationText(stats["work_seconds"] as? Int ?? session.workSeconds), "Session time", Crab.blue),
            ("thinkingTime", "◐", durationText(stats["thinking_seconds"] as? Int ?? session.thinkingSeconds), "Thinking time", Crab.mint),
            ("prompts", "⟩", stats["prompts"].map { String(describing: $0) } ?? "—", "Prompts", Crab.muted),
            ("completions", "⋖", stats["completions"].map { String(describing: $0) } ?? "—", "Completions", Crab.muted),
            ("tools", "⚒", stats["tools"].map { String(describing: $0) } ?? session.tools.map(String.init) ?? "—", "Tools", Crab.orange),
            ("compactions", "⊜", stats["compressions"].map { String(describing: $0) } ?? session.compactions.map(String.init) ?? "—", "Compactions", Crab.purple)
        ].filter { prefs.visible($0.0) }
    }
    private var metricStrip: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 14) {
                SessionStatus(session: session)
                ForEach(metrics, id: \.key) { metric in
                    stat(metric.symbol, value: metric.value, label: metric.label, color: metric.color)
                }
            }.padding(.horizontal, 12).frame(height: 44)
        }.frame(height: 44)
    }
    private func stat(_ symbol: String, value: String, label: String, color: Color) -> some View {
        HStack(spacing: 4) { Text(symbol).foregroundStyle(color); Text(value).foregroundStyle(.secondary) }.font(.system(size: 11, design: .monospaced)).fixedSize().accessibilityElement(children: .ignore).accessibilityLabel(label + ": " + value)
    }
}

private struct ModuleDiff: View {
    var additions: Int?, deletions: Int?
    var body: some View {
        HStack(spacing: 5) {
            if let additions { Text("+\(additions)").foregroundStyle(Crab.mint) }
            if let deletions { Text("−\(deletions)").foregroundStyle(Crab.red) }
        }.font(.system(size: 10, design: .monospaced)).fixedSize()
    }
}
private struct ModulePRRow: View {
    var pr: PullRequest
    var stateColor: Color { pr.state == "MERGED" ? Crab.purple : pr.state == "CLOSED" ? Crab.red : pr.draft ? Crab.muted : Crab.mint }
    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(spacing: 8) {
                Text((pr.primary ? "★ " : "") + (pr.repo.split(separator: "/").last.map(String.init) ?? pr.repo) + " #\(pr.number)").font(.system(size: 12, weight: .semibold)).foregroundStyle(Crab.purple).lineLimit(1)
                Spacer(minLength: 4)
                Text(pr.draft ? "draft" : pr.state.lowercased()).font(.system(size: 10, weight: .medium)).foregroundStyle(stateColor).padding(.horizontal, 5).padding(.vertical, 2).background(stateColor.opacity(0.08), in: RoundedRectangle(cornerRadius: 3)).overlay(RoundedRectangle(cornerRadius: 3).stroke(stateColor.opacity(0.5), lineWidth: 0.5))
            }
            Text(pr.title).font(.system(size: 12)).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 8) {
                if !pr.branch.isEmpty { Text("⑂ " + pr.branch).lineLimit(1).truncationMode(.middle).frame(maxWidth: .infinity, alignment: .leading) }
                ModuleDiff(additions: pr.additions, deletions: pr.deletions)
                if !pr.checks.isEmpty { Text(pr.checks).foregroundStyle(pr.checksFailed > 0 ? Crab.red : pr.checksPending > 0 ? Crab.yellow : Crab.mint).fixedSize() }
            }.font(.system(size: 10)).foregroundStyle(Crab.muted)
        }.padding(.horizontal, 10).padding(.vertical, 9).background(Color.white.opacity(0.025), in: RoundedRectangle(cornerRadius: 5))
            .overlay(alignment: .leading) { RoundedRectangle(cornerRadius: 1).fill(Crab.purple.opacity(0.6)).frame(width: 2) }
            .overlay(RoundedRectangle(cornerRadius: 5).stroke(Color.white.opacity(0.08), lineWidth: 0.5))
    }
}

#if DEBUG
/// Local, read-only content for checking phone and tablet module layouts.
struct SessionModulesFixture: View {
    @StateObject private var model: SessionModel = {
        let model = SessionModel(); model.preferences = Preferences(); return model
    }()
    @StateObject private var stream: SessionStream = {
        let stream = SessionStream()
        stream.selected = Session(["id": "module-layout", "title": "Internationalize Turn Touch", "device_name": "Studio", "state": "complete", "branch": "codex/localization", "active": true])
        stream.screenLoaded = true; stream.historyLoaded = true; stream.connected = true
        stream.screen = "Added language selection on all three platforms.\n\n  ✓ iOS build passed\n  ✓ Android tests passed\n  ✓ Mac build passed\n\nThe pull requests are ready for review.\n\n› Ask Codex to do anything"
        stream.details = [
            "stats": ["work_seconds": 17160, "thinking_seconds": 9120, "prompts": 6, "completions": 4, "tools": 24],
            "recap": ["latest": ["headline": "Added localization and language selection", "bullets": ["Translated the app into 21 languages.", "Added native language selection on each platform."]]],
            "git": ["branch": "codex/localization", "files": [["path": "Settings/LanguagePicker.swift", "status": "M", "additions": 42, "deletions": 8]]],
            "changes": ["by_language": [["changes": [["name": "LanguagePicker", "kind": "struct", "additions": 42, "deletions": 8]]]]],
            "commit_history": ["history": [["short_hash": "abc1234", "subject": "Add language selection"]]],
            "prs": ["prs": [
                ["owner": "samuelclay", "repo": "turntouch-ios", "number": 15, "title": "Add 21-language localization and iOS language selection", "url": "https://github.com/samuelclay/turntouch-ios/pull/15", "state": "OPEN", "primary": true, "branch": "codex/localization-21-languages", "additions": 41683, "deletions": 738, "checks_total": 3, "mergeable": "MERGEABLE"],
                ["owner": "samuelclay", "repo": "turntouch-android", "number": 1, "title": "Add 21-language localization and Android language selection", "url": "https://github.com/samuelclay/turntouch-android/pull/1", "state": "MERGED", "primary": true, "branch": "codex/localization-21-languages", "additions": 20920, "deletions": 299]
            ]]
        ]
        return stream
    }()
    var body: some View { SessionDetail(model: model, stream: stream, wide: false, active: false, close: {}) }
}
#endif
