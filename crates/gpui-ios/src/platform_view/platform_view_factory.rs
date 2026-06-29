use super::PlatformView;
use super::platform_view_kind::PlatformViewKind;
use super::types::PlatformViewParams;

pub trait PlatformViewFactory: Send + Sync {
    fn create(&self, params: &PlatformViewParams) -> Result<Box<dyn PlatformView>, String>;
    fn view_type(&self) -> &str;
    fn kind(&self) -> PlatformViewKind {
        PlatformViewKind::Custom
    }
}
