import SwiftUI

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
        return view
    }
    func updateUIView(_ view: TerminalScrollView, context: Context) { context.coordinator.parent = self; context.coordinator.openURL = context.environment.openURL; context.coordinator.update() }

    final class Coordinator: NSObject, UITextViewDelegate, UIScrollViewDelegate, UIGestureRecognizerDelegate {
        var parent: TerminalViewport
        var openURL: OpenURLAction?
        weak var view: TerminalScrollView?
        let queue = DispatchQueue(label: "com.newsblur.crabigator.ansi", qos: .userInitiated)
        var sourceHistory = "", sourceScreen = "", style: Preferences?
        var cachedHistory = NSAttributedString(string: ""), cachedSource = "", cachedStyle: Preferences?
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
            if parent.pinned && !wasPinned { view.animateNextFollow = true }
            if !parent.pinned {
                view.animateNextFollow = false
                if view.isFollowingAnimation {
                    view.text.setContentOffset(view.text.contentOffset, animated: false)
                    view.isFollowingAnimation = false
                }
            }
            wasPinned = parent.pinned
            view.follow = parent.pinned
            if sourceHistory == parent.history && sourceScreen == parent.screen && style == parent.preferences {
                if parent.pinned { view.followBottom() }
                renderLatest()
                return
            }
            sourceHistory = parent.history; sourceScreen = parent.screen; style = parent.preferences; revision += 1
            renderLatest()
        }
        private func renderLatest() {
            // TextKit edits and offset restoration can cancel native deceleration.
            // Keep only the latest source while the user drags or coasts.
            guard !rendering, revision != appliedRevision, let view, !view.isUserScrolling, !view.isFollowingAnimation,
                  let prefs = style else { return }
            rendering = true
            let revision = revision, history = sourceHistory, screen = sourceScreen
            queue.async { [weak self] in
                guard let self else { return }
                if self.cachedSource != history || self.cachedStyle != prefs {
                    self.cachedHistory = TerminalText.parse(history, size: CGFloat(prefs.fontSize), spacing: CGFloat(prefs.lineSpacing) / 100, wrap: prefs.wrap)
                    self.cachedSource = history; self.cachedStyle = prefs
                    self.cachedWidth = prefs.wrap ? 0 : self.cachedHistory.maximumLineWidth()
                }
                let result = NSMutableAttributedString(attributedString: self.cachedHistory)
                if !history.isEmpty && !history.hasSuffix("\n") { result.append(NSAttributedString(string: "\n")) }
                let historyLength = result.length
                let renderedScreen = TerminalText.parse(screen, size: CGFloat(prefs.fontSize), spacing: CGFloat(prefs.lineSpacing) / 100, wrap: prefs.wrap)
                result.append(renderedScreen)
                // Horizontal scrolling preserves the desktop columns. Layout remains native TextKit 2.
                let width = prefs.wrap ? 0 : max(self.cachedWidth, renderedScreen.maximumLineWidth())
                DispatchQueue.main.async { [weak self] in
                    guard let self else { return }
                    self.rendering = false
                    guard self.style == prefs else { self.renderLatest(); return }
                    // A swipe may have started while ANSI parsing was off the main thread.
                    guard let view = self.view, !view.isUserScrolling, !view.isFollowingAnimation else { return }
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
                    self.appliedHistory = history; self.appliedStyle = prefs; self.appliedRevision = revision
                    if selection.location <= result.length { text.selectedRange = NSRange(location: selection.location, length: min(selection.length, result.length - selection.location)) }
                    view.setNeedsLayout(); view.layoutIfNeeded()
                    if self.parent.pinned { view.followBottom() }
                    else if text.contentOffset != offset { text.setContentOffset(offset, animated: false) }
                    // Apply completed frames even while newer output arrives. Dropping every
                    // older frame would freeze a busy session until its output paused.
                    if self.revision != revision { self.renderLatest() }
                }
            }
        }
        func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
            if scrollView === view?.text {
                view?.follow = false; view?.animateNextFollow = false; view?.isFollowingAnimation = false
                wasPinned = false; parent.pinned = false
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
        func scrollViewDidEndScrollingAnimation(_ scrollView: UIScrollView) {
            guard scrollView === view?.text else { return }
            view?.isFollowingAnimation = false
            if parent.pinned { view?.followBottom() }
            renderLatest()
        }
        func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) { finishScrolling(scrollView) }
        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if !decelerate { finishScrolling(scrollView) }
        }
        private func finishScrolling(_ scrollView: UIScrollView) {
            repin(scrollView)
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
    var documentWidth: CGFloat = 0
    var follow = true
    var animateNextFollow = false
    var isFollowingAnimation = false
    var isUserScrolling: Bool { isDragging || isDecelerating || text.isDragging || text.isDecelerating }
    override init(frame: CGRect) { super.init(frame: frame); addSubview(text); alwaysBounceHorizontal = false; showsVerticalScrollIndicator = false }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    override func layoutSubviews() {
        super.layoutSubviews()
        let width = max(bounds.width, documentWidth)
        let frame = CGRect(x: 0, y: 0, width: width, height: bounds.height)
        if text.frame != frame { text.frame = frame }
        if contentSize != frame.size { contentSize = frame.size }
    }
    func followBottom() {
        guard !isUserScrolling, !isFollowingAnimation else { return }
        let offset = CGPoint(x: 0, y: max(-text.adjustedContentInset.top, text.contentSize.height - text.bounds.height + text.adjustedContentInset.bottom))
        let animate = animateNextFollow && !UIAccessibility.isReduceMotionEnabled
        animateNextFollow = false
        if text.contentOffset != offset {
            isFollowingAnimation = animate
            text.setContentOffset(offset, animated: animate)
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
