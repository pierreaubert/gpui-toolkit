//! iOS display handling using UIScreen.

use core_graphics::geometry::CGRect;
use gpui::{Bounds, DisplayId, Pixels, PlatformDisplay, px, size};
use objc::{class, msg_send, runtime::Object, sel, sel_impl};
use std::cell::Cell;
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct IosDisplay {
    screen: *mut Object,
    cached_uuid: Cell<Option<Uuid>>,
}

unsafe impl Send for IosDisplay {}
unsafe impl Sync for IosDisplay {}

impl IosDisplay {
    pub fn main() -> Self {
        unsafe {
            let screen: *mut Object = msg_send![class!(UIScreen), mainScreen];
            Self::retain(screen)
        }
    }

    pub fn all() -> Vec<Self> {
        unsafe {
            let screens: *mut Object = msg_send![class!(UIScreen), screens];
            let count: usize = msg_send![screens, count];
            let mut result = Vec::with_capacity(count);
            for i in 0..count {
                let screen: *mut Object = msg_send![screens, objectAtIndex: i];
                result.push(Self::retain(screen));
            }
            result
        }
    }

    unsafe fn retain(screen: *mut Object) -> Self {
        if !screen.is_null() {
            let _: *mut Object = msg_send![screen, retain];
        }
        Self {
            screen,
            cached_uuid: Cell::new(None),
        }
    }

    fn bounds_in_points(&self) -> CGRect {
        unsafe { msg_send![self.screen, bounds] }
    }

    pub fn native_scale(&self) -> f32 {
        unsafe {
            let scale: f64 = msg_send![self.screen, nativeScale];
            scale as f32
        }
    }

    pub fn scale(&self) -> f32 {
        unsafe {
            let scale: f64 = msg_send![self.screen, scale];
            scale as f32
        }
    }
}

impl Drop for IosDisplay {
    fn drop(&mut self) {
        unsafe {
            if !self.screen.is_null() {
                let _: () = msg_send![self.screen, release];
            }
        }
    }
}

impl PlatformDisplay for IosDisplay {
    fn id(&self) -> DisplayId {
        DisplayId::new(self.screen as u64)
    }

    fn uuid(&self) -> anyhow::Result<Uuid> {
        if let Some(uuid) = self.cached_uuid.get() {
            return Ok(uuid);
        }
        let bounds = self.bounds_in_points();
        let scale = self.native_scale();
        let bytes = format!(
            "ios-screen-{}-{}-{}",
            bounds.size.width as u32,
            bounds.size.height as u32,
            (scale * 100.0) as u32
        );
        let uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, bytes.as_bytes());
        self.cached_uuid.set(Some(uuid));
        Ok(uuid)
    }

    fn bounds(&self) -> Bounds<Pixels> {
        let bounds = self.bounds_in_points();
        Bounds {
            origin: Default::default(),
            size: size(px(bounds.size.width as f32), px(bounds.size.height as f32)),
        }
    }
}
