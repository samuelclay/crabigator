import SwiftUI

extension View {
    func nativePopover() -> some View { self.frame(idealWidth: 330, maxWidth: 330, idealHeight: 480, maxHeight: 480).presentationCompactAdaptation(.popover).preferredColorScheme(.dark) }
    /// A View panel or Account: a popover beside its button on a tablet, and on a phone a
    /// sheet that can leave the screen it changes in view.
    func panelPresentation(tall: Bool = false) -> some View { modifier(PanelPresentation(detent: tall ? .large : .fraction(0.8), detents: tall ? [.medium, .large] : [.medium, .fraction(0.8)], ground: tall ? Crab.ink : Crab.panel)) }
}
private struct PanelPresentation: ViewModifier {
    @State var detent: PresentationDetent
    var detents: Set<PresentationDetent>
    var ground: Color
    func body(content: Content) -> some View {
        content.frame(idealWidth: 400, idealHeight: 860)
            .presentationDetents(detents, selection: $detent).presentationDragIndicator(.visible).presentationContentInteraction(.scrolls)
            .presentationBackground(ground).presentationCornerRadius(22).preferredColorScheme(.dark)
    }
}
/// The account, its devices and its alerts. Nothing here changes how a screen looks.
struct AccountScreen: View {
    @ObservedObject var model: SessionModel
    @Environment(\.dismiss) private var dismiss
    @State private var identities: [JSON] = []
    @State private var accountLoaded = false
    @State private var notifications = ""
    @State private var invite = ""
    @State private var error = ""
    @State private var busy = false
    @State private var confirm = false
    private var sample: Bool {
        #if DEBUG
        return ProcessInfo.processInfo.environment["CRABIGATOR_TEST_LAYOUT"] == "1"
        #else
        return false
        #endif
    }
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
            PanelHeader(title: "Account", size: 20) { dismiss() }
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    group("Alerts and history") {
                        NavigationLink { NotificationSettings() } label: { row { Text("Notifications"); Spacer(); Text(notificationSummary.text).font(.system(size: 14)).foregroundStyle(notificationSummary.color).lineLimit(1); chevron } }.buttonStyle(.plain).accessibilityIdentifier("Notifications")
                        divider
                        NavigationLink { VoiceTranscripts(model: model) } label: { row { Text("Voice transcripts"); Spacer(); chevron } }.buttonStyle(.plain).accessibilityIdentifier("Voice transcripts")
                    }
                    group("Sign-in") {
                        if !accountLoaded { row { ProgressView() } }
                        else if identities.isEmpty { row { Text("Pairing code") } }
                        ForEach(identities.indices, id: \.self) { index in row { Text(Self.identity(identities[index])).lineLimit(2) } }
                        divider
                        link("Manage sign-in on the web", model.origin + "/dashboard")
                    }
                    group("Devices and agents") {
                        Button {
                            busy = true
                            Task { defer { busy = false }; do { invite = try await model.api.call("/api/pairing/invite", body: [:]).text("code") } catch { self.error = error.localizedDescription } }
                        } label: { row { Text(busy ? "Generating…" : "Pair another device").foregroundStyle(Crab.cyan) } }.buttonStyle(.plain).disabled(busy)
                        if !invite.isEmpty { Text(invite).font(.title2.monospaced()).textSelection(.enabled).foregroundStyle(Crab.cyan).padding(.bottom, 12).accessibilityIdentifier("Pairing code for another device") }
                        divider
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Connect an agent with MCP")
                            Text(model.origin + "/mcp").font(.system(size: 13, design: .monospaced)).foregroundStyle(Crab.soft).textSelection(.enabled)
                            Link(destination: URL(string: model.origin + "/mcp-tools")!) { Text("Tools and examples").font(.system(size: 14)).foregroundStyle(Crab.cyan).frame(maxWidth: .infinity, minHeight: panelTouch, alignment: .leading).contentShape(Rectangle()) }
                        }.padding(.top, 10)
                    }
                    group("This device") {
                        VStack(alignment: .leading, spacing: 3) {
                            Text(UIDevice.current.name)
                            Text("\(URL(string: model.origin)?.host ?? model.origin) · Version \(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0")").font(.system(size: 13)).foregroundStyle(Crab.dim)
                        }.padding(.vertical, 10)
                        divider
                        link("Privacy policy", "https://drinkcrabigator.com/privacy")
                        divider
                        Button { confirm = true } label: { row { Text("Unpair this device").foregroundStyle(Crab.red) } }.buttonStyle(.plain)
                    }
                    if !error.isEmpty { Text(error).font(.system(size: 13)).foregroundStyle(Crab.red) }
                    PanelNote("Nothing here changes how a screen looks. Search, New session and Watch PR stay on the board's own buttons.")
                }.font(.system(size: 15)).padding(.horizontal, 16).padding(.top, 4).padding(.bottom, 24)
            }.accessibilityIdentifier("Account screen")
            // Runs again on the way back from the Notifications page, where the status can change.
            }.task { notifications = await Notifications.shared.status() }.background(Crab.ink).foregroundStyle(Crab.text)
                .navigationTitle("Account").navigationBarTitleDisplayMode(.inline).toolbar(.hidden, for: .navigationBar)
                .confirmationDialog("Unpair this device?", isPresented: $confirm, titleVisibility: .visible) { Button("Unpair", role: .destructive) { Task { await model.unpair() } } }
        }.tint(Crab.cyan).task {
            if sample { identities = [["name": "Sample account", "provider": "github"]] }
            else { do { identities = try await model.api.call("/api/account").array("identities") } catch { self.error = error.localizedDescription } }
            accountLoaded = true
        }
    }
    /// "On, silent" in the design's words, from the notifications page's own status.
    private var notificationSummary: (text: String, color: Color) {
        if notifications == "Notifications on" { return (Credentials.defaults.bool(forKey: "notificationSound") ? "On, with sound" : "On, silent", Crab.mint) }
        if notifications == "Notifications off" { return ("Off", Crab.dim) }
        return (notifications, Crab.yellow)
    }
    static func identity(_ identity: JSON) -> String {
        let who = [identity.text("email"), identity.text("name"), identity.text("username")].first { !$0.isEmpty }
        let provider = ["github": "GitHub", "google": "Google"][identity.text("provider")] ?? identity.text("provider").capitalized
        return [who, provider.isEmpty ? nil : (who == nil ? provider : "with " + provider)].compactMap { $0 }.joined(separator: " ")
    }
    private var chevron: some View { Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold)).foregroundStyle(Crab.dim).accessibilityHidden(true) }
    private var divider: some View { Crab.line.frame(height: 1) }
    private func row<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        HStack(spacing: 8) { content() }.frame(maxWidth: .infinity, minHeight: 52, alignment: .leading).contentShape(Rectangle())
    }
    private func link(_ title: String, _ address: String) -> some View {
        Link(destination: URL(string: address)!) { row { Text(title).foregroundStyle(Crab.cyan) } }
    }
    private func group<Content: View>(_ title: String, @ViewBuilder _ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title).font(.system(size: 13, weight: .semibold)).foregroundStyle(Crab.dim).accessibilityAddTraits(.isHeader)
            VStack(alignment: .leading, spacing: 0) { content() }.padding(.horizontal, 14).frame(maxWidth: .infinity, alignment: .leading)
                .background(Crab.panel, in: RoundedRectangle(cornerRadius: 14)).overlay { RoundedRectangle(cornerRadius: 14).strokeBorder(Crab.line) }
        }
    }
}
/// Every voice transcript the account has made. The server saves each one as it is
/// transcribed, so a transcript that never reached its session can still be copied here.
struct VoiceTranscripts: View {
    @ObservedObject var model: SessionModel
    var sessionID: String?
    @State private var rows: [JSON] = []
    @State private var next: Int?
    @State private var loaded = false
    @State private var error = ""
    @State private var copied = ""
    @State private var everySession = false
    private var scope: String? { everySession ? nil : sessionID }
    var body: some View {
        List {
            if sessionID != nil {
                Picker("Show", selection: $everySession) { Text("This session").tag(false); Text("All sessions").tag(true) }.pickerStyle(.segmented)
            }
            if !error.isEmpty { Text(error).foregroundStyle(Crab.red) }
            if !loaded { ProgressView() }
            else if rows.isEmpty && error.isEmpty { Text("No voice transcripts yet. Every recording you transcribe is saved here.").foregroundStyle(Crab.muted) }
            // Rows are identified by transcript, so a swipe-delete removes the right one.
            ForEach(rows, id: \.transcriptID) { entry($0) }
            if next != nil { Button("Load more") { Task { await load(more: true) } } }
        }.navigationTitle("Voice transcripts").navigationBarTitleDisplayMode(.inline)
            .toolbar { Button(copied == "all" ? "Copied" : "Copy all") { copy(rows.map { $0.text("text") }.joined(separator: "\n\n"), id: "all") }.disabled(rows.isEmpty) }
            .task(id: everySession) { await load(more: false) }
    }
    private func entry(_ row: JSON) -> some View {
        let id = row.text("id"), recorded = seconds(row, "recorded_seconds"), audio = seconds(row, "audio_seconds")
        return VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text([Date(timeIntervalSince1970: TimeInterval(row.int("created_at"))).formatted(date: .abbreviated, time: .shortened), (audio ?? recorded).map(clock)].compactMap { $0 }.joined(separator: " · "))
                    .font(.caption).foregroundStyle(Crab.muted)
                Spacer()
                Button(copied == id ? "Copied" : "Copy") { copy(row.text("text"), id: id) }.font(.caption.weight(.semibold)).buttonStyle(.borderless).frame(minHeight: 44)
                    .accessibilityIdentifier("Copy transcript")
            }
            if scope == nil && !row.text("session_title").isEmpty { Text(row.text("session_title")).font(.caption).foregroundStyle(Crab.cyan).lineLimit(1) }
            // The app timed more speech than the server received: audio was lost before upload.
            if let recorded, let audio, recorded - audio > 5 {
                Text("Recorded \(clock(recorded)), but only \(clock(audio)) of audio arrived.").font(.caption).foregroundStyle(Crab.orange)
            }
            Text(row.text("text")).font(.callout).textSelection(.enabled)
        }.swipeActions { Button("Delete", role: .destructive) { Task { await delete(id) } } }
    }
    private func seconds(_ row: JSON, _ key: String) -> Double? { (row[key] as? NSNumber)?.doubleValue }
    private func clock(_ seconds: Double) -> String { "\(Int(seconds) / 60):\(String(format: "%02d", Int(seconds) % 60))" }
    private func copy(_ text: String, id: String) {
        UIPasteboard.general.string = text; copied = id
        Task { try? await Task.sleep(for: .seconds(2)); if copied == id { copied = "" } }
    }
    private func load(more: Bool) async {
        do {
            let page = try await model.api.voiceTranscripts(sessionID: scope, before: more ? next : nil)
            rows = (more ? rows : []) + page.array("transcripts"); next = page["next_cursor"] as? Int; error = ""
        } catch { if !Task.isCancelled { self.error = error.localizedDescription } }
        loaded = true
    }
    private func delete(_ id: String) async {
        do { try await model.api.deleteVoiceTranscript(id); rows.removeAll { $0.transcriptID == id } } catch { self.error = error.localizedDescription }
    }
}
private extension Dictionary where Key == String, Value == Any { var transcriptID: String { text("id") } }
struct NewSessionButton: View {
    @ObservedObject var model: SessionModel
    @State private var presented = false
    var body: some View {
        IconButton(icon: "plus", label: "New session", selected: presented) { presented = true }
            .popover(isPresented: $presented, attachmentAnchor: .rect(.bounds), arrowEdge: .top) {
                NavigationStack {
                    NewSessionForm(model: model)
                        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { presented = false } } }
                }.nativePopover()
            }
    }
}

/// The board's + button: start a session, or watch a pull request that no session touches.
struct AddButton: View {
    enum Choice: String, Identifiable { case session, watch; var id: String { rawValue } }
    @ObservedObject var model: SessionModel
    @State private var presented: Choice?
    var body: some View {
        Menu {
            Button("New session") { presented = .session }
            Button("Watch a pull request") { presented = .watch }
        } label: {
            Image(systemName: "plus").font(.system(size: 17, weight: .medium)).frame(width: 44, height: 44).foregroundStyle(presented == nil ? Crab.muted : Crab.orange).background(presented == nil ? .clear : Crab.orange.opacity(0.13), in: RoundedRectangle(cornerRadius: 10))
        }.accessibilityLabel("New session or watch a pull request").accessibilityIdentifier("Add")
            .popover(item: $presented, attachmentAnchor: .rect(.bounds), arrowEdge: .top) { choice in
                NavigationStack {
                    BoardActionForm(model: model, watch: choice == .watch)
                        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { presented = nil } } }
                }.nativePopover()
            }
    }
}

struct NewSessionForm: View {
    @ObservedObject var model: SessionModel
    @State private var computer = ""
    @State private var folder = ""
    @State private var customFolder = ""
    @State private var platform = "codex"
    private var busy: Bool { model.sessionLaunch.busy }
    private var message: String { model.sessionLaunch.message }
    private var failed: Bool { model.sessionLaunch.failed }
    private let assistants = [("Codex", "codex"), ("Claude", "claude"), ("OpenCode", "opencode"), ("Grok", "grok"), ("Antigravity", "antigravity")]
    private var desktops: [Session] {
        var seen = Set<String>()
        return model.sessions.filter { $0.active && seen.insert($0.machine).inserted }
    }
    private var source: Session? { desktops.first { $0.machine == computer } }
    private var projects: [Session] {
        var seen = Set<String>()
        return model.sessions.filter { $0.machine == computer && !$0.cwd.isEmpty && seen.insert($0.cwd).inserted }
            .sorted { projectName($0).localizedStandardCompare(projectName($1)) == .orderedAscending }
    }
    private var path: String { (folder.isEmpty ? customFolder : folder).trimmingCharacters(in: .whitespacesAndNewlines) }
    private func projectName(_ session: Session) -> String { session.repo.isEmpty ? URL(fileURLWithPath: session.cwd).lastPathComponent : session.repo }
    private func chooseComputer(_ name: String) {
        computer = name
        let selected = model.sessions.first { $0.id == model.selectionID && $0.machine == name }
        folder = selected?.cwd ?? projects.first?.cwd ?? ""
        customFolder = ""
    }
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Project").font(.caption).foregroundStyle(Crab.muted)
                    VStack(alignment: .leading, spacing: 12) {
                        Menu {
                            ForEach(projects, id: \.cwd) { project in
                                Button { folder = project.cwd } label: {
                                    Text(projectName(project))
                                    Text(project.cwd)
                                }
                            }
                            Button("Other folder…") { folder = "" }
                        } label: {
                            HStack {
                                Text(projects.first { $0.cwd == folder }.map(projectName) ?? "Other folder…").lineLimit(2)
                                Spacer(minLength: 4)
                                Image(systemName: "chevron.down").font(.caption)
                            }.foregroundStyle(Crab.orange).frame(minHeight: 32)
                        }.accessibilityIdentifier("New session project")
                        if folder.isEmpty {
                            TextField("/path/to/project", text: $customFolder).textInputAutocapitalization(.never).autocorrectionDisabled().font(.system(.caption, design: .monospaced)).accessibilityLabel("Folder")
                        } else { Text(folder).font(.system(.caption, design: .monospaced)).foregroundStyle(Crab.muted).textSelection(.enabled) }
                        if desktops.count > 1 || (source == nil && !desktops.isEmpty) {
                            Picker("Computer", selection: Binding(get: { computer }, set: chooseComputer)) {
                                ForEach(desktops) { Text($0.machine).tag($0.machine) }
                                if source == nil { Text(computer.isEmpty ? "Choose computer" : "\(computer) · Offline").tag(computer) }
                            }.pickerStyle(.menu)
                        } else if let source { Label(source.machine, systemImage: "desktopcomputer").font(.caption).foregroundStyle(Crab.muted) }
                        else { Text("Open Crabigator on a desktop to start a session.").foregroundStyle(Crab.muted) }
                    }.padding(12).frame(maxWidth: .infinity, alignment: .leading).background(Crab.panel, in: RoundedRectangle(cornerRadius: 12))
                }.disabled(busy)
                VStack(alignment: .leading, spacing: 8) {
                    Text("Assistant").font(.caption).foregroundStyle(Crab.muted)
                    LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 8) {
                        ForEach(assistants, id: \.1) { name, value in
                            Button { platform = value } label: {
                                HStack { Text(name); Spacer(minLength: 4); if platform == value { Image(systemName: "checkmark") } }
                                    .font(.system(size: 14, weight: .medium)).padding(.horizontal, 10).frame(maxWidth: .infinity, minHeight: 44)
                                    .background(platform == value ? Crab.orange.opacity(0.18) : Crab.panel, in: RoundedRectangle(cornerRadius: 10))
                            }.buttonStyle(.plain).foregroundStyle(platform == value ? Crab.orange : Crab.muted)
                                .accessibilityLabel(name).accessibilityValue(platform == value ? "Selected" : "")
                        }
                    }
                }.disabled(busy)
            }.padding(.horizontal, 20).padding(.vertical, 12)
        }
            .safeAreaInset(edge: .bottom) {
                VStack(spacing: 8) {
                    Button {
                        guard let source else { return }
                        model.startSession(on: source, path: path, platform: platform)
                    } label: { HStack { Spacer(); if busy { ProgressView() }; Text(busy ? "Starting…" : "Start session"); Spacer() }.frame(minHeight: 36) }
                        .buttonStyle(.borderedProminent).tint(Crab.orange)
                        .disabled(busy || source == nil || !path.hasPrefix("/") || path.contains("\0"))
                    if !message.isEmpty { Text(message).font(.caption).foregroundStyle(failed ? Crab.red : Crab.mint) }
                }.padding(.horizontal, 20).padding(.vertical, 12).background(.regularMaterial)
            }.navigationTitle("New session").navigationBarTitleDisplayMode(.inline)
            .onAppear {
                if busy {
                    computer = model.sessionLaunch.computer
                    folder = projects.contains { $0.cwd == model.sessionLaunch.path } ? model.sessionLaunch.path : ""
                    customFolder = model.sessionLaunch.path
                    platform = model.sessionLaunch.platform
                } else if computer.isEmpty {
                    chooseComputer(model.sessions.first { $0.id == model.selectionID && $0.active }?.machine ?? desktops.first?.machine ?? "")
                }
            }
            .onChange(of: desktops.map(\.machine)) { _, names in if computer.isEmpty, let name = names.first { chooseComputer(name) } }
    }
}

struct BoardActionForm: View {
    @ObservedObject var model: SessionModel
    let watch: Bool
    @State private var target = ""
    @State private var busy = false
    @State private var message = ""
    @State private var failed = false
    var body: some View {
        if !watch { NewSessionForm(model: model) }
        else {
            Form {
                Section { TextField("GitHub URL or owner/repo#123", text: $target).textInputAutocapitalization(.never).autocorrectionDisabled().accessibilityIdentifier("Watch PR target") }
                Section {
                    Button(busy ? "Sending…" : "Watch PR") {
                        guard let body = BoardActions.watchTarget(target) else { return }
                        busy = true; message = ""; failed = false
                        Task {
                            defer { busy = false }
                            do {
                                _ = try await model.api.call("/api/prs/watched", body: body)
                                message = "Watching \(body.text("owner"))/\(body.text("repo"))#\(body.int("number"))."
                                await model.refresh()
                            } catch { failed = true; message = error.localizedDescription }
                        }
                    }.disabled(busy || BoardActions.watchTarget(target) == nil)
                    if !message.isEmpty { Text(message).foregroundStyle(failed ? Crab.red : Crab.mint) }
                }
            }.navigationTitle("Watch PR").navigationBarTitleDisplayMode(.inline)
        }
    }
}
struct NotificationSettings: View {
    @State private var status = ""
    @State private var sound = Credentials.defaults.bool(forKey: "notificationSound")
    var body: some View {
        Form {
            Section("Questions and permissions") {
                Text(status).foregroundStyle(Crab.cyan)
                Toggle("Sound", isOn: $sound).onChange(of: sound) { _, value in Credentials.defaults.set(value, forKey: "notificationSound"); Task { await Notifications.shared.register() } }
                Button("Enable notifications") { Task { await Notifications.shared.requestPermission(); await refresh() } }
                Link("iOS notification settings", destination: URL(string: UIApplication.openNotificationSettingsURLString)!)
            }
        }.navigationTitle("Notifications").task { await refresh() }
    }
    func refresh() async { status = await Notifications.shared.status() }
}
struct PairingFields: View {
    @State private var demo = false
    @ObservedObject var model: SessionModel
    @State private var code = ""
    @State private var server = ""
    @State private var advanced = false
    var body: some View {
        VStack(spacing: 20) {
            Text("crabigator pair").font(.system(size: 18, design: .monospaced)).foregroundStyle(Crab.mint).padding(20).frame(maxWidth: .infinity, alignment: .leading).background(Crab.panel, in: RoundedRectangle(cornerRadius: 16)).textSelection(.enabled)
            TextField("ABC-DEF-GHI", text: $code).textInputAutocapitalization(.characters).autocorrectionDisabled().textFieldStyle(.roundedBorder).accessibilityLabel("Pairing code").accessibilityIdentifier("Pairing code")
            Button(model.loading ? "Connecting…" : "Pair") { Task { await model.pair(code: code, origin: server.isEmpty ? model.origin : server) } }.buttonStyle(.borderedProminent).tint(Crab.orange).foregroundStyle(Crab.ink).frame(minHeight: 44).disabled(code.isEmpty || model.loading)
            Button("Try a demo") { demo = true }.sheet(isPresented: $demo) { DemoView() }
            Link("Privacy policy", destination: URL(string: "https://drinkcrabigator.com/privacy")!)
            Button("Server") { advanced.toggle() }.foregroundStyle(Crab.muted)
            if advanced { TextField("HTTPS server address", text: $server).keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled().textFieldStyle(.roundedBorder) }
        }.frame(maxWidth: 430)
    }
}
