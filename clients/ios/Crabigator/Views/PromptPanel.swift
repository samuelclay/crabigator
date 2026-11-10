import SwiftUI

/// Hold the outgoing card through the short gap between an accepted answer
/// and the desktop's next page. Retained and outgoing cards cannot send input.
struct PromptTransition: View {
    var prompt: JSON?
    var revision: Int?
    var disabled: Bool
    var send: (TerminalAction, Int?) -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var retained: JSON?
    private var displayed: JSON? { prompt ?? retained }
    private var updateID: String {
        guard let prompt else { return "" }
        return String(data: (try? JSONSerialization.data(withJSONObject: prompt, options: [.sortedKeys])) ?? Data(), encoding: .utf8) ?? ""
    }
    var body: some View {
        ZStack {
            if let displayed {
                let page = PromptActions.pageID(displayed)
                PromptPanel(prompt: displayed, revision: revision,
                            disabled: disabled || prompt.map(PromptActions.pageID) != page, send: send)
                    .id(page)
                    .transition(reduceMotion ? .opacity : .opacity.combined(with: .offset(y: 8)))
            }
        }.clipped()
            .animation(.easeInOut(duration: reduceMotion ? 0.12 : 0.22), value: displayed.map(PromptActions.pageID))
            .task(id: updateID) {
                if prompt == nil {
                    try? await Task.sleep(for: .milliseconds(300))
                    guard !Task.isCancelled else { return }
                }
                retained = prompt
            }
    }
}

struct PromptPanel: View {
    var prompt: JSON
    var revision: Int?
    var disabled: Bool
    var send: (TerminalAction, Int?) -> Void
    @State private var text = ""
    @State private var error: String?
    @State private var focused = false
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                Text(PromptActions.title(prompt)).font(.system(size: 14, weight: .semibold)).foregroundStyle(Crab.orange)
                let details = PromptActions.details(prompt)
                if !details.isEmpty { Text(details).font(.system(size: 12, design: .monospaced)).textSelection(.enabled) }
                ForEach(Array(prompt.array("review").enumerated()), id: \.offset) { _, answer in
                    Text(answer.text("question") + "\n" + answer.text("answer")).font(.system(size: 13)).foregroundStyle(Crab.muted)
                }
                ForEach(Array(PromptActions.options(prompt).enumerated()), id: \.offset) { i, option in
                    Button { act(option: i) } label: {
                        HStack(alignment: .top, spacing: 8) {
                            // Pick-any pages tick boxes; pick-one pages and the review page's two choices are radio buttons.
                            if PromptActions.question(prompt)?.bool("multi_select") == true { Image(systemName: (prompt["checked"] as? [Int] ?? []).contains(i + 1) ? "checkmark.square.fill" : "square") }
                            else if prompt.text("prompt_type") == "question" { Image(systemName: "circle") }
                            VStack(alignment: .leading, spacing: 3) { Text(option.text("label")).font(.system(size: 14)); if !option.text("description").isEmpty { Text(option.text("description")).font(.system(size: 12)).foregroundStyle(Crab.muted) } }
                            .frame(maxWidth: .infinity, alignment: .leading).padding(10).background(Crab.panel, in: RoundedRectangle(cornerRadius: 8))
                        }
                    }.buttonStyle(.plain).disabled(disabled).accessibilityIdentifier("Prompt option \(i + 1)")
                    if let preview = option["preview"] as? String {
                        ScrollView(.horizontal) { Text(preview).font(.system(size: 11, design: .monospaced)).fixedSize().padding(8) }
                            .background(Crab.panel, in: RoundedRectangle(cornerRadius: 6))
                    }
                }
                if PromptActions.allowsText(prompt) {
                    HStack { MessageField(placeholder: "Your answer", text: $text, focused: $focused, maxLines: 4) { if !disabled && !text.isEmpty { act(text: text) } }.padding(.horizontal, 8).padding(.vertical, 7).background(Crab.panel, in: RoundedRectangle(cornerRadius: 6)); IconButton(icon: "arrow.up", label: "Send answer", enabled: !disabled && !text.isEmpty) { act(text: text) } }
                }
                if PromptActions.question(prompt)?.bool("multi_select") == true && prompt["review"] == nil { Button("Continue") { act(submit: true) }.disabled(disabled) }
                if let error { Text(error).font(.caption).foregroundStyle(Crab.red) }
            }.padding(12)
        }.frame(maxHeight: 240).background(Crab.orange.opacity(0.06)).onChange(of: PromptActions.pageID(prompt)) { _, _ in text = "" }
    }
    func act(option: Int? = nil, text: String? = nil, submit: Bool = false) {
        do { send(try PromptActions.action(prompt, option: option, text: text, submit: submit), revision) } catch { self.error = error.localizedDescription }
    }
}

#if DEBUG
/// A local interview for UI tests; never connects or sends terminal input.
struct QuestionTransitionFixture: View {
    @State private var page = 0
    @State private var hidden = false
    @State private var cursor = 1
    @State private var sent = ""
    private var prompt: JSON {
        ["prompt_type": "question", "current_question": page, "cursor_row": cursor, "questions": [
            ["question": "Choose a layout", "options": [["label": "List", "preview": "┌─────────────┐\n│ Story list  │\n└─────────────┘"], ["label": "Cards", "preview": "[ Story ] [ Story ]"]]],
            ["question": "What happens at the end?", "options": [["label": "Repeat"], ["label": "Stop"]]],
            ["question": "Start automatically?", "options": [["label": "Yes"], ["label": "No"]]]]]
    }
    var body: some View {
        VStack {
            Text("Interview transition test").font(.headline)
            Text(sent).accessibilityIdentifier("last-question-action")
            Button("Refresh cursor") { cursor = cursor == 1 ? 2 : 1 }
            Spacer()
            PromptTransition(prompt: hidden ? nil : prompt, revision: hidden ? nil : page + cursor,
                             disabled: hidden) { action, _ in
                sent = action.body.array("steps").map { $0.text("text", $0.text("key")) }.joined(separator: ",")
                hidden = true
                Task {
                    try? await Task.sleep(for: .milliseconds(120))
                    if page < 2 { page += 1; cursor = 1; hidden = false }
                }
            }
            Text("Message composer").frame(maxWidth: .infinity).padding()
        }.background(Crab.ink)
    }
}
#endif
