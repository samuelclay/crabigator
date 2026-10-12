import SwiftUI

enum Crab {
    static let ink = Color(uiColor: TerminalText.rgb(0x101419)), panel = Color(uiColor: TerminalText.rgb(0x1B2129))
    static func color(_ n: Int) -> Color { Color(uiColor: TerminalText.palette(n)) }
    static let muted = color(245), orange = color(179), deepOrange = color(208), mint = color(83), blue = color(39), title = color(75), purple = color(141), cyan = color(45), red = color(203), yellow = color(220)
    static func hex(_ value: String) -> Color { Color(uiColor: TerminalText.rgb(UInt32(value.trimmingCharacters(in: CharacterSet(charactersIn: "#")), radix: 16) ?? 0xFFAA77)) }
}
struct IconButton: View {
    var icon: String, label: String
    var selected = false, enabled = true
    var action: () -> Void
    var body: some View {
        Button(action: action) { Image(systemName: icon).font(.system(size: 17, weight: .medium)).frame(width: 44, height: 44).foregroundStyle(selected ? Crab.orange : Crab.muted).background(selected ? Crab.orange.opacity(0.13) : .clear, in: RoundedRectangle(cornerRadius: 10)) }
            .buttonStyle(.plain).disabled(!enabled).opacity(enabled ? 1 : 0.4).accessibilityLabel(label).accessibilityIdentifier(label)
    }
}
struct IdentityChip: View {
    var session: Session
    var body: some View { Text(session.glyph).font(.system(size: 13, design: .monospaced)).lineLimit(1).frame(width: 38, height: 25).foregroundStyle(Crab.hex(session.color)).background(Crab.hex(session.background), in: RoundedRectangle(cornerRadius: 5)).accessibilityHidden(true) }
}
struct SessionStatus: View {
    var session: Session
    var body: some View {
        if session.active && session.state == "thinking" {
            TimelineView(.animation(minimumInterval: 0.1)) { timeline in
                let frames: [Int] = [0x0b, 0x19, 0x39, 0x38, 0x3c, 0x34, 0x26, 0x27, 0x07, 0x0f]
                let bits = frames[Int(timeline.date.timeIntervalSince1970 * 10) % frames.count]
                Canvas { context, size in
                    for col in 0..<2 { for row in 0..<3 where bits & (1 << (col * 3 + row)) != 0 {
                        let rect = CGRect(x: CGFloat(col) * 6 + 2, y: CGFloat(row) * 5 + 3, width: 3, height: 3)
                        context.fill(Path(ellipseIn: rect), with: .color(Crab.mint))
                    } }
                }.frame(width: 14, height: 21)
            }.accessibilityLabel("Working")
        } else {
            let state = session.active ? session.state : "ended"
            let symbol = ["permission": "!", "question": "?", "complete": "✓", "interrupted": "⊘"][state] ?? "○"
            let color = state == "question" ? Crab.deepOrange : state == "permission" ? Crab.yellow : state == "complete" ? Crab.purple : state == "interrupted" ? Crab.red : Crab.muted
            Text(symbol).font(.system(size: 14, weight: session.attention ? .bold : .regular, design: .monospaced)).frame(width: 14, height: 21).foregroundStyle(session.attention ? Crab.ink : color).background(session.attention ? color : .clear, in: RoundedRectangle(cornerRadius: 3)).accessibilityLabel(state.capitalized)
        }
    }
}
struct SessionTiming: View {
    var session: Session, preferences: Preferences
    var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { timeline in
            let now = Int(timeline.date.timeIntervalSince1970)
            ViewThatFits(in: .horizontal) {
                timing(now, compact: false, duration: true)
                timing(now, compact: true, duration: true)
                timing(now, compact: true, duration: false)
            }.font(.system(size: 11)).monospacedDigit().lineLimit(1)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel("Session time \(durationText(session.duration(now))), thinking time \(durationText(session.thinkingSeconds)), last prompt \(ageText(session.promptAt, now: now)), last completion \(ageText(session.completionAt, now: now))")
        }
    }
    func timing(_ now: Int, compact: Bool, duration: Bool) -> some View {
        HStack(spacing: 7) {
            if duration {
                if preferences.visible("sessionTime") { Text("◉ " + short(durationText(session.duration(now)), compact)).foregroundStyle(Crab.blue).fixedSize() }
                if preferences.visible("thinkingTime") { Text("◐ " + short(durationText(session.thinkingSeconds), compact)).foregroundStyle(Crab.mint).fixedSize() }
            }
            Spacer(minLength: 4)
            if preferences.visible("promptRecency") { Text("⟩ " + (compact ? ageText(session.promptAt, now: now).replacingOccurrences(of: " ago", with: "") : ageText(session.promptAt, now: now))).foregroundStyle(Crab.orange).fixedSize() }
            if preferences.visible("completionRecency") { Text("⋖ " + ageText(session.completionAt, now: now)).foregroundStyle(Crab.deepOrange).fixedSize() }
        }
    }
    func short(_ text: String, _ compact: Bool) -> String { compact ? String(text.split(separator: " ").first ?? "—") : text }
}
