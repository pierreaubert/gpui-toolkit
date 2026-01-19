//! Scale module for mapping data domains to visual ranges
//!
//! Scales are the foundation of data visualization, providing functions to map
//! abstract data values to visual representations.
//!
//! ## Continuous Scales
//!
//! - [`LinearScale`]: Map continuous numeric domain to continuous range
//! - [`LogScale`]: Logarithmic mapping for exponential data
//! - [`PowScale`]: Power/exponential mapping (includes sqrt)
//! - [`SymlogScale`]: Symmetric log that handles negative values and zero
//!
//! ## Quantizing Scales
//!
//! - [`QuantizeScale`]: Map continuous domain to discrete range (uniform segments)
//! - [`QuantileScale`]: Map sampled domain to discrete range (equal-count segments)
//! - [`ThresholdScale`]: Map continuous domain to discrete range (explicit thresholds)
//!
//! ## Categorical Scales
//!
//! - [`OrdinalScale`]: Map discrete domain values to discrete range values
//! - [`BandScale`]: Divide range into bands for categorical data (bar charts)
//! - [`PointScale`]: Position categorical data at points (scatter plots)

mod linear;
mod log;
mod ordinal;
mod pow;
mod quantile;
mod quantize;
mod symlog;
mod threshold;
mod ticks;
mod traits;

pub use linear::LinearScale;
pub use log::LogScale;
pub use ordinal::{BandScale, OrdinalScale, PointScale};
pub use pow::{PowScale, SqrtScale, sqrt_scale};
pub use quantile::QuantileScale;
pub use quantize::QuantizeScale;
pub use symlog::SymlogScale;
pub use threshold::ThresholdScale;
pub use ticks::{generate_linear_ticks, generate_log_ticks, nice_number};
pub use traits::Scale;
