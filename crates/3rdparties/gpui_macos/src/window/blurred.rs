use cocoa :: { appkit :: { NSVisualEffectMaterial , NSVisualEffectState , NSVisualEffectView } , base :: { id } , foundation :: { NSRect } } ;
use objc :: { class , msg_send , runtime :: { Object , Sel } , sel , sel_impl } ;
use super::misc::remove_layer_background;

pub(super) extern "C" fn blurred_view_init_with_frame(this: &Object, _: Sel, frame: NSRect) -> id {
    unsafe {
        let view = msg_send![super(this, class!(NSVisualEffectView)), initWithFrame: frame];
        // Use a colorless semantic material. The default value `AppearanceBased`, though not
        // manually set, is deprecated.
        NSVisualEffectView::setMaterial_(view, NSVisualEffectMaterial::Selection);
        NSVisualEffectView::setState_(view, NSVisualEffectState::Active);
        view
    }
}

pub(super) extern "C" fn blurred_view_update_layer(this: &Object, _: Sel) {
    unsafe {
        let _: () = msg_send![super(this, class!(NSVisualEffectView)), updateLayer];
        let layer: id = msg_send![this, layer];
        if !layer.is_null() {
            remove_layer_background(layer);
        }
    }
}

