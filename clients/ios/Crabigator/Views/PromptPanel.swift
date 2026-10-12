import SwiftUI

struct PromptPanel: View {
    var prompt: JSON
    var revision: Int?
    var disabled: Bool
    var send: (TerminalAction, Int?) -> Void
    @State private var text = ""
    @State private var error: String?
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
                            if PromptActions.question(prompt)?.bool("multi_select") == true { Image(systemName: (prompt["checked"] as? [Int] ?? []).contains(i + 1) ? "checkmark.square.fill" : "square") }
                            VStack(alignment: .leading, spacing: 3) { Text(option.text("label")).font(.system(size: 14)); if !option.text("description").isEmpty { Text(option.text("description")).font(.system(size: 12)).foregroundStyle(Crab.muted) } }
                            .frame(maxWidth: .infinity, alignment: .leading).padding(10).background(Crab.panel, in: RoundedRectangle(cornerRadius: 8))
                        }
                    }.buttonStyle(.plain).disabled(disabled).accessibilityIdentifier("Prompt option \(i + 1)")
                }
                if PromptActions.allowsText(prompt) {
                    HStack { TextField("Your answer", text: $text, axis: .vertical).lineLimit(1...4).textFieldStyle(.roundedBorder); IconButton(icon: "arrow.up", label: "Send answer", enabled: !disabled && !text.isEmpty) { act(text: text) } }
                }
                if PromptActions.question(prompt)?.bool("multi_select") == true && prompt["review"] == nil { Button("Continue") { act(submit: true) }.disabled(disabled) }
                if let error { Text(error).font(.caption).foregroundStyle(Crab.red) }
            }.padding(12)
        }.frame(maxHeight: 240).background(Crab.orange.opacity(0.06)).onChange(of: revision) { _, _ in text = "" }
    }
    func act(option: Int? = nil, text: String? = nil, submit: Bool = false) {
        do { send(try PromptActions.action(prompt, option: option, text: text, submit: submit), revision) } catch { self.error = error.localizedDescription }
    }
}
