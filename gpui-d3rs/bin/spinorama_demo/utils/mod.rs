pub mod colors;
pub mod interpolation;

pub use colors::{CEA2034_CURVES, cea2034_colors, interpolate_colors};
pub use interpolation::{format_frequency, get_angle_range, interpolate_spl_at_frequency};
