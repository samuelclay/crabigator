import SwiftUI

extension View {
    func nativePopover() -> some View { self.frame(width: 330, height: 480).presentationCompactAdaptation(.popover).preferredColorScheme(.dark) }
}
struct StyleMenu: View {
    @ObservedObject var model: SessionModel
    var standalone = true
    var body: some View {
        Group {
            if standalone { NavigationStack { form } } else { form }
        }.tint(Crab.cyan)
    }
    var form: some View {
            Form {
                Section("List / PR board text size") { Stepper("\(model.preferences.boardTextSize) pt", value: $model.preferences.boardTextSize, in: 10...24).accessibilityIdentifier("Board text size") }
                Section("Terminal text size") { Stepper("\(model.preferences.fontSize) pt", value: $model.preferences.fontSize, in: 9...24).accessibilityIdentifier("Terminal text size") }
                Section("Line spacing") { Picker("Line spacing", selection: $model.preferences.lineSpacing) { Text("Tight").tag(120); Text("Normal").tag(145); Text("Relaxed").tag(170) }.pickerStyle(.segmented) }
                Section("Text wrap") { Picker("Text wrap", selection: $model.preferences.wrap) { Text("Wrap").tag(true); Text("Scroll").tag(false) }.pickerStyle(.segmented) }
                Section("Terminal height") { Picker("Height", selection: $model.preferences.terminalHeight) { Text("Full").tag(0); ForEach([250, 350, 500, 700], id: \.self) { Text("\($0)").tag($0) } }.pickerStyle(.menu) }
                Section("Widgets") { Toggle("Expanded", isOn: $model.preferences.widgets); visibility([("recap", "Recap"), ("prs", "Pull requests"), ("commits", "Commits"), ("git", "Git status"), ("changes", "Changes")]) }
                Section("Columns") { Picker("Columns", selection: $model.preferences.columns) { Text("Fit").tag(0); ForEach(1...4, id: \.self) { Text("\($0)").tag($0) } }.pickerStyle(.segmented) }
                Section("Projects within each machine") {
                    Picker("Grouping", selection: $model.preferences.grouping) { Text("Together").tag("all"); Text("Grouped").tag("project") }.pickerStyle(.segmented)
                    if model.preferences.grouping == "project" { Picker("Order", selection: $model.preferences.order) { Text("Most recent").tag("recent"); Text("Alphabetical").tag("alpha") }.pickerStyle(.segmented) }
                }
            }.navigationTitle("Style").navigationBarTitleDisplayMode(.inline)
    }
    func visibility(_ keys: [(String, String)]) -> some View { ForEach(keys, id: \.0) { key, label in Toggle(label, isOn: Binding(get: { model.preferences.visible(key) }, set: { _ in model.preferences.toggle(key) })) } }
}
struct SettingsMenu: View {
    @ObservedObject var model: SessionModel
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
                Section {
                    NavigationLink("Style") { StyleMenu(model: model, standalone: false) }
                    NavigationLink("Notifications") { NotificationSettings() }
                }
                Section("Session list") {
                    Toggle("Show ended sessions", isOn: $model.preferences.showEnded)
                    Picker("Position", selection: $model.preferences.sidebarRight) { Text("Left").tag(false); Text("Right").tag(true) }.pickerStyle(.segmented)
                    Picker("Density", selection: $model.preferences.density) { Text("Compact").tag("compact"); Text("Comfortable").tag("comfortable") }.pickerStyle(.segmented)
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
