import UIKit

/// A complete row-formatted ANSI snapshot; this is not an incremental VT emulator.
enum TerminalText {
    /// Recover only clear prose wraps. Capture has no soft-wrap metadata, so
    /// ambiguous boundaries stay intact. Work after ANSI decoding and keep spans.
    static func reflowParagraphs(_ text: NSMutableAttributedString, indent: CGFloat) {
        let rows = text.string.components(separatedBy: "\n")
        let trimmed = rows.map { $0.replacingOccurrences(of: " +$", with: "", options: .regularExpression) }
        let indents = trimmed.map { $0.prefix(while: { $0 == " " }).count }
        let bodies = zip(trimmed, indents).map { String($0.dropFirst($1)) }
        let codeWords: Set<String> = ["import", "from", "export", "return", "throw", "let", "var", "const", "func", "fun", "def", "class", "if", "else", "for", "while", "switch", "case", "git", "npm", "npx", "yarn", "pnpm", "cargo", "make", "curl", "sudo", "pip", "python", "python3", "node"]
        var fence: (Character, Int)?
        let prose = bodies.enumerated().map { index, body -> Bool in
            if let marker = body.first, marker == "`" || marker == "~" {
                let count = body.prefix(while: { $0 == marker }).count
                if count >= 3 {
                    if let open = fence {
                        if marker == open.0 && count >= open.1 && body.dropFirst(count).allSatisfy({ $0 == " " }) { fence = nil }
                    } else { fence = (marker, count) }
                    return false
                }
            }
            // These characters have a known one-cell width. Leave wide glyphs,
            // combining marks and tabs alone instead of guessing terminal columns.
            let singleCell = trimmed[index].unicodeScalars.allSatisfy {
                (32...126).contains($0.value) || (160...255).contains($0.value) || (0x2010...0x2027).contains($0.value)
            }
            return fence == nil && singleCell && [0, 2].contains(indents[index])
                && body.first?.isLetter == true && !body.contains("  ")
                && !codeWords.contains(String(body.split(separator: " ").first ?? ""))
                && body.range(of: #"(?:^| )--?[a-zA-Z]"#, options: .regularExpression) == nil
                && !body.contains(where: { "{}[]=|\\<>`".contains($0) })
        }
        let ruleWidths = trimmed.map { row -> Int? in
            guard row.count >= 40, let first = row.first, "-─━═".contains(first), row.allSatisfy({ $0 == first }) else { return nil }
            return row.count
        }
        var offsets = [Int](), offset = 0
        for row in rows { offsets.append(offset); offset += row.utf16.count + 1 }
        var joins = [NSRange](), inParagraph = false
        for i in rows.indices.dropLast() {
            if !prose[i] { inParagraph = false; continue }
            if !inParagraph { inParagraph = indents[i] == 2 }
            guard inParagraph, prose[i + 1],
                  bodies[i].split(separator: " ").count >= 3,
                  let last = bodies[i].last, !".!?:;-/".contains(last),
                  bodies[i + 1].first?.isLowercase == true,
                  let word = bodies[i + 1].split(separator: " ").first,
                  let previousWord = bodies[i].split(separator: " ").last,
                  previousWord.count < 24, word.count < 24,
                  !previousWord.contains(where: { "/_".contains($0) }), !word.contains(where: { "/_".contains($0) }) else { inParagraph = false; continue }
            var nearby: Int?, conflicting = false
            for distance in 1...64 {
                let before = i >= distance ? ruleWidths[i - distance] : nil
                let after = i + distance < rows.count ? ruleWidths[i + distance] : nil
                if before != nil || after != nil {
                    conflicting = before != nil && after != nil && before != after
                    nearby = after ?? before
                    break
                }
            }
            guard !conflicting else { inParagraph = false; continue }
            let margin: Int
            if let nearby { margin = nearby }
            else {
                // Without a separator, require two prose rows at the same margin.
                let widths = (max(0, i - 32)...min(rows.count - 1, i + 32)).filter { prose[$0] }.map { trimmed[$0].count }
                guard let longest = widths.max(), longest >= 40, widths.filter({ $0 == longest }).count >= 2 else { inParagraph = false; continue }
                margin = longest
            }
            let width = trimmed[i].count
            // Claude leaves the final cell unused on some prose rows, so an
            // exact fit also counts as a wrap (114 + space + "wrapped" at 122).
            guard width <= margin, width >= margin - 12, width + 1 + word.count >= margin,
                  // A flush unindented continuation could be half of a word.
                  indents[i + 1] == 2 || width < margin || rows[i].hasSuffix(" ") else { inParagraph = false; continue }
            let range = NSRange(location: offsets[i], length: offsets[i + 1] + rows[i + 1].utf16.count - offsets[i])
            if let style = text.attribute(.paragraphStyle, at: offsets[i], effectiveRange: nil) as? NSParagraphStyle,
               let indented = style.mutableCopy() as? NSMutableParagraphStyle {
                // Literal spaces indent the first line; layout indents every wrap.
                indented.headIndent = indent
                text.addAttribute(.paragraphStyle, value: indented, range: range)
            }
            let start = offsets[i] + trimmed[i].utf16.count
            joins.append(NSRange(location: start, length: offsets[i + 1] + indents[i + 1] - start))
        }
        for range in joins.reversed() { text.replaceCharacters(in: range, with: " ") }
    }
    static func webURL(_ value: String) -> URL? {
        guard let url = URL(string: value), ["http", "https"].contains(url.scheme?.lowercased() ?? ""),
              let host = url.host, !host.isEmpty else { return nil }
        return url
    }
    static let base: [UInt32] = [0x20242A, 0xEC7777, 0x98C379, 0xE5C07B, 0x82AAFF, 0xC792EA, 0x89DDFF, 0xDEE4EC, 0x697584, 0xFF8B8B, 0xB2E394, 0xFFE09B, 0xA5C6FF, 0xE1B2FF, 0xB4F0FF, 0xFFFFFF]
    static func rgb(_ rgb: UInt32) -> UIColor { UIColor(red: CGFloat((rgb >> 16) & 255) / 255, green: CGFloat((rgb >> 8) & 255) / 255, blue: CGFloat(rgb & 255) / 255, alpha: 1) }
    static func palette(_ n: Int) -> UIColor {
        let n = min(255, max(0, n)); if n < 16 { return rgb(base[n]) }
        if n >= 232 { return UIColor(white: CGFloat(8 + (n - 232) * 10) / 255, alpha: 1) }
        let c = n - 16
        func channel(_ v: Int) -> CGFloat { CGFloat(v == 0 ? 0 : 55 + v * 40) / 255 }
        return UIColor(red: channel(c / 36), green: channel(c / 6 % 6), blue: channel(c % 6), alpha: 1)
    }
    static func parse(_ source: String, size: CGFloat, spacing: CGFloat, wrap: Bool) -> NSAttributedString {
        let out = NSMutableAttributedString(string: "")
        let font = UIFont.monospacedSystemFont(ofSize: size, weight: .regular)
        let boldFont = UIFont.monospacedSystemFont(ofSize: size, weight: .bold)
        let symbolFont = UIFont(name: "NotoSansSymbols2-Regular", size: size) ?? font
        let cell = (" " as NSString).size(withAttributes: [.font: font]).width
        let paragraph = NSMutableParagraphStyle(); paragraph.minimumLineHeight = size * spacing; paragraph.maximumLineHeight = size * spacing; paragraph.lineBreakMode = wrap ? .byWordWrapping : .byClipping
        var fg = rgb(0xDDE3ED), bg: UIColor?, bold = false, dim = false, underline = false, inverse = false
        var link: URL?
        var pending = "", column = 0
        func flush() {
            guard !pending.isEmpty else { return }
            let foreground = inverse ? (bg ?? rgb(0x11151B)) : fg
            var attrs: [NSAttributedString.Key: Any] = [.font: bold ? boldFont : font, .foregroundColor: foreground.withAlphaComponent(dim ? 0.5 : 1), .paragraphStyle: paragraph]
            if let background = inverse ? fg : bg { attrs[.backgroundColor] = background }
            if underline { attrs[.underlineStyle] = NSUnderlineStyle.single.rawValue }
            if let link { attrs[.link] = link }
            let part = NSMutableAttributedString(string: pending, attributes: attrs)
            var offset = 0
            for character in pending {
                let s = String(character), length = s.utf16.count
                if "⏴⏵⏶⏷⏸⏹⏺·✢✳✶✻✽".contains(character) {
                    let width = (s as NSString).size(withAttributes: [.font: symbolFont]).width
                    part.addAttributes([.font: symbolFont, .kern: cell - width], range: NSRange(location: offset, length: length))
                }
                offset += length
            }
            out.append(part); pending = ""
        }
        let chars = Array(source.unicodeScalars); var i = 0
        func append(_ s: String) { pending += s; column += s.count }
        while i < chars.count {
            let c = chars[i]
            if c.value == 27 {
                if i + 1 < chars.count && chars[i + 1] == "[" {
                    let start = i + 2; var end = start
                    while end < chars.count && !(64...126).contains(chars[end].value) { end += 1 }
                    guard end < chars.count else { break }
                    let nums = String(String.UnicodeScalarView(chars[start..<end])).split(separator: ";", omittingEmptySubsequences: false).map { Int($0) ?? 0 }
                    let values = nums.isEmpty ? [0] : nums
                    if chars[end] == "m" {
                        flush(); var p = 0
                        while p < values.count {
                            let n = values[p]
                            switch n {
                            case 0: fg = rgb(0xDDE3ED); bg = nil; bold = false; dim = false; underline = false; inverse = false
                            case 1: bold = true; dim = false
                            case 2: dim = true; bold = false
                            case 22: bold = false; dim = false
                            case 4: underline = true
                            case 24: underline = false
                            case 7: inverse = true
                            case 27: inverse = false
                            case 30...37: fg = palette(n - 30)
                            case 90...97: fg = palette(n - 90 + 8)
                            case 40...47: bg = palette(n - 40)
                            case 100...107: bg = palette(n - 100 + 8)
                            case 39: fg = rgb(0xDDE3ED)
                            case 49: bg = nil
                            case 38, 48:
                                var color: UIColor?
                                if p + 2 < values.count && values[p + 1] == 5 { color = palette(values[p + 2]); p += 2 }
                                else if p + 4 < values.count && values[p + 1] == 2 {
                                    color = UIColor(red: CGFloat(min(255, max(0, values[p + 2]))) / 255, green: CGFloat(min(255, max(0, values[p + 3]))) / 255, blue: CGFloat(min(255, max(0, values[p + 4]))) / 255, alpha: 1); p += 4
                                }
                                if let color { if n == 38 { fg = color } else { bg = color } }
                            default: break
                            }; p += 1
                        }
                    } else if chars[end] == "C" { append(String(repeating: " ", count: min(500, max(1, values[0])))) }
                    else if chars[end] == "G" { append(String(repeating: " ", count: min(500, max(0, values[0] - 1 - column)))) }
                    i = end + 1; continue
                }
                if i + 1 < chars.count && chars[i + 1] == "]" {
                    i += 2
                    let start = i
                    while i < chars.count && chars[i].value != 7 && !(chars[i].value == 27 && i + 1 < chars.count && chars[i + 1] == "\\") { i += 1 }
                    let parts = String(String.UnicodeScalarView(chars[start..<i])).split(separator: ";", maxSplits: 2, omittingEmptySubsequences: false)
                    if parts.count == 3 && parts[0] == "8" { flush(); link = webURL(String(parts[2])) }
                    i += i < chars.count && chars[i].value == 27 ? 2 : 1; continue
                }
                i += 2; continue
            }
            switch c.value {
            case 10: pending += "\n"; column = 0
            case 13: break
            case 9: append(String(repeating: " ", count: 8 - column % 8))
            case 32...: append(String(c))
            default: break
            }; i += 1
        }
        flush()
        if wrap { reflowParagraphs(out, indent: cell * 2) }
        if wrap, let regex = try? NSRegularExpression(pattern: " +(?=\\n|$)") {
            for match in regex.matches(in: out.string, range: NSRange(location: 0, length: out.length)).reversed() { out.deleteCharacters(in: match.range) }
        }
        // Keep separator rows at their captured length, but clip them to the
        // current viewport. Paragraph layout adapts when the device rotates.
        if wrap {
            let clipped = paragraph.mutableCopy() as! NSMutableParagraphStyle
            clipped.lineBreakMode = .byClipping
            let text = out.string as NSString
            var start = 0
            while start < text.length {
                let range = text.lineRange(for: NSRange(location: start, length: 0))
                let row = text.substring(with: range).trimmingCharacters(in: .whitespacesAndNewlines)
                if row.count >= 3, let first = row.first, row.allSatisfy({ $0 == first }) {
                    out.addAttribute(.paragraphStyle, value: clipped, range: range)
                }
                start = NSMaxRange(range)
            }
        }
        // Detect after stripping ANSI, so a color change inside a URL does not split it.
        if let regex = try? NSRegularExpression(pattern: #"https?://[^\s<>"']+"#, options: .caseInsensitive) {
            for match in regex.matches(in: out.string, range: NSRange(location: 0, length: out.length)) {
                var value = (out.string as NSString).substring(with: match.range)
                while let last = value.last, ".,;:!?".contains(last) { value.removeLast() }
                for (open, close) in [("(", ")"), ("[", "]"), ("{", "}")] {
                    while value.hasSuffix(close) && value.filter({ String($0) == close }).count > value.filter({ String($0) == open }).count { value.removeLast() }
                }
                let range = NSRange(location: match.range.location, length: value.utf16.count)
                var embedded = false
                out.enumerateAttribute(.link, in: range) { value, _, _ in if value != nil { embedded = true } }
                if !embedded, let url = webURL(value) { out.addAttribute(.link, value: url, range: range) }
            }
        }
        return out
    }
}
