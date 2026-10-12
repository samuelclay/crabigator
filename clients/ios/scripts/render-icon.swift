import Foundation
import CoreGraphics
import ImageIO
let root = CommandLine.arguments[1]
let points: [(CGFloat, CGFloat)] = [(12,19),(5,15),(3,5),(9,9),(14,4),(15,14),(19,18),(29,18),(33,14),(34,4),(39,9),(45,5),(43,15),(36,19),(38,25),(45,23),(46,27),(38,29),(37,32),(45,35),(43,39),(34,35),(31,39),(17,39),(14,35),(5,39),(3,35),(11,32),(10,29),(2,27),(3,23),(10,25)]
func render(_ size: Int, icon: Bool, path: String) {
    let context = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: size * 4, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
    context.setFillColor(CGColor(red: 16/255, green: 20/255, blue: 25/255, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: size, height: size))
    let scale = CGFloat(size) / (icon ? 64 : 48), offset = icon ? CGFloat(size) * 0.125 : 0
    for (i, point) in points.enumerated() {
        let p = CGPoint(x: point.0 * scale + offset, y: CGFloat(size) - point.1 * scale - offset)
        if i == 0 { context.move(to: p) } else { context.addLine(to: p) }
    }
    context.closePath(); context.setFillColor(CGColor(red: 1, green: 170/255, blue: 119/255, alpha: 1)); context.fillPath()
    context.setFillColor(CGColor(red: 23/255, green: 27/255, blue: 32/255, alpha: 1))
    for x: CGFloat in [17,27] { context.fill(CGRect(x: x * scale + offset, y: CGFloat(size) - 27 * scale - offset, width: 4 * scale, height: 4 * scale)) }
    let destination = CGImageDestinationCreateWithURL(URL(fileURLWithPath: path) as CFURL, "public.png" as CFString, 1, nil)!
    CGImageDestinationAddImage(destination, context.makeImage()!, nil)
    precondition(CGImageDestinationFinalize(destination))
}
render(1024, icon: true, path: root + "/AppIcon.appiconset/AppIcon.png")
render(192, icon: false, path: root + "/Crabigator.imageset/Crabigator.png")
