import SwiftUI
import UserNotifications

@main struct CrabigatorApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var model = SessionModel()
    @Environment(\.scenePhase) private var phase
    var body: some Scene {
        WindowGroup {
            content.preferredColorScheme(.dark).tint(Crab.orange)
                .onAppear { delegate.model = model; model.start() }
                .onChange(of: phase) { _, phase in if phase == .active { model.start() } else { model.stop() } }
                .onOpenURL { url in
                    if url.scheme == "crabigator", url.host == "session", let id = url.pathComponents.dropFirst().first { Task { await model.open(id) } }
                    if url.scheme == "crabigator", url.host == "pair", let code = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems?.first(where: { $0.name == "code" })?.value { Task { await model.pair(code: code, origin: model.origin) } }
                }
        }
    }
    private var rootView: some View {
        let connection = Credentials.shared.connection()
        // A new account gets new view state, including Conductor's conversation and draft.
        return RootView(model: model).id(model.paired ? connection.origin + "|" + connection.token : "unpaired")
    }
    @ViewBuilder private var content: some View {
        #if DEBUG
        if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_CONDUCTOR"] == "1" {
            ConductorFixture(model: model)
        } else if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_VOICE"] == "1" {
            VoiceComposerFixture()
        } else if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_LINKS"] == "1" {
            TerminalLinksFixture()
        } else if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_MOMENTUM"] == "1" {
            TerminalMomentumFixture()
        } else if ProcessInfo.processInfo.environment["CRABIGATOR_TEST_MODULES"] == "1" {
            SessionModulesFixture()
        } else { rootView }
        #else
        rootView
        #endif
    }
}
final class AppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    weak var model: SessionModel?
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        UNUserNotificationCenter.current().delegate = self
        Task { @MainActor in
            await Notifications.shared.installCategories(); UIApplication.shared.registerForRemoteNotifications()
            #if DEBUG
            // Posts a prompt notification from a snapshot, or from a live session's
            // current one, through the service extension's code, for device checks.
            let env = ProcessInfo.processInfo.environment
            var snapshot = env["CRABIGATOR_TEST_NOTIFICATION"].flatMap { try? JSONSerialization.jsonObject(with: Data($0.utf8)) as? JSON }
            if let id = env["CRABIGATOR_TEST_NOTIFICATION_SESSION"] { snapshot = try? await API.shared.snapshot(id); snapshot?["id"] = id }
            if let snapshot {
                let id = snapshot.text("id", "fixture"), content = NotificationContent.content(snapshot, id: id)
                await NotificationContent.register()
                try? await UNUserNotificationCenter.current().add(UNNotificationRequest(identifier: id, content: content, trigger: UNTimeIntervalNotificationTrigger(timeInterval: 3, repeats: false)))
            }
            #endif
        }
        return true
    }
    func application(_ application: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data) {
        Credentials.defaults.set(deviceToken.map { String(format: "%02x", $0) }.joined(), forKey: "apnsToken")
        Task { await Notifications.shared.register() }
    }
    func application(_ application: UIApplication, didFailToRegisterForRemoteNotificationsWithError error: Error) { Credentials.defaults.set(error.localizedDescription, forKey: "pushError") }
    func application(_ application: UIApplication, didReceiveRemoteNotification userInfo: [AnyHashable: Any], fetchCompletionHandler completionHandler: @escaping (UIBackgroundFetchResult) -> Void) {
        Task { await Notifications.shared.reconcile(id: userInfo["session_id"] as? String); completionHandler(.newData) }
    }
    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification, withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        Task {
            let info = notification.request.content.userInfo
            if let id = info["session_id"] as? String, let snap = try? await API.shared.snapshot(id), !snap.bool("attention_pending") { completionHandler([]); return }
            completionHandler(Credentials.defaults.bool(forKey: "notificationSound") ? [.banner, .list, .sound] : [.banner, .list])
        }
    }
    func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse, withCompletionHandler completionHandler: @escaping () -> Void) {
        Task { @MainActor in
            defer { completionHandler() }
            guard let id = response.notification.request.content.userInfo["session_id"] as? String else { return }
            if [UNNotificationDefaultActionIdentifier, NotificationContent.open].contains(response.actionIdentifier) { await model?.open(id); return }
            if response.actionIdentifier == UNNotificationDismissActionIdentifier { return }
            await Notifications.shared.respond(response, id: id)
        }
    }
}
struct RootView: View {
    @StateObject private var conductor: ConductorModel
    @ObservedObject var model: SessionModel
    @State private var outgoing: SessionStream?
    @State private var pageOffset: CGFloat = 0
    @State private var outgoingOffset: CGFloat = 0
    @State private var navigation = 0
    init(model: SessionModel, conductor: ConductorModel? = nil) {
        self.model = model
        _conductor = StateObject(wrappedValue: conductor ?? ConductorModel())
    }
    var body: some View {
        VStack(spacing: 0) {
            if let error = model.error { HStack { Text(error).font(.caption); Spacer(); Button("Dismiss") { model.error = nil } }.padding(10).background(Crab.red.opacity(0.25)) }
            if !model.paired {
                ScrollView { VStack(alignment: .leading, spacing: 24) { BrandHeader(); PairingFields(model: model) }.frame(maxWidth: 430).padding(28).frame(maxWidth: .infinity) }.defaultScrollAnchor(.center)
            } else {
                GeometryReader { geometry in
                    let wide = geometry.size.width >= 720, width = wide ? geometry.size.width - 361 : geometry.size.width
                    if !wide {
                        PhoneNavigation(model: model, listMode: $model.navigation.listMode, expanded: $model.navigation.expanded)
                    } else {
                        ZStack(alignment: model.preferences.sidebarRight ? .trailing : .leading) {
                            HStack(spacing: 0) {
                                if !model.preferences.sidebarRight { Spacer().frame(width: 361) }
                                detail(wide: true, width: width).frame(maxWidth: .infinity)
                                if model.preferences.sidebarRight { Spacer().frame(width: 361) }
                            }
                            BoardView(model: model, listMode: $model.navigation.listMode, expanded: $model.navigation.expanded, wide: true) { session, order in select(session, order: order, width: width) }
                                .frame(width: model.navigation.expanded ? geometry.size.width : 360)
                                .animation(.easeInOut(duration: 0.36), value: model.navigation.expanded)
                        }.clipped()
                    }
                }
            }
        }.background(Crab.ink).foregroundStyle(Color(uiColor: TerminalText.rgb(0xE9EEF4)))
            .opacity(model.paired && model.navigation.conductor ? 0 : 1)
            .allowsHitTesting(!(model.paired && model.navigation.conductor))
            .accessibilityHidden(model.paired && model.navigation.conductor)
            .overlay { if model.paired && model.navigation.conductor { ConductorView(model: model, conductor: conductor) } }
    }
    @ViewBuilder func detail(wide: Bool, width: CGFloat) -> some View {
        ZStack {
            if model.selectionID == nil {
                VStack(spacing: 12) { Image(systemName: "terminal").font(.system(size: 40)).opacity(0.4); Text("Select a session").font(.system(size: 14)) }.foregroundStyle(Crab.muted).frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                SessionDetail(model: model, stream: model.stream, wide: wide, active: !wide || !model.navigation.expanded, close: close).offset(x: pageOffset)
                if let outgoing { SessionDetail(model: model, stream: outgoing, wide: wide, active: false, close: {}).allowsHitTesting(false).accessibilityHidden(true).offset(x: outgoingOffset) }
            }
        }.background(Crab.ink).clipped().accessibilityHidden(wide && model.navigation.expanded)
    }
    func close() { withAnimation(.easeInOut(duration: 0.32)) { outgoing = nil; model.close(); pageOffset = 0 } }
    func select(_ session: Session, order: [Session], width: CGFloat) {
        navigation += 1; let epoch = navigation
        if let old = model.stream.selected, old.id != session.id {
            let frozen = SessionStream(); frozen.selected = old; frozen.history = model.stream.history; frozen.screen = model.stream.screen; frozen.historyLoaded = true; frozen.screenLoaded = true; frozen.details = model.stream.details
            outgoing = frozen
        }
        model.select(session, order: order)
        if outgoing != nil {
            let distance = CGFloat(model.pageDirection) * width
            outgoingOffset = 0; pageOffset = distance
            DispatchQueue.main.async {
                withAnimation(.easeInOut(duration: 0.36)) { outgoingOffset = -distance; pageOffset = 0; model.navigation.expanded = false }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.38) { if navigation == epoch { outgoing = nil } }
            }
        } else { withAnimation(.easeInOut(duration: 0.36)) { model.navigation.expanded = false } }
    }
}
