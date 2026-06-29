pub(crate) struct WindowListWrapper(
    pub(crate) std::cell::UnsafeCell<Vec<*const super::super::window::IosWindow>>,
);

unsafe impl Send for WindowListWrapper {}

unsafe impl Sync for WindowListWrapper {}
