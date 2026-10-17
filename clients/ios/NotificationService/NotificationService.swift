import UserNotifications

final class NotificationService: UNNotificationServiceExtension {
    private var handler: ((UNNotificationContent) -> Void)?
    private var fallback: UNMutableNotificationContent?
    private var work: Task<Void, Never>?
    private let lock = NSLock()
    override func didReceive(_ request: UNNotificationRequest, withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void) {
        lock.lock(); handler = contentHandler; fallback = request.content.mutableCopy() as? UNMutableNotificationContent; fallback?.sound = nil; lock.unlock()
        work = Task {
            guard let id = request.content.userInfo["session_id"] as? String else { finish(fallback ?? UNMutableNotificationContent()); return }
            do {
                let snapshot = try await API.shared.snapshot(id)
                await NotificationContent.remove(id: id)
                if NotificationContent.answerable(snapshot) {
                    let content = NotificationContent.content(snapshot, id: id)
                    await NotificationContent.register()
                    finish(content)
                } else {
                    // iOS does not let a service extension suppress an alert already in transit.
                    // Make a stale arrival passive; the next foreground/background reconciliation removes it.
                    let content = UNMutableNotificationContent(); content.title = "Crabigator"; content.body = "This request has been resolved."; content.interruptionLevel = .passive; content.threadIdentifier = id; content.userInfo = ["session_id": id]
                    finish(content)
                }
            } catch { finish(fallback ?? UNMutableNotificationContent()) }
        }
    }
    override func serviceExtensionTimeWillExpire() { work?.cancel(); finish(fallback ?? UNMutableNotificationContent()) }
    private func finish(_ content: UNNotificationContent) {
        lock.lock(); let callback = handler; handler = nil; lock.unlock(); callback?(content)
    }
}
