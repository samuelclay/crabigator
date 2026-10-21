import SwiftUI

extension View {
    func nativePopover() -> some View { self.frame(width: 330, height: 480).presentationCompactAdaptation(.popover).preferredColorScheme(.dark) }
}
struct StyleMenu: View {
    @ObservedObject var model: SessionModel
    var standalone = true
    @State private var terminal = false
    var body: some View {
        Group {
            if standalone { NavigationStack { form } } else { form }
        }.tint(Crab.cyan)
    }
    var form: some View {
        VStack(spacing: 0) {
            Picker("Style applies to", selection: $terminal) {
                Text("List / PR board").tag(false)
                Text("Terminal").tag(true)
            }.pickerStyle(.segmented).padding(.horizontal, 16).padding(.vertical, 10).accessibilityIdentifier("Style scope")
            Form {
                if terminal {
                    Section("Text") {
                        Stepper("Text size: \(model.preferences.fontSize) pt", value: $model.preferences.fontSize, in: 9...24).accessibilityIdentifier("Terminal text size")
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Line spacing")
                            Picker("Line spacing", selection: $model.preferences.lineSpacing) { Text("Tight").tag(120); Text("Normal").tag(145); Text("Relaxed").tag(170) }.pickerStyle(.segmented).accessibilityIdentifier("Terminal line spacing")
                        }
                        Picker("Text wrap", selection: $model.preferences.wrap) { Text("Wrap").tag(true); Text("Horizontal scroll").tag(false) }.pickerStyle(.menu)
                    }
                    Section("Layout") {
                        Picker("Terminal height", selection: $model.preferences.terminalHeight) { Text("Full").tag(0); ForEach([250, 350, 500, 700], id: \.self) { Text("\($0)").tag($0) } }.pickerStyle(.menu)
                    }
                    Section("Widgets below the terminal") {
                        Toggle("Expanded", isOn: $model.preferences.widgets)
                        visibility([("recap", "Recap"), ("prs", "Pull requests"), ("commits", "Commits"), ("git", "Git status"), ("changes", "Changes")])
                    }
                } else {
                    Section("Text and spacing") {
                        Stepper("Text size: \(model.preferences.boardTextSize) pt", value: $model.preferences.boardTextSize, in: 10...24).accessibilityIdentifier("Board text size")
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Row spacing")
                            Picker("Row spacing", selection: $model.preferences.density) { Text("Tight").tag("compact"); Text("Normal").tag("comfortable"); Text("Relaxed").tag("relaxed") }.pickerStyle(.segmented).accessibilityIdentifier("Board spacing")
                            Text("Tight keeps titles on one line.").font(.caption).foregroundStyle(Crab.muted)
                        }
                    }
                    Section("Layout") {
                        Picker("List columns", selection: $model.preferences.columns) { Text("Fit").tag(0); ForEach(1...4, id: \.self) { Text("\($0)").tag($0) } }.pickerStyle(.menu)
                        Picker("Projects", selection: $model.preferences.grouping) { Text("Together").tag("all"); Text("Grouped").tag("project") }.pickerStyle(.menu)
                        if model.preferences.grouping == "project" { Picker("Project order", selection: $model.preferences.order) { Text("Most recent").tag("recent"); Text("Alphabetical").tag("alpha") }.pickerStyle(.menu) }
                    }
                }
            }.id(terminal)
        }.navigationTitle("Style").navigationBarTitleDisplayMode(.inline).onAppear { terminal = standalone }
    }
    func visibility(_ keys: [(String, String)]) -> some View { ForEach(keys, id: \.0) { key, label in Toggle(label, isOn: Binding(get: { model.preferences.visible(key) }, set: { _ in model.preferences.toggle(key) })) } }
}
struct SettingsMenu: View {
    @ObservedObject var model: SessionModel
    var search: () -> Void = {}
    var fullscreen: (() -> Void)?
    var expanded = false
    @State private var identities: [JSON] = []
    @State private var accountLoaded = false
    @State private var invite = ""
    @State private var error = ""
    @State private var busy = false
    @State private var confirm = false
    @Environment(\.openURL) private var openURL
    var body: some View {
        NavigationStack {
            Form {
                Section("List / PR board") {
                    Picker("Activity", selection: $model.preferences.boardAgeHours) {
                        ForEach([1, 3, 6, 9, 12, 24], id: \.self) { Text($0 == 1 ? "Last hour" : "Last \($0) hours").tag($0) }
                        Text("All ages").tag(0)
                    }.pickerStyle(.menu).accessibilityIdentifier("Activity cutoff")
                    Picker("Sessions", selection: $model.preferences.showEnded) {
                        Text("Live").tag(false); Text("Live and ended").tag(true)
                    }.pickerStyle(.menu)
                    Picker("Computers", selection: $model.preferences.boardComputer) {
                        Text("All computers").tag("")
                        ForEach(Array(Set(model.ordering.machines + [model.preferences.boardComputer])).filter { !$0.isEmpty }.sorted(), id: \.self) { Text($0).tag($0) }
                    }.pickerStyle(.menu)
                    Toggle("Show recaps", isOn: $model.preferences.boardRecaps)
                    Picker("PR board groups", selection: $model.preferences.boardGrouping) {
                        Text("Sessions").tag("sessions"); Text("Pull requests").tag("prs")
                    }.pickerStyle(.menu)
                }
                Section("Actions") {
                    Button("Search sessions", action: search)
                    NavigationLink("New session") { BoardActionForm(model: model, watch: false) }
                    NavigationLink("Watch PR") { BoardActionForm(model: model, watch: true) }
                    if let fullscreen { Button(expanded ? "Return to session" : "Expand List / PR board", action: fullscreen) }
                }
                Section {
                    NavigationLink("Style") { StyleMenu(model: model, standalone: false) }
                    NavigationLink("Notifications") { NotificationSettings() }
                }
                Section("Session list") {
                    Picker("Position", selection: $model.preferences.sidebarRight) { Text("Left").tag(false); Text("Right").tag(true) }.pickerStyle(.segmented)
                }
                Section("Visible stats") { StyleMenu(model: model).visibility([("sessionTime", "Session time"), ("thinkingTime", "Thinking time"), ("promptRecency", "Last prompt"), ("completionRecency", "Last completion"), ("tools", "Tools"), ("compactions", "Compactions")]) }
                Section("Sign-in") {
                    if !accountLoaded { ProgressView() }
                    else if identities.isEmpty { Text("Pairing code") }
                    ForEach(identities.indices, id: \.self) { i in Text(identities[i].text("email", identities[i].text("name", identities[i].text("provider")))) }
                    Link("Manage sign-in", destination: URL(string: model.origin + "/dashboard")!)
                }
                Section("Pair another device") {
                    if !invite.isEmpty { Text(invite).font(.title2.monospaced()).textSelection(.enabled).foregroundStyle(Crab.cyan) }
                    Button(busy ? "Generating…" : "Generate pairing code") {
                        busy = true
                        Task { defer { busy = false }; do { invite = try await model.api.call("/api/pairing/invite", body: [:]).text("code") } catch { self.error = error.localizedDescription } }
                    }.disabled(busy)
                }
                Section("MCP") { Text(model.origin + "/mcp").font(.caption).textSelection(.enabled); Link("Tools and examples", destination: URL(string: model.origin + "/mcp-tools")!) }
                Section("This device") {
                    Text(UIDevice.current.name); Text(model.origin).font(.caption).foregroundStyle(Crab.muted)
                    Text("Version \(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0")").font(.caption)
                    Button("Unpair this device", role: .destructive) { confirm = true }
                }
                if !error.isEmpty { Text(error).foregroundStyle(Crab.red) }
            }.accessibilityIdentifier("Settings form").navigationTitle("Settings").navigationBarTitleDisplayMode(.inline)
                .confirmationDialog("Unpair this device?", isPresented: $confirm, titleVisibility: .visible) { Button("Unpair", role: .destructive) { Task { await model.unpair() } } }
        }.tint(Crab.cyan).task {
            do { identities = try await model.api.call("/api/account").array("identities") } catch { self.error = error.localizedDescription }; accountLoaded = true
        }
    }
}
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

struct NewSessionForm: View {
    @ObservedObject var model: SessionModel
    @State private var computer = ""
    @State private var folder = ""
    @State private var customFolder = ""
    @State private var platform = "codex"
    private var busy: Bool { model.sessionLaunch.busy }
    private var message: String { model.sessionLaunch.message }
    private var failed: Bool { model.sessionLaunch.failed }
    private let assistants = [("Codex", "codex"), ("Claude", "claude"), ("OpenCode", "opencode"), ("Grok", "grok")]
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
    @ObservedObject var model: SessionModel
    @State private var code = ""
    @State private var server = ""
    @State private var advanced = false
    var body: some View {
        VStack(spacing: 20) {
            Text("crabigator pair").font(.system(size: 18, design: .monospaced)).foregroundStyle(Crab.mint).padding(20).frame(maxWidth: .infinity, alignment: .leading).background(Crab.panel, in: RoundedRectangle(cornerRadius: 16)).textSelection(.enabled)
            TextField("ABC-DEF-GHI", text: $code).textInputAutocapitalization(.characters).autocorrectionDisabled().textFieldStyle(.roundedBorder).accessibilityLabel("Pairing code").accessibilityIdentifier("Pairing code")
            Button(model.loading ? "Connecting…" : "Pair") { Task { await model.pair(code: code, origin: server.isEmpty ? model.origin : server) } }.buttonStyle(.borderedProminent).tint(Crab.orange).foregroundStyle(Crab.ink).frame(minHeight: 44).disabled(code.isEmpty || model.loading)
            Button("Server") { advanced.toggle() }.foregroundStyle(Crab.muted)
            if advanced { TextField("HTTPS server address", text: $server).keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled().textFieldStyle(.roundedBorder) }
        }.frame(maxWidth: 430)
    }
}
