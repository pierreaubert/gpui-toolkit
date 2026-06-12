import UIKit

@main
class AppDelegate: UIResponder, UIApplicationDelegate {

    var window: UIWindow?
    private var displayLink: CADisplayLink?

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        // Launch the GPUI app — this creates the UIWindow, Metal view, and
        // opens the showcase view.  It returns immediately because on iOS
        // the run loop is managed by UIApplicationMain.
        showcase_ios_start()

        // Set up a CADisplayLink so GPUI gets a render tick every frame.
        // gpui_ios_request_frame pumps momentum scrolling, checks for dirty
        // text input, and invokes the GPUI render callback.
        displayLink = CADisplayLink(target: self, selector: #selector(renderFrame))
        displayLink?.add(to: .main, forMode: .common)

        return true
    }

    @objc private func renderFrame() {
        let win = gpui_ios_get_window()
        if win != nil {
            gpui_ios_request_frame(win)
        }
    }

    // MARK: - Lifecycle forwarding

    func applicationWillEnterForeground(_ application: UIApplication) {
        gpui_ios_will_enter_foreground(nil)
    }

    func applicationDidBecomeActive(_ application: UIApplication) {
        gpui_ios_did_become_active(nil)
    }

    func applicationWillResignActive(_ application: UIApplication) {
        gpui_ios_will_resign_active(nil)
    }

    func applicationDidEnterBackground(_ application: UIApplication) {
        gpui_ios_did_enter_background(nil)
    }

    func applicationWillTerminate(_ application: UIApplication) {
        gpui_ios_will_terminate(nil)
    }
}
