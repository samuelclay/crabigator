import SwiftUI
import UIKit
import UserNotifications
import UserNotificationsUI

final class NotificationViewController: UIViewController, UNNotificationContentExtension {
    private let model = PromptCardModel()
    private var loaded = false
    private lazy var host = UIHostingController(rootView: PromptCard(model: model))
    /// Measures the content without its scroll view, which would report no height.
    private lazy var sizer = UIHostingController(rootView: PromptCardContent(model: model))

    override func viewDidLoad() {
        super.viewDidLoad()
        overrideUserInterfaceStyle = .dark
        view.backgroundColor = UIColor(Crab.ink)
        host.view.backgroundColor = .clear
        addChild(host)
        host.view.frame = view.bounds
        host.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(host.view)
        host.didMove(toParent: self)
        model.context = extensionContext
        model.changed = { [weak self] in self?.fit() }
    }
    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        fit()
    }
    func didReceive(_ notification: UNNotification) {
        model.show(notification)
        loaded = true
        fit()
    }
    func didReceive(_ response: UNNotificationResponse, completionHandler completion: @escaping (UNNotificationContentExtensionResponseOption) -> Void) {
        guard model.canAnswer, ![UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier].contains(response.actionIdentifier) else {
            completion(.dismissAndForwardAction); return
        }
        // The view shows the progress and closes itself once the session moves on.
        model.respond(to: response)
        completion(.doNotDismiss)
    }
    /// Fits the content, scrolling only when it is taller than most of the screen.
    /// iOS keeps the first size it sees, so nothing is measured before the content arrives.
    private func fit() {
        let width = view.bounds.width
        guard loaded, width > 0 else { return }
        let height = sizer.sizeThatFits(in: CGSize(width: width, height: .greatestFiniteMagnitude)).height
        let limit = (view.window?.windowScene?.screen.bounds.height ?? 640) * 0.8
        let size = CGSize(width: width, height: ceil(min(height, limit)))
        if preferredContentSize != size { preferredContentSize = size }
    }
}
