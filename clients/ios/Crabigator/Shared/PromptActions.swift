import Foundation

struct TerminalAction {
    var route: String
    var body: JSON
}
enum PromptActions {
    static func question(_ p: JSON) -> JSON? { let qs = p.array("questions"), i = p.int("current_question"); return qs.indices.contains(i) ? qs[i] : nil }
    static func details(_ p: JSON) -> String {
        guard let input = p["tool_input"], !(input is NSNull) else { return "" }
        if p.text("tool_name") == "Bash", let j = input as? JSON, Set(j.keys).isSubset(of: ["command", "description"]), !j.text("command").isEmpty { return j.text("command") }
        if let text = input as? String { return text }
        return (try? JSONSerialization.data(withJSONObject: input, options: [.prettyPrinted, .sortedKeys, .fragmentsAllowed])).flatMap { String(data: $0, encoding: .utf8) } ?? ""
    }
    static func title(_ p: JSON) -> String {
        switch p.text("prompt_type") {
        case "question": return p["review"] != nil ? "Review your answers" : question(p)?.text("question", "A question needs your answer") ?? "A question needs your answer"
        case "permission": return "Allow \(p.text("tool_name", "this action"))?"
        case "text": return p.text("state") == "permission" ? "Permission requested. Open the session to review." : "Your session has a question."
        default: return "Ready to start the plan?"
        }
    }
    static func options(_ p: JSON) -> [JSON] {
        if p.text("prompt_type") != "question" { return p.array("options") }
        if p["review"] != nil { return [["label": "Submit answers", "value": "1"], ["label": "Cancel", "value": "2"]] }
        return question(p)?.array("options") ?? []
    }
    static func allowsText(_ p: JSON) -> Bool {
        if p.text("prompt_type") == "text" { return p.text("state") == "question" }
        return p["review"] == nil && question(p)?.bool("allows_other", true) == true
    }
    /// `toggling` flips those checkboxes of a multi-select page before submitting it.
    static func action(_ p: JSON, option: Int? = nil, text: String? = nil, submit: Bool = false, toggling: [Int] = []) throws -> TerminalAction {
        func invalid(_ message: String) -> APIError { APIError(status: 409, message: message) }
        if let option, !options(p).indices.contains(option) { throw invalid("This choice is no longer available.") }
        if !toggling.isEmpty, question(p)?.bool("multi_select") != true || p["review"] != nil || !toggling.allSatisfy(options(p).indices.contains) { throw invalid("This choice is no longer available.") }
        if p.text("prompt_type") == "text" {
            guard p.text("state") == "question", let text, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { throw invalid("Open this session to review the request.") }
            return TerminalAction(route: "answer", body: ["text": text])
        }
        if p.text("prompt_type") != "question" {
            guard let option else { throw invalid("Choose an option in the app.") }
            return TerminalAction(route: "answer", body: ["text": options(p)[option].text("value")])
        }
        var steps: [JSON] = []
        func key(_ value: String) { steps.append(["type": "key", "key": value]) }
        func type(_ value: String) { steps.append(["type": "text", "text": value]) }
        if p["review"] != nil {
            guard let option else { throw invalid("Choose Submit or Cancel.") }
            type(String(option + 1)); return TerminalAction(route: "key-sequence", body: ["steps": steps])
        }
        guard let q = question(p) else { throw invalid("This question is no longer available.") }
        let count = q.array("options").count, grok = p.text("ui") == "grok_card", multi = q.bool("multi_select")
        var cursor = p.int("cursor_row", 1)
        func move(_ target: Int) { for _ in 0..<min(100, abs(target - cursor)) { key(target > cursor ? "down" : "up") }; cursor = target }
        func pick(_ option: Int) {
            if cursor == count + 1 { key(grok ? "escape" : "up"); cursor -= 1 }
            if grok && multi { move(option + 1); key("space") }
            else if grok { type(String(UnicodeScalar(97 + option)!)) }
            else if p["cursor_row"] != nil || multi { type(String(option + 1)) }
            else { move(option + 1); key("enter") }
        }
        if let option { pick(option) }
        else if text != nil || submit {
            // Claude's digits tick a row without moving the cursor; Grok's Space ticks the row under it.
            toggling.forEach(pick)
            if let text, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                guard q.bool("allows_other", true) else { throw invalid("This question requires one of its choices.") }
                if grok { type("z") }
                else if !multi { if cursor != count + 1 { type(String(count + 1)) }; cursor = count + 1 }
                else { move(count + 1) }
                for _ in 0..<min(2000, p.text("custom_text").count) { key("backspace") }
                type(text); steps.append(["type": "delay", "ms": 50])
                if grok && multi { key("escape") }
            }
            if grok && multi { key(p.int("current_question") + 1 < p.array("questions").count ? "right" : "enter") }
            else if multi { move(count + 2); key("enter") }
            else { key("enter") }
        }
        return TerminalAction(route: "key-sequence", body: ["steps": steps])
    }
}
func terminalKey(_ key: String, shift: Bool = false, control: Bool = false, alt: Bool = false) -> String {
    let m = 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (control ? 4 : 0)
    if let cursor = ["↑": "A", "↓": "B", "→": "C", "←": "D", "Home": "H", "End": "F"][key] { return m == 1 ? "\u{1b}[\(cursor)" : "\u{1b}[1;\(m)\(cursor)" }
    if let n = ["Insert": 2, "Delete": 3, "PgUp": 5, "PgDn": 6][key] { return m == 1 ? "\u{1b}[\(n)~" : "\u{1b}[\(n);\(m)~" }
    if key == "⌫" && (control || alt) { return "\u{1b}\u{7f}" }
    if key == "Tab" { return shift ? "\u{1b}[Z" : control ? "\u{1b}[9;\(m)~" : "\t" }
    var value: String
    switch key {
    case "Esc": value = "\u{1b}"
    case "Enter": value = shift || control ? "\u{1b}[13;\(m)u" : "\r"
    case "⌫": value = "\u{7f}"
    case "Space": value = control ? "\0" : " "
    default:
        if control, key.count == 1, let scalar = key.uppercased().unicodeScalars.first, (65...90).contains(scalar.value) { value = String(UnicodeScalar(scalar.value - 64)!) }
        else { value = shift ? key.uppercased() : key.lowercased() }
    }
    return alt && !(key == "Enter" && (shift || control)) ? "\u{1b}" + value : value
}
