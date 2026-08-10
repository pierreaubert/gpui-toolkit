//! FFI module for iOS — C-compatible functions called from Objective-C app delegate.

mod app_callback_cell;
mod asset_source_cell;
mod boxed_asset_source;
mod consts;
mod gpui_mod;
mod ios_app_state;
mod misc;
mod set;
mod take;
#[cfg(test)]
mod tests;
mod window_list_wrapper;

pub(crate) use consts::*;
pub use gpui_mod::*;
pub use ios_app_state::*;
pub use set::*;
