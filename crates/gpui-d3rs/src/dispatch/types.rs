use super::event::Event;

/// A listener callback
pub(super) type ListenerFn = Box<dyn FnMut(&Event) + Send + Sync>;
