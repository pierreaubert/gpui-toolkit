//! macOS display handling using NSScreen.

use core_graphics::geometry::CGRect;
use gpui::{Bounds, DisplayId, Pixels, PlatformDisplay, px, size};
use objc::{class, msg_send, sel, sel_impl};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct AuDisplay {
    screen: *mut objc::runtime::Object,
    uuid: Uuid,
}

unsafe impl Send for AuDisplay {}
unsafe impl Sync for AuDisplay {}

impl AuDisplay {
    const ID: u64 = 1;

    fn stable_uuid() -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, b"gpui-au-primary-display")
    }

    pub fn main() -> Self {
        unsafe {
            let screen: *mut objc::runtime::Object = msg_send![class!(NSScreen), mainScreen];
            Self {
                screen,
                uuid: Self::stable_uuid(),
            }
        }
    }

    fn bounds_in_points(&self) -> CGRect {
        unsafe { msg_send![self.screen, frame] }
    }
}

impl PlatformDisplay for AuDisplay {
    fn id(&self) -> DisplayId {
        DisplayId::new(AuDisplay::ID)
    }

    fn uuid(&self) -> anyhow::Result<Uuid> {
        Ok(self.uuid)
    }

    fn bounds(&self) -> Bounds<Pixels> {
        if self.screen.is_null() {
            return Bounds::default();
        }
        let bounds = self.bounds_in_points();
        Bounds {
            origin: Default::default(),
            size: size(px(bounds.size.width as f32), px(bounds.size.height as f32)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_screen_has_stable_identity_and_empty_bounds() {
        let display = AuDisplay {
            screen: std::ptr::null_mut(),
            uuid: AuDisplay::stable_uuid(),
        };
        assert_eq!(display.id(), DisplayId::new(AuDisplay::ID));
        assert_eq!(display.uuid().unwrap(), AuDisplay::stable_uuid());
        assert_eq!(display.bounds(), Bounds::default());
    }
}
