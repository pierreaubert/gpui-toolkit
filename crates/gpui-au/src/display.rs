//! macOS display handling using NSScreen.

use core_graphics::geometry::CGRect;
use gpui::{Bounds, DisplayId, Pixels, PlatformDisplay, px, size};
use objc::{class, msg_send, sel, sel_impl};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct AuDisplay {
    screen: *mut objc::runtime::Object,
}

unsafe impl Send for AuDisplay {}
unsafe impl Sync for AuDisplay {}

impl AuDisplay {
    pub fn main() -> Self {
        unsafe {
            let screen: *mut objc::runtime::Object = msg_send![class!(NSScreen), mainScreen];
            Self { screen }
        }
    }

    fn bounds_in_points(&self) -> CGRect {
        unsafe { msg_send![self.screen, frame] }
    }

    pub fn scale(&self) -> f32 {
        unsafe {
            let scale: f64 = msg_send![self.screen, backingScaleFactor];
            scale as f32
        }
    }
}

impl PlatformDisplay for AuDisplay {
    fn id(&self) -> DisplayId {
        DisplayId::new(self.screen as u32)
    }

    fn uuid(&self) -> anyhow::Result<Uuid> {
        let bounds = self.bounds_in_points();
        let scale = self.scale();
        let bytes = format!(
            "au-screen-{}-{}-{}",
            bounds.size.width as u32,
            bounds.size.height as u32,
            (scale * 100.0) as u32
        );
        Ok(Uuid::new_v5(&Uuid::NAMESPACE_OID, bytes.as_bytes()))
    }

    fn bounds(&self) -> Bounds<Pixels> {
        let bounds = self.bounds_in_points();
        Bounds {
            origin: Default::default(),
            size: size(px(bounds.size.width as f32), px(bounds.size.height as f32)),
        }
    }
}
