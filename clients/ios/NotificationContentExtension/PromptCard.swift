import Security
import SwiftUI
import UserNotifications
import UserNotificationsUI

/// The expanded notification: every question and choice, answered in place
/// through the same revision-guarded path as the app.
@MainActor final class PromptCardModel: ObservableObject {
    enum Status: Equatable { case ready, sending(String), sent(String), failed(String), resolved }
    @Published private(set) var title = ""
    @Published private(set) var body = ""
    @Published private(set) var prompt: JSON = [:]
    @Published private(set) var status = Status.ready
    /// Ticked rows of the current multi-select question, sent together on Submit.
    @Published var selected = Set<Int>()
    /// Choices sent from this view, by question, for prompts with several questions.
    @Published private(set) var answered: [Int: String] = [:]
    private(set) var id = ""
    private(set) var revision = -1
    /// The revision of the last answer sent from this view, and whether it is being watched.
    private var lastSent: Int?
    private var watching: Task<Void, Never>?
    /// The extension context, read when needed; it is not set yet when the view loads.
    var context: () -> NSExtensionContext? = { nil }
    var changed: () -> Void = {}

    /// False when this extension cannot read the pairing credential; the app answers instead.
    var canAnswer: Bool { !API.shared.credentials.connection().token.isEmpty }
    var busy: Bool { switch status { case .sending, .sent: return true; default: return false } }
    var open: Bool { status != .resolved && !busy }
    var canSubmit: Bool { open && !selected.isEmpty }
    var current: Int { prompt.int("current_question") }

    func show(_ notification: UNNotification) {
        let content = notification.request.content, info = content.userInfo
        stopWatching()
        if id != info["session_id"] as? String {
            prompt = [:]; revision = -1; selected = []; answered = [:]; lastSent = nil
        }
        id = info["session_id"] as? String ?? ""
        status = .ready
        title = content.title; body = content.body
        if let text = info["prompt"] as? String, let prompt = try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? JSON {
            adopt(prompt, revision: info["revision"] as? Int ?? 0)
        } else { changed() }
        startWatching()
    }
    func startWatching() {
        guard watching == nil, !id.isEmpty else { return }
        watching = Task { await refresh(); if status != .resolved && !Task.isCancelled { await watch() } }
    }
    func stopWatching() { watching?.cancel(); watching = nil }
    func reply(_ text: String) {
        guard open, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        let toggling = PromptActions.question(prompt)?.bool("multi_select") == true
            ? selected.symmetricDifference(Self.checked(prompt)).sorted() : []
        answer(text, text: text, toggling: toggling)
    }

    func tap(question: Int? = nil, option: Int) {
        guard open, let label = label(option), question == nil || question == current else { return }
        if question != nil, PromptActions.question(prompt)?.bool("multi_select") == true {
            if !selected.insert(option).inserted { selected.remove(option) }
            changed(); return
        }
        answer(label, option: option)
    }
    func submit() {
        answer(selected.sorted().compactMap(label).joined(separator: ", "), submit: true, toggling: selected.symmetricDifference(Self.checked(prompt)).sorted())
    }
    /// An action button under the view. iOS has already asked for authentication.
    func respond(to response: UNNotificationResponse) {
        guard let choice = NotificationContent.choice(response.actionIdentifier, id: id) else { return }
        let text = (response as? UNTextInputNotificationResponse)?.userText
        let revision = choice.revision ?? response.notification.request.content.userInfo["revision"] as? Int ?? -1
        answer(text ?? choice.option.flatMap(label) ?? "your answer", option: choice.option, text: text, revision: revision, authenticated: true)
    }
    func openSession() { context()?.performNotificationDefaultAction() }

    private func label(_ option: Int) -> String? {
        let options = PromptActions.options(prompt)
        return options.indices.contains(option) ? options[option].text("label") : nil
    }

    private func answer(_ label: String, option: Int? = nil, text: String? = nil, submit: Bool = false, toggling: [Int] = [], revision: Int? = nil, authenticated: Bool = false) {
        guard !busy else { return }
        // Rows carry no system authentication, so a locked device opens the app instead.
        guard canAnswer, NotificationContent.quickAnswers(prompt), authenticated || Self.unlocked() else { openSession(); return }
        let id = self.id, revision = revision ?? self.revision
        let question = revision == self.revision && prompt["review"] == nil && !prompt.array("questions").isEmpty ? current : nil
        status = .sending(label); changed()
        Task {
            do {
                try await PromptAnswer.send(id, revision: revision, option: option, text: text, submit: submit, toggling: toggling)
                guard self.id == id else { return }
                // A newer notification may already show what came next.
                lastSent = revision
                guard self.revision == revision else { status = .ready; changed(); return }
                if let question { answered[question] = label }
                status = .sent(label); changed()
            } catch {
                guard self.id == id else { return }
                // Show the prompt as it is now, and say that the tap sent nothing.
                let stale = (error as? APIError)?.status == 409
                status = .failed(error.localizedDescription)
                if stale { await refresh() }
                guard self.id == id else { return }
                if status != .resolved { status = .failed(stale ? "The question changed, so nothing was sent." : error.localizedDescription) }
                changed()
            }
        }
    }
    /// Follows the session while the view is open: shows each newer prompt, such as
    /// the next question, and closes once the session has moved past this view's answer.
    private func watch() async {
        var quiet = 0
        while !Task.isCancelled {
            try? await Task.sleep(nanoseconds: 1_000_000_000)
            guard !Task.isCancelled else { return }
            if case .sending = status { continue }
            guard let snap = try? await API.shared.snapshot(id) else { continue }
            guard !Task.isCancelled else { return }
            if NotificationContent.answerable(snap) {
                quiet = 0
                adopt(NotificationContent.prompt(snap), revision: snap.int("prompt_revision"))
                continue
            }
            guard let sent = lastSent else {
                break
            }
            // An answered prompt keeps its revision until the terminal moves on, if it ever does.
            quiet += 1
            if snap.int("prompt_revision") > sent || quiet >= 5 { break }
        }
        guard !Task.isCancelled else { return }
        await resolve()
    }
    private func resolve() async {
        status = .resolved; context()?.notificationActions = []; changed()
        // Close first: removing the notification under an open view can bring back an older copy.
        context()?.dismissNotificationContentExtension()
        await NotificationContent.remove(id: id)
    }
    private func refresh() async {
        let id = self.id
        guard let snap = try? await API.shared.snapshot(id) else { return }
        guard !Task.isCancelled, self.id == id else { return }
        if NotificationContent.answerable(snap) {
            adopt(NotificationContent.prompt(snap), revision: snap.int("prompt_revision"))
        } else if !busy {
            await resolve()
        }
    }
    private func adopt(_ next: JSON, revision: Int) {
        // Revisions only grow, and a late notification can carry an older prompt.
        guard revision > self.revision || prompt.isEmpty else { return }
        func questions(_ p: JSON) -> [String] { p.array("questions").map { $0.text("question") } }
        let sameQuestions = questions(next) == questions(prompt)
        if !sameQuestions { answered = [:] }
        if !sameQuestions || next.int("current_question") != current || next["review"] != nil { selected = Self.checked(next) }
        prompt = next; self.revision = revision
        if case .sending = status {} else { status = .ready }
        // The custom card already provides every choice and a text field.
        context()?.notificationActions = []
        changed()
    }
    /// The terminal's ticked rows, which count from 1, as option indices.
    private static func checked(_ p: JSON) -> Set<Int> {
        let count = PromptActions.question(p)?.array("options").count ?? 0
        return Set((p["checked"] as? [Int] ?? []).filter { (1...max(1, count)).contains($0) }.map { $0 - 1 })
    }

    /// Readable only while the device is unlocked, unlike the pairing credential.
    static func unlocked() -> Bool {
        let item: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: "com.newsblur.crabigator", kSecAttrAccount as String: "unlocked"]
        var result: CFTypeRef?
        var status = SecItemCopyMatching(item.merging([kSecReturnData as String: true]) { $1 } as CFDictionary, &result)
        if status == errSecItemNotFound {
            status = SecItemAdd(item.merging([kSecValueData as String: Data([1]), kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly]) { $1 } as CFDictionary, nil)
        }
        return status == errSecSuccess
    }
}

private let textColor = Color(uiColor: TerminalText.rgb(0xE9EEF4))

struct PromptCard: View {
    @ObservedObject var model: PromptCardModel
    var body: some View {
        ScrollViewReader { proxy in
            ScrollView { PromptCardContent(model: model) }.scrollBounceBehavior(.basedOnSize)
                // Bring the next question into view once the terminal moves on to it.
                .onChange(of: model.current) { _, current in withAnimation { proxy.scrollTo(current, anchor: .top) } }
        }
    }
}

struct PromptCardContent: View {
    @ObservedObject var model: PromptCardModel
    @State private var reply = ""
    @State private var focused = false
    private var replyIdentity: Data? {
        let prompt = model.prompt
        let identity: JSON = ["session": model.id, "type": prompt.text("prompt_type"),
            "page": model.current, "question": PromptActions.question(prompt) ?? [:],
            "review": prompt["review"] != nil]
        return try? JSONSerialization.data(withJSONObject: identity, options: [.sortedKeys])
    }
    var body: some View {
        let p = model.prompt, questions = p.array("questions")
        VStack(alignment: .leading, spacing: 14) {
            Text(model.title).font(.system(size: 13, weight: .semibold)).foregroundStyle(Crab.muted).lineLimit(2)
            if p.isEmpty {
                Text(model.body).font(.system(size: 15)).foregroundStyle(textColor)
            } else if p.text("prompt_type") == "question" && p["review"] == nil && !questions.isEmpty {
                ForEach(questions.indices, id: \.self) { question($0, questions[$0], total: questions.count) }
            } else {
                request(p)
            }
            if PromptActions.allowsText(p) && model.open {
                MessageField(placeholder: "Your answer", text: $reply, focused: $focused, maxLines: 4) { model.reply(reply) }
                    .padding(10).background(Crab.panel, in: RoundedRectangle(cornerRadius: 8))
                Button("Send answer") { model.reply(reply) }
                    .disabled(reply.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            footer
        }
        .padding(14).frame(maxWidth: .infinity, alignment: .leading)
        .onChange(of: replyIdentity) { _, _ in reply = ""; focused = false }
    }

    private func question(_ index: Int, _ q: JSON, total: Int) -> some View {
        let current = index == model.current, multi = q.bool("multi_select")
        return VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                if !q.text("header").isEmpty { Text(q.text("header").uppercased()).font(.system(size: 11, weight: .bold)).foregroundStyle(Crab.orange) }
                if total > 1 { Text("Question \(index + 1) of \(total)").font(.system(size: 11)).foregroundStyle(Crab.muted) }
                if let answer = model.answered[index] { Text("✓ " + answer).font(.system(size: 11, weight: .semibold)).foregroundStyle(Crab.mint).lineLimit(1) }
                else if index < model.current { Text("✓ Answered").font(.system(size: 11, weight: .semibold)).foregroundStyle(Crab.mint) }
            }
            Text(q.text("question")).font(.system(size: 16, weight: .semibold)).foregroundStyle(textColor).fixedSize(horizontal: false, vertical: true)
            if multi { Text("Choose any, then Submit").font(.system(size: 12)).foregroundStyle(Crab.muted) }
            ForEach(Array(q.array("options").enumerated()), id: \.offset) { option, choice in
                row(choice, check: multi ? current && model.selected.contains(option) : nil, enabled: current && model.open) { model.tap(question: index, option: option) }
            }
            if current && multi {
                Button { model.submit() } label: {
                    Text("Submit").font(.system(size: 15, weight: .semibold)).frame(maxWidth: .infinity).padding(.vertical, 10)
                        .foregroundStyle(Crab.ink).background(Crab.orange, in: RoundedRectangle(cornerRadius: 10))
                }.buttonStyle(.plain).disabled(!model.canSubmit).opacity(model.canSubmit ? 1 : 0.45)
                    .accessibilityIdentifier("Submit choices")
            }
        }.opacity(current || index < model.current ? 1 : 0.6).id(index)
    }

    /// Permissions, plans, the review page, and questions read only from the screen.
    @ViewBuilder private func request(_ p: JSON) -> some View {
        Text(PromptActions.title(p)).font(.system(size: 16, weight: .semibold)).foregroundStyle(textColor).fixedSize(horizontal: false, vertical: true)
        ForEach(Array(p.array("review").enumerated()), id: \.offset) { _, answer in
            VStack(alignment: .leading, spacing: 2) {
                Text(answer.text("question")).font(.system(size: 13)).foregroundStyle(Crab.muted)
                Text("→ " + answer.text("answer")).font(.system(size: 14, weight: .medium)).foregroundStyle(textColor)
            }
        }
        let details = PromptActions.details(p)
        if !details.isEmpty {
            Text(String(details.prefix(NotificationContent.detailLimit))).font(.system(size: 12, design: .monospaced)).foregroundStyle(textColor)
                .padding(10).frame(maxWidth: .infinity, alignment: .leading).background(Crab.panel, in: RoundedRectangle(cornerRadius: 8))
            if !NotificationContent.quickAnswers(p) { Text("This request is too long to review here. Open the session to see all of it.").font(.system(size: 12)).foregroundStyle(Crab.yellow) }
        }
        ForEach(Array(PromptActions.options(p).enumerated()), id: \.offset) { option, choice in
            row(choice, check: nil, enabled: model.open) { model.tap(option: option) }
        }
    }

    private func row(_ choice: JSON, check: Bool?, enabled: Bool, action: @escaping () -> Void) -> some View {
        let label = choice.text("label"), sending = model.status == .sending(label)
        return Button(action: action) {
            HStack(alignment: .top, spacing: 10) {
                if let check { Image(systemName: check ? "checkmark.square.fill" : "square").font(.system(size: 17)).foregroundStyle(check ? Crab.orange : Crab.muted) }
                VStack(alignment: .leading, spacing: 3) {
                    Text(label).font(.system(size: 15, weight: .medium)).foregroundStyle(textColor)
                    if !choice.text("description").isEmpty { Text(choice.text("description")).font(.system(size: 12.5)).foregroundStyle(Crab.muted) }
                }.fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 0)
                if sending { ProgressView().controlSize(.small) }
            }
            .padding(.horizontal, 12).padding(.vertical, 10).frame(maxWidth: .infinity, alignment: .leading)
            .background(Crab.panel, in: RoundedRectangle(cornerRadius: 10))
            .overlay(RoundedRectangle(cornerRadius: 10).stroke(check == true ? Crab.orange.opacity(0.7) : .clear))
            .contentShape(RoundedRectangle(cornerRadius: 10))
        }.buttonStyle(.plain).disabled(!enabled).accessibilityIdentifier("Choice " + label)
    }

    @ViewBuilder private var footer: some View {
        switch model.status {
        case .ready:
            if model.prompt.text("prompt_type") == "text" {
                if !PromptActions.allowsText(model.prompt) { Text("Open the session to answer.").font(.system(size: 12)).foregroundStyle(Crab.muted) }
            }
        case .sending(let label):
            HStack(spacing: 6) { ProgressView().controlSize(.small); Text("Sending “\(label)”…") }.font(.system(size: 13)).foregroundStyle(Crab.muted)
        case .sent(let label):
            Text("✓ Sent “\(label)”").font(.system(size: 13, weight: .semibold)).foregroundStyle(Crab.mint)
        case .failed(let message):
            Text(message).font(.system(size: 13)).foregroundStyle(Crab.red)
        case .resolved:
            Text("This request was answered or has ended.").font(.system(size: 13)).foregroundStyle(Crab.muted)
        }
        Button("Open session") { model.openSession() }.font(.system(size: 13, weight: .medium)).foregroundStyle(Crab.orange)
    }
}
