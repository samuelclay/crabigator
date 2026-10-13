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
            }.navigationTitle("Settings").navigationBarTitleDisplayMode(.inline)
                .confirmationDialog("Unpair this device?", isPresented: $confirm, titleVisibility: .visible) { Button("Unpair", role: .destructive) { Task { await model.unpair() } } }
        }.tint(Crab.cyan).task {
            do { identities = try await model.api.call("/api/account").array("identities") } catch { self.error = error.localizedDescription }; accountLoaded = true
        }
    }
}
struct BoardActionForm: View {
    @ObservedObject var model: SessionModel
    let watch: Bool
    @State private var target = ""
    @State private var computer = ""
    @State private var cwd = ""
    @State private var platform = "codex"
    @State private var busy = false
    @State private var message = ""
    @State private var failed = false
    var desktops: [Session] {
        var seen = Set<String>()
        return model.sessions.filter { $0.active && seen.insert($0.machine).inserted }
    }
    var source: Session? { desktops.first { $0.machine == computer } }
    var body: some View {
        Form {
            if watch {
                Section {
                    TextField("GitHub URL or owner/repo#123", text: $target).textInputAutocapitalization(.never).autocorrectionDisabled().accessibilityIdentifier("Watch PR target")
                }
            } else {
                Section {
                    Picker("Computer", selection: $computer) { ForEach(desktops) { Text($0.machine).tag($0.machine) } }.pickerStyle(.menu)
                    TextField("/path/to/project", text: $cwd).textInputAutocapitalization(.never).autocorrectionDisabled().accessibilityLabel("Folder")
                    let folders = Array(Set(model.sessions.filter { $0.machine == computer && !$0.cwd.isEmpty }.map(\.cwd))).sorted()
                    if !folders.isEmpty {
                        Menu("Recent folders") { ForEach(folders, id: \.self) { folder in Button(folder) { cwd = folder } } }
                    }
                    Picker("Assistant", selection: $platform) {
                        Text("Claude").tag("claude"); Text("Codex").tag("codex"); Text("opencode").tag("opencode"); Text("Grok").tag("grok")
                    }.pickerStyle(.menu)
                    if desktops.isEmpty { Text("Open Crabigator on a desktop to start a session.").foregroundStyle(Crab.muted) }
                }
            }
            Section {
                Button(busy ? "Sending…" : watch ? "Watch PR" : "Start session") {
                    busy = true; message = ""; failed = false
                    Task {
                        defer { busy = false }
                        do {
                            if watch {
                                guard let body = BoardActions.watchTarget(target) else { return }
                                _ = try await model.api.call("/api/prs/watched", body: body)
                                message = "Watching \(body.text("owner"))/\(body.text("repo"))#\(body.int("number"))."
                            } else {
                                guard let source else { return }
                                _ = try await model.api.call("/api/sessions/\(source.id)/spawn", body: ["cwd": cwd.trimmingCharacters(in: .whitespacesAndNewlines), "platform": platform])
                                message = "Requested on \(computer). The session will appear when it connects."
                            }
                            await model.refresh()
                        } catch { failed = true; message = error.localizedDescription }
                    }
                }.disabled(busy || (watch ? BoardActions.watchTarget(target) == nil : source == nil || !cwd.trimmingCharacters(in: .whitespacesAndNewlines).hasPrefix("/")))
                if !message.isEmpty { Text(message).foregroundStyle(failed ? Crab.red : Crab.mint) }
            }
        }.navigationTitle(watch ? "Watch PR" : "New session").navigationBarTitleDisplayMode(.inline)
            .onAppear { if computer.isEmpty { computer = desktops.first?.machine ?? ""; cwd = source?.cwd ?? "" } }
            .onChange(of: computer) { _, _ in cwd = source?.cwd ?? "" }
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
