import UIKit

/// Scene entry point (required from iOS 27). The first scene to connect
/// claims the single GPUI window; gpui-ios logs and ignores later scenes
/// until the primary disconnects.
final class SceneDelegate: UIResponder, UIWindowSceneDelegate {

    func scene(
        _ scene: UIScene,
        willConnectTo session: UISceneSession,
        options connectionOptions: UIScene.ConnectionOptions
    ) {
        gpui_ios_scene_will_connect(
            UnsafeMutableRawPointer(Unmanaged.passUnretained(scene).toOpaque()))
    }

    func sceneDidDisconnect(_ scene: UIScene) {
        gpui_ios_scene_did_disconnect(
            UnsafeMutableRawPointer(Unmanaged.passUnretained(scene).toOpaque()))
    }

    // Scene life cycle replaces the app-delegate foreground/background
    // callbacks, so forward scene state to the GPUI window from here.
    func sceneWillEnterForeground(_ scene: UIScene) {
        gpui_ios_lifecycle_forwarding_will_enter_foreground(gpui_ios_get_window())
    }

    func sceneDidBecomeActive(_ scene: UIScene) {
        gpui_ios_lifecycle_forwarding_did_become_active(gpui_ios_get_window())
    }

    func sceneWillResignActive(_ scene: UIScene) {
        gpui_ios_lifecycle_forwarding_will_resign_active(gpui_ios_get_window())
    }

    func sceneDidEnterBackground(_ scene: UIScene) {
        gpui_ios_lifecycle_forwarding_did_enter_background(gpui_ios_get_window())
    }
}
