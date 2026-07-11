//! iOS text system using CoreText.
//! Adapted from the macOS text system since both platforms share CoreText.


mod font;
mod ios_text_system;
mod ios_text_system_state;
mod misc;
mod string_index_converter;
mod types;

pub use ios_text_system::*;
