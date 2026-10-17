import Foundation
import UserNotifications

/// Shared by the app and both notification extensions. Answers always re-fetch state.
enum NotificationContent {
    static let category = "CRABIGATOR_PROMPT"
    /// Keep old identifiers registered so already-delivered notifications also
    /// lose their system buttons when the app updates.
    static var extensionCategories: [String] {
        [category] + (0...3).flatMap { count in ["\(category)_\(count)_0", "\(category)_\(count)_1"] }
            + (0..<24).map { "CRABIGATOR_PROMPT_SLOT_" + String($0) }
    }
    static let detailLimit = 2000

    static func prompt(_ snapshot: JSON) -> JSON { snapshot["prompt"] as? JSON ?? ["prompt_type": "text", "state": snapshot.text("state")] }
    /// A prompt is waiting and its desktop can still receive the answer.
    static func answerable(_ snapshot: JSON) -> Bool { snapshot.bool("attention_pending") && snapshot.bool("desktop_connected") }
    static func content(_ snapshot: JSON, id: String) -> UNMutableNotificationContent {
        let content = UNMutableNotificationContent()
        content.title = snapshot.text("title", "Crabigator")
        let prompt = prompt(snapshot), questions = prompt.array("questions")
        if questions.count > 1 && prompt["review"] == nil { content.subtitle = "Question \(prompt.int("current_question") + 1) of \(questions.count)" }
        content.body = body(prompt)
        content.categoryIdentifier = category
        content.threadIdentifier = id
        var info: [String: Any] = ["session_id": id, "revision": snapshot.int("prompt_revision")]
        // The content extension draws this copy at once; it never leaves the device.
        if let data = try? JSONSerialization.data(withJSONObject: prompt), data.count <= 32_000 { info["prompt"] = String(decoding: data, as: UTF8.self) }
        content.userInfo = info
        content.sound = Credentials.defaults.bool(forKey: "notificationSound") ? .default : nil
        content.interruptionLevel = .active
        return content
    }
    /// The question first, then every choice on one line, so a collapsed banner keeps both.
    static func body(_ prompt: JSON) -> String {
        let details = PromptActions.details(prompt)
        let labels = PromptActions.options(prompt).map { $0.text("label") }.filter { !$0.isEmpty }
        return ([PromptActions.title(prompt)] + [String(details.prefix(detailLimit)), labels.joined(separator: " · ")].filter { !$0.isEmpty }).joined(separator: "\n")
    }
    static func quickAnswers(_ prompt: JSON) -> Bool { PromptActions.details(prompt).count <= detailLimit }

    // MARK: Actions

    /// Retained for actions on notifications delivered by older app versions.
    static let open = "open"
    struct Choice: Equatable {
        var option: Int?
        var revision: Int?
    }
    /// The answer an action stands for, or nil when it was made for another session.
    /// Fallback actions carry no revision; the notification's own revision applies.
    static func choice(_ identifier: String, id: String) -> Choice? {
        let parts = identifier.split(separator: "@", maxSplits: 2, omittingEmptySubsequences: false).map(String.init)
        var choice = Choice()
        if parts.count == 3 {
            guard parts[2] == id, let revision = Int(parts[1]) else { return nil }
            choice.revision = revision
        } else if parts.count != 1 { return nil }
        if parts[0] == "reply" { return choice }
        guard parts[0].hasPrefix("option_"), let option = Int(parts[0].dropFirst(7)), option >= 0 else { return nil }
        choice.option = option
        return choice
    }

    // MARK: Categories

    /// All interaction lives in the expanded card, including typed answers.
    /// Register no system actions, so they cannot appear before the card loads.
    static func categories() -> Set<UNNotificationCategory> {
        Set(extensionCategories.map {
            UNNotificationCategory(identifier: $0, actions: [], intentIdentifiers: [],
                hiddenPreviewsBodyPlaceholder: "A session needs your attention", options: [.customDismissAction])
        })
    }
    static func register() async {
        let center = UNUserNotificationCenter.current()
        center.setNotificationCategories(categories())
        _ = await center.notificationCategories()
    }

    static func remove(id: String) async {
        let center = UNUserNotificationCenter.current()
        let shown = await center.deliveredNotifications()
        center.removeDeliveredNotifications(withIdentifiers: shown.filter { $0.request.content.userInfo["session_id"] as? String == id }.map { $0.request.identifier })
        center.removePendingNotificationRequests(withIdentifiers: [id])
    }
}

enum PromptAnswer {
    /// Answers only the revision the person saw. A changed, answered or
    /// disconnected prompt throws 409 before anything reaches the terminal.
    static func send(_ id: String, revision: Int, option: Int? = nil, text: String? = nil, submit: Bool = false, toggling: [Int] = []) async throws {
        let snap = try await API.shared.snapshot(id)
        guard NotificationContent.answerable(snap), snap.int("prompt_revision") == revision else {
            throw APIError(status: 409, message: "This question has changed or was already answered.")
        }
        let action = try PromptActions.action(NotificationContent.prompt(snap), option: option, text: text, submit: submit, toggling: toggling)
        try await API.shared.action(id, action, revision: revision)
    }
}
