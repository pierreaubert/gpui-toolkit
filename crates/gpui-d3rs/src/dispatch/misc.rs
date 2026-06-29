use std::sync::atomic::AtomicU64;

pub(super) static LISTENER_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
