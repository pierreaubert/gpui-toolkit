//! # d3-dispatch - Event Dispatcher
//!
//! This module provides an event dispatcher inspired by D3.js's d3-dispatch module.
//! It allows registering callbacks for named events and dispatching events with payloads.
//!
//! ## Key Features
//!
//! - **Type-safe events**: Support for typed event payloads
//! - **Listener management**: Register, unregister, and list listeners
//! - **Context support**: Pass context to callbacks for state access
//! - **Copy-free dispatch**: Efficient event dispatch without unnecessary copying
//!
//! ## Example
//!
//! ```rust
//! use d3rs::dispatch::{Dispatcher, Event};
//!
//! let mut dispatcher = Dispatcher::new();
//!
//! // Register a listener
//! let handle = dispatcher.on("update", |event: &Event| {
//!     println!("Update received: {:?}", event.data);
//! });
//!
//! // Dispatch an event
//! dispatcher.dispatch("update", Some(Box::new("hello".to_string())));
//!
//! // Remove listener
//! dispatcher.off(handle);
//! ```

mod dispatcher;
mod event;
mod listener;
mod listener_id;
mod misc;
#[cfg(test)]
mod tests;
mod types;

pub use dispatcher::*;
pub use event::*;
pub use listener_id::*;
