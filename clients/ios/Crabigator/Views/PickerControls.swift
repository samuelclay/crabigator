import SwiftUI

/// A thumbnail drawn from the shared design's geometry: marks placed in a view box and
/// scaled to the tile. Android and the dashboard draw the same marks.
struct Thumb {
    enum Mark {
        case fill(CGRect, CGFloat, Color)
        case stroke(CGRect, CGFloat, Color, CGFloat)
        case path(Path, Color)
    }
    var size: CGSize
    var ground = Crab.hex("12171D")
    var marks: [Mark]
}
struct Thumbnail: View {
    var thumb: Thumb
    var radius: CGFloat = 4
    var body: some View {
        Canvas { context, size in
            context.scaleBy(x: size.width / thumb.size.width, y: size.height / thumb.size.height)
            for mark in thumb.marks {
                switch mark {
                case .fill(let rect, let corner, let color): context.fill(Path(roundedRect: rect, cornerRadius: corner), with: .color(color))
                case .stroke(let rect, let corner, let color, let width): context.stroke(Path(roundedRect: rect, cornerRadius: corner), with: .color(color), lineWidth: width)
                case .path(let path, let color): context.stroke(path, with: .color(color), style: StrokeStyle(lineWidth: 1.4, lineCap: .round, lineJoin: .round))
                }
            }
        }.aspectRatio(thumb.size.width / thumb.size.height, contentMode: .fit).background(thumb.ground).clipShape(RoundedRectangle(cornerRadius: radius)).accessibilityHidden(true)
    }
}
private func f(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat, _ r: CGFloat, _ hex: String) -> Thumb.Mark { .fill(CGRect(x: x, y: y, width: w, height: h), r, Crab.hex(hex)) }
private func o(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat, _ r: CGFloat, _ hex: String, _ width: CGFloat) -> Thumb.Mark { .stroke(CGRect(x: x, y: y, width: w, height: h), r, Crab.hex(hex), width) }
private func line(_ points: [(CGFloat, CGFloat)]...) -> Path {
    var path = Path()
    for run in points { path.addLines(run.map { CGPoint(x: $0.0, y: $0.1) }) }
    return path
}
// Rows, title bars, the two computers, a project and a pull request, then the session marks.
private let slab = "2A323D", bar = "C4CDD8", one = "5f87ff", two = "ff8700", repo = "ffd700", pull = "af87ff", grey = "6B7684"
private let peach = "FFAA77", green = "7ee787", gold = "ffd75f", pass = "5fff5f", fail = "ff5f5f", accent = "00d7ff"
extension Thumb {
    static let groupOne = Thumb(size: CGSize(width: 120, height: 80), marks: [
        f(4, 4, 112, 14, 3, slab), f(8, 8, 7, 6, 1.5, peach), f(19, 7.5, 52, 2.6, 1.3, bar), f(19, 12, 22, 2.2, 1.1, one),
        f(4, 22, 112, 14, 3, slab), f(8, 26, 7, 6, 1.5, green), f(19, 25.5, 40, 2.6, 1.3, bar), f(19, 30, 18, 2.2, 1.1, two),
        f(4, 40, 112, 14, 3, slab), f(8, 44, 7, 6, 1.5, pull), f(19, 43.5, 58, 2.6, 1.3, bar), f(19, 48, 22, 2.2, 1.1, one),
        f(4, 58, 112, 14, 3, slab), f(8, 62, 7, 6, 1.5, gold), f(19, 61.5, 46, 2.6, 1.3, bar), f(19, 66, 18, 2.2, 1.1, two)])
    static let groupComputer = Thumb(size: CGSize(width: 120, height: 80), marks: [
        o(5, 4, 8, 5.6, 1, one, 1.3), f(17, 5.4, 34, 2.8, 1.4, one),
        f(4, 13, 112, 11, 3, slab), f(8, 15.5, 7, 6, 1.5, peach), f(19, 17.2, 52, 2.6, 1.3, bar),
        f(4, 26, 112, 11, 3, slab), f(8, 28.5, 7, 6, 1.5, pull), f(19, 30.2, 58, 2.6, 1.3, bar),
        o(5, 42.6, 8, 5.6, 1, two, 1.3), f(17, 44, 28, 2.8, 1.4, two),
        f(4, 52, 112, 11, 3, slab), f(8, 54.5, 7, 6, 1.5, green), f(19, 56.2, 40, 2.6, 1.3, bar),
        f(4, 65, 112, 11, 3, slab), f(8, 67.5, 7, 6, 1.5, gold), f(19, 69.2, 46, 2.6, 1.3, bar)])
    static let groupProject = Thumb(size: CGSize(width: 120, height: 80), marks: [
        f(5, 5, 30, 2.8, 1.4, repo),
        f(4, 12, 112, 12, 3, slab), f(8, 15, 7, 6, 1.5, peach), f(19, 14.6, 52, 2.6, 1.3, bar), f(19, 19, 22, 2.2, 1.1, one),
        f(4, 26, 112, 12, 3, slab), f(8, 29, 7, 6, 1.5, green), f(19, 28.6, 40, 2.6, 1.3, bar), f(19, 33, 18, 2.2, 1.1, two),
        f(5, 44, 24, 2.8, 1.4, repo),
        f(4, 51, 112, 12, 3, slab), f(8, 54, 7, 6, 1.5, pull), f(19, 53.6, 58, 2.6, 1.3, bar), f(19, 58, 22, 2.2, 1.1, one),
        f(4, 65, 112, 12, 3, slab), f(8, 68, 7, 6, 1.5, gold), f(19, 67.6, 46, 2.6, 1.3, bar), f(19, 72, 18, 2.2, 1.1, two)])
    static let groupBoth = Thumb(size: CGSize(width: 120, height: 80), marks: [
        o(5, 3, 8, 5.6, 1, one, 1.3), f(17, 4.4, 34, 2.8, 1.4, one),
        f(13, 12.5, 26, 2.4, 1.2, repo),
        f(12, 17, 104, 10, 3, slab), f(16, 19, 7, 6, 1.5, peach), f(27, 20.7, 48, 2.6, 1.3, bar),
        f(13, 31.5, 20, 2.4, 1.2, repo),
        f(12, 36, 104, 10, 3, slab), f(16, 38, 7, 6, 1.5, pull), f(27, 39.7, 56, 2.6, 1.3, bar),
        o(5, 50.6, 8, 5.6, 1, two, 1.3), f(17, 52, 28, 2.8, 1.4, two),
        f(13, 60.5, 26, 2.4, 1.2, repo),
        f(12, 65, 104, 10, 3, slab), f(16, 67, 7, 6, 1.5, green), f(27, 68.7, 40, 2.6, 1.3, bar)])
    static let leadSession = Thumb(size: CGSize(width: 120, height: 50), marks: [
        f(6, 6, 9, 8, 2, peach), f(20, 8.5, 60, 3, 1.5, bar),
        f(20, 19, 66, 2.6, 1.3, pull), f(98, 17.8, 15, 5, 2.5, pass),
        f(20, 28, 52, 2.6, 1.3, pull), f(98, 26.8, 15, 5, 2.5, fail),
        f(20, 37, 60, 2.6, 1.3, pull), f(98, 35.8, 15, 5, 2.5, pass)])
    static let leadPullRequest = Thumb(size: CGSize(width: 120, height: 50), marks: [
        f(6, 8, 74, 3, 1.5, pull), f(98, 7, 15, 5, 2.5, pass),
        f(14, 17, 9, 8, 2, peach), f(28, 19.5, 50, 2.6, 1.3, bar),
        f(14, 30, 9, 8, 2, green), f(28, 32.5, 40, 2.6, 1.3, bar)])
    static let wrap: Thumb = {
        // The long first line turns the corner onto the next one.
        var turn = Path()
        turn.move(to: CGPoint(x: 112, y: 10.5)); turn.addLine(to: CGPoint(x: 112, y: 15.5))
        turn.addArc(center: CGPoint(x: 109, y: 15.5), radius: 3, startAngle: .zero, endAngle: .degrees(90), clockwise: false)
        turn.addLine(to: CGPoint(x: 61, y: 18.5))
        return Thumb(size: CGSize(width: 120, height: 60), ground: Crab.well, marks: [
            f(8, 9, 104, 3, 1.5, bar), f(8, 17, 46, 3, 1.5, bar),
            .path(turn, Crab.cyan), .path(line([(64, 15.5), (60.5, 18.5), (64, 21.5)]), Crab.cyan),
            f(8, 31, 70, 3, 1.5, grey), f(8, 42, 104, 3, 1.5, grey), f(8, 50, 30, 3, 1.5, grey)])
    }()
    static let scroll = Thumb(size: CGSize(width: 120, height: 60), ground: Crab.well, marks: [
        f(8, 9, 112, 3, 0, bar), f(8, 20, 70, 3, 1.5, grey), f(8, 31, 112, 3, 0, grey), f(8, 42, 52, 3, 1.5, grey),
        .path(line([(104, 21.5), (112, 21.5)], [(109, 18.5), (112, 21.5), (109, 24.5)]), Crab.cyan),
        f(8, 52, 104, 3, 1.5, "232B35"), f(8, 52, 44, 3, 1.5, accent)])
    static func side(right: Bool) -> Thumb {
        let list: CGFloat = right ? 82 : 11, lines: CGFloat = right ? 14 : 50
        return Thumb(size: CGSize(width: 120, height: 60), marks: [
            f(6, 6, 108, 48, 5, "0B0F14"), o(6, 6, 108, 48, 5, "4A5563", 1.2),
            f(right ? 77.4 : 6.6, 6.6, 36, 46.8, 4.4, slab),
            f(list, 13, 26, 6, 2, "4A5563"), f(list, 23, 26, 6, 2, "d7af5f"), f(list, 33, 26, 6, 2, "4A5563"),
            f(lines, 14, 52, 2.6, 1.3, grey), f(lines, 21, 36, 2.6, 1.3, grey), f(lines, 28, 46, 2.6, 1.3, pass), f(lines, 35, 30, 2.6, 1.3, grey)])
    }
}
extension GroupBy {
    var thumb: Thumb { [.one: Thumb.groupOne, .computer: .groupComputer, .project: .groupProject, .both: .groupBoth][self]! }
}

/// The least height and width of a control in a panel. A phone draws a sheet at about 96%
/// of its size until it is fully raised, so 46 keeps each control at 44 points on screen.
let panelTouch: CGFloat = 46

/// A drawn choice: a thumbnail, its name and sometimes a second line. The chosen tile
/// has the accent border and a faint accent wash.
struct PickerTile<Art: View>: View {
    var label: String
    var caption = ""
    var selected: Bool
    /// Four tiles share a phone's width: tighter padding and a centred name.
    var compact = false
    var identifier: String?
    var action: () -> Void
    @ViewBuilder var art: Art
    var body: some View {
        let shape = RoundedRectangle(cornerRadius: compact ? 10 : 12)
        Button(action: action) {
            VStack(alignment: compact ? .center : .leading, spacing: compact ? 5 : 6) {
                art
                Text(label).font(.system(size: compact ? 12 : 14, weight: .semibold)).foregroundStyle(Crab.text).multilineTextAlignment(compact ? .center : .leading).fixedSize(horizontal: false, vertical: true)
                if !caption.isEmpty { Text(caption).font(.system(size: 12)).foregroundStyle(Crab.dim).fixedSize(horizontal: false, vertical: true) }
            }.padding(compact ? 5 : 8).frame(maxWidth: .infinity, minHeight: panelTouch, maxHeight: .infinity, alignment: compact ? .top : .topLeading)
                .background { ZStack { shape.fill(Crab.tile); shape.fill(Crab.cyan.opacity(selected ? 0.13 : 0)) } }
                .overlay { shape.strokeBorder(selected ? Crab.cyan : Crab.line, lineWidth: 2) }.contentShape(shape)
        }.buttonStyle(.plain).accessibilityLabel(label).accessibilityHint(caption).accessibilityIdentifier(identifier ?? label)
            .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
    }
}

/// A switch drawn as a chip: a check box, sometimes a coloured dot, and a name.
struct ToggleChip: View {
    var label: String, on: Bool
    var tone: Color?
    var action: () -> Void
    var body: some View {
        let shape = RoundedRectangle(cornerRadius: 10), box = RoundedRectangle(cornerRadius: 4)
        Button(action: action) {
            HStack(spacing: 7) {
                box.fill(on ? Crab.cyan : .clear).overlay { box.strokeBorder(on ? Crab.cyan : Crab.box, lineWidth: 2) }.frame(width: 14, height: 14)
                if let tone { Circle().fill(tone).frame(width: 8, height: 8) }
                Text(label).font(.system(size: 13)).foregroundStyle(on ? Crab.text : Crab.dim).lineLimit(1)
            }.padding(.horizontal, 11).frame(minHeight: panelTouch)
                .background { ZStack { shape.fill(Crab.tile.opacity(on ? 1 : 0)); shape.fill(Crab.cyan.opacity(on ? 0.13 : 0)) } }
                .overlay { shape.strokeBorder(on ? Crab.cyan : Crab.line, lineWidth: 2) }.contentShape(shape)
        }.buttonStyle(.plain).accessibilityLabel(label).accessibilityIdentifier(label)
            .accessibilityValue(on ? "On" : "Off").accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
    }
}

/// One choice from a short row. The chosen segment is filled with the accent.
struct Segments<Value: Hashable>: View {
    /// Names the row for tests and VoiceOver: "Row spacing Tight".
    var name: String
    var options: [(Value, String)]
    @Binding var selection: Value
    var size: CGFloat = 12
    var color: (Value) -> Color? = { _ in nil }
    /// A row with many or long names scrolls sideways rather than squeezing them.
    var scrolls = false
    var body: some View {
        Group {
            if scrolls { ViewThatFits(in: .horizontal) { row(fill: true); ScrollView(.horizontal, showsIndicators: false) { row(fill: false) } } }
            else { row(fill: true) }
        }.padding(3).background(Crab.ink, in: RoundedRectangle(cornerRadius: 10)).accessibilityElement(children: .contain).accessibilityLabel(name)
    }
    private func row(fill: Bool) -> some View {
        HStack(spacing: 3) {
            ForEach(options.indices, id: \.self) { index in
                let value = options[index].0, label = options[index].1, on = value == selection
                Button { selection = value } label: {
                    Text(label).font(.system(size: size, weight: on ? .bold : .semibold)).lineLimit(1).minimumScaleFactor(fill && !scrolls ? 0.75 : 1)
                        .padding(.horizontal, fill ? 6 : 14).frame(maxWidth: fill ? .infinity : nil, minHeight: panelTouch)
                        .foregroundStyle(on ? Crab.ink : color(value) ?? Crab.soft).background(on ? Crab.cyan : .clear, in: RoundedRectangle(cornerRadius: 7)).contentShape(Rectangle())
                }.buttonStyle(.plain).accessibilityLabel(label).accessibilityIdentifier(name + " " + label).accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
            }
        }
    }
}

/// The activity window as a ruler: every band up to the choice is lit in the board's own
/// time-band colours.
struct TimeRuler: View {
    @Binding var hours: Int
    var body: some View {
        let chosen = Preferences.ageChoices.firstIndex(of: hours) ?? 5
        HStack(spacing: 4) {
            ForEach(Array(Preferences.ageChoices.enumerated()), id: \.element) { index, choice in
                let on = index == chosen
                Button { hours = choice } label: {
                    VStack(spacing: 7) {
                        Capsule().fill(index <= chosen ? Crab.color(Preferences.agePalette(choice)) : Crab.hex(slab)).frame(height: 8)
                        Text(Preferences.ageLabel(choice)).font(.system(size: 12, weight: on ? .bold : .medium)).foregroundStyle(on ? Crab.text : Crab.dim).lineLimit(1)
                    }.padding(.vertical, 7).padding(.horizontal, 4).frame(maxWidth: .infinity, minHeight: 52)
                        .overlay { RoundedRectangle(cornerRadius: 9).strokeBorder(on ? Crab.cyan : .clear, lineWidth: 2) }.contentShape(Rectangle())
                }.buttonStyle(.plain).accessibilityLabel(Preferences.ageLabel(choice)).accessibilityHint(Preferences.ageMenuLabel(choice))
                    .accessibilityIdentifier("Ruler " + Preferences.ageLabel(choice)).accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
            }
        }
    }
}

/// A smaller and a larger "A" either side of the size in points.
struct TextSizeStepper: View {
    /// Names the size readout for tests: "Board text size".
    var name: String
    @Binding var value: Int
    var range: ClosedRange<Int>
    var body: some View {
        HStack(spacing: 8) {
            step("Smaller text", size: 13, to: value - 1)
            Text("\(value) pt").font(.system(size: 14, weight: .semibold)).monospacedDigit().foregroundStyle(Crab.cyan).frame(width: 42).accessibilityIdentifier(name)
            step("Larger text", size: 19, to: value + 1)
        }
    }
    private func step(_ label: String, size: CGFloat, to next: Int) -> some View {
        let shape = RoundedRectangle(cornerRadius: 10)
        return Button { value = next } label: {
            Text("A").font(.system(size: size, weight: .semibold)).foregroundStyle(Crab.text).frame(width: panelTouch, height: panelTouch).background(Crab.tile, in: shape).overlay { shape.strokeBorder(Crab.line) }
        }.buttonStyle(.plain).disabled(!range.contains(next)).opacity(range.contains(next) ? 1 : 0.4).accessibilityLabel(label).accessibilityIdentifier(label)
    }
}

/// Lays chips out in rows, starting a new row when the next chip does not fit.
struct FlowLayout: Layout {
    var spacing: CGFloat = 8
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let frames = arrange(subviews, width: proposal.width ?? .infinity)
        return CGSize(width: proposal.width ?? frames.map(\.maxX).max() ?? 0, height: frames.map(\.maxY).max() ?? 0)
    }
    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        for (subview, frame) in zip(subviews, arrange(subviews, width: bounds.width)) {
            subview.place(at: CGPoint(x: bounds.minX + frame.minX, y: bounds.minY + frame.minY), proposal: ProposedViewSize(frame.size))
        }
    }
    private func arrange(_ subviews: Subviews, width: CGFloat) -> [CGRect] {
        var x: CGFloat = 0, y: CGFloat = 0, tallest: CGFloat = 0
        return subviews.map { subview in
            let size = subview.sizeThatFits(.unspecified)
            if x > 0 && x + size.width > width { x = 0; y += tallest + spacing; tallest = 0 }
            defer { x += size.width + spacing; tallest = max(tallest, size.height) }
            return CGRect(origin: CGPoint(x: x, y: y), size: size)
        }
    }
}

/// The two sliders beside the word "View".
struct SlidersIcon: View {
    /// The knobs take the button's own colour, so the lines seem to pass behind them.
    var knob: Color
    var body: some View {
        Canvas { context, size in
            context.scaleBy(x: size.width / 16, y: size.height / 16)
            let style = StrokeStyle(lineWidth: 1.6, lineCap: .round)
            context.stroke(line([(2, 4.5), (14, 4.5)], [(2, 11.5), (14, 11.5)]), with: .foreground, style: style)
            for center in [CGPoint(x: 6, y: 4.5), CGPoint(x: 10.5, y: 11.5)] {
                let circle = Path(ellipseIn: CGRect(x: center.x - 2, y: center.y - 2, width: 4, height: 4))
                context.fill(circle, with: .color(knob)); context.stroke(circle, with: .foreground, style: style)
            }
        }.frame(width: 14, height: 14).accessibilityHidden(true)
    }
}
/// Opens a screen's View panel. Filled with the accent while the panel is open.
struct ViewButton: View {
    var open: Bool
    var identifier: String
    var action: () -> Void
    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) { SlidersIcon(knob: open ? Crab.cyan : Crab.panel); Text("View").font(.system(size: 13, weight: .bold)).fixedSize() }
                .foregroundStyle(open ? Crab.ink : Crab.text).padding(.horizontal, 12).frame(minHeight: 44)
                .background(open ? Crab.cyan : Crab.panel, in: Capsule()).overlay { Capsule().strokeBorder(open ? .clear : Crab.line) }.contentShape(Capsule())
        }.buttonStyle(.plain).accessibilityLabel("View").accessibilityIdentifier(identifier).accessibilityAddTraits(open ? [.isButton, .isSelected] : .isButton)
    }
}
/// A filter chip's face: sometimes a dot, the current choice, and a small chevron.
struct FilterChip: View {
    var text: String
    var dot: Color?
    var color = Crab.text
    var body: some View {
        HStack(spacing: 6) {
            if let dot { Circle().fill(dot).frame(width: 8, height: 8) }
            Text(text).font(.system(size: 13, weight: .semibold)).foregroundStyle(color).lineLimit(1).fixedSize()
            Image(systemName: "chevron.down").font(.system(size: 9, weight: .semibold)).foregroundStyle(Crab.dim).accessibilityHidden(true)
        }.padding(.horizontal, 10).frame(minHeight: 44).background(Crab.panel, in: Capsule()).overlay { Capsule().strokeBorder(Crab.line) }.contentShape(Capsule())
    }
}

/// A panel's title and Done, which stay put above its scrolling sections. A View panel names
/// the screen it changes in the brand gold.
struct PanelHeader: View {
    var title = "View"
    var scope = ""
    var size: CGFloat = 17
    var done: () -> Void
    var body: some View {
        HStack {
            (Text(title).fontWeight(.bold) + Text(scope.isEmpty ? "" : " " + scope).fontWeight(.medium).foregroundColor(Crab.orange)).font(.system(size: size)).foregroundStyle(Crab.text)
                .accessibilityAddTraits(.isHeader).accessibilityIdentifier(scope.isEmpty ? title : title + " " + scope)
            Spacer()
            Button("Done", action: done).font(.system(size: 16, weight: .semibold)).foregroundStyle(Crab.cyan).frame(minWidth: panelTouch, minHeight: panelTouch).contentShape(Rectangle())
        }.frame(minHeight: 48).padding(.horizontal, 16).padding(.top, 6)
    }
}
struct PanelSection<Content: View>: View {
    var title: String
    @ViewBuilder var content: Content
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title).font(.system(size: 15, weight: .semibold)).foregroundStyle(Crab.text).accessibilityAddTraits(.isHeader)
            content
        }.frame(maxWidth: .infinity, alignment: .leading)
    }
}
struct PanelNote: View {
    var text: String
    init(_ text: String) { self.text = text }
    var body: some View { Text(text).font(.system(size: 12)).foregroundStyle(Crab.dim).fixedSize(horizontal: false, vertical: true) }
}
