import SwiftUI
import OSLog

@MainActor final class SessionStream: ObservableObject {
    @Published var selected: Session?
    @Published var screen = ""
    @Published var history = ""
    @Published var screenLoaded = false
    @Published var historyLoaded = false
    @Published var connected = false
    @Published var desktopStatusKnown = false
    @Published var sending = false
    @Published var prompt: JSON?
    @Published var revision: Int?
    @Published var details: [String: JSON] = [:]
    @Published var error: String?
    private var task: Task<Void, Never>?, heartbeat: Task<Void, Never>?, promptTask: Task<Void, Never>?
    private var socket: URLSessionWebSocketTask?
    private var terminalTask: Task<Void, Never>?, screenWatchdog: Task<Void, Never>?
    private var socketScreenReceived = false, socketHistoryReceived = false
    private var historyRevision = 0
    private let log = Logger(subsystem: "com.newsblur.crabigator", category: "TerminalStream")
    private var generation = 0, snapshotRequest = 0
    private let api = API.shared
    var foreground = false

    func select(_ session: Session) {
        if selected?.id == session.id { return }
        stop(); selected = session; screen = ""; history = ""; screenLoaded = false; historyLoaded = false
        prompt = nil; details = [:]; error = nil; sending = false
        if foreground { start() }
    }
    func close() { stop(); selected = nil; screen = ""; history = ""; details = [:]; prompt = nil; sending = false }
    func stop() {
        generation += 1; snapshotRequest += 1; task?.cancel(); task = nil; heartbeat?.cancel(); heartbeat = nil
        promptTask?.cancel(); promptTask = nil
        terminalTask?.cancel(); terminalTask = nil; screenWatchdog?.cancel(); screenWatchdog = nil
        socket?.cancel(with: .goingAway, reason: nil); socket = nil; connected = false; desktopStatusKnown = false; revision = nil
    }
    func start() {
        #if DEBUG
        // UI regression tests hold the real loading page without changing saved credentials.
        if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_HOLD_SESSION_LOADING"] == "1" { return }
        #endif
        guard foreground, let id = selected?.id, task == nil else { return }
        let epoch = generation
        task = Task {
            while !Task.isCancelled && epoch == generation {
                do {
                    let ws = api.session.webSocketTask(with: try api.request("/api/sessions/\(id)/events"))
                    // The server caps history at 500K characters; JSON escaping can multiply its byte size.
                    ws.maximumMessageSize = 8 * 1024 * 1024
                    socket = ws; socketScreenReceived = false; socketHistoryReceived = false
                    #if DEBUG
                    if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_STALL_TERMINAL_SOCKET"] != "1" { ws.resume() }
                    #else
                    ws.resume()
                    #endif
                    log.info("Connecting terminal session \(id, privacy: .public)")
                    if terminalTask == nil { terminalTask = Task { await recoverTerminal(id: id, epoch: epoch) } }
                    screenWatchdog?.cancel()
                    screenWatchdog = Task {
                        try? await Task.sleep(for: .seconds(10))
                        guard !Task.isCancelled, epoch == generation, socket === ws, !socketScreenReceived else { return }
                        log.notice("Restarting terminal socket without a screen for \(id, privacy: .public)")
                        ws.cancel(with: .goingAway, reason: nil)
                    }
                    // Cached terminal frames must not wait for the separate HTTP metadata request.
                    schedulePromptRefresh()
                    heartbeat?.cancel()
                    heartbeat = Task {
                        while !Task.isCancelled && epoch == generation {
                            _ = try? await api.call("/api/sessions/\(id)/viewer-active", body: [:])
                            ws.sendPing { error in
                                if error != nil { ws.cancel(with: .goingAway, reason: nil) }
                            }
                            try? await Task.sleep(for: .seconds(5))
                        }
                    }
                    while !Task.isCancelled && epoch == generation {
                        let message = try await ws.receive()
                        guard epoch == generation else { return }
                        let data: Data
                        switch message { case .string(let s): data = Data(s.utf8); case .data(let d): data = d; @unknown default: continue }
                        guard let event = try? JSONSerialization.jsonObject(with: data) as? JSON else { continue }
                        receive(event)
                    }
                } catch {
                    guard !Task.isCancelled, epoch == generation else { return }
                    log.error("Terminal socket failed for \(id, privacy: .public): \(error.localizedDescription, privacy: .public)")
                    screenWatchdog?.cancel(); screenWatchdog = nil
                    snapshotRequest += 1; promptTask?.cancel(); promptTask = nil
                    connected = false; desktopStatusKnown = false; revision = nil; heartbeat?.cancel(); socket?.cancel(with: .goingAway, reason: nil)
                    try? await Task.sleep(for: .seconds(3))
                }
            }
        }
    }
    private func recoverTerminal(id: String, epoch: Int) async {
        let started = Date()
        while !Task.isCancelled && epoch == generation && !socketScreenReceived {
            do {
                let previousHistoryRevision = historyRevision
                let snapshot = try await api.terminalSnapshot(id)
                guard !Task.isCancelled, epoch == generation else { return }
                // A late HTTP response must never replace newer WebSocket output.
                if !socketScreenReceived, let content = snapshot["screen"] as? String {
                    if !screenLoaded { log.info("Cached terminal loaded for \(id, privacy: .public) after \(Date().timeIntervalSince(started)) seconds") }
                    screen = content; screenLoaded = true
                }
                if !socketHistoryReceived, historyRevision == previousHistoryRevision, let content = snapshot["scrollback"] as? String {
                    history = String(content.suffix(250000)); historyLoaded = true
                }
            } catch {
                guard !Task.isCancelled else { return }
                log.error("Terminal cache request failed for \(id, privacy: .public): \(error.localizedDescription, privacy: .public)")
            }
            try? await Task.sleep(for: .seconds(2))
        }
    }
    func receive(_ e: JSON) {
        switch e.text("type") {
        case "screen":
            if !socketScreenReceived { log.info("Received terminal screen for \(self.selected?.id ?? "", privacy: .public)") }
            socketScreenReceived = true; terminalTask?.cancel(); terminalTask = nil
            screenWatchdog?.cancel(); screenWatchdog = nil
            screen = e.text("content"); screenLoaded = true
        case "scrollback_history": historyRevision += 1; socketHistoryReceived = true; history = String(e.text("content").suffix(250000)); historyLoaded = true
        case "scrollback": historyRevision += 1; socketHistoryReceived = true; history = String((history + e.text("diff")).suffix(250000)); historyLoaded = true
        case "desktop_status":
            connected = e.bool("connected"); desktopStatusKnown = true
            if !connected { prompt = nil }
        case "prompt": prompt = e["prompt"] as? JSON; revision = nil
        case "state": selected?.state = e.text("state"); revision = nil; if !["question", "permission"].contains(e.text("state")) { prompt = nil }
        case "git", "changes", "recap", "recap_history", "prs", "commit_history", "stats": details[e.text("type")] = e
        default: break
        }
        if ["prompt", "state", "desktop_status"].contains(e.text("type")) { schedulePromptRefresh(reconcile: true) }
    }
    private func schedulePromptRefresh(reconcile: Bool = false) {
        promptTask?.cancel()
        let id = selected?.id
        promptTask = Task {
            await refreshPrompt()
            if !Task.isCancelled, reconcile, let id { await Notifications.shared.reconcile(id: id) }
        }
    }
    func refreshPrompt() async {
        guard !Task.isCancelled, let id = selected?.id else { return }
        snapshotRequest += 1; let request = snapshotRequest, epoch = generation
        do {
            #if DEBUG
            // Exercise a stalled metadata request without delaying the terminal socket.
            if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_SLOW_PROMPT"] == "1" {
                try await Task.sleep(for: .seconds(15))
            }
            #endif
            let snap = try await api.snapshot(id)
            guard !Task.isCancelled, request == snapshotRequest, epoch == generation, id == selected?.id else { return }
            prompt = NotificationContent.answerable(snap) ? snap["prompt"] as? JSON : nil
            revision = snap["prompt_revision"] as? Int; connected = snap.bool("desktop_connected"); desktopStatusKnown = true
        } catch { /* Screen streaming still works with servers without the mobile snapshot route. */ }
    }
    func send(_ action: TerminalAction, guarded: Bool = false, expectedRevision: Int? = nil, onSent: @escaping () -> Void = {}) {
        guard let id = selected?.id, connected, !sending else { return }
        if guarded && (expectedRevision == nil || expectedRevision != revision) { error = "This question changed. Please review it again."; Task { await refreshPrompt() }; return }
        let epoch = generation; sending = true; error = nil
        Task {
            defer { if epoch == generation { sending = false } }
            do {
                try await api.action(id, action, revision: guarded ? expectedRevision : nil)
                if guarded { await Notifications.shared.reconcile(id: id) }
                if epoch == generation { onSent(); await refreshPrompt() }
            } catch {
                if epoch == generation { self.error = error.localizedDescription; if (error as? APIError)?.status == 409 { await refreshPrompt() } }
            }
        }
    }
}

@MainActor final class SessionModel: ObservableObject {
    struct SessionLaunch {
        var computer = ""
        var path = ""
        var platform = "codex"
        var busy = false
        var message = ""
        var failed = false
    }
    @Published var sessionLaunch = SessionLaunch()

    func startSession(on source: Session, path: String, platform: String) {
        guard !sessionLaunch.busy else { return }
        sessionLaunch = SessionLaunch(computer: source.machine, path: path, platform: platform, busy: true)
        Task {
            do {
                _ = try await api.call("/api/sessions/\(source.id)/spawn", body: ["cwd": path, "platform": platform])
                sessionLaunch.busy = false
                sessionLaunch.message = "Requested \(platform) in \(path) on \(source.machine). Waiting for it to connect."
                await refresh()
            } catch {
                sessionLaunch.busy = false
                sessionLaunch.failed = true
                sessionLaunch.message = "\(platform) in \(path) on \(source.machine): \(error.localizedDescription) Check the desktop before trying again."
            }
        }
    }

    enum RosterStatus { case loading, ready, failed }
    @Published var paired = !Credentials.shared.connection().token.isEmpty
    @Published var sessions: [Session] = []
    @Published var prs: [PullRequest] = []
    @Published var rosterStatus = RosterStatus.loading
    @Published var loading = false
    @Published var error: String?
    @Published var preferences = Preferences.load() { didSet { preferences.save(); updateOrder() } }
    @Published var layoutVersion = 0
    @Published var selectionID: String?
    @Published var navigation = NavigationState.load() { didSet { navigation.save() } }
    @Published var pageDirection = 1
    let stream = SessionStream(), ordering = RosterOrder(), api = API.shared
    private var polling: Task<Void, Never>?
    private var epoch = 0, foreground = false
    private var restoreSelection = true
    private var selectionEpoch = 0
    var drafts: [String: String] = [:]
    var origin: String { api.credentials.connection().origin }
    func start() {
        #if DEBUG
        if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_CONDUCTOR_ROSTER"] == "1" { return }
        #endif
        foreground = true; stream.foreground = true
        guard paired, polling == nil else { return }
        polling = Task {
            while !Task.isCancelled {
                await refresh()
                await stream.refreshPrompt()
                await Notifications.shared.reconcile()
                try? await Task.sleep(for: .seconds(12))
            }
        }
        stream.start(); Task { await Notifications.shared.register(); await Notifications.shared.reconcile() }
    }
    func stop() { foreground = false; stream.foreground = false; epoch += 1; polling?.cancel(); polling = nil; stream.stop() }
    func refresh() async {
        let generation = epoch
        if rosterStatus != .ready { rosterStatus = .loading }
        do {
            let result = try await api.call("/api/prs/board")
            guard !Task.isCancelled, generation == epoch else { return }
            prs = result.array("prs").map(PullRequest.init)
            var seen = Set<String>()
            sessions = (result.array("sessions").map(Session.init) + prs.flatMap(\.sessions)).filter { !$0.id.isEmpty && seen.insert($0.id).inserted }
            rosterStatus = .ready; error = nil; updateOrder()
            if restoreSelection {
                restoreSelection = false
                if let id = navigation.selectedID {
                    if let session = sessions.first(where: { $0.id == id }) {
                        // Keep the saved full-width/sidebar choice during restoration.
                        selectionID = id; stream.select(session)
                    } else {
                        navigation.selectedID = nil; navigation.expanded = true
                    }
                }
            }
        } catch {
            guard !Task.isCancelled, generation == epoch else { return }
            self.error = error.localizedDescription; if rosterStatus != .ready { rosterStatus = .failed }
        }
    }
    private func updateOrder() { ordering.update(sessions, prs: prs, prefs: preferences); layoutVersion += 1 }
    func select(_ session: Session, order: [Session]) {
        selectionEpoch += 1
        restoreSelection = false
        navigation.selectedID = session.id; navigation.expanded = false
        pageDirection = (order.firstIndex(where: { $0.id == session.id }) ?? 0) < (order.firstIndex(where: { $0.id == selectionID }) ?? 0) ? -1 : 1
        selectionID = session.id; stream.select(session)
    }
    func open(_ id: String) async {
        guard paired else { return }
        navigation.conductor = false
        restoreSelection = false; selectionEpoch += 1
        let request = selectionEpoch
        await refresh()
        guard request == selectionEpoch, foreground, !Task.isCancelled else { return }
        if let session = sessions.first(where: { $0.id == id }) { select(session, order: sessions) }
    }
    func close() { selectionEpoch += 1; restoreSelection = false; navigation.selectedID = nil; selectionID = nil; stream.close() }
    func pair(code: String, origin: String) async {
        guard !loading else { return }; loading = true; error = nil; defer { loading = false }
        do {
            try await api.pair(code, origin: origin, name: UIDevice.current.name)
            let resume = foreground; stop(); close(); sessions = []; prs = []; rosterStatus = .loading; paired = true
            if resume { start() }
        } catch { self.error = error.localizedDescription }
    }
    func unpair() async {
        do { _ = try await api.call("/api/account/logout", body: [:]); stop(); close(); api.credentials.clear(); paired = false; sessions = []; prs = []; drafts = [:]; Notifications.shared.clear() }
        catch { self.error = error.localizedDescription }
    }
}
