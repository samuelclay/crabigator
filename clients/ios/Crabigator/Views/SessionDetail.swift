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
                                }
                                TerminalViewport(history: stream.history, screen: stream.screen, preferences: model.preferences, pinned: $pinned, swipe: wide ? { distance, velocity, ended in
                                    drag = distance
                                    if ended {
                                        let dismiss = velocity > 600 || (distance > geometry.size.width / 3 && velocity > -600)
                                        withAnimation(.easeOut(duration: 0.28)) { drag = dismiss ? geometry.size.width : 0 }
                                        if dismiss { DispatchQueue.main.asyncAfter(deadline: .now() + 0.28) { if model.selectionID == session.id { close() } } }
                                    }
                                } : nil).id(session.id).opacity(stream.history.isEmpty && stream.screen.isEmpty ? 0 : 1)
                                    .allowsHitTesting(!stream.history.isEmpty || !stream.screen.isEmpty)
                            }.frame(height: terminalHeight(area.size.height)).frame(maxWidth: .infinity).background(Color(uiColor: TerminalText.rgb(0x0B0F14)))
                            if model.preferences.widgets { SessionWidgets(stream: stream, prefs: model.preferences, prs: prs).frame(maxHeight: .infinity) }
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
        let maxHeight = model.preferences.widgets ? available * 0.65 : available
        return model.preferences.terminalHeight == 0 ? maxHeight : min(CGFloat(model.preferences.terminalHeight), maxHeight)
    }
}
struct SessionWidgets: View {
    @ObservedObject var stream: SessionStream
    var prefs: Preferences, prs: [PullRequest]
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                let recap = stream.details["recap"]?.object("latest").text("headline") ?? stream.selected?.recap ?? ""
                if prefs.visible("recap") && !recap.isEmpty { section("Recap", rows: [recap]) }
                if prefs.visible("prs") && !prs.isEmpty { Text("Pull requests").foregroundStyle(Crab.title); ForEach(prs) { pr in if let url = URL(string: pr.url), url.scheme == "https" { Link(String("#\(pr.number) \(pr.title)"), destination: url) } } }
                if prefs.visible("git") {
                    let git = stream.details["git"] ?? [:], branch = git.text("branch", stream.selected?.branch ?? "")
                    section("Git status", rows: (branch.isEmpty ? [] : ["⎇ " + branch]) + git.array("files").map { "\($0.text("status")) \($0.text("path"))  +\($0.int("additions")) −\($0.int("deletions"))" })
                }
                if prefs.visible("commits") { section("Commits", rows: (stream.details["commit_history"]?.array("history") ?? stream.details["git"]?.array("recent_commits") ?? []).map { "\($0.text("short_hash")) \($0.text("subject"))" }) }
                if prefs.visible("changes") { section("Changes", rows: (stream.details["changes"]?.array("by_language") ?? []).flatMap { $0.array("changes") }.map { "\($0.text("name"))  +\($0.int("additions")) −\($0.int("deletions"))" }) }
            }.font(.system(size: 12)).frame(maxWidth: .infinity, alignment: .leading).padding(16).textSelection(.enabled)
        }.background(Crab.panel)
    }
    @ViewBuilder func section(_ title: String, rows: [String]) -> some View { if !rows.isEmpty { Text(title).foregroundStyle(Crab.title); ForEach(Array(rows.enumerated()), id: \.offset) { _, row in Text(row) } } }
}
