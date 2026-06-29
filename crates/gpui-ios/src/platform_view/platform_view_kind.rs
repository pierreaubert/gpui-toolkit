/// Native view family requested by GPUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlatformViewKind {
    SwiftUi,
    UiKit,
    WebView,
    Map,
    Camera,
    #[default]
    Custom,
}

impl PlatformViewKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SwiftUi => "swiftui",
            Self::UiKit => "uikit",
            Self::WebView => "webview",
            Self::Map => "map",
            Self::Camera => "camera",
            Self::Custom => "custom",
        }
    }
}

impl std::str::FromStr for PlatformViewKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "swiftui" | "swift_ui" | "swift-ui" => Ok(Self::SwiftUi),
            "uikit" | "ui_kit" | "ui-kit" => Ok(Self::UiKit),
            "webview" | "web_view" | "web-view" => Ok(Self::WebView),
            "map" => Ok(Self::Map),
            "camera" => Ok(Self::Camera),
            "custom" => Ok(Self::Custom),
            _ => Err(()),
        }
    }
}
