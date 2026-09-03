use gpui::App;

pub(super) type AppCallback = Box<dyn FnOnce(&mut App)>;

pub(super) struct AppCallbackCell(pub(super) std::cell::UnsafeCell<Option<AppCallback>>);

unsafe impl Send for AppCallbackCell {}

unsafe impl Sync for AppCallbackCell {}
