import Foundation
import UserNotifications

/// Shared by the app and service extension. Notification actions always re-fetch state.
enum NotificationContent {
    static let category = "CRABIGATOR_PROMPT"
    /// Per-prompt categories carry the prompt's real choice labels. They reuse
    /// numbered slots, so registered categories do not pile up.
    static let slotPrefix = "CRABIGATOR_PROMPT_SLOT_"
    static let slotCount = 24
    /// iOS 27 shows at most ten actions under an expanded notification
    /// (measured on iPhone SE with sixteen registered).
    static let actionLimit = 10
    /// Longer requests do not fit a notification; they must be reviewed in the app.
    static let detailLimit = 2000

    static func prompt(_ snapshot: JSON) -> JSON { snapshot["prompt"] as? JSON ?? ["prompt_type": "text", "state": snapshot.text("state")] }
    static func content(_ snapshot: JSON, id: String) -> UNMutableNotificationContent {
        let content = UNMutableNotificationContent()
        content.title = snapshot.text("title", "Crabigator")
        let prompt = prompt(snapshot), questions = prompt.array("questions")
        if questions.count > 1 && prompt["review"] == nil { content.subtitle = "Question \(prompt.int("current_question") + 1) of \(questions.count)" }
        content.body = body(prompt)
        content.categoryIdentifier = categoryID(prompt)
        content.threadIdentifier = id
        content.userInfo = ["session_id": id, "revision": snapshot.int("prompt_revision")]
        content.sound = Credentials.defaults.bool(forKey: "notificationSound") ? .default : nil
        content.interruptionLevel = .active
        return content
    }
    /// The question first, then every choice on one line, so a collapsed banner keeps both.
    static func body(_ prompt: JSON) -> String {
        let details = PromptActions.details(prompt)
        let labels = PromptActions.options(prompt).map { $0.text("label") }.filter { !$0.isEmpty }
        return ([PromptActions.title(prompt)] + (details.isEmpty ? [] : [String(details.prefix(detailLimit))]) + (labels.isEmpty ? [] : [labels.joined(separator: " · ")])).joined(separator: "\n")
    }
    static func quickAnswers(_ prompt: JSON) -> Bool { PromptActions.details(prompt).count <= detailLimit }

    // MARK: Actions

    /// Buttons named after the prompt's choices. Each identifier carries the
    /// session and revision it was made for, so a button can never answer another prompt.
    static func actions(_ prompt: JSON, id: String, revision: Int) -> [UNNotificationAction] {
        guard quickAnswers(prompt) else { return [] }
        let token = "@\(revision)@\(id)", reply = PromptActions.allowsText(prompt)
        // A tap on one checkbox would only tick it. The expanded view has Submit.
        let multi = prompt["review"] == nil && PromptActions.question(prompt)?.bool("multi_select") == true
        var actions: [UNNotificationAction] = multi ? [] : PromptActions.options(prompt).prefix(actionLimit - (reply ? 1 : 0)).enumerated().map {
            UNNotificationAction(identifier: "option_\($0.offset)" + token, title: $0.element.text("label", "Choice \($0.offset + 1)"), options: [.authenticationRequired])
        }
        if reply { actions.append(UNTextInputNotificationAction(identifier: "reply" + token, title: "Reply", options: [.authenticationRequired], textInputButtonTitle: "Send", textInputPlaceholder: "Your answer")) }
        return actions
    }
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

    /// Generic fallback categories, used when no per-prompt slot is free.
    static func categoryID(_ prompt: JSON) -> String {
        let freeText = PromptActions.allowsText(prompt)
        let count = quickAnswers(prompt) ? min(freeText ? 2 : 3, PromptActions.options(prompt).count) : 0
        return "\(category)_\(count)_\(freeText ? 1 : 0)"
    }
    static func categories() -> Set<UNNotificationCategory> {
        var result = Set<UNNotificationCategory>()
        for count in 0...3 { for freeText in [false, true] {
            var actions: [UNNotificationAction] = (0..<count).map { UNNotificationAction(identifier: "option_\($0)", title: "Choose \($0 + 1)", options: [.authenticationRequired]) }
            if freeText { actions.append(UNTextInputNotificationAction(identifier: "reply", title: "Reply", options: [.authenticationRequired], textInputButtonTitle: "Send", textInputPlaceholder: "Your answer")) }
            result.insert(category("\(category)_\(count)_\(freeText ? 1 : 0)", actions))
        } }
        return result
    }
    private static func category(_ identifier: String, _ actions: [UNNotificationAction]) -> UNNotificationCategory {
        UNNotificationCategory(identifier: identifier, actions: actions, intentIdentifiers: [], hiddenPreviewsBodyPlaceholder: "A session needs your attention", options: [.customDismissAction])
    }
    static func slot(_ category: String) -> Int? {
        guard category.hasPrefix(slotPrefix), let slot = Int(category.dropFirst(slotPrefix.count)), (0..<slotCount).contains(slot) else { return nil }
        return slot
    }

    struct Claim: Codable, Equatable {
        var session: String
        var time: TimeInterval
    }
    /// How long a claimed slot stays reserved before its notification appears.
    static let claimLifetime: TimeInterval = 120
    /// Picks a slot that no other session's notification shows or has just claimed.
    /// `shown` maps slots on screen to their sessions. Returns nil when all are busy.
    static func claim(_ session: String, shown: [Int: String], claims: [Int: Claim], now: TimeInterval) -> (slot: Int?, claims: [Int: Claim]) {
        let own = shown.first { $0.value == session }?.key ?? claims.first { $0.value.session == session }?.key
        var others = claims.filter { $0.value.session != session && now - $0.value.time < claimLifetime }
        func free(_ slot: Int) -> Bool { (shown[slot] ?? session) == session && others[slot] == nil }
        guard let slot = own.flatMap({ free($0) ? $0 : nil }) ?? (0..<slotCount).first(where: free) else { return (nil, others) }
        others[slot] = Claim(session: session, time: now)
        return (slot, others)
    }

    /// Gives `content` a category whose buttons use this prompt's real labels.
    /// Keeps the generic category when every slot is busy or registration fails.
    static func install(_ content: UNMutableNotificationContent, prompt: JSON, id: String) async {
        await serial.run {
            let center = UNUserNotificationCenter.current()
            var shown: [Int: String] = [:]
            for note in await center.deliveredNotifications() {
                if let slot = slot(note.request.content.categoryIdentifier) { shown[slot] = note.request.content.userInfo["session_id"] as? String ?? "" }
            }
            let key = "notificationSlots", saved = Credentials.defaults.data(forKey: key).flatMap { try? JSONDecoder().decode([Int: Claim].self, from: $0) } ?? [:]
            let result = claim(id, shown: shown, claims: saved, now: Date().timeIntervalSince1970)
            if let data = try? JSONEncoder().encode(result.claims) { Credentials.defaults.set(data, forKey: key) }
            guard let slot = result.slot else { return }
            let identifier = slotPrefix + String(slot), revision = content.userInfo["revision"] as? Int ?? 0
            await register(category(identifier, actions(prompt, id: id, revision: revision)), keeping: Set(shown.keys).union(result.claims.keys))
            content.categoryIdentifier = identifier
        }
    }
    /// Replaces the generic categories and adds `adding`. Slot categories outside
    /// `keeping` are pruned; nil keeps them all.
    static func register(_ adding: UNNotificationCategory? = nil, keeping: Set<Int>? = nil) async {
        let center = UNUserNotificationCenter.current()
        var result: [String: UNNotificationCategory] = [:]
        for existing in await center.notificationCategories() {
            if let slot = slot(existing.identifier), keeping?.contains(slot) ?? true { result[existing.identifier] = existing }
        }
        for fixed in categories() { result[fixed.identifier] = fixed }
        if let adding { result[adding.identifier] = adding }
        center.setNotificationCategories(Set(result.values))
        // Registration is asynchronous. Reading it back waits until it is saved,
        // so the notification appears with its new buttons rather than the old ones.
        _ = await center.notificationCategories()
    }
    private static let serial = SerialWork()

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
    static func send(_ id: String, revision: Int, option: Int? = nil, text: String? = nil) async throws {
        let snap = try await API.shared.snapshot(id)
        guard snap.bool("attention_pending"), snap.bool("desktop_connected"), snap.int("prompt_revision") == revision else {
            throw APIError(status: 409, message: "This question has changed or was already answered.")
        }
        let action = try PromptActions.action(NotificationContent.prompt(snap), option: option, text: text)
        try await API.shared.action(id, action, revision: revision)
    }
}

/// Runs async work one piece at a time, in the order it was started.
final class SerialWork {
    private let lock = NSLock()
    private var last: Task<Void, Never>?
    func run(_ work: @escaping () async -> Void) async {
        lock.lock()
        let previous = last
        let task = Task { await previous?.value; await work() }
        last = task
        lock.unlock()
        await task.value
    }
}
