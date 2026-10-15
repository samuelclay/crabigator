// Draws every Crabigator brand image from two pixel grids: the crab and the wordmark.
// Edit a grid, then run from anywhere: swift scripts/render-brand.swift
// It rewrites the iOS and Android icons and logos, the dashboard's SVGs, and assets/brand/.
import Foundation
import CoreGraphics
import ImageIO

// 32×32 cells. P peach, C coral extrusion, L eye highlight, O eyes. The extrusion
// falls one cell down and left, like the wordmark, and skips the claw and leg gaps.
let crabGrid = """
................................
................................
................................
................................
................................
................................
................................
.......CPPP.........CPPP........
......CPPPP.P.....CP.PPPP.......
.....CPPPP..P.....CP..PPPP......
.....CPPP..PP.....CPP..PPP......
.....CPPPPPPP.....CPPPPPPP......
.....CPPPPPP......CCPPPPPP......
.....CCPPPP..PPPPPP..PPPP.......
......CCPP..PPPPPPPP..PP........
.......CCPPPPPPPPPPPPPP.........
........CCPPPLOPPLOPPP..........
..........PPPOOPPOOPPP..........
........CPPPPPPPPPPPPPP.........
.......CPP.PPPPPPPPPP.PP........
.......CP..PPPPPPPPPP..P........
.......CP..PCCCCCCCCP..P........
.......C..CP.......CP.C.........
..........C........C............
................................
................................
................................
................................
................................
................................
................................
................................
"""

// 220×23 cells, from the "Leftward Regular" wordmark: peach faces with a two-cell
// coral extrusion down and to the left.
let wordGrid = """
....CPPPPPPPPPPP........CPPPPPPPPPPPPPPP..........CPPPPPPPPPPP........CPPPPPPPPPPPPPPP.......CPPPPPPPPPPP........CPPPPPPPPPPP...........CPPPPPPPPPPP........CPPPPPPPPPPPPPPPPP........CPPPPPPPPPPP........CPPPPPPPPPPPPPPP..
...CPPPPPPPPPPPPP......CCPPPPPPPPPPPPPPPP........CPPPPPPPPPPPPP......CCPPPPPPPPPPPPPPPP.....CCPPPPPPPPPPP.......CPPPPPPPPPPPPP.........CPPPPPPPPPPPPP......CCPPPPPPPPPPPPPPPPP.......CPPPPPPPPPPPPP......CCPPPPPPPPPPPPPPPP.
..CPPPPPPPPPPPPPPP.....CCPPPPPPPPPPPPPPPPP......CPPPPPPPPPPPPPPP.....CCPPPPPPPPPPPPPPPPP....CCPPPPPPPPPPP......CPPPPPPPPPPPPPPP.......CPPPPPPPPPPPPPPP.....CCPPPPPPPPPPPPPPPPP......CPPPPPPPPPPPPPPP.....CCPPPPPPPPPPPPPPPPP
.CPPPPPCCCCCCCPPPPP....CCPPPCCCCCCCCCCCPPP.....CPPPPPCCCCCCCPPPPP....CCPPPCCCCCCCCCCCPPP....CCCCCCPPPCCC......CPPPPPCCCCCCCPPPPP.....CPPPPPCCCCCCCPPPPP....CCCCCCCCCPPPCCCCCC......CPPPPPCCCCCCCPPPPP....CCPPPCCCCCCCCCCCPPP
CCPPPPCCCCCCCCCPPPP....CCPPP..........CPPP....CCPPPP.......CCPPPP....CCPPP..........CPPP....CCCCCCPPPCC......CCPPPPCCCCCCCCCPPPP....CCPPPP.......CCPPPP....CCCCCCCCCPPPCCCCC......CCPPPP.......CCPPPP....CCPPP..........CPPP
CCPPP.......CCCCCC.....CCPPP..........CPPP....CCPPP.........CCPPP....CCPPP..........CPPP........CCPPP........CCPPP.......CCCCCC.....CCPPP.........CCPPP...........CCPPP...........CCPPP.........CCPPP....CCPPP..........CPPP
CCPPP........CCCC......CCPPP..........CPPP....CCPPP..........CPPP....CCPPP..........CPPP........CCPPP........CCPPP........CCCC......CCPPP..........CPPP...........CCPPP...........CCPPP..........CPPP....CCPPP..........CPPP
CCPPP..................CCPPP..........CPPP....CCPPP..........CPPP....CCPPP..........CPPP........CCPPP........CCPPP..................CCPPP..........CPPP...........CCPPP...........CCPPP..........CPPP....CCPPP..........CPPP
CCPPP..................CCPPP..........CPPP....CCPPP..........CPPP....CCPPP..........CPPP........CCPPP........CCPPP..................CCPPP..........CPPP...........CCPPP...........CCPPP..........CPPP....CCPPP..........CPPP
CCPPP..................CCPPPPPPPPPPPPPPPPP....CCPPP..........CPPP....CCPPPPPPPPPPPPPPPPP........CCPPP........CCPPP..................CCPPP..........CPPP...........CCPPP...........CCPPP..........CPPP....CCPPPPPPPPPPPPPPPPP
CCPPP..................CCPPPPPPPPPPPPPPPP.....CCPPP..........CPPP....CCPPPPPPPPPPPPPPPP.........CCPPP........CCPPP....CPPPPPPPPP....CCPPP..........CPPP...........CCPPP...........CCPPP..........CPPP....CCPPPPPPPPPPPPPPPP.
CCPPP..................CCPPPPPPPPPPPPPPP......CCPPPPPPPPPPPPPPPPP....CCPPPPPPPPPPPPPPPPP........CCPPP........CCPPP...CCPPPPPPPPP....CCPPPPPPPPPPPPPPPPP...........CCPPP...........CCPPP..........CPPP....CCPPPPPPPPPPPPPPP..
CCPPP..................CCPPPCCCPPPCCCCC.......CCPPPPPPPPPPPPPPPPP....CCPPPCCCCCCCCCCCPPP........CCPPP........CCPPP...CCPPPPPPPPP....CCPPPPPPPPPPPPPPPPP...........CCPPP...........CCPPP..........CPPP....CCPPPCCCPPPCCCCC...
CCPPP..................CCPPPCCCCPPPCCC........CCPPPPPPPPPPPPPPPPP....CCPPP..........CPPP........CCPPP........CCPPP...CCCCCCCCPPP....CCPPPPPPPPPPPPPPPPP...........CCPPP...........CCPPP..........CPPP....CCPPPCCCCPPPCCC....
CCPPP..................CCPPP.CCCCPPP..........CCPPPCCCCCCCCCCCPPP....CCPPP..........CPPP........CCPPP........CCPPP...CCCCCCCCPPP....CCPPPCCCCCCCCCCCPPP...........CCPPP...........CCPPP..........CPPP....CCPPP.CCCCPPP......
CCPPP..................CCPPP..CCCCPPP.........CCPPPCCCCCCCCCCCPPP....CCPPP..........CPPP........CCPPP........CCPPP.........CCPPP....CCPPPCCCCCCCCCCCPPP...........CCPPP...........CCPPP..........CPPP....CCPPP..CCCCPPP.....
CCPPPP........CPPPP....CCPPP...CCCCPPP........CCPPP.........CCPPP....CCPPP..........CPPP........CCPPP........CCPPPP........CPPPP....CCPPP.........CCPPP...........CCPPP...........CCPPPP........CPPPP....CCPPP...CCCCPPP....
CCPPPPP......CPPPPP....CCPPP....CCCCPPP.......CCPPP.........CCPPP....CCPPP..........CPPP........CCPPP........CCPPPPP......CPPPPP....CCPPP.........CCPPP...........CCPPP...........CCPPPPP......CPPPPP....CCPPP....CCCCPPP...
CCCPPPPPPPPPPPPPPP.....CCPPP.....CCCCPPP......CCPPP.........CCPPP....CCPPPPPPPPPPPPPPPPP.....CPPPPPPPPPPP....CCCPPPPPPPPPPPPPPP.....CCPPP.........CCPPP...........CCPPP...........CCCPPPPPPPPPPPPPPP.....CCPPP.....CCCCPPP..
CCCCPPPPPPPPPPPPP......CCPPP......CCCCPPP.....CCPPP.........CCPPP....CCPPPPPPPPPPPPPPPP.....CCPPPPPPPPPPP....CCCCPPPPPPPPPPPPP......CCPPP.........CCPPP...........CCPPP...........CCCCPPPPPPPPPPPPP......CCPPP......CCCCPPP.
.CCCCPPPPPPPPPPP.......CCPPP.......CCCCPPP....CCPPP.........CCPPP....CCPPPPPPPPPPPPPPP......CCPPPPPPPPPPP.....CCCCPPPPPPPPPPP.......CCPPP.........CCPPP...........CCPPP............CCCCPPPPPPPPPPP.......CCPPP.......CCCCPPP
..CCCCCCCCCCCCC........CCCC.........CCCCC.....CCCC..........CCCC.....CCCCCCCCCCCCCCCC.......CCCCCCCCCCCC.......CCCCCCCCCCCCC........CCCC..........CCCC............CCCC..............CCCCCCCCCCCCC........CCCC.........CCCCC.
...CCCCCCCCCCC.........CCC...........CCC......CCC...........CCC......CCCCCCCCCCCCCCC........CCCCCCCCCCC.........CCCCCCCCCCC.........CCC...........CCC.............CCC................CCCCCCCCCCC.........CCC...........CCC..
"""

let colors: [Character: UInt32] = ["P": 0xFFB98A, "C": 0xE8775A, "L": 0xFFE1B5, "O": 0x080E14]
/// The staff dashboard's purple crab.
let staffColors: [Character: UInt32] = ["P": 0xC084FC, "C": 0x8B5CF6, "L": 0xF3E8FF, "O": 0x080E14]
let ink: UInt32 = 0x141A21
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
let fm = FileManager.default

// MARK: Art

struct Art {
    var width: Int, height: Int
    var cells: [Int: Character] = [:]  // y * width + x
    init(width: Int, height: Int) { self.width = width; self.height = height }
    init(_ text: String) {
        let rows = text.split(separator: "\n").map(Array.init)
        precondition(rows.allSatisfy { $0.count == rows[0].count }, "ragged grid")
        self.init(width: rows[0].count, height: rows.count)
        for (y, row) in rows.enumerated() { for (x, c) in row.enumerated() where c != "." { cells[y * width + x] = c } }
    }
    subscript(x: Int, y: Int) -> Character? { cells[y * width + x] }
    func crop(x: Int, y: Int, width w: Int, height h: Int) -> Art {
        var out = Art(width: w, height: h)
        for (i, c) in cells { let cx = i % width - x, cy = i / width - y; if (0..<w).contains(cx) && (0..<h).contains(cy) { out.cells[cy * w + cx] = c } }
        return out
    }
    /// The smallest box around the drawn cells.
    var trimmed: Art {
        let xs = cells.keys.map { $0 % width }, ys = cells.keys.map { $0 / width }
        return crop(x: xs.min()!, y: ys.min()!, width: xs.max()! - xs.min()! + 1, height: ys.max()! - ys.min()! + 1)
    }
    func scaled(_ n: Int) -> Art {
        var out = Art(width: width * n, height: height * n)
        for (i, c) in cells { for dy in 0..<n { for dx in 0..<n { out.cells[(i / width * n + dy) * out.width + i % width * n + dx] = c } } }
        return out
    }
    mutating func draw(_ art: Art, x: Int, y: Int) {
        for (i, c) in art.cells { cells[(i / art.width + y) * width + i % art.width + x] = c }
    }
    func filter(_ keep: (Character) -> Bool) -> Art { var out = self; out.cells = cells.filter { keep($0.value) }; return out }
    /// Rows of same-colored runs, as (x, y, length, color).
    func runs(_ palette: [Character: UInt32]) -> [(x: Int, y: Int, n: Int, rgb: UInt32)] {
        var out: [(Int, Int, Int, UInt32)] = []
        for y in 0..<height {
            var x = 0
            while x < width {
                guard let c = self[x, y] else { x += 1; continue }
                var end = x
                while end + 1 < width, let next = self[end + 1, y], palette[next] == palette[c] { end += 1 }
                out.append((x, y, end - x + 1, palette[c]!)); x = end + 1
            }
        }
        return out
    }
}

let crab = Art(crabGrid), word = Art(wordGrid)
/// The crab with even padding: the in-app logo, the favicon, and "icon only".
let crabSquare = crab.crop(x: 3, y: 3, width: 24, height: 24)
/// Crab beside wordmark. The crab is drawn at twice the wordmark's cell size,
/// so their extrusions match; it is 34 cells tall and the words sit centered.
let lockup: Art = {
    let big = crab.trimmed.scaled(2), gap = 8
    var art = Art(width: big.width + gap + word.width, height: big.height)
    art.draw(big, x: 0, y: 0)
    art.draw(word, x: big.width + gap, y: (big.height - word.height) / 2)
    return art
}()

// MARK: Writers

func write(_ data: Data, _ path: String) {
    let url = root.appendingPathComponent(path)
    try! fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
    try! data.write(to: url)
}
func cgColor(_ rgb: UInt32) -> CGColor {
    CGColor(srgbRed: CGFloat(rgb >> 16 & 0xFF) / 255, green: CGFloat(rgb >> 8 & 0xFF) / 255, blue: CGFloat(rgb & 0xFF) / 255, alpha: 1)
}
func image(_ art: Art, cell: Int, background: UInt32? = nil) -> CGImage {
    let w = art.width * cell, h = art.height * cell
    let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!,
                        bitmapInfo: (background == nil ? CGImageAlphaInfo.premultipliedLast : .noneSkipLast).rawValue)!
    if let background { ctx.setFillColor(cgColor(background)); ctx.fill(CGRect(x: 0, y: 0, width: w, height: h)) }
    for run in art.runs(colors) {
        ctx.setFillColor(cgColor(run.rgb))
        // Core Graphics counts rows from the bottom.
        ctx.fill(CGRect(x: run.x * cell, y: h - (run.y + 1) * cell, width: run.n * cell, height: cell))
    }
    return ctx.makeImage()!
}
func pngData(_ image: CGImage) -> Data {
    let data = NSMutableData()
    let dest = CGImageDestinationCreateWithData(data, "public.png" as CFString, 1, nil)!
    CGImageDestinationAddImage(dest, image, nil); precondition(CGImageDestinationFinalize(dest))
    return data as Data
}
func png(_ art: Art, cell: Int, background: UInt32? = nil, _ path: String) {
    write(pngData(image(art, cell: cell, background: background)), path)
}
/// A smooth downscale, for favicon sizes the grid can't hit exactly.
func resized(_ source: CGImage, to size: Int) -> CGImage {
    let ctx = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: size * 4,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.interpolationQuality = .high
    ctx.draw(source, in: CGRect(x: 0, y: 0, width: size, height: size))
    return ctx.makeImage()!
}
func svg(_ art: Art, palette: [Character: UInt32] = colors, background: UInt32? = nil, quote: Character = "\"", hash: String = "#", attributes: String = "") -> String {
    var byColor: [UInt32: String] = [:]
    for run in art.runs(palette) { byColor[run.rgb, default: ""] += "M\(run.x) \(run.y)h\(run.n)v1h-\(run.n)z" }
    let q = String(quote)
    let fill = { (rgb: UInt32) in "fill=\(q)\(hash)" + String(format: "%06X", rgb) + q }
    let bg = background.map { "<rect width=\(q)\(art.width)\(q) height=\(q)\(art.height)\(q) \(fill($0))/>" } ?? ""
    let paths = byColor.keys.sorted().map { "<path \(fill($0)) d=\(q)\(byColor[$0]!)\(q)/>" }.joined()
    return "<svg xmlns=\(q)http://www.w3.org/2000/svg\(q) viewBox=\(q)0 0 \(art.width) \(art.height)\(q) shape-rendering=\(q)crispEdges\(q)\(attributes)>\(bg)\(paths)</svg>"
}
/// The data-URI form the dashboard uses for favicons: single quotes and %23.
func encodedSvg(_ art: Art, palette: [Character: UInt32]) -> String { svg(art, palette: palette, quote: "'", hash: "%23") }
func androidVector(_ art: Art, keep: (Character) -> Bool = { _ in true }, white: Bool = false, size: (Int, Int), viewport: (Int, Int), transform: (x: Double, y: Double, scale: Double)? = nil) -> Data {
    let palette = white ? colors.mapValues { _ in UInt32(0xFFFFFF) } : colors
    var byColor: [UInt32: String] = [:]
    for run in art.filter(keep).runs(palette) { byColor[run.rgb, default: ""] += "M\(run.x),\(run.y)h\(run.n)v1h-\(run.n)z" }
    let indent = transform == nil ? "    " : "        "
    var body = byColor.keys.sorted().map { String(format: "\(indent)<path android:fillColor=\"#%06X\" android:pathData=\"%@\" />\n", $0, byColor[$0]!) }.joined()
    if let t = transform {
        func n(_ v: Double) -> String { v == v.rounded() ? String(Int(v)) : String(v) }
        body = "    <group android:translateX=\"\(n(t.x))\" android:translateY=\"\(n(t.y))\" android:scaleX=\"\(n(t.scale))\" android:scaleY=\"\(n(t.scale))\">\n\(body)    </group>\n"
    }
    return Data("""
    <vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="\(size.0)dp" android:height="\(size.1)dp" android:viewportWidth="\(viewport.0)" android:viewportHeight="\(viewport.1)">
    \(body)</vector>

    """.utf8)
}
/// An .ico that holds PNG images, which every current browser reads.
func ico(_ images: [CGImage]) -> Data {
    let blobs = images.map(pngData)
    var data = Data([0, 0, 1, 0, UInt8(images.count), 0])
    var offset = 6 + 16 * images.count
    for (image, blob) in zip(images, blobs) {
        let side = UInt8(image.width >= 256 ? 0 : image.width)
        data += [side, side, 0, 0, 1, 0, 32, 0]
        withUnsafeBytes(of: UInt32(blob.count).littleEndian) { data += $0 }
        withUnsafeBytes(of: UInt32(offset).littleEndian) { data += $0 }
        offset += blob.count
    }
    blobs.forEach { data += $0 }
    return data
}

// MARK: iOS

let assets = "clients/ios/Crabigator/Resources/Assets.xcassets"
png(crab, cell: 32, background: ink, "\(assets)/AppIcon.appiconset/AppIcon.png")
png(crabSquare, cell: 8, "\(assets)/Crabigator.imageset/Crabigator.png")
// One pixel per cell; the app scales it by whole device pixels with interpolation off.
png(word, cell: 1, "\(assets)/Wordmark.imageset/Wordmark.png")
write(Data(#"{"images": [{"filename": "Wordmark.png", "idiom": "universal", "scale": "1x"}], "info": {"author": "xcode", "version": 1}}"#.utf8), "\(assets)/Wordmark.imageset/Contents.json")

// MARK: Android

let res = "clients/android/app/src/main/res"
// The notification and themed icons use only the body, so the eyes become holes.
let body = { (c: Character) in c == "P" }
write(androidVector(crabSquare, size: (48, 48), viewport: (24, 24)), "\(res)/drawable/ic_crabigator.xml")
write(androidVector(crabSquare, keep: body, white: true, size: (24, 24), viewport: (24, 24)), "\(res)/drawable/ic_notification.xml")
write(androidVector(word, size: (220, 23), viewport: (220, 23)), "\(res)/drawable/ic_wordmark.xml")
// Adaptive icons are 108dp with a 66dp safe circle; 2.25dp cells keep the crab inside it.
let adaptive = (x: 18.0, y: 18.0, scale: 2.25)
write(androidVector(crab, size: (108, 108), viewport: (108, 108), transform: adaptive), "\(res)/drawable/ic_launcher_foreground.xml")
write(androidVector(crab, keep: body, white: true, size: (108, 108), viewport: (108, 108), transform: adaptive), "\(res)/drawable/ic_launcher_monochrome.xml")
write(Data("""
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@drawable/ic_launcher_foreground" />
    <monochrome android:drawable="@drawable/ic_launcher_monochrome" />
</adaptive-icon>

""".utf8), "\(res)/mipmap-anydpi-v26/ic_launcher.xml")
write(Data(String(format: "<resources>\n    <color name=\"ic_launcher_background\">#%06X</color>\n</resources>\n", ink).utf8), "\(res)/values/ic_launcher.xml")

// MARK: Dashboard

let iconsPath = root.appendingPathComponent("workers/crabigator-api/src/dashboard/icons.ts")
let begin = "// BEGIN GENERATED BRAND: edit scripts/render-brand.swift and rerun it.\n"
let end = "// END GENERATED BRAND\n"
let generated = begin + """
export const faviconSvg = `\(encodedSvg(crabSquare, palette: colors))`;
export const faviconStaffSvg = `\(encodedSvg(crabSquare, palette: staffColors))`;
export const brandCrabSvg = `\(svg(crabSquare, attributes: " class=\"brand-crab\" role=\"img\" aria-label=\"Crabigator\""))`;
export const brandLogoSvg = `\(svg(lockup, attributes: " class=\"brand-logo\" role=\"img\" aria-label=\"Crabigator\""))`;

""" + end
var icons = try! String(contentsOf: iconsPath, encoding: .utf8)
let start = icons.range(of: begin)!.lowerBound, stop = icons.range(of: end)!.upperBound
icons.replaceSubrange(start..<stop, with: generated)
try! icons.write(to: iconsPath, atomically: true, encoding: .utf8)

// MARK: Brand kit

let kit = "assets/brand"
let svgs: [(String, String)] = [
    ("crabigator-icon.svg", svg(crabSquare)),
    ("crabigator-app-icon.svg", svg(crab, background: ink)),
    ("crabigator-wordmark.svg", svg(word)),
    ("crabigator-logo.svg", svg(lockup)),
    ("favicon.svg", svg(crabSquare)),
]
for (name, text) in svgs { write(Data((text + "\n").utf8), "\(kit)/\(name)") }
// Whole-pixel cells only, so every PNG stays sharp.
for cell in [1, 2, 4, 8, 16, 32] { png(crabSquare, cell: cell, "\(kit)/png/crabigator-icon-\(24 * cell).png") }
for cell in [1, 2, 4, 8] {
    png(word, cell: cell, "\(kit)/png/crabigator-wordmark-\(word.width * cell)x\(word.height * cell).png")
    png(lockup, cell: cell, "\(kit)/png/crabigator-logo-\(lockup.width * cell)x\(lockup.height * cell).png")
}
for cell in [1, 2, 4, 8, 16, 32] { png(crab, cell: cell, background: ink, "\(kit)/png/crabigator-app-icon-\(32 * cell).png") }
let favicon48 = image(crabSquare, cell: 2)
write(ico([resized(favicon48, to: 16), resized(favicon48, to: 32), favicon48]), "\(kit)/favicon.ico")
write(pngData(image(crab, cell: 6, background: ink)), "\(kit)/png/apple-touch-icon-192.png")
