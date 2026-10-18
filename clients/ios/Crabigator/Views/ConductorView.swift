import SwiftUI

struct BrowserModeControl: View {
    @ObservedObject var model: SessionModel
    var wide = false
    var body: some View {
        HStack(spacing: 3) {
            segment("List", "list.bullet", 0)
            segment("PR board", "rectangle.split.3x1", 1)
            segment("Conductor", "point.3.connected.trianglepath.dotted", 2)
        }.padding(5).background(.ultraThinMaterial, in: Capsule())
            .overlay(Capsule().strokeBorder(Color.white.opacity(0.12)))
            .shadow(color: .black.opacity(0.3), radius: 10, y: 4)
    }
    private var selected: Int { model.navigation.conductor ? 2 : (model.navigation.listMode ? 0 : 1) }
    private func segment(_ label: String, _ icon: String, _ mode: Int) -> some View {
        Button {
            withAnimation(.easeInOut(duration: 0.36)) {
                if mode == 2 { model.navigation.conductor = true }
                else {
                    if selected == mode && wide { model.navigation.expanded.toggle() }
                    model.navigation.conductor = false
                    model.navigation.listMode = mode == 0
                }
            }
        } label: {
            HStack(spacing: 5) {
                Image(systemName: icon)
                Text(label).font(.system(size: 12, weight: .medium))
                if wide && selected == mode && mode != 2 {
                    Image(systemName: model.navigation.expanded ? "arrow.down.right.and.arrow.up.left" : "arrow.up.left.and.arrow.down.right").font(.system(size: 10))
                }
            }.padding(.horizontal, 9).frame(minHeight: 40)
                .background(selected == mode ? Crab.orange.opacity(0.18) : .clear, in: Capsule())
                .foregroundStyle(selected == mode ? Crab.orange : Crab.muted)
        }.buttonStyle(.plain).accessibilityIdentifier(label)
            .accessibilityValue(selected == mode ? (wide && mode != 2 ? (model.navigation.expanded ? "Selected, full width" : "Selected, sidebar") : "Selected") : "")
    }
}

@MainActor final class ConductorModel: ObservableObject {
    @Published var snapshot: JSON = [:]
    @Published var error: String?
    @Published var sending = false
    @Published var draft = ""
    @Published var pane = "Conversation"
    @Published var query = ""
    @Published var outcome = "all"
    @Published var archive: JSON = [:]
    @Published var archiveError: String?
    @Published var searching = false
    private var searchEpoch = 0
    private var archiveQuery = "", archiveOutcome = ""
    private let request: (String, JSON?) async throws -> JSON
    init(request: @escaping (String, JSON?) async throws -> JSON = { path, body in try await API.shared.call(path, body: body) }) {
        self.request = request
    }
    private var messageID = UUID().uuidString
    private var messageText = ""
    func refresh() async {
        do { snapshot = try await request("/api/conductor", nil); error = snapshot.text("error").isEmpty ? nil : snapshot.text("error") }
        catch is CancellationError {} catch {
            self.error = (error as? APIError)?.status == 404 ? "Conductor is not available on this server yet." : error.localizedDescription
        }
    }
    func search(more: Bool = false) async {
        if more && (searching || archiveQuery != query || archiveOutcome != outcome) { return }
        searchEpoch += 1; let epoch = searchEpoch
        let currentQuery = query, currentOutcome = outcome
        searching = true; archiveError = nil
        defer { if epoch == searchEpoch { searching = false } }
        do {
            let page = try await request("/api/conductor/archive", ["query": currentQuery, "status": currentOutcome, "offset": more ? archive.int("next_offset") : 0])
            guard !Task.isCancelled, epoch == searchEpoch, query == currentQuery, outcome == currentOutcome else { return }
            archiveQuery = currentQuery; archiveOutcome = currentOutcome
            if more {
                var combined = page
                let existing = archive.array("entries"), ids = Set(existing.map { $0.text("id") })
                combined["entries"] = existing + page.array("entries").filter { !ids.contains($0.text("id")) }
                archive = combined
            } else { archive = page }
        } catch is CancellationError {} catch { if epoch == searchEpoch { archiveError = error.localizedDescription } }
    }
    func send() async {
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty, !sending else { return }
        if text != messageText { messageID = UUID().uuidString; messageText = text }
        sending = true; defer { sending = false }
        do {
            _ = try await request("/api/conductor/message", ["id": messageID, "text": text])
            if draft.trimmingCharacters(in: .whitespacesAndNewlines) == text { draft = "" }
            messageText = ""; pane = "Conversation"; await refresh()
        } catch { self.error = error.localizedDescription }
    }
}

struct ConductorView: View {
    @ObservedObject var model: SessionModel
    @ObservedObject var conductor: ConductorModel
    @Environment(\.scenePhase) private var phase
    @State private var showHistory = false
    @State private var recentOutcome = "all"
    @FocusState private var composerFocused: Bool
    private var tracks: [JSON] { conductor.snapshot.array("tracks") }
    private var messages: [JSON] { conductor.snapshot.array("messages") }
    private var exchanges: [[JSON]] {
        var result: [[JSON]] = []
        for message in messages {
            if message.text("role") == "user" || result.isEmpty { result.append([]) }
            result[result.count - 1].append(message)
        }
        return showHistory ? result : Array(result.suffix(1))
    }
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Conductor").font(.headline)
                Spacer()
                if conductor.snapshot.bool("working") { ProgressView().tint(Crab.orange) }
                Text(conductor.snapshot.bool("host_online") ? "Desktop ready" : "Desktop offline").font(.caption).foregroundStyle(Crab.muted)
            }.padding()
            Picker("Conductor views", selection: $conductor.pane) {
                ForEach(["Conversation", "Sessions", "Archive"], id: \.self) { Text($0).tag($0) }
            }.pickerStyle(.segmented).padding(.horizontal).padding(.bottom, 12)
            if let error = conductor.error {
                HStack { Text(error).font(.caption); Spacer(); Button("Retry") { Task { await conductor.refresh() } } }
                    .padding(12).background(Crab.red.opacity(0.18))
            }
            if conductor.pane == "Conversation" { conversation }
            else if conductor.pane == "Sessions" { sessions }
            else { archive }
            BrowserModeControl(model: model).padding(.vertical, 10)
        }.background(Crab.ink).fontDesign(.monospaced)
            .onChange(of: model.navigation.conductor) { _, active in if !active { composerFocused = false } }
            .onChange(of: conductor.pane) { _, _ in composerFocused = false }
            .task(id: phase) {
                guard phase == .active else { return }
                while !Task.isCancelled { await conductor.refresh(); try? await Task.sleep(for: .seconds(3)) }
            }
            .task(id: conductor.pane + "|" + conductor.query + "|" + conductor.outcome) {
                guard conductor.pane == "Archive" else { return }
                do { try await Task.sleep(for: .milliseconds(250)); await conductor.search() } catch {}
            }
    }
    private var conversation: some View {
        VStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    if messages.isEmpty {
                        Text("One conversation. Your existing sessions.").font(.headline)
                        Text("Ask here. Conductor finds the right sessions and brings their answers back.").foregroundStyle(Crab.muted)
                    }
                    ForEach(exchanges, id: \.firstID) { current in
                    let replies = current.filter { $0.text("role") != "user" }
                    ForEach(current.filter { $0.text("role") == "user" }, id: \.selfID) { message in
                        VStack(alignment: .leading, spacing: 10) {
                            Text("You · " + conductorDate(message.int("at"))).font(.caption).foregroundStyle(Crab.muted)
                            ConductorRichText(blocks: message.array("blocks"), fallback: message.text("text"))
                        }.padding(.leading, 14).overlay(alignment: .leading) { Rectangle().fill(Crab.orange).frame(width: 2) }
                    }
                    if let reply = replies.last {
                        VStack(alignment: .leading, spacing: 10) {
                            Text("Latest reply · " + conductorDate(reply.int("at"))).font(.caption).foregroundStyle(Crab.orange)
                            ConductorRichText(blocks: reply.array("blocks"), fallback: reply.text("text"))
                            highlights(reply)
                        }
                    }
                    if replies.count > 1 {
                        DisclosureGroup("Earlier updates · \(replies.count - 1)") {
                            ForEach(Array(replies.dropLast()), id: \.selfID) { reply in
                                VStack(alignment: .leading, spacing: 8) {
                                    Text(conductorDate(reply.int("at"))).font(.caption).foregroundStyle(Crab.muted)
                                    ConductorRichText(blocks: reply.array("blocks"), fallback: reply.text("text"))
                            highlights(reply)
                                }.padding(.vertical, 12)
                            }
                        }
                    }
                    if replies.last?.array("highlights").isEmpty ?? true {
                        ForEach(tracks.filter { $0.text("request_id") == current.first?.text("id") }, id: \.selfID) { track in
                            VStack(alignment: .leading, spacing: 10) {
                                ConductorIdentity(entry: track)
                                Text(conductorStatus(track) + " · " + conductorDate(track.int("updated_at"))).font(.caption).foregroundStyle(conductorColor(track))
                                Button("View requests") { conductor.pane = "Sessions" }
                            }.padding(16).background(Crab.panel, in: RoundedRectangle(cornerRadius: 6))
                        }
                    }
                    }
                    if messages.filter({ $0.text("role") == "user" }).count > 1 {
                        Button(showHistory ? "Show latest exchange" : "Earlier conversation") { showHistory.toggle() }
                    }
                    if !conductor.snapshot.bool("host_online") { Text("Start on your computer: crabigator conductor").font(.caption).foregroundStyle(Crab.muted) }
                }.frame(maxWidth: 760, alignment: .leading).padding(20).frame(maxWidth: .infinity)
            }.contentShape(Rectangle()).clipped()
            HStack(alignment: .bottom, spacing: 10) {
                TextField("Ask your sessions…", text: Binding(get: { conductor.draft }, set: { conductor.draft = String($0.prefix(16000)) }), axis: .vertical)
                    .lineLimit(2...5).focused($composerFocused).textFieldStyle(.plain).padding(12)
                    .background(Crab.panel, in: RoundedRectangle(cornerRadius: 6)).accessibilityIdentifier("Conductor message")
                Button { Task { showHistory = false; await conductor.send() } } label: {
                    Image(systemName: "arrow.up").font(.headline).frame(width: 40, height: 40)
                }.buttonStyle(.borderedProminent).disabled(conductor.sending || conductor.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityLabel("Send to Conductor")
            }.padding(12).background(Crab.ink)
        }
    }
    private var sessions: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("One row per actual session. Requests appear underneath.").font(.caption).foregroundStyle(Crab.muted).padding(.horizontal)
            Picker("Recent request outcome", selection: $recentOutcome) {
                Text("All recent").tag("all"); Text("Active").tag("active"); Text("Answered").tag("answered"); Text("Failed").tag("failed")
            }.pickerStyle(.segmented).padding(.horizontal)
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    if tracks.isEmpty { Text("No requests sent yet. Start in Conversation.").foregroundStyle(Crab.muted) }
                    ForEach(projectNames(tracks), id: \.self) { project in
                        Text(project).font(.headline)
                        let projectTracks = tracks.filter { $0.text("project", "Unknown project") == project }
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), alignment: .top)], spacing: 14) {
                            ForEach(sessionIDs(projectTracks), id: \.self) { id in sessionView(projectTracks.filter { $0.text("session_id") == id }) }
                        }
                    }
                    Text("Older requests and answers are in Archive.").font(.caption).foregroundStyle(Crab.muted)
                }.padding(16)
            }
        }
    }
    private func sessionView(_ items: [JSON]) -> some View {
        let first = items.first ?? [:]
        let visible = items.filter { recentOutcome == "all" || (recentOutcome == "active" ? !conductorEnded($0) : $0.text("state") == recentOutcome) }
        return DisclosureGroup {
            VStack(alignment: .leading, spacing: 12) {
                ForEach(visible, id: \.selfID) { entry in requestView(entry, live: true) }
                if visible.isEmpty { Text("No matching requests.").font(.caption).foregroundStyle(Crab.muted) }
            }.padding(.top, 12)
        } label: {
            VStack(alignment: .leading, spacing: 6) {
                ConductorIdentity(entry: first)
                ConductorContext(entry: first)
                Text("\(items.filter { !conductorEnded($0) }.count) active · \(items.filter { $0.text("state") == "answered" }.count) answered · \(items.filter { $0.text("state") == "failed" }.count) failed").font(.caption2).foregroundStyle(Crab.muted)
                Text(first.bool("connected") ? "● Online" : "○ Offline").font(.caption2).foregroundStyle(Crab.muted)
            }
        }.padding(14).background(Crab.panel, in: RoundedRectangle(cornerRadius: 6))
    }
    private var archive: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                TextField("Find a question or answer…", text: $conductor.query).textFieldStyle(.roundedBorder).accessibilityIdentifier("Search Conductor archive")
                Picker("Request outcome", selection: $conductor.outcome) {
                    Text("All outcomes").tag("all"); Text("Active").tag("active"); Text("Answered").tag("answered"); Text("Failed").tag("failed"); Text("Stopped").tag("stopped")
                }.pickerStyle(.menu)
            }.padding(.horizontal)
            if conductor.searching { ProgressView().padding(.horizontal) }
            if let error = conductor.archiveError { Text(error).font(.caption).foregroundStyle(Crab.red).padding(.horizontal) }
            Text("\(conductor.archive.int("total")) saved matches · by project and session").font(.caption).foregroundStyle(Crab.muted).padding(.horizontal)
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    let entries = conductor.archive.array("entries")
                    ForEach(projectNames(entries), id: \.self) { project in
                        Text(project).font(.headline)
                        let projectEntries = entries.filter { $0.text("project", "Unknown project") == project }
                        ForEach(sessionIDs(projectEntries), id: \.self) { id in
                            if id.isEmpty { Text("Coordinator conversation").font(.caption).foregroundStyle(Crab.muted) }
                            else { ConductorIdentity(entry: projectEntries.first { $0.text("session_id") == id } ?? [:]) }
                            ForEach(projectEntries.filter { $0.text("session_id") == id }, id: \.selfID) { entry in requestView(entry, live: false) }
                        }
                    }
                    if entries.isEmpty && !conductor.searching { Text("No matches. Try another word or outcome.").foregroundStyle(Crab.muted) }
                    if conductor.archive["next_offset"] is NSNumber { Button("Load more") { Task { await conductor.search(more: true) } }.disabled(conductor.searching) }
                }.frame(maxWidth: 900, alignment: .leading).padding(16).frame(maxWidth: .infinity)
            }
        }
    }
    private func highlights(_ message: JSON) -> some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 280), alignment: .top)], spacing: 16) {
            ForEach(message.array("highlights"), id: \.sessionID) { entry in
                VStack(alignment: .leading, spacing: 14) {
                    Button { Task { await model.open(entry.text("session_id")) } } label: { ConductorIdentity(entry: entry) }.buttonStyle(.plain)
                    Text(conductorStatus(entry) + " · " + conductorDate(entry.int("updated_at"))).font(.caption2).foregroundStyle(conductorColor(entry))
                    ConductorRichText(blocks: entry.array("blocks"), fallback: entry.text("text"))
                }.padding(18).frame(maxWidth: .infinity, alignment: .leading).background(Crab.panel, in: RoundedRectangle(cornerRadius: 6))
                    .overlay(alignment: .top) { Rectangle().fill(conductorColor(entry)).frame(height: 2) }
            }
        }
    }
    private func requestView(_ entry: JSON, live: Bool) -> some View {
        DisclosureGroup {
            VStack(alignment: .leading, spacing: 16) {
                if !entry.text("question", entry.text("instruction")).isEmpty {
                    DisclosureGroup("Original question") { ConductorRichText(blocks: entry.array("question_blocks"), fallback: entry.text("question", entry.text("instruction"))).padding(.top, 10) }
                }
                ConductorRichText(blocks: entry.array("blocks"), fallback: entry.text("result", entry.text("text", entry.array("events").last?.text("text") ?? "Waiting for a report.")))
                highlights(entry)
                HStack {
                    if !entry.text("result", entry.text("text")).isEmpty { Button("Copy answer") { UIPasteboard.general.string = ([entry.text("result", entry.text("text"))] + entry.array("highlights").map { conductorSessionName($0) + "\n" + $0.text("text") }).joined(separator: "\n\n") } }
                    if !entry.text("session_id").isEmpty { Button("Open actual session") { Task { await model.open(entry.text("session_id")) } } }
                }.font(.caption)
                let events = entry.array("events").filter { $0.text("text") != entry.text("result") }
                if !events.isEmpty {
                    DisclosureGroup("Progress · \(events.count) updates") {
                        ForEach(Array(events.enumerated()), id: \.offset) { _, event in
                            VStack(alignment: .leading, spacing: 6) {
                                Text(conductorDate(event.int("at"))).font(.caption2).foregroundStyle(Crab.muted)
                                Text(event.text("text")).font(.caption).textSelection(.enabled)
                            }.padding(.vertical, 8)
                        }
                    }
                }
                if live {
                    DisclosureGroup("Live session activity") {
                        Text("Includes unrelated work in this session.").font(.caption).foregroundStyle(Crab.muted)
                        Text(AttributedString(TerminalText.parse(entry.text("activity", entry.text("screen")), size: 12, spacing: 1.5, wrap: true))).textSelection(.enabled)
                    }
                }
            }.padding(.top, 14)
        } label: {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text(conductorStatus(entry)).font(.caption2).foregroundStyle(conductorColor(entry)).padding(.horizontal, 7).padding(.vertical, 3).background(conductorColor(entry).opacity(0.12), in: RoundedRectangle(cornerRadius: 3))
                    Text(conductorDate(entry.int("updated_at", entry.int("at")))).font(.caption2).foregroundStyle(Crab.muted)
                }
                Text(entry.text("title", entry.text("text"))).lineLimit(2).foregroundStyle(.primary)
                if !live { Text(entry.text("excerpt")).font(.caption).foregroundStyle(Crab.muted).lineLimit(3) }
            }
        }.padding(14).background(Crab.panel, in: RoundedRectangle(cornerRadius: 6))
    }
}

private func projectNames(_ entries: [JSON]) -> [String] { Array(Set(entries.map { $0.text("project", "Unknown project") })).sorted { a, b in a == "Conductor" ? false : b == "Conductor" ? true : a < b } }
private func sessionIDs(_ entries: [JSON]) -> [String] { var seen = Set<String>(); return entries.map { $0.text("session_id") }.filter { seen.insert($0).inserted } }
private func conductorSessionName(_ entry: JSON) -> String { entry.text("session_title").isEmpty ? entry.text("project", "Session") : entry.text("session_title") }
private func conductorEnded(_ entry: JSON) -> Bool { ["answered", "failed", "cancelled"].contains(entry.text("state")) }
private func conductorStatus(_ entry: JSON) -> String {
    if entry.text("kind") == "message" { return entry.text("role") == "user" ? "Question" : "Coordinator reply" }
    return ["sending": "Sending", "forwarded": "Awaiting delivery", "delivered": "Queued", "working": "Working", "waiting": "Needs attention", "delivery_unknown": "Check delivery", "answered": "Answered", "failed": "Failed", "cancelled": "Stopped"][entry.text("state")] ?? "Pending"
}
private func conductorColor(_ entry: JSON) -> Color { entry.text("state") == "answered" ? .green : entry.text("state") == "failed" ? Crab.red : Crab.orange }
private func conductorDate(_ value: Int) -> String { value > 0 ? Date(timeIntervalSince1970: Double(value) / 1000).formatted(date: .abbreviated, time: .shortened) : "" }
private extension Dictionary where Key == String, Value == Any { var selfID: String { text("id") }; var sessionID: String { text("session_id") } }

private extension Array where Element == JSON { var firstID: String { first?.text("id") ?? "" } }
private struct ConductorIdentity: View {
    let entry: JSON
    var body: some View {
        HStack(alignment: .center, spacing: 12) {
            IdentityChip(session: Session(entry))
            VStack(alignment: .leading, spacing: 5) {
                Text(conductorSessionName(entry)).font(.system(size: 14, weight: .semibold, design: .monospaced)).foregroundStyle(.primary)
                Text([entry.text("project"), entry.text("platform")].filter { !$0.isEmpty }.joined(separator: " · ")).font(.caption2).foregroundStyle(Crab.muted)
            }
        }
    }
}
private struct ConductorContext: View {
    let entry: JSON
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if !entry.text("recap").isEmpty { Text(entry.text("recap")).font(.caption).foregroundStyle(Crab.muted).lineLimit(2) }
            HStack(spacing: 8) {
                if entry.int("additions") > 0 || entry.int("deletions") > 0 {
                    Text("+\(entry.int("additions"))").foregroundStyle(.green)
                    Text("−\(entry.int("deletions"))").foregroundStyle(Crab.red)
                }
                Text(entry.text("session_state")).foregroundStyle(Crab.muted)
                if entry.int("prompts") > 0 { Text("\(entry.int("prompts")) prompts").foregroundStyle(Crab.muted) }
            }.font(.caption2)
        }
    }
}
private struct ConductorRichText: View {
    var blocks: [JSON]
    var fallback: String
    private func inline(_ value: String) -> AttributedString {
        var result = (try? AttributedString(markdown: value, options: .init(interpretedSyntax: .inlineOnlyPreservingWhitespace))) ?? AttributedString(value)
        for run in result.runs {
            if let link = run.link, !["http", "https"].contains(link.scheme?.lowercased() ?? "") { result[run.range].link = nil }
            if run.inlinePresentationIntent?.contains(.stronglyEmphasized) == true { result[run.range].foregroundColor = Crab.orange }
        }
        return result
    }
    var body: some View {
        let content: [JSON] = blocks.isEmpty ? [["type": "paragraph", "text": fallback]] : blocks
        VStack(alignment: .leading, spacing: 16) {
            ForEach(Array(content.enumerated()), id: \.offset) { _, block in
                if block.text("type") == "table" {
                    let headers = block["headers"] as? [String] ?? [], rows = block["rows"] as? [[String]] ?? []
                    ScrollView(.horizontal) {
                        VStack(alignment: .leading, spacing: 0) {
                            tableRow(headers, heading: true)
                            ForEach(Array(rows.enumerated()), id: \.offset) { _, row in Divider(); tableRow(row, heading: false) }
                        }.background(Crab.panel, in: RoundedRectangle(cornerRadius: 4))
                    }
                } else if block.text("type") == "code" {
                    Text(block.text("text")).font(.system(size: 12, design: .monospaced)).padding(12).background(Crab.panel).textSelection(.enabled)
                } else {
                    Text(inline(block.text("text"))).font(block.text("type") == "heading" ? .headline : .body).lineSpacing(5).textSelection(.enabled)
                }
            }
        }.frame(maxWidth: .infinity, alignment: .leading)
    }
    private func tableRow(_ cells: [String], heading: Bool) -> some View {
        HStack(alignment: .top, spacing: 0) {
            ForEach(Array(cells.enumerated()), id: \.offset) { _, cell in
                Text(inline(cell)).font(.caption).fontWeight(heading ? .semibold : .regular).frame(width: 170, alignment: .leading).padding(10).textSelection(.enabled)
            }
        }
    }
}
