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
    @Environment(\.boardTextSize) private var size
    var body: some View { Text(session.glyph).font(.system(size: 13 * size / 14, design: .monospaced)).lineLimit(1).frame(width: 38, height: max(16, 25 * size / 14)).foregroundStyle(Crab.hex(session.color)).background(Crab.hex(session.background), in: RoundedRectangle(cornerRadius: 5)).accessibilityHidden(true) }
}
struct SessionStatus: View {
    var session: Session
    var label: String { !session.active ? "Ended" : session.state == "thinking" ? "Working" : session.state.capitalized }
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
            }.accessibilityLabel(label)
        } else {
            let state = session.active ? session.state : "ended"
            let symbol = ["permission": "!", "question": "?", "complete": "✓", "interrupted": "⊘"][state] ?? "○"
            let color = state == "question" ? Crab.deepOrange : state == "permission" ? Crab.yellow : state == "complete" ? Crab.purple : state == "interrupted" ? Crab.red : Crab.muted
            Text(symbol).font(.system(size: 14, weight: session.attention ? .bold : .regular, design: .monospaced)).frame(width: 14, height: 21).foregroundStyle(session.attention ? Crab.ink : color).background(session.attention ? color : .clear, in: RoundedRectangle(cornerRadius: 3)).accessibilityLabel(label)
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
            }.boardFont(11).monospacedDigit().lineLimit(1)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel("\(SessionStatus(session: session).label), session time \(durationText(session.duration(now))), thinking time \(durationText(session.thinkingSeconds)), last prompt \(ageText(session.promptAt, now: now)), last completion \(ageText(session.completionAt, now: now))")
        }
    }
    func timing(_ now: Int, compact: Bool, duration: Bool) -> some View {
        HStack(spacing: 7) {
            if duration {
                if preferences.visible("sessionTime") { Text("◉ " + short(durationText(session.duration(now)), compact)).foregroundStyle(Crab.blue).fixedSize() }
                if preferences.visible("thinkingTime") { Text("◐ " + short(durationText(session.thinkingSeconds), compact)).foregroundStyle(Crab.mint).fixedSize() }
            }
            Spacer(minLength: 4)
            SessionStatus(session: session)
            RecencyColumns(session: session, now: now, showPrompt: preferences.visible("promptRecency"), showCompletion: preferences.visible("completionRecency"))
        }
    }
    func short(_ text: String, _ compact: Bool) -> String { compact ? String(text.split(separator: " ").first ?? "—") : text }
}

struct RecencyColumns: View {
    var session: Session, now: Int
    var showPrompt = true, showCompletion = true
    var body: some View {
        HStack(spacing: 8) {
            if showPrompt { column("⟩", timestamp: session.promptAt, label: "Last prompt", color: Crab.orange) }
            if showCompletion { column("⋖", timestamp: session.completionAt, label: "Last completion", color: Crab.deepOrange) }
        }.boardFont(11).monospacedDigit().lineLimit(1)
    }
    func column(_ symbol: String, timestamp: Int, label: String, color: Color) -> some View {
        Text(symbol + " " + ageText(timestamp, now: now)).foregroundStyle(color)
            .minimumScaleFactor(0.65).frame(width: 66, alignment: .leading)
            .accessibilityLabel(label + ": " + ageText(timestamp, now: now))
    }
}

private struct BoardTextSizeKey: EnvironmentKey {
    static let defaultValue: CGFloat = 14
}
extension EnvironmentValues {
    var boardTextSize: CGFloat {
        get { self[BoardTextSizeKey.self] }
        set { self[BoardTextSizeKey.self] = newValue }
    }
}
private struct BoardFont: ViewModifier {
    @Environment(\.boardTextSize) var size
    var base: CGFloat
    var weight: Font.Weight
    func body(content: Content) -> some View {
        content.font(.system(size: base * size / 14, weight: weight))
    }
}
extension View {
    func boardFont(_ size: CGFloat, weight: Font.Weight = .regular) -> some View {
        modifier(BoardFont(base: size, weight: weight))
    }
}

// The saved density also controls PR rows; terminal spacing stays independent.
struct BoardSpacing {
    var density: String = "comfortable"
    var tight: Bool { density == "compact" }
    func gap(_ normal: CGFloat) -> CGFloat { normal * (tight ? 0.45 : density == "relaxed" ? 1.5 : 1) }
}
private struct BoardSpacingKey: EnvironmentKey {
    static let defaultValue = BoardSpacing()
}
extension EnvironmentValues {
    var boardSpacing: BoardSpacing {
        get { self[BoardSpacingKey.self] }
        set { self[BoardSpacingKey.self] = newValue }
    }
}

// Propagate the first title line's center through the row's nested stacks.
private enum SessionTitleCenter: AlignmentID {
    static func defaultValue(in context: ViewDimensions) -> CGFloat { context[VerticalAlignment.center] }
}
extension VerticalAlignment {
    static let sessionTitleCenter = VerticalAlignment(SessionTitleCenter.self)
}
private struct SessionTitleAlignment: ViewModifier {
    @Environment(\.boardTextSize) var size
    func body(content: Content) -> some View {
        let font = UIFont.systemFont(ofSize: size, weight: .medium)
        content.alignmentGuide(.sessionTitleCenter) { $0[.firstTextBaseline] - font.ascender + font.lineHeight / 2 }
    }
}
extension View {
    func sessionTitleAlignment() -> some View { modifier(SessionTitleAlignment()) }
}
