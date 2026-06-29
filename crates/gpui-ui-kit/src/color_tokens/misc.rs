use gpui::{Hsla, Rgba};

/// Helper function to create a muted version of a color
pub fn with_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

/// Helper function to lighten a color
pub fn lighten(color: Rgba, amount: f32) -> Rgba {
    let hsla = Hsla::from(color);
    Hsla {
        h: hsla.h,
        s: hsla.s,
        l: (hsla.l + amount).min(1.0),
        a: hsla.a,
    }
    .into()
}

/// Helper function to darken a color
pub fn darken(color: Rgba, amount: f32) -> Rgba {
    let hsla = Hsla::from(color);
    Hsla {
        h: hsla.h,
        s: hsla.s,
        l: (hsla.l - amount).max(0.0),
        a: hsla.a,
    }
    .into()
}

/// Helper function to adjust saturation
pub fn saturate(color: Rgba, amount: f32) -> Rgba {
    let hsla = Hsla::from(color);
    Hsla {
        h: hsla.h,
        s: (hsla.s + amount).clamp(0.0, 1.0),
        l: hsla.l,
        a: hsla.a,
    }
    .into()
}

/// Helper function to desaturate a color
pub fn desaturate(color: Rgba, amount: f32) -> Rgba {
    saturate(color, -amount)
}
