use gpui::{Rgba, WindowAppearance, rgb};

#[derive(Clone, Copy)]
pub(super) struct ShowcaseTheme {
    pub(super) background: Rgba,
    pub(super) surface: Rgba,
    pub(super) muted: Rgba,
    pub(super) border: Rgba,
    pub(super) accent: Rgba,
    pub(super) text_primary: Rgba,
    pub(super) text_muted: Rgba,
}

impl ShowcaseTheme {
    pub(super) fn from_window_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
        }
    }

    pub(super) fn dark() -> Self {
        Self {
            background: rgb(0x181818),
            surface: rgb(0x242424),
            muted: rgb(0x2d2d2d),
            border: rgb(0x3a3a3a),
            accent: rgb(0x0a84ff),
            text_primary: rgb(0xf2f2f2),
            text_muted: rgb(0x9a9a9a),
        }
    }

    fn light() -> Self {
        Self {
            background: rgb(0xf6f7f9),
            surface: rgb(0xffffff),
            muted: rgb(0xe9edf2),
            border: rgb(0xc7cdd5),
            accent: rgb(0x0067c7),
            text_primary: rgb(0x1b1f24),
            text_muted: rgb(0x58616e),
        }
    }
}
