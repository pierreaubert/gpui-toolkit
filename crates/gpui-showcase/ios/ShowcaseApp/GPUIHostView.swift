import SwiftUI
import UIKit

final class GPUIHostContainerView: UIView {
    private var gpuiWindow: UnsafeMutableRawPointer?

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil, gpuiWindow == nil {
            gpuiWindow = gpui_ios_attach_to_view(Unmanaged.passUnretained(self).toOpaque())
        }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        subviews.first?.frame = bounds
    }

    func detachGPUIWindow() {
        if let gpuiWindow {
            gpui_ios_detach_view(gpuiWindow)
            self.gpuiWindow = nil
        }
    }

    deinit {
        detachGPUIWindow()
    }
}

struct GPUIHostView: UIViewRepresentable {
    func makeUIView(context: Context) -> GPUIHostContainerView {
        GPUIHostContainerView()
    }

    func updateUIView(_ uiView: GPUIHostContainerView, context: Context) {
        uiView.setNeedsLayout()
    }

    static func dismantleUIView(_ uiView: GPUIHostContainerView, coordinator: ()) {
        uiView.detachGPUIWindow()
    }
}
