use gpui::App;

pub(super) struct AppCallbackCell(
    pub(super) std::cell::UnsafeCell<Option<Box<dyn FnOnce(&mut App)>>>,
);

unsafe impl Send for AppCallbackCell {}

unsafe impl Sync for AppCallbackCell {}
