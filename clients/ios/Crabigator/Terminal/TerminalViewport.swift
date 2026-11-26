import SwiftUI

private struct TerminalPrompt: Equatable {
    let offset: Int
    let excerpt: String

    static func scan(_ rendered: NSAttributedString, plainTranscript: Bool, startingAt base: Int = 0, liveScreen: Bool = false) -> [Self] {
        var offset = base
        let lines = rendered.string.split(separator: "\n", omittingEmptySubsequences: false)
        // Older snapshots lack styling. Only the final Codex input on the live
        // screen can be the composer; history may contain the same words.
        let lastCodexRow = liveScreen && plainTranscript ? lines.lastIndex { $0.hasPrefix("› ") } : nil
        return lines.enumerated().compactMap { index, line in
            defer { offset += line.utf16.count + 1 }
            guard let marker = line.first, "❯›>".contains(marker),
                  line.dropFirst().first?.isWhitespace == true else { return nil }
            let attrs = rendered.attributes(at: offset - base, effectiveRange: nil)
            let bold = (attrs[.font] as? UIFont)?.fontDescriptor.symbolicTraits.contains(.traitBold) == true
            // Codex shades both submitted turns and the composer. Only the
            // composer has a bold ›, including when it contains an unsent draft.
            if marker == "›" && bold { return nil }
            if marker == ">" {
                guard bold else { return nil }
            } else if !plainTranscript && attrs[.backgroundColor] == nil { return nil }
            let text = line.dropFirst().trimmingCharacters(in: .whitespaces)
            guard !text.isEmpty else { return nil }
            if index == lastCodexRow {
                let footer = lines[(index + 1)...].first { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
                if let footer {
                    if footer.contains("for agents") || footer.contains("? for shortcuts") || footer.contains("context left") { return nil }
                } else if text == "Ask Codex to do anything" { return nil }
            }
            return Self(offset: offset, excerpt: String(text.prefix(80)))
        }
    }
}

struct TerminalViewport: UIViewRepresentable {
    var history: String, screen: String, preferences: Preferences
    @Binding var pinned: Bool
    var swipe: ((CGFloat, CGFloat, Bool) -> Void)? = nil
    func makeCoordinator() -> Coordinator { Coordinator(self) }
    func makeUIView(context: Context) -> TerminalScrollView {
        let view = TerminalScrollView()
        context.coordinator.openURL = context.environment.openURL
        view.text.isEditable = false; view.text.isSelectable = true; view.text.backgroundColor = .clear
        view.text.linkTextAttributes = [.foregroundColor: TerminalText.rgb(0x82AAFF), .underlineStyle: NSUnderlineStyle.single.rawValue]
        view.text.accessibilityIdentifier = "terminal-output"; view.backgroundColor = TerminalText.rgb(0x0B0F14)
        view.text.textContainerInset = UIEdgeInsets(top: 16, left: 12, bottom: 16, right: 12)
        view.text.delegate = context.coordinator; view.delegate = context.coordinator
        view.text.keyboardDismissMode = .interactive
        if swipe != nil {
            let pan = UIPanGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.pan(_:)))
            pan.delegate = context.coordinator; view.text.addGestureRecognizer(pan)
            view.text.panGestureRecognizer.require(toFail: pan)
        }
        context.coordinator.view = view
        view.previous.addTarget(context.coordinator, action: #selector(Coordinator.jump(_:)), for: .touchUpInside)
        view.nextPrompt.addTarget(context.coordinator, action: #selector(Coordinator.jump(_:)), for: .touchUpInside)
        view.bottomButton.addTarget(context.coordinator, action: #selector(Coordinator.jumpToBottom), for: .touchUpInside)
        view.onScrollAnimationEnd = { [weak coordinator = context.coordinator] in coordinator?.renderLatest() }
        return view
    }
    static func dismantleUIView(_ view: TerminalScrollView, coordinator: Coordinator) { view.cancelScrollAnimation() }
    func updateUIView(_ view: TerminalScrollView, context: Context) { context.coordinator.parent = self; context.coordinator.openURL = context.environment.openURL; context.coordinator.update() }

    final class Coordinator: NSObject, UITextViewDelegate, UIScrollViewDelegate, UIGestureRecognizerDelegate {
        var parent: TerminalViewport
        var openURL: OpenURLAction?
        weak var view: TerminalScrollView?
        let queue = DispatchQueue(label: "com.newsblur.crabigator.ansi", qos: .userInitiated)
        var sourceHistory = "", sourceScreen = "", style: Preferences?
        var cachedHistory = NSAttributedString(string: ""), cachedSource = "", cachedStyle: Preferences?
        private var cachedPrompts: [TerminalPrompt] = []
        private var prompts: [TerminalPrompt] = []
        private var selected: TerminalPrompt?
        private var applyingFrame = false
        var cachedWidth: CGFloat = 0
        var appliedHistory = "", appliedStyle: Preferences?
        var rendering = false
        var wasPinned: Bool
        var revision = 0, appliedRevision = 0
        init(_ parent: TerminalViewport) { self.parent = parent; wasPinned = parent.pinned }
        func textView(_ textView: UITextView, primaryActionFor textItem: UITextItem, defaultAction: UIAction) -> UIAction? {
            guard case let .link(url) = textItem.content else { return defaultAction }
            return UIAction { [openURL] _ in openURL?(url) }
        }
        func update() {
            guard let view else { return }
            if parent.pinned && !wasPinned {
                view.cancelScrollAnimation()
                view.animateNextFollow = true
            }
            if !parent.pinned {
                view.animateNextFollow = false
                if wasPinned { view.cancelScrollAnimation() }
            }
            wasPinned = parent.pinned
            view.follow = parent.pinned
            if parent.pinned { selected = nil }
            updatePills()
            if sourceHistory == parent.history && sourceScreen == parent.screen && style == parent.preferences {
                if parent.pinned { view.followBottom() }
                renderLatest()
                return
            }
            sourceHistory = parent.history; sourceScreen = parent.screen; style = parent.preferences; revision += 1
            renderLatest()
        }
        fileprivate func renderLatest() {
            // TextKit edits and offset restoration can cancel native deceleration.
            // Keep only the latest source while the user drags or coasts.
            guard !rendering, revision != appliedRevision, let view, !view.isUserScrolling, !view.isAnimatingScroll,
                  let prefs = style else { return }
            rendering = true
            let revision = revision, history = sourceHistory, screen = sourceScreen
            queue.async { [weak self] in
                guard let self else { return }
                if self.cachedSource != history || self.cachedStyle != prefs {
                    self.cachedHistory = TerminalText.parse(history, size: CGFloat(prefs.fontSize), spacing: CGFloat(prefs.lineSpacing) / 100, wrap: prefs.wrap)
                    self.cachedPrompts = TerminalPrompt.scan(self.cachedHistory, plainTranscript: !history.contains("\u{1b}") && !history.contains("────"))
                    self.cachedSource = history; self.cachedStyle = prefs
                    self.cachedWidth = prefs.wrap ? 0 : self.cachedHistory.maximumLineWidth()
                }
                let result = NSMutableAttributedString(attributedString: self.cachedHistory)
                if !history.isEmpty && !history.hasSuffix("\n") { result.append(NSAttributedString(string: "\n")) }
                let historyLength = result.length
                let renderedScreen = TerminalText.parse(screen, size: CGFloat(prefs.fontSize), spacing: CGFloat(prefs.lineSpacing) / 100, wrap: prefs.wrap)
                let prompts = self.cachedPrompts + TerminalPrompt.scan(renderedScreen, plainTranscript: !screen.contains("\u{1b}") && !screen.contains("────"), startingAt: historyLength, liveScreen: true)
                result.append(renderedScreen)
                // Horizontal scrolling preserves the desktop columns. Layout remains native TextKit 2.
                let width = prefs.wrap ? 0 : max(self.cachedWidth, renderedScreen.maximumLineWidth())
                DispatchQueue.main.async { [weak self] in
                    guard let self else { return }
                    self.rendering = false
                    guard self.style == prefs else { self.renderLatest(); return }
                    // A swipe may have started while ANSI parsing was off the main thread.
                    guard let view = self.view, !view.isUserScrolling, !view.isAnimatingScroll else { return }
                    self.applyingFrame = true
                    view.documentWidth = prefs.wrap ? 0 : ceil(width) + 32
                    let text = view.text, offset = text.contentOffset, selection = text.selectedRange
                    let old = text.attributedText ?? NSAttributedString(string: "")
                    // Only the live screen changes on most frames. Always replace its attributes,
                    // even when the characters are unchanged (for example a selection highlight).
                    var prefix = 0
                    if self.appliedStyle == prefs {
                        prefix = self.appliedHistory == history ? min(historyLength, old.length) : old.commonAttributedPrefix(with: result)
                        if prefix > 0 && self.appliedHistory != history {
                            prefix = (old.string as NSString).lineRange(for: NSRange(location: prefix - 1, length: 0)).location
                        }
                    }
                    text.textStorage.beginEditing()
                    text.textStorage.replaceCharacters(in: NSRange(location: prefix, length: old.length - prefix), with: result.attributedSubstring(from: NSRange(location: prefix, length: result.length - prefix)))
                    text.textStorage.endEditing()
                    // VoiceOver needs the text, not thousands of terminal color runs.
                    text.accessibilityAttributedValue = NSAttributedString(string: result.string)
                    self.prompts = prompts
                    self.appliedHistory = history; self.appliedStyle = prefs; self.appliedRevision = revision
                    if selection.location <= result.length { text.selectedRange = NSRange(location: selection.location, length: min(selection.length, result.length - selection.location)) }
                    view.setNeedsLayout(); view.layoutIfNeeded()
                    if self.parent.pinned { view.followBottom() }
                    else if text.contentOffset != offset { text.setContentOffset(offset, animated: false) }
                    self.applyingFrame = false
                    self.updatePills()
                    // Apply completed frames even while newer output arrives. Dropping every
                    // older frame would freeze a busy session until its output paused.
                    if self.revision != revision { self.renderLatest() }
                }
            }
        }
        private func updatePills() {
            guard let view else { return }
            if view.bottomButton.isHidden != view.follow {
                view.bottomButton.isHidden = view.follow
                view.setNeedsLayout()
            }
            // Ignore offsets from text replacement until the viewport is restored.
            guard !applyingFrame else { return }
            var after = prompts.count
            var before = after - 1
            if !view.follow && !prompts.isEmpty {
                if let selected, let current = prompts.firstIndex(of: selected) {
                    before = current - 1
                    after = current + 1
                } else {
                    guard let index = firstPromptAtOrBelowTop(of: view.text) else { return }
                    after = index
                    before = after - 1
                }
            }
            for (button, index, label) in [(view.previous, before, "Previous prompt"), (view.nextPrompt, after, "Next prompt")] {
                guard prompts.indices.contains(index) else { button.isHidden = true; continue }
                let prompt = prompts[index]
                let title = "\(label)   \(index + 1)/\(prompts.count)"
                button.tag = index
                if button.configuration?.title != title || button.configuration?.subtitle != prompt.excerpt {
                    var configuration = button.configuration
                    configuration?.title = title
                    configuration?.subtitle = prompt.excerpt
                    UIView.performWithoutAnimation { button.configuration = configuration }
                    button.accessibilityLabel = "\(label), \(index + 1) of \(prompts.count), \(prompt.excerpt)"
                    view.setNeedsLayout()
                }
                button.isHidden = false
            }
        }
        // Use the same caret geometry as a prompt jump. Asking UITextView for the
        // closest insertion point can return the end of a long transcript.
        private func firstPromptAtOrBelowTop(of text: UITextView) -> Int? {
            let top = text.bounds.minY + 1
            var lower = 0, upper = prompts.count
            while lower < upper {
                let middle = lower + (upper - lower) / 2
                guard let position = text.position(from: text.beginningOfDocument, offset: prompts[middle].offset) else { return nil }
                let caret = text.caretRect(for: position)
                guard !caret.isNull, !caret.isEmpty else { return nil }
                if caret.minY < top { lower = middle + 1 } else { upper = middle }
            }
            return lower
        }
        @objc func jump(_ button: UIButton) {
            guard let view, prompts.indices.contains(button.tag) else { return }
            let prompt = prompts[button.tag], text = view.text
            guard let pos = text.position(from: text.beginningOfDocument, offset: prompt.offset) else { return }
            selected = prompt
            view.follow = false
            wasPinned = false
            parent.pinned = false
            let y = text.caretRect(for: pos).minY - 6
            let maxY = max(0, text.contentSize.height - text.bounds.height + text.adjustedContentInset.bottom)
            view.setContentOffset(.zero, animated: false)
            view.scroll(to: CGPoint(x: 0, y: min(maxY, max(0, y))), animated: true)
            updatePills()
            renderLatest()
        }
        @objc func jumpToBottom() {
            guard let view else { return }
            selected = nil
            parent.pinned = true
            view.follow = true
            wasPinned = true
            view.cancelScrollAnimation()
            view.animateNextFollow = true
            view.followBottom()
            updatePills()
            renderLatest()
        }
        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            if scrollView === view?.text { updatePills() }
        }
        func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
            if scrollView === view?.text {
                selected = nil
                view?.follow = false; view?.animateNextFollow = false; view?.cancelScrollAnimation()
                wasPinned = false; parent.pinned = false
                updatePills()
            }
        }
        #if DEBUG
        func scrollViewWillBeginDecelerating(_ scrollView: UIScrollView) {
            guard ProcessInfo.processInfo.environment["CRABIGATOR_TEST_MOMENTUM"] == "1", scrollView === view?.text else { return }
            let offset = scrollView.contentOffset.y
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) { [weak scrollView] in
                guard let scrollView else { return }
                scrollView.accessibilityLabel = "Coasting: \(scrollView.isDecelerating); distance: \(Int(abs(scrollView.contentOffset.y - offset)))"
            }
        }
        #endif
        func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) { finishScrolling(scrollView) }
        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if !decelerate { finishScrolling(scrollView) }
        }
        private func finishScrolling(_ scrollView: UIScrollView) {
            if scrollView === view?.text {
                repin(scrollView)
                updatePills()
            }
            if parent.pinned { view?.followBottom() }
            renderLatest()
        }
        func repin(_ scrollView: UIScrollView) {
            guard scrollView === view?.text else { return }
            if scrollView.contentOffset.y + scrollView.bounds.height >= scrollView.contentSize.height - 12 { parent.pinned = true; view?.follow = true }
        }
        func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
            guard let pan = gestureRecognizer as? UIPanGestureRecognizer, let view else { return false }
            let velocity = pan.velocity(in: view)
            return velocity.x > abs(velocity.y) * 1.5 && view.contentOffset.x <= 0 && view.text.selectedRange.length == 0
        }
        @objc func pan(_ recognizer: UIPanGestureRecognizer) {
            let ended = [.ended, .cancelled, .failed].contains(recognizer.state)
            parent.swipe?(max(0, recognizer.translation(in: view).x), recognizer.state == .cancelled ? -1000 : recognizer.velocity(in: view).x, ended)
        }
    }
}
final class TerminalScrollView: UIScrollView {
    let text = UITextView(usingTextLayoutManager: true)
    let previous = TerminalScrollView.promptButton(), nextPrompt = TerminalScrollView.promptButton()
    let bottomButton: UIButton = {
        let button = promptButton()
        button.configuration?.image = UIImage(systemName: "arrow.down.to.line")
        button.accessibilityLabel = "Scroll to bottom"
        return button
    }()
    private static func promptButton() -> UIButton {
        var config = UIButton.Configuration.filled()
        config.cornerStyle = .fixed
        config.background.cornerRadius = 14
        config.background.strokeWidth = 1
        config.background.strokeColor = TerminalText.rgb(0x374654)
        config.baseBackgroundColor = TerminalText.rgb(0x19232D)
        config.baseForegroundColor = TerminalText.rgb(0xEDF3F7)
        config.contentInsets = NSDirectionalEdgeInsets(top: 5, leading: 12, bottom: 5, trailing: 12)
        config.titleAlignment = .leading
        config.titleLineBreakMode = .byTruncatingTail
        config.subtitleLineBreakMode = .byTruncatingTail
        config.titleTextAttributesTransformer = UIConfigurationTextAttributesTransformer { incoming in
            var outgoing = incoming
            outgoing.font = UIFontMetrics(forTextStyle: .caption1).scaledFont(for: .systemFont(ofSize: 12, weight: .semibold))
            return outgoing
        }
        config.subtitleTextAttributesTransformer = UIConfigurationTextAttributesTransformer { incoming in
            var outgoing = incoming
            outgoing.font = UIFont.preferredFont(forTextStyle: .caption1)
            outgoing.foregroundColor = TerminalText.rgb(0xB8C6D1)
            return outgoing
        }
        let button = UIButton(configuration: config)
        button.isHidden = true
        return button
    }
    var documentWidth: CGFloat = 0
    var follow = true
    var animateNextFollow = false
    private var displayLink: CADisplayLink?
    private var startOffset = CGPoint.zero, endOffset = CGPoint.zero
    private var startTime: CFTimeInterval = 0
    var onScrollAnimationEnd: (() -> Void)?
    var isAnimatingScroll: Bool { displayLink != nil }
    var isUserScrolling: Bool { isDragging || isDecelerating || text.isDragging || text.isDecelerating }
    override init(frame: CGRect) { super.init(frame: frame); addSubview(text); addSubview(previous); addSubview(nextPrompt); addSubview(bottomButton); alwaysBounceHorizontal = false; showsVerticalScrollIndicator = false }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    override func layoutSubviews() {
        super.layoutSubviews()
        let width = max(bounds.width, documentWidth)
        let frame = CGRect(x: 0, y: 0, width: width, height: bounds.height)
        if text.frame != frame { text.frame = frame }
        if contentSize != frame.size { contentSize = frame.size }
        bottomButton.frame = CGRect(x: bounds.maxX - 50, y: bounds.maxY - 50, width: 44, height: 44)
        for button in [previous, nextPrompt] where !button.isHidden {
            let size = button.sizeThatFits(CGSize(width: CGFloat.greatestFiniteMagnitude, height: 48))
            let width = min(size.width, min(260, bounds.width * 0.54)), height = max(44, size.height)
            let trailing = button === nextPrompt && !bottomButton.isHidden ? bottomButton.frame.minX - 6 : bounds.maxX - 6
            button.frame = CGRect(x: trailing - width,
                                  y: button === previous ? bounds.minY + 6 : bounds.maxY - height - 6,
                                  width: width, height: height)
        }
    }
    func followBottom() {
        guard !isUserScrolling, !isAnimatingScroll else { return }
        let offset = CGPoint(x: 0, y: max(-text.adjustedContentInset.top, text.contentSize.height - text.bounds.height + text.adjustedContentInset.bottom))
        let animate = animateNextFollow
        animateNextFollow = false
        scroll(to: offset, animated: animate)
    }
    func cancelScrollAnimation() {
        displayLink?.invalidate()
        displayLink = nil
    }
    func scroll(to offset: CGPoint, animated: Bool) {
        cancelScrollAnimation()
        guard text.contentOffset != offset else { return }
        guard animated, !UIAccessibility.isReduceMotionEnabled else {
            text.setContentOffset(offset, animated: false)
            return
        }
        startOffset = text.contentOffset
        endOffset = offset
        startTime = CACurrentMediaTime()
        let link = CADisplayLink(target: self, selector: #selector(advanceScroll(_:)))
        displayLink = link
        link.add(to: .main, forMode: .common)
    }
    @objc private func advanceScroll(_ link: CADisplayLink) {
        let progress = min(1, max(0, (link.targetTimestamp - startTime) / 0.35))
        let remaining = 1 - progress
        let eased = 1 - remaining * remaining * remaining
        text.setContentOffset(CGPoint(x: startOffset.x + (endOffset.x - startOffset.x) * eased,
                                      y: startOffset.y + (endOffset.y - startOffset.y) * eased), animated: false)
        if progress == 1 {
            cancelScrollAnimation()
            if follow { followBottom() }
            onScrollAnimationEnd?()
        }
    }
}
private extension String.UTF16View {
    func commonPrefixCount(with other: String.UTF16View) -> Int { var count = 0; for (a, b) in zip(self, other) { if a != b { break }; count += 1 }; return count }
}
private extension NSAttributedString {
    func maximumLineWidth() -> CGFloat {
        let ns = string as NSString; var start = 0, width: CGFloat = 0
        while start < ns.length {
            let range = ns.lineRange(for: NSRange(location: start, length: 0))
            width = max(width, attributedSubstring(from: range).size().width); start = NSMaxRange(range)
        }
        return width
    }
    func commonAttributedPrefix(with other: NSAttributedString) -> Int {
        let common = string.utf16.commonPrefixCount(with: other.string.utf16)
        var index = 0
        while index < common {
            var lhs = NSRange(), rhs = NSRange()
            let a = attributes(at: index, effectiveRange: &lhs), b = other.attributes(at: index, effectiveRange: &rhs)
            guard NSDictionary(dictionary: a).isEqual(to: b) else { return index }
            index = min(common, min(NSMaxRange(lhs), NSMaxRange(rhs)))
        }
        return common
    }
}

#if DEBUG
struct TerminalLinksFixture: View {
    @State private var opened = "No link opened"
    @State private var pinned = true
    @State private var preferences = Preferences()
    var body: some View {
        VStack {
            TerminalViewport(
                history: "🦀 \u{1b}]8;id=history;https://example.com/history\u{1b}\\History PR\u{1b}]8;;\u{7}\n",
                screen: "\u{1b}]8;;https://example.com/docs\u{7}Codex docs\u{1b}]8;;\u{1b}\\\nhttps://example.com/plain",
                preferences: preferences, pinned: $pinned)
                .environment(\.openURL, OpenURLAction { url in opened = url.absoluteString; return .handled })
            Text(opened).accessibilityIdentifier("opened-terminal-link")
            Button("Toggle wrapping") { preferences.wrap.toggle() }
        }
    }
}

/// Repeatable incoming output for the native momentum regression test. No session input.
struct TerminalMomentumFixture: View {
    @State private var frame = 0
    @State private var pinned = true
    private let history = (0..<1000).map { "History row \($0): terminal scrollback" }.joined(separator: "\n") + "\n"
    var body: some View {
        TerminalViewport(history: history, screen: "Live frame \(frame)", preferences: Preferences(), pinned: $pinned)
            .task {
                while !Task.isCancelled {
                    try? await Task.sleep(for: .milliseconds(100))
                    frame += 1
                }
            }
    }
}
#endif
