import Foundation
import UserNotifications

/// Shared by the app and service extension. Notification actions always re-fetch state.
enum NotificationContent {
    static let category = "CRABIGATOR_PROMPT"
    static func content(_ snapshot: JSON, id: String) -> UNMutableNotificationContent {
        let content = UNMutableNotificationContent()
        content.title = snapshot.text("title", "Crabigator")
        let prompt = snapshot["prompt"] as? JSON ?? ["prompt_type": "text", "state": snapshot.text("state")]
        let options = PromptActions.options(prompt)
        let details = PromptActions.details(prompt)
        content.body = ([PromptActions.title(prompt)] + (details.isEmpty ? [] : [String(details.prefix(2000))]) + options.prefix(3).enumerated().map { "\($0.offset + 1). \($0.element.text("label"))" }).joined(separator: "\n")
        content.categoryIdentifier = categoryID(prompt)
        content.threadIdentifier = id
        content.userInfo = ["session_id": id, "revision": snapshot.int("prompt_revision")]
        content.sound = Credentials.defaults.bool(forKey: "notificationSound") ? .default : nil
        content.interruptionLevel = .active
        return content
    }
    static func categoryID(_ prompt: JSON) -> String {
        let freeText = PromptActions.allowsText(prompt)
        let count = PromptActions.details(prompt).count > 2000 ? 0 : min(freeText ? 2 : 3, PromptActions.options(prompt).count)
        return "\(category)_\(count)_\(freeText ? 1 : 0)"
    }
    static func categories() -> Set<UNNotificationCategory> {
        var result = Set<UNNotificationCategory>()
        for count in 0...3 { for freeText in [false, true] {
            var actions: [UNNotificationAction] = (0..<count).map { UNNotificationAction(identifier: "option_\($0)", title: "Choose \($0 + 1)", options: [.authenticationRequired]) }
            if freeText { actions.append(UNTextInputNotificationAction(identifier: "reply", title: "Reply", options: [.authenticationRequired], textInputButtonTitle: "Send", textInputPlaceholder: "Your answer")) }
            result.insert(UNNotificationCategory(identifier: "\(category)_\(count)_\(freeText ? 1 : 0)", actions: actions, intentIdentifiers: [], hiddenPreviewsBodyPlaceholder: "A session needs your attention", options: [.customDismissAction]))
        } }
        return result
    }
    static func remove(id: String) async {
        let center = UNUserNotificationCenter.current()
        let shown = await center.deliveredNotifications()
        center.removeDeliveredNotifications(withIdentifiers: shown.filter { $0.request.content.userInfo["session_id"] as? String == id }.map { $0.request.identifier })
        center.removePendingNotificationRequests(withIdentifiers: [id])
    }
}
