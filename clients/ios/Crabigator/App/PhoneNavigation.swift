import SwiftUI

/// Keeps the board alive underneath the entire session page, including its loading state.
struct PhoneNavigation: UIViewControllerRepresentable {
    @ObservedObject var model: SessionModel
    @Binding var listMode: Bool
    @Binding var expanded: Bool

    func makeCoordinator() -> Coordinator { Coordinator(model: model) }
    func makeUIViewController(context: Context) -> UINavigationController {
        let coordinator = context.coordinator
        let board = UIHostingController(rootView: boardView)
        board.view.backgroundColor = TerminalText.rgb(0x0B0F14)
        let navigation = UINavigationController(rootViewController: board)
        navigation.setNavigationBarHidden(true, animated: false)
        navigation.setToolbarHidden(true, animated: false)
        navigation.toolbar.isHidden = true
        navigation.overrideUserInterfaceStyle = .dark
        navigation.delegate = coordinator
        navigation.interactivePopGestureRecognizer?.isEnabled = false
        coordinator.navigation = navigation
        coordinator.board = board
        let pan = UIPanGestureRecognizer(target: coordinator, action: #selector(Coordinator.pan(_:)))
        pan.maximumNumberOfTouches = 1
        pan.delegate = coordinator
        navigation.view.addGestureRecognizer(pan)
        return navigation
    }
    func updateUIViewController(_ navigation: UINavigationController, context: Context) {
        context.coordinator.board?.rootView = boardView
        context.coordinator.synchronize()
        navigation.view.accessibilityElementsHidden = model.navigation.conductor
    }
    private var boardView: AnyView {
        AnyView(BoardView(model: model, listMode: $listMode, expanded: $expanded, wide: false) { session, order in
            model.select(session, order: order)
        }.foregroundStyle(Color(uiColor: TerminalText.rgb(0xE9EEF4))).tint(Crab.orange))
    }

    @MainActor final class Coordinator: NSObject, UINavigationControllerDelegate, UIGestureRecognizerDelegate {
        let model: SessionModel
        weak var navigation: UINavigationController?
        weak var board: UIHostingController<AnyView>?
        private var displayedID: String?
        private var interaction: UIPercentDrivenInteractiveTransition?

        init(model: SessionModel) { self.model = model }

        func synchronize() {
            guard let navigation, navigation.transitionCoordinator == nil else { return }
            guard let id = model.selectionID else {
                if navigation.viewControllers.count > 1 { navigation.popToRootViewController(animated: true) }
                return
            }
            displayedID = id
            guard navigation.viewControllers.count == 1 else { return }
            let detail = UIHostingController(rootView:
                SessionDetail(model: model, stream: model.stream, wide: false, active: true) { [weak self] in self?.close() }
                    .foregroundStyle(Color(uiColor: TerminalText.rgb(0xE9EEF4))).tint(Crab.orange))
            detail.view.backgroundColor = TerminalText.rgb(0x0B0F14)
            navigation.pushViewController(detail, animated: navigation.view.window != nil)
        }

        private func close() {
            guard let navigation, navigation.transitionCoordinator == nil else { return }
            navigation.popToRootViewController(animated: true)
        }

        func navigationController(_ navigationController: UINavigationController, didShow viewController: UIViewController, animated: Bool) {
            navigationController.setToolbarHidden(true, animated: false)
            navigationController.toolbar.isHidden = true
            interaction = nil
            if viewController === board {
                // Leave the stream intact until a pop finishes. A cancelled swipe keeps it live.
                if let displayedID, model.selectionID == displayedID { model.close() }
                displayedID = nil
            }
            synchronize()
        }

        func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
            guard let navigation, navigation.viewControllers.count > 1,
                  navigation.transitionCoordinator == nil, navigation.presentedViewController == nil,
                  let pan = gestureRecognizer as? UIPanGestureRecognizer else { return false }
            let velocity = pan.velocity(in: navigation.view)
            guard velocity.x > abs(velocity.y) * 1.5 else { return false }
            // A terminal scrolled sideways or an active text selection owns its gesture.
            var touched = navigation.view.hitTest(pan.location(in: navigation.view), with: nil)
            while let view = touched, view !== navigation.view {
                if view is UITextField { return false }
                if let text = view as? UITextView, text.isEditable || text.selectedRange.length > 0 { return false }
                if let scroll = view as? UIScrollView, scroll.contentOffset.x > -scroll.adjustedContentInset.left + 1 { return false }
                touched = view.superview
            }
            return true
        }

        func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldBeRequiredToFailBy otherGestureRecognizer: UIGestureRecognizer) -> Bool {
            // Decide direction before a child scroll view claims this touch.
            otherGestureRecognizer is UIPanGestureRecognizer
        }

        @objc func pan(_ recognizer: UIPanGestureRecognizer) {
            guard let navigation else { return }
            let width = max(1, navigation.view.bounds.width)
            let progress = min(1, max(0, recognizer.translation(in: navigation.view).x / width))
            switch recognizer.state {
            case .began:
                interaction = UIPercentDrivenInteractiveTransition()
                interaction?.completionCurve = .easeOut
                navigation.popViewController(animated: true)
            case .changed: interaction?.update(progress)
            case .ended:
                let velocity = recognizer.velocity(in: navigation.view).x
                if velocity > 600 || (progress > 1 / 3 && velocity > -600) { interaction?.finish() }
                else { interaction?.cancel() }
            case .cancelled, .failed: interaction?.cancel()
            default: break
            }
        }

        func navigationController(_ navigationController: UINavigationController, animationControllerFor operation: UINavigationController.Operation, from fromVC: UIViewController, to toVC: UIViewController) -> UIViewControllerAnimatedTransitioning? {
            PhoneSlideTransition(push: operation == .push)
        }
        func navigationController(_ navigationController: UINavigationController, interactionControllerFor animationController: UIViewControllerAnimatedTransitioning) -> UIViewControllerInteractiveTransitioning? { interaction }
    }
}

/// Uses only public navigation APIs so a pan anywhere on the page can drive the pop.
final class PhoneSlideTransition: NSObject, UIViewControllerAnimatedTransitioning {
    let push: Bool
    init(push: Bool) { self.push = push }
    func transitionDuration(using transitionContext: UIViewControllerContextTransitioning?) -> TimeInterval { UIAccessibility.isReduceMotionEnabled ? 0.2 : 0.35 }
    func animateTransition(using context: UIViewControllerContextTransitioning) {
        guard let from = context.view(forKey: .from), let to = context.view(forKey: .to),
              let destination = context.viewController(forKey: .to) else { context.completeTransition(false); return }
        let container = context.containerView
        to.frame = context.finalFrame(for: destination)
        let width = container.bounds.width
        let front = push ? to : from, back = push ? from : to
        if push { container.addSubview(to) } else { container.insertSubview(to, belowSubview: from) }
        let shade = UIView(frame: back.frame)
        shade.backgroundColor = .black
        shade.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        shade.isUserInteractionEnabled = false
        container.insertSubview(shade, belowSubview: front)
        front.transform = push ? CGAffineTransform(translationX: width, y: 0) : .identity
        back.transform = push ? .identity : CGAffineTransform(translationX: -width * 0.3, y: 0)
        shade.transform = back.transform
        shade.alpha = push ? 0 : 0.22
        UIView.animate(withDuration: transitionDuration(using: context), delay: 0, options: [.curveEaseOut]) {
            front.transform = self.push ? .identity : CGAffineTransform(translationX: width, y: 0)
            back.transform = self.push ? CGAffineTransform(translationX: -width * 0.3, y: 0) : .identity
            shade.transform = back.transform
            shade.alpha = self.push ? 0.22 : 0
        } completion: { _ in
            front.transform = .identity
            back.transform = .identity
            shade.removeFromSuperview()
            context.completeTransition(!context.transitionWasCancelled)
        }
    }
}
