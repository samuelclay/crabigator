import SwiftUI

struct SessionDetail: View {
    @ObservedObject var model: SessionModel
    @ObservedObject var stream: SessionStream
    var wide: Bool, active: Bool
    var close: () -> Void
    @State private var pinned = true
    @State private var style = false
    @State private var drag: CGFloat = 0
    @State private var loadingSlowly = false
    var prs: [PullRequest] {
        if let live = stream.details["prs"] { return live.array("prs").filter { !$0.bool("dismissed") }.map(PullRequest.init) }
        return model.prs.filter { $0.sessions.contains { $0.id == stream.selected?.id } }
    }
    var widgetSections: [SessionWidgetSection] {
        sessionWidgetSections(session: stream.selected, details: stream.details, prefs: model.preferences, prs: prs)
    }
    var showsWidgets: Bool { model.preferences.widgets && !widgetSections.isEmpty }
    var body: some View {
        GeometryReader { geometry in
            if let session = stream.selected {
                VStack(spacing: 0) {
                    HStack(spacing: 2) {
                        IconButton(icon: wide ? "xmark" : "chevron.left", label: wide ? "Close session" : "Back to list", action: close)
                        VStack(alignment: .leading, spacing: 3) {
                            Text(session.title).font(.system(size: 15, weight: .medium)).foregroundStyle(Crab.title).lineLimit(2).accessibilityIdentifier("Selected session \(session.id)")
                            Text(stream.connected ? "\(session.machine) · Live" : session.active ? "Reconnecting…" : "\(session.machine) · Ended").font(.system(size: 11)).foregroundStyle(stream.connected ? Crab.mint : Crab.muted).lineLimit(1)
                        }.frame(maxWidth: .infinity, alignment: .leading)
                        if !prs.isEmpty {
                            ScrollView(.horizontal, showsIndicators: false) {
                                HStack(spacing: 8) { ForEach(prs) { pr in if let url = URL(string: pr.url), url.scheme == "https" { Link("#" + String(pr.number), destination: url).font(.system(size: 12)).foregroundStyle(Crab.purple).padding(.horizontal, 10).frame(minHeight: 44).overlay(RoundedRectangle(cornerRadius: 8).stroke(Crab.purple.opacity(0.3))).accessibilityLabel("Open \(pr.repo)#\(pr.number) on GitHub") } } }
                            }.frame(maxWidth: max(60, min(240, geometry.size.width - 288))).padding(.horizontal, 8)
                        }
                        IconButton(icon: "arrow.down.to.line", label: pinned ? "Stop following output" : "Follow latest output", selected: pinned) { pinned.toggle() }
                        IconButton(icon: "textformat.size", label: "Style") { style = true }.popover(isPresented: $style, arrowEdge: .top) { StyleMenu(model: model).nativePopover() }
                    }.padding(.horizontal, 4).padding(.vertical, 8)
                    Divider()
                    Button { model.preferences.widgets.toggle() } label: {
                        HStack {
                            Image(systemName: "square.grid.2x2")
                            Text("Status · Git · Changes · PRs · Recap").font(.system(size: 11))
                            Spacer()
                            Image(systemName: model.preferences.widgets ? "chevron.down" : "chevron.right")
                        }.foregroundStyle(Crab.muted).padding(.horizontal, 12).frame(minHeight: 44)
                    }.accessibilityLabel(model.preferences.widgets ? "Hide session modules" : "Show session modules")
                    if let error = stream.error { HStack { Text(error).font(.caption).foregroundStyle(Crab.red); Spacer(); IconButton(icon: "xmark", label: "Dismiss session error") { stream.error = nil } }.padding(.leading, 12) }
                    GeometryReader { area in
                        VStack(spacing: 0) {
                            ZStack {
                                if stream.history.isEmpty && stream.screen.isEmpty {
                                    if stream.screenLoaded && stream.historyLoaded {
                                        Text("No terminal output").font(.system(size: 13)).foregroundStyle(Crab.muted)
                                    } else if !session.active {
                                        Text("Terminal unavailable for this ended session").font(.system(size: 13)).foregroundStyle(Crab.muted)
                                    } else {
                                        VStack(spacing: 12) {
                                            ProgressView().tint(Crab.orange).accessibilityIdentifier("session-loading")
                                            if loadingSlowly {
                                                Text("Waiting for terminal output…").font(.system(size: 13)).foregroundStyle(Crab.muted)
                                                Button("Reconnect") { stream.stop(); stream.start() }
                                            }
                                        }
                                    }
                                } else {
                                    TerminalViewport(history: stream.history, screen: stream.screen, preferences: model.preferences, pinned: $pinned, swipe: wide ? { distance, velocity, ended in
                                        drag = distance
                                        if ended {
                                            let dismiss = velocity > 600 || (distance > geometry.size.width / 3 && velocity > -600)
                                            withAnimation(.easeOut(duration: 0.28)) { drag = dismiss ? geometry.size.width : 0 }
                                            if dismiss { DispatchQueue.main.asyncAfter(deadline: .now() + 0.28) { if model.selectionID == session.id { close() } } }
                                        }
                                    } : nil).id(session.id)
                                }
                            }.frame(height: terminalHeight(area.size.height)).frame(maxWidth: .infinity).background(Color(uiColor: TerminalText.rgb(0x0B0F14)))
                            if showsWidgets { SessionWidgets(sections: widgetSections).frame(maxHeight: .infinity) }
                            else if model.preferences.terminalHeight > 0 { Spacer(minLength: 0) }
                        }
                    }
                    if let prompt = stream.prompt { PromptPanel(prompt: prompt, revision: stream.revision, disabled: stream.sending || !stream.connected || stream.revision == nil) { action, revision in stream.send(action, guarded: true, expectedRevision: revision) } }
                    SessionComposer(stream: stream, model: model, sessionID: session.id, active: active).id(session.id)
                }.background(Crab.ink).offset(x: drag).clipped()
                    .task(id: session.id) { pinned = true; drag = 0; loadingSlowly = false; try? await Task.sleep(for: .seconds(8)); if !Task.isCancelled { loadingSlowly = true } }
            }
        }.background(Crab.ink)
    }
    func terminalHeight(_ available: CGFloat) -> CGFloat {
        let maxHeight = showsWidgets ? available * 0.65 : available
        return model.preferences.terminalHeight == 0 ? maxHeight : min(CGFloat(model.preferences.terminalHeight), maxHeight)
    }
}
struct SessionWidgetSection {
    struct Row {
        var text: String
        var url: URL? = nil
    }
    var title: String
    var rows: [Row]
}
func sessionWidgetSections(session: Session?, details: [String: JSON], prefs: Preferences, prs: [PullRequest]) -> [SessionWidgetSection] {
    var sections: [SessionWidgetSection] = []
    func append(_ key: String, _ title: String, _ rows: [String]) {
        if prefs.visible(key) && !rows.isEmpty {
            sections.append(SessionWidgetSection(title: title, rows: rows.map { .init(text: $0) }))
        }
    }
    let stats = details["stats"] ?? [:]
    var status = [session?.state.capitalized ?? "Connecting"]
    if !stats.text("model").isEmpty { status.append(stats.text("model")) }
    if !stats.isEmpty {
        status.append("\(durationText(stats.int("work_seconds"))) session · \(durationText(stats.int("thinking_seconds"))) thinking")
        status.append("\(stats.int("prompts")) prompts · \(stats.int("completions")) completions · \(stats.int("tools")) tools · \(stats.int("compressions")) compactions")
    }
    append("stats", "Status", status)
    let recap = details["recap"]?.object("latest").text("headline", session?.recap ?? "") ?? session?.recap ?? ""
    let bullets = details["recap"]?.object("latest")["bullets"] as? [String] ?? []
    append("recap", "Recap", (recap.isEmpty ? [] : [recap]) + bullets)
    if prefs.visible("prs") {
        let links: [SessionWidgetSection.Row] = prs.compactMap { pr in
            guard let url = URL(string: pr.url), url.scheme == "https" else { return nil }
            return .init(text: "#\(pr.number) \(pr.title)", url: url)
        }
        if !links.isEmpty { sections.append(SessionWidgetSection(title: "Pull requests", rows: links)) }
    }
    let git = details["git"] ?? [:], branch = git.text("branch", session?.branch ?? "")
    append("git", "Git status", (branch.isEmpty ? [] : ["⎇ " + branch]) + git.array("files").map { "\($0.text("status")) \($0.text("path"))  +\($0.int("additions")) −\($0.int("deletions"))" })
    append("commits", "Commits", (details["commit_history"]?.array("history") ?? git.array("recent_commits")).map { "\($0.text("short_hash")) \($0.text("subject"))" })
    append("changes", "Changes", (details["changes"]?.array("by_language") ?? []).flatMap { $0.array("changes") }.map { "\($0.text("name"))  +\($0.int("additions")) −\($0.int("deletions"))" })
    return sections
}
struct SessionWidgets: View {
    var sections: [SessionWidgetSection]
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                ForEach(Array(sections.enumerated()), id: \.offset) { _, section in
                    Text(section.title).foregroundStyle(Crab.title)
                    ForEach(Array(section.rows.enumerated()), id: \.offset) { _, row in
                        if let url = row.url { Link(row.text, destination: url) }
                        else { Text(row.text) }
                    }
                }
            }.font(.system(size: 12)).frame(maxWidth: .infinity, alignment: .leading).padding(16).textSelection(.enabled)
        }.background(Crab.panel)
    }
}
