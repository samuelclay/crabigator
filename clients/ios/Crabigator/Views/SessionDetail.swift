import SwiftUI

struct SessionDetail: View {
    @ObservedObject var model: SessionModel
    @ObservedObject var stream: SessionStream
    var wide: Bool, active: Bool
    var close: () -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var pinned = true
    @State private var style = false
    @State private var drag: CGFloat = 0
    @State private var loadingSlowly = false
    var prs: [PullRequest] {
        if let live = stream.details["prs"] { return live.array("prs").filter { !$0.bool("dismissed") }.map(PullRequest.init) }
        return model.prs.filter { $0.sessions.contains { $0.id == stream.selected?.id } }
    }
    @State private var modulesHeight: CGFloat = 140
    var showsWidgets: Bool { model.preferences.widgets }
    var body: some View {
        GeometryReader { geometry in
            if let session = stream.selected {
                VStack(spacing: 0) {
                    HStack(spacing: 2) {
                        IconButton(icon: wide ? "xmark" : "chevron.left", label: wide ? "Close session" : "Back to list", action: close)
                        VStack(alignment: .leading, spacing: 3) {
                            Text(session.title).font(.system(size: 15, weight: .medium)).foregroundStyle(Crab.title).lineLimit(2).accessibilityIdentifier("Selected session \(session.id)")
                            Text(stream.connected ? "\(session.machine) · Live" : !session.active ? "\(session.machine) · Ended" : stream.desktopStatusKnown ? "\(session.machine) · Disconnected" : "Connecting…").font(.system(size: 11)).foregroundStyle(stream.connected ? Crab.mint : Crab.muted).lineLimit(1)
                        }.frame(maxWidth: .infinity, alignment: .leading)
                        IconButton(icon: "square.grid.2x2", label: model.preferences.widgets ? "Hide session modules" : "Show session modules", selected: model.preferences.widgets) {
                            withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.25)) { model.preferences.widgets.toggle() }
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
                                    } else if stream.desktopStatusKnown && !stream.connected {
                                        Text("Desktop disconnected. Terminal output will resume when it reconnects.")
                                            .font(.system(size: 13)).foregroundStyle(Crab.muted)
                                            .multilineTextAlignment(.center).padding()
                                            .accessibilityIdentifier("session-disconnected")
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
                            if showsWidgets {
                                SessionModules(session: session, data: SessionModuleData(session: session, details: stream.details), prefs: model.preferences, prs: prs)
                                    .id(session.id).frame(height: min(modulesHeight, area.size.height * 0.45))
                                    .onPreferenceChange(ModuleContentHeight.self) { height in
                                        guard showsWidgets, height != modulesHeight else { return }
                                        withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.25)) { modulesHeight = height }
                                    }
                                    .transition(.opacity)
                            }
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
        let maxHeight = max(0, available - (showsWidgets ? min(modulesHeight, available * 0.45) : 0))
        return model.preferences.terminalHeight == 0 ? maxHeight : min(CGFloat(model.preferences.terminalHeight), maxHeight)
    }
}
