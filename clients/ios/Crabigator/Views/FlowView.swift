import SwiftUI
import FlowRender

/// A session's flow scene, run on the device with the desktop's own scene code
/// (crates/flow-render), following the dials the desktop sends in "flow" events:
/// the same scene in the same colours, moving with the same work.
final class FlowPlayer {
    private var scene: OpaquePointer?
    private var key = ""
    private var cells: [UInt32] = []
    private var columns = 0, rows = 0
    private var next = Date.distantPast

    deinit { flow_free(scene) }

    /// Make the scene a "flow" event names, or set the dials of the one already running.
    func apply(_ event: JSON) {
        let name = event.text("scene"), seed = UInt32(truncatingIfNeeded: event.int("seed"))
        let hue = (event["hue"] as? NSNumber)?.doubleValue ?? .nan
        let key = "\(name)|\(seed)|\(hue)"
        if key != self.key {
            flow_free(scene)
            scene = Array(name.utf8).withUnsafeBufferPointer { flow_new($0.baseAddress, $0.count, seed, hue) }
            self.key = key
            next = .distantPast
        }
        let tint: UInt32 = ["smoke": 1, "blue": 2][event.text("tint")] ?? 0
        flow_set(scene, (event["level"] as? NSNumber)?.doubleValue ?? 1, event.bool("waiting"), event.bool("night"), tint,
                 (event["boost"] as? NSNumber)?.doubleValue ?? 0)
    }

    /// The frame to draw at `now` in `size`: stepped when the scene's pace says so.
    func frame(at now: Date, size: CGSize) -> (cells: [UInt32], columns: Int, rows: Int) {
        guard let scene, size.width > 0, size.height > 0 else { return ([], 0, 0) }
        // Cells about 7.5pt tall, shaped like a terminal's (0.6 wide to 1.45 tall).
        let rows = max(3, Int((size.height / 7.5).rounded()))
        let columns = max(6, Int((size.width / (size.height / CGFloat(rows) * 0.6 / 1.45)).rounded()))
        if now >= next || columns != self.columns || rows != self.rows {
            let words = flow_frame(scene, UInt32(columns), UInt32(rows))
            cells = Array(UnsafeBufferPointer(start: words, count: columns * rows * 3))
            self.columns = columns; self.rows = rows
            next = now.addingTimeInterval(Double(flow_pace(scene)) / 1000)
        }
        return (cells, columns, rows)
    }
}

struct FlowView: View {
    var event: JSON
    @State private var player = FlowPlayer()
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 15, paused: reduceMotion)) { timeline in
            Canvas { context, size in
                let frame = player.frame(at: timeline.date, size: size)
                FlowCells.draw(frame.cells, columns: frame.columns, rows: frame.rows, in: &context, size: size)
            }
        }
        .background(Color(uiColor: TerminalText.rgb(0x0B0F14)))
        .onChange(of: signature, initial: true) { player.apply(event) }
        .accessibilityHidden(true)
    }

    private var signature: String {
        ["scene", "seed", "hue", "level", "waiting", "night", "tint", "boost"].map { "\(event[$0] ?? "")" }.joined(separator: "|")
    }
}

/// Draws a frame's cells with flow's 2x4 masks, as its desktop renderer does.
enum FlowCells {
    private static let quad: [UInt32] = [0x20, 0x2598, 0x259D, 0x2580, 0x2596, 0x258C, 0x259E, 0x259B,
                                         0x2597, 0x259A, 0x2590, 0x259C, 0x2584, 0x2599, 0x259F, 0x2588]
    private static let shade: [UInt32: Double] = [0x2591: 0.3, 0x2592: 0.55, 0x2593: 0.8]
    private static let clear: UInt32 = 0x0100_0000

    /// The lit sub-blocks of a glyph (bit x + 2y), or nil when it isn't a block.
    static func mask(_ cp: UInt32) -> Int? {
        if cp == 0x20 || cp == 0xA0 { return 0 }
        if let q = quad.firstIndex(of: cp) { return (q & 1 != 0 ? 5 : 0) | (q & 2 != 0 ? 10 : 0) | (q & 4 != 0 ? 80 : 0) | (q & 8 != 0 ? 160 : 0) }
        if cp > 0x2580 && cp < 0x2588 { return (0xFF << ((4 - Int((Double(cp - 0x2580) / 2 + 0.5).rounded(.down))) * 2)) & 0xFF }
        if cp > 0x2588 && cp < 0x2590 { let c = Int((Double(0x2590 - cp) / 4 + 0.5).rounded(.down)); return c >= 2 ? 0xFF : c == 1 ? 0x55 : 0 }
        if (0x2800...0x28FF).contains(cp) {
            let bits = Int(cp - 0x2800), dots = [[1, 2, 4, 64], [8, 16, 32, 128]]
            var mask = 0
            for x in 0..<2 { for y in 0..<4 where bits & dots[x][y] != 0 { mask |= 1 << (x + 2 * y) } }
            return mask
        }
        return nil
    }

    static func draw(_ cells: [UInt32], columns: Int, rows: Int, in context: inout GraphicsContext, size: CGSize) {
        guard columns > 0, rows > 0, cells.count >= columns * rows * 3 else { return }
        let subColumns = columns * 2, subRows = rows * 4
        let sx = size.width / CGFloat(subColumns), sy = size.height / CGFloat(subRows)
        var sub = [Int64](repeating: -1, count: subColumns * subRows)
        var text: [(CGPoint, UInt32, Character)] = []
        for i in 0..<(columns * rows) {
            let cp = cells[i * 3], fg = cells[i * 3 + 1], bg = cells[i * 3 + 2]
            let cx = (i % columns) * 2, cy = (i / columns) * 4
            let back: Int64 = bg == clear ? -1 : Int64(bg), front: Int64 = fg == clear ? -1 : Int64(fg)
            let mask = mask(cp)
            for y in 0..<4 { for x in 0..<2 {
                var color = back
                if let k = shade[cp] { color = front < 0 ? back : Int64(mix(back < 0 ? 0x0B0F14 : UInt32(back), UInt32(front), k)) }
                else if let mask, mask & (1 << (x + 2 * y)) != 0 { color = front < 0 ? back : front }
                sub[(cy + y) * subColumns + cx + x] = color
            } }
            if mask == nil, front >= 0, let scalar = Unicode.Scalar(cp), !Character(scalar).isWhitespace {
                text.append((CGPoint(x: CGFloat(cx + 1) * sx, y: CGFloat(cy + 2) * sy), UInt32(front), Character(scalar)))
            }
        }
        // Runs of one colour along each sub-row, so the blocks meet without seams.
        for y in 0..<subRows {
            let y0 = (CGFloat(y) * sy).rounded(), y1 = (CGFloat(y + 1) * sy).rounded()
            var x = 0
            while x < subColumns {
                let color = sub[y * subColumns + x]
                var end = x + 1
                while end < subColumns && sub[y * subColumns + end] == color { end += 1 }
                if color >= 0 {
                    let x0 = (CGFloat(x) * sx).rounded()
                    context.fill(Path(CGRect(x: x0, y: y0, width: (CGFloat(end) * sx).rounded() - x0, height: y1 - y0)),
                                 with: .color(Color(uiColor: TerminalText.rgb(UInt32(color)))))
                }
                x = end
            }
        }
        for (point, color, character) in text {
            context.draw(Text(String(character)).font(.system(size: sy * 3.1, design: .monospaced))
                .foregroundColor(Color(uiColor: TerminalText.rgb(color))), at: point)
        }
    }

    private static func mix(_ a: UInt32, _ b: UInt32, _ k: Double) -> UInt32 {
        func channel(_ shift: UInt32) -> UInt32 {
            let from = Double((a >> shift) & 255), to = Double((b >> shift) & 255)
            return UInt32((from + (to - from) * k).rounded())
        }
        return (channel(16) << 16) | (channel(8) << 8) | channel(0)
    }
}
