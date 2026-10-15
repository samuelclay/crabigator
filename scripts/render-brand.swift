// Renders the iOS and Android icons from one pixel crab.
// Run from anywhere: swift clients/scripts/render-icon.swift
import Foundation
import CoreGraphics
import ImageIO

// 32×32 cells. P peach, C coral extrusion, L eye highlight, O eyes. The extrusion
// falls one cell down and left, like the wordmark, and skips the claw and leg gaps.
let sprite = """
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
let rows = sprite.split(separator: "\n").map { Array($0) }
precondition(rows.count == 32 && rows.allSatisfy { $0.count == 32 })
let colors: [Character: UInt32] = ["P": 0xFFB98A, "C": 0xE8775A, "L": 0xFFE1B5, "O": 0x080E14]
let background: UInt32 = 0x141A21
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
let assets = root.appendingPathComponent("ios/Crabigator/Resources/Assets.xcassets")
let res = root.appendingPathComponent("android/app/src/main/res")

/// The in-app logo and the notification icon crop to the crab: 24×24 cells.
let logo = (x: 3, y: 3, size: 24)

func cells(_ keep: (Character) -> Bool) -> [(x: Int, y: Int, char: Character)] {
    rows.enumerated().flatMap { y, row in row.enumerated().compactMap { x, c in keep(c) ? (x, y, c) : nil } }
}

// MARK: iOS PNGs

func png(_ path: URL, size: Int, area: (x: Int, y: Int, size: Int), fill: UInt32?) {
    let context = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: size * 4,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!,
                            bitmapInfo: (fill == nil ? CGImageAlphaInfo.premultipliedLast : .noneSkipLast).rawValue)!
    func color(_ rgb: UInt32) -> CGColor {
        CGColor(srgbRed: CGFloat(rgb >> 16 & 0xFF) / 255, green: CGFloat(rgb >> 8 & 0xFF) / 255, blue: CGFloat(rgb & 0xFF) / 255, alpha: 1)
    }
    if let fill { context.setFillColor(color(fill)); context.fill(CGRect(x: 0, y: 0, width: size, height: size)) }
    let cell = size / area.size
    precondition(cell * area.size == size, "cells must land on whole pixels")
    for c in cells({ $0 != "." }) {
        context.setFillColor(color(colors[c.char]!))
        // Core Graphics counts rows from the bottom.
        context.fill(CGRect(x: (c.x - area.x) * cell, y: size - (c.y - area.y + 1) * cell, width: cell, height: cell))
    }
    let destination = CGImageDestinationCreateWithURL(path as CFURL, "public.png" as CFString, 1, nil)!
    CGImageDestinationAddImage(destination, context.makeImage()!, nil)
    precondition(CGImageDestinationFinalize(destination))
}
png(assets.appendingPathComponent("AppIcon.appiconset/AppIcon.png"), size: 1024, area: (0, 0, 32), fill: background)
png(assets.appendingPathComponent("Crabigator.imageset/Crabigator.png"), size: 192, area: logo, fill: nil)

// MARK: Android vectors

/// One path per color, each row's runs merged into rectangles.
func paths(_ keep: (Character) -> Bool, color: (Character) -> UInt32, dx: Int = 0, dy: Int = 0) -> String {
    var byColor: [UInt32: String] = [:]
    for (y, row) in rows.enumerated() {
        var x = 0
        while x < row.count {
            guard keep(row[x]) else { x += 1; continue }
            let rgb = color(row[x]); var end = x
            while end + 1 < row.count, keep(row[end + 1]), color(row[end + 1]) == rgb { end += 1 }
            byColor[rgb, default: ""] += "M\(x - dx),\(y - dy)h\(end - x + 1)v1h-\(end - x + 1)z"
            x = end + 1
        }
    }
    return byColor.keys.sorted().map { String(format: "    <path android:fillColor=\"#%06X\" android:pathData=\"%@\" />\n", $0, byColor[$0]!) }.joined()
}
func vector(_ name: String, size: Int, viewport: Int, body: String) {
    let xml = """
    <vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="\(size)dp" android:height="\(size)dp" android:viewportWidth="\(viewport)" android:viewportHeight="\(viewport)">
    \(body)</vector>

    """
    try! xml.write(to: res.appendingPathComponent("drawable/\(name).xml"), atomically: true, encoding: .utf8)
}
/// Adaptive icons are 108dp with a 66dp safe circle; 2.25dp cells keep the crab inside it.
func adaptive(_ body: String) -> String {
    "    <group android:translateX=\"18\" android:translateY=\"18\" android:scaleX=\"2.25\" android:scaleY=\"2.25\">\n\(body.split(separator: "\n").map { "    " + $0 }.joined(separator: "\n"))\n    </group>\n"
}
let full = paths({ $0 != "." }, color: { colors[$0]! })
// The notification and themed icons use only the body, so the eyes become holes.
let silhouette = { (c: Character) in c == "P" }
vector("ic_crabigator", size: 48, viewport: logo.size, body: paths({ $0 != "." }, color: { colors[$0]! }, dx: logo.x, dy: logo.y))
vector("ic_notification", size: 24, viewport: logo.size, body: paths(silhouette, color: { _ in 0xFFFFFF }, dx: logo.x, dy: logo.y))
vector("ic_launcher_foreground", size: 108, viewport: 108, body: adaptive(full))
vector("ic_launcher_monochrome", size: 108, viewport: 108, body: adaptive(paths(silhouette, color: { _ in 0xFFFFFF })))
try! FileManager.default.createDirectory(at: res.appendingPathComponent("mipmap-anydpi-v26"), withIntermediateDirectories: true)
try! """
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@drawable/ic_launcher_foreground" />
    <monochrome android:drawable="@drawable/ic_launcher_monochrome" />
</adaptive-icon>

""".write(to: res.appendingPathComponent("mipmap-anydpi-v26/ic_launcher.xml"), atomically: true, encoding: .utf8)
try! String(format: """
<resources>
    <color name="ic_launcher_background">#%06X</color>
</resources>

""", background).write(to: res.appendingPathComponent("values/ic_launcher.xml"), atomically: true, encoding: .utf8)
