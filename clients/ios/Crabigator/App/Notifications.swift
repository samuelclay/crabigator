import UIKit
import UserNotifications

@MainActor final class Notifications {
    static let shared = Notifications()
    private let api = API.shared
    private var generation = 0
    private var dirty = Set<String>()
    private var tasks: [String: Task<Void, Never>] = [:]
    // Keeps the per-prompt categories that delivered notifications still use.
    func installCategories() async { await NotificationContent.register() }
    func requestPermission() async {
        do { _ = try await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]); UIApplication.shared.registerForRemoteNotifications(); await register() }
        catch { Credentials.defaults.set(error.localizedDescription, forKey: "pushError") }
    }
    func status() async -> String {
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        if settings.authorizationStatus == .denied || settings.authorizationStatus == .notDetermined { return "Notifications off" }
        if let error = Credentials.defaults.string(forKey: "pushError"), !error.isEmpty { return error }
        return Credentials.defaults.bool(forKey: "pushConfigured") ? "Notifications on" : "Push not configured"
    }
    func register() async {
        guard !api.credentials.connection().token.isEmpty, let token = Credentials.defaults.string(forKey: "apnsToken") else { return }
        do {
            let result = try await api.call("/api/mobile/push", body: ["platform": "ios", "token": token, "environment": Self.pushEnvironment, "sound": Credentials.defaults.bool(forKey: "notificationSound")])
            Credentials.defaults.set(result.bool("configured"), forKey: "pushConfigured"); Credentials.defaults.removeObject(forKey: "pushError")
        } catch { Credentials.defaults.set(error.localizedDescription, forKey: "pushError") }
    }
    func reconcile(id: String? = nil) async {
        guard !api.credentials.connection().token.isEmpty else { clear(); return }
        let epoch = generation
        if let id { await sync(id); return }
        let ids = Set(await UNUserNotificationCenter.current().deliveredNotifications().compactMap { $0.request.content.userInfo["session_id"] as? String })
        for id in ids where !id.isEmpty {
            guard epoch == generation else { return }
            await sync(id)
        }
    }
    // Release builds can also be development-signed. The profile determines APNs,
    // not the compiler's optimization configuration. Store builds use production.
    static var pushEnvironment: String {
        if let url = Bundle.main.url(forResource: "embedded", withExtension: "mobileprovision"),
           let data = try? Data(contentsOf: url),
           let start = data.range(of: Data("<?xml".utf8)),
           let end = data.range(of: Data("</plist>".utf8), in: start.lowerBound..<data.endIndex),
           let profile = try? PropertyListSerialization.propertyList(from: data[start.lowerBound..<end.upperBound], format: nil) as? JSON {
            return profile.object("Entitlements").text("aps-environment") == "development" ? "sandbox" : "production"
        }
        #if targetEnvironment(simulator)
        return "sandbox"
        #else
        return "production"
        #endif
    }
    private func sync(_ id: String) async {
        // A hint received during a snapshot must trigger another read after that snapshot.
        // Merely awaiting the first read could leave an answered prompt on the lock screen.
        dirty.insert(id)
        if let task = tasks[id] { await task.value; return }
        let epoch = generation
        let task = Task {
            defer { if epoch == generation { tasks[id] = nil } }
            while epoch == generation && !Task.isCancelled && dirty.remove(id) != nil {
                await syncCurrent(id, generation: epoch)
            }
        }
        tasks[id] = task; await task.value
    }
    private func syncCurrent(_ id: String, generation epoch: Int) async {
        do {
            let snap = try await api.snapshot(id)
            guard epoch == generation, !Task.isCancelled else { return }
            guard snap.bool("attention_pending"), snap.bool("desktop_connected") else { await NotificationContent.remove(id: id); return }
            let revision = snap.int("prompt_revision")
            let delivered = await UNUserNotificationCenter.current().deliveredNotifications()
            guard epoch == generation, !Task.isCancelled else { return }
            if delivered.contains(where: { $0.request.content.userInfo["session_id"] as? String == id && $0.request.content.userInfo["revision"] as? Int == revision }) { return }
            // APNs and its service extension own new alert presentation. A local
            // replacement here could race the remote alert and show two banners.
            await NotificationContent.remove(id: id)
        } catch {
            guard epoch == generation, !Task.isCancelled else { return }
            if let error = error as? APIError, [401, 403, 404].contains(error.status) { await NotificationContent.remove(id: id) }
        }
    }
    func respond(_ response: UNNotificationResponse, id: String) async {
        let epoch = generation
        guard let choice = NotificationContent.choice(response.actionIdentifier, id: id),
              let revision = choice.revision ?? response.notification.request.content.userInfo["revision"] as? Int else { await reconcile(id: id); return }
        do {
            try await PromptAnswer.send(id, revision: revision, option: choice.option, text: (response as? UNTextInputNotificationResponse)?.userText)
            guard epoch == generation else { return }
            await NotificationContent.remove(id: id); await reconcile(id: id)
        } catch {
            guard epoch == generation else { return }
            if (error as? APIError)?.status == 409 { await reconcile(id: id); return }
            let content = UNMutableNotificationContent(); content.title = "Answer was not sent"; content.body = "Open the session to review before trying again."; content.userInfo = ["session_id": id]; content.threadIdentifier = id
            try? await UNUserNotificationCenter.current().add(UNNotificationRequest(identifier: id, content: content, trigger: nil))
        }
    }
    func clear() {
        generation += 1; dirty.removeAll()
        tasks.values.forEach { $0.cancel() }; tasks.removeAll()
        UNUserNotificationCenter.current().removeAllDeliveredNotifications()
        UNUserNotificationCenter.current().removeAllPendingNotificationRequests()
    }
}
