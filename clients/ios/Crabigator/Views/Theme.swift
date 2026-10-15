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

/// The crab above the pixel wordmark, as in the brand lockup. Each wordmark cell is
/// a whole number of device pixels, and each crab cell is twice that, so both stay sharp.
struct BrandHeader: View {
    @Environment(\.displayScale) private var scale
    var body: some View {
        ViewThatFits(in: .horizontal) { lockup(pixels: 4); lockup(pixels: 3); lockup(pixels: 2); lockup(pixels: 1) }
            .accessibilityElement(children: .ignore).accessibilityLabel("Crabigator").accessibilityAddTraits(.isHeader)
    }
    private func lockup(pixels: CGFloat) -> some View {
        let cell = pixels / scale
        return VStack(alignment: .leading, spacing: 12 * cell) {
            Image("Crabigator").interpolation(.none).resizable().frame(width: 48 * cell, height: 48 * cell)
            Image("Wordmark").interpolation(.none).resizable().frame(width: 220 * cell, height: 23 * cell)
        }
    }
}

/// A growing multi-line text box. Return on a hardware keyboard calls `onReturn`;
/// Shift+Return and the on-screen keyboard's Return still add a line. SwiftUI's
/// vertical TextField consumes a hardware Return before `onKeyPress` sees it.
struct MessageField: View {
    var placeholder: String
    @Binding var text: String
    @Binding var focused: Bool
    var maxLines = 6
    var onReturn: () -> Void
    var body: some View {
        ReturnTextField(placeholder: placeholder, text: $text, focused: $focused, maxLines: maxLines, onReturn: onReturn)
            .overlay(alignment: .topLeading) { if text.isEmpty { Text(placeholder).foregroundStyle(Color(uiColor: .placeholderText)).allowsHitTesting(false).accessibilityHidden(true) } }
    }
}
private struct ReturnTextField: UIViewRepresentable {
    var placeholder: String
    @Binding var text: String
    @Binding var focused: Bool
    var maxLines: Int
    var onReturn: () -> Void
    func makeCoordinator() -> Coordinator { Coordinator(self) }
    func makeUIView(context: Context) -> ReturnTextView {
        let view = ReturnTextView()
        view.delegate = context.coordinator
        view.font = .preferredFont(forTextStyle: .body); view.adjustsFontForContentSizeCategory = true
        view.backgroundColor = .clear; view.textColor = .label
        view.textContainerInset = .zero; view.textContainer.lineFragmentPadding = 0
        view.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        view.accessibilityLabel = placeholder; view.accessibilityIdentifier = placeholder
        return view
    }
    func updateUIView(_ view: ReturnTextView, context: Context) {
        context.coordinator.parent = self
        view.onReturn = onReturn
        if view.text != text { view.text = text }
        if focused != view.isFirstResponder {
            DispatchQueue.main.async { _ = focused ? view.becomeFirstResponder() : view.resignFirstResponder() }
        }
    }
    func sizeThatFits(_ proposal: ProposedViewSize, uiView view: ReturnTextView, context: Context) -> CGSize? {
        guard let width = proposal.width, width.isFinite, width > 0 else { return nil }
        // Beyond its line limit or the offered height (a landscape keyboard leaves little), it scrolls.
        let line = ceil(view.font?.lineHeight ?? 22), limit = line * CGFloat(maxLines)
        let height = ceil(view.sizeThatFits(CGSize(width: width, height: .greatestFiniteMagnitude)).height)
        return CGSize(width: width, height: max(line, min(height, limit, proposal.height ?? .infinity)))
    }
    final class Coordinator: NSObject, UITextViewDelegate {
        var parent: ReturnTextField
        init(_ parent: ReturnTextField) { self.parent = parent }
        func textViewDidChange(_ view: UITextView) { parent.text = view.text; view.invalidateIntrinsicContentSize() }
        func textViewDidBeginEditing(_ view: UITextView) { if !parent.focused { parent.focused = true } }
        func textViewDidEndEditing(_ view: UITextView) { if parent.focused { parent.focused = false } }
    }
}
final class ReturnTextView: UITextView {
    var onReturn: (() -> Void)?
    /// The Return press taken for sending. Only its own release is withheld: the
    /// text system must see every other press end, or Shift can stay held.
    private var taken: UIPress?
    // Only presses carry the key; a UIKeyCommand for Return never fires in a text view.
    override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        if let press = presses.first, presses.count == 1, onReturn != nil, let key = press.key,
           [.keyboardReturnOrEnter, .keypadEnter].contains(key.keyCode),
           key.modifierFlags.isDisjoint(with: [.shift, .alternate, .command, .control]) {
            taken = press; onReturn?()
        } else { super.pressesBegan(presses, with: event) }
    }
    override func pressesEnded(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        let rest = release(presses); if !rest.isEmpty { super.pressesEnded(rest, with: event) }
    }
    override func pressesCancelled(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        let rest = release(presses); if !rest.isEmpty { super.pressesCancelled(rest, with: event) }
    }
    override func pressesChanged(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        let rest = presses.filter { $0 !== taken }; if !rest.isEmpty { super.pressesChanged(rest, with: event) }
    }
    private func release(_ presses: Set<UIPress>) -> Set<UIPress> {
        guard let taken, presses.contains(taken) else { return presses }
        self.taken = nil
        return presses.filter { $0 !== taken }
    }
}
