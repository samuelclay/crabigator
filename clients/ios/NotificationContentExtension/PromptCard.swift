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
    weak var context: NSExtensionContext?
    var changed: () -> Void = {}

    /// False when this extension cannot read the pairing credential; the app answers instead.
    var canAnswer: Bool { !API.shared.credentials.connection().token.isEmpty }
    var busy: Bool { if case .sending = status { return true }; if case .sent = status { return true }; return false }
    var open: Bool { status != .resolved && !busy }
    var current: Int { prompt.int("current_question") }

    func show(_ notification: UNNotification) {
        let content = notification.request.content, info = content.userInfo
        id = info["session_id"] as? String ?? ""
        title = content.title; body = content.body
        if let text = info["prompt"] as? String, let prompt = try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? JSON {
            adopt(prompt, revision: info["revision"] as? Int ?? 0)
        } else { changed() }
        Task { await refresh() }
    }

    func tap(question: Int? = nil, option: Int) {
        let options = PromptActions.options(prompt)
        guard open, options.indices.contains(option), question == nil || question == current else { return }
        if question != nil, PromptActions.question(prompt)?.bool("multi_select") == true {
            if !selected.insert(option).inserted { selected.remove(option) }
            changed(); return
        }
        answer(options[option].text("label"), option: option)
    }
    func submit() {
        let count = PromptActions.question(prompt)?.array("options").count ?? 0
        let checked = Set((prompt["checked"] as? [Int] ?? []).filter { (1...max(1, count)).contains($0) }.map { $0 - 1 })
        let labels = selected.sorted().map { PromptActions.options(prompt)[$0].text("label") }
        answer(labels.joined(separator: ", "), submit: true, toggling: selected.symmetricDifference(checked).sorted())
    }
    /// An action button under the view. iOS has already asked for authentication.
    func respond(to response: UNNotificationResponse) {
        guard let choice = NotificationContent.choice(response.actionIdentifier, id: id) else { return }
        let text = (response as? UNTextInputNotificationResponse)?.userText
        let revision = choice.revision ?? response.notification.request.content.userInfo["revision"] as? Int ?? -1
        let label = text ?? choice.option.flatMap { PromptActions.options(prompt).indices.contains($0) ? PromptActions.options(prompt)[$0].text("label") : nil } ?? "your answer"
        answer(label, option: choice.option, text: text, revision: revision, authenticated: true)
    }
    func openSession() { context?.performNotificationDefaultAction() }

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
                // A newer notification may already show what came next.
                guard self.revision == revision else { status = .ready; changed(); return }
                if let question { answered[question] = label }
                status = .sent(label); changed()
                await follow(after: revision)
            } catch {
                status = .failed(error.localizedDescription); changed()
                if (error as? APIError)?.status == 409 { await refresh() }
            }
        }
    }
    /// Shows the next question when the terminal moves on, and closes once it is done.
    private func follow(after sent: Int) async {
        for _ in 0..<12 {
            try? await Task.sleep(nanoseconds: 600_000_000)
            guard revision == sent else { return }
            guard let snap = try? await API.shared.snapshot(id), revision == sent else { continue }
            if snap.int("prompt_revision") == sent { continue }
            if snap.bool("attention_pending") && snap.bool("desktop_connected") {
                adopt(NotificationContent.prompt(snap), revision: snap.int("prompt_revision"))
                return
            }
            break
        }
        await NotificationContent.remove(id: id)
        context?.dismissNotificationContentExtension()
    }
    private func refresh() async {
        guard let snap = try? await API.shared.snapshot(id) else { return }
        if snap.bool("attention_pending") && snap.bool("desktop_connected") {
            adopt(NotificationContent.prompt(snap), revision: snap.int("prompt_revision"))
        } else if !busy {
            status = .resolved; context?.notificationActions = []; changed()
        }
    }
    private func adopt(_ next: JSON, revision: Int) {
        guard revision != self.revision || prompt.isEmpty else { return }
        func questions(_ p: JSON) -> [String] { p.array("questions").map { $0.text("question") } }
        if questions(next) != questions(prompt) { answered = [:] }
        if questions(next) != questions(prompt) || next.int("current_question") != current || next["review"] != nil {
            let count = PromptActions.question(next)?.array("options").count ?? 0
            selected = Set((next["checked"] as? [Int] ?? []).filter { (1...max(1, count)).contains($0) }.map { $0 - 1 })
        }
        prompt = next; self.revision = revision
        if case .sending = status {} else { status = .ready }
        context?.notificationActions = NotificationContent.actions(next, id: id, revision: revision)
        changed()
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
        ScrollView { PromptCardContent(model: model) }.scrollBounceBehavior(.basedOnSize)
    }
}

struct PromptCardContent: View {
    @ObservedObject var model: PromptCardModel
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
            footer
        }
        .padding(14).frame(maxWidth: .infinity, alignment: .leading)
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
                }.buttonStyle(.plain).disabled(!model.open || model.selected.isEmpty).opacity(model.open && !model.selected.isEmpty ? 1 : 0.45)
                    .accessibilityIdentifier("Submit choices")
            }
        }.opacity(current || index < model.current ? 1 : 0.6)
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
                Text(PromptActions.allowsText(model.prompt) ? "Use Reply to answer." : "Open the session to answer.").font(.system(size: 12)).foregroundStyle(Crab.muted)
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
