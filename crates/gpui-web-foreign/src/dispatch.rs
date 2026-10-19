//! Serial provider calls with bounded reentrant work and deferred replacement.

// Rust guideline compliant 2026-02-21

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

// Bound retained reentrant commands and synchronous work from recursive providers.
const MAX_PENDING: usize = 256;
const MAX_PENDING_BYTES: usize = 1 << 20;
const MAX_DRAIN_WORK: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DispatchError {
    QueueLimit,
    DrainLimit,
}

pub(crate) struct SerializedProvider<T, C> {
    provider: RefCell<Option<T>>,
    pending: RefCell<VecDeque<(C, usize)>>,
    pending_bytes: Cell<usize>,
    running: Cell<bool>,
    closed: Cell<bool>,
}

impl<T, C> SerializedProvider<T, C> {
    pub(crate) fn new(provider: Option<T>) -> Self {
        Self {
            provider: RefCell::new(provider),
            pending: RefCell::new(VecDeque::new()),
            pending_bytes: Cell::new(0),
            running: Cell::new(false),
            closed: Cell::new(false),
        }
    }

    pub(crate) fn replace(&self, provider: T) {
        if self.closed.get() {
            return;
        }
        let old = self.provider.replace(Some(provider));
        drop(old);
    }

    pub(crate) fn close(&self) {
        self.closed.set(true);
        let old_provider = self.provider.take();
        let pending = std::mem::take(&mut *self.pending.borrow_mut());
        self.pending_bytes.set(0);
        // User destructors may reenter; no RefCell borrow survives these drops.
        drop(old_provider);
        drop(pending);
    }

    /// Returns false without a provider; nested calls are queued in FIFO order.
    ///
    /// Replacement becomes active after the current call returns. Overflow,
    /// recursive drain exhaustion and unwinding close this provider permanently.
    pub(crate) fn dispatch(
        &self,
        command: C,
        dynamic_bytes: usize,
        mut invoke: impl FnMut(&mut T, C),
    ) -> Result<bool, DispatchError> {
        if self.closed.get() {
            return Ok(false);
        }
        if self.running.get() {
            let cost = std::mem::size_of::<(C, usize)>().checked_add(dynamic_bytes);
            let total = cost.and_then(|cost| self.pending_bytes.get().checked_add(cost));
            if self.pending.borrow().len() >= MAX_PENDING
                || total.is_none_or(|total| total > MAX_PENDING_BYTES)
            {
                self.close();
                return Err(DispatchError::QueueLimit);
            }
            let cost = cost.expect("validated command cost");
            self.pending.borrow_mut().push_back((command, cost));
            self.pending_bytes
                .set(total.expect("validated pending cost"));
            return Ok(true);
        }
        let Some(mut provider) = self.provider.take() else {
            return Ok(false);
        };
        self.running.set(true);
        let mut guard = DrainGuard {
            slot: self,
            completed: false,
        };
        let mut next = Some(command);
        let mut work = 0;
        while let Some(command) = next.take() {
            if work == MAX_DRAIN_WORK {
                return Err(DispatchError::DrainLimit);
            }
            work += 1;
            invoke(&mut provider, command);
            if self.closed.get() {
                break;
            }
            loop {
                let replacement = self.provider.take();
                let Some(replacement) = replacement else {
                    break;
                };
                if work == MAX_DRAIN_WORK {
                    return Err(DispatchError::DrainLimit);
                }
                work += 1;
                // The old provider's destructor may install another replacement.
                let old = std::mem::replace(&mut provider, replacement);
                drop(old);
                if self.closed.get() {
                    break;
                }
            }
            if self.closed.get() {
                break;
            }
            next = self
                .pending
                .borrow_mut()
                .pop_front()
                .map(|(command, cost)| {
                    self.pending_bytes.set(self.pending_bytes.get() - cost);
                    command
                });
        }
        if !self.closed.get() {
            let old = self.provider.replace(Some(provider));
            debug_assert!(old.is_none());
        }
        guard.completed = true;
        Ok(true)
    }
}

struct DrainGuard<'a, T, C> {
    slot: &'a SerializedProvider<T, C>,
    completed: bool,
}

impl<T, C> Drop for DrainGuard<'_, T, C> {
    fn drop(&mut self) {
        if !self.completed {
            self.slot.close();
        }
        self.slot.running.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn nested_calls_are_fifo_and_replacement_wins() {
        let slot = SerializedProvider::<u32, u32>::new(Some(1));
        let seen = RefCell::new(Vec::new());
        slot.dispatch(10, 0, |provider, command| {
            seen.borrow_mut().push((*provider, command));
            if command == 10 {
                slot.dispatch(20, 0, |_, _| panic!("nested invocation"))
                    .unwrap();
                slot.replace(2);
                slot.dispatch(30, 0, |_, _| panic!("nested invocation"))
                    .unwrap();
            }
        })
        .unwrap();
        slot.dispatch(40, 0, |provider, command| {
            seen.borrow_mut().push((*provider, command));
        })
        .unwrap();
        assert_eq!(*seen.borrow(), [(1, 10), (2, 20), (2, 30), (2, 40)]);
        assert_eq!(slot.pending_bytes.get(), 0);
    }

    #[test]
    fn close_cancels_queued_work_and_rejects_replacement() {
        let slot = SerializedProvider::<u32, u32>::new(Some(1));
        let calls = Cell::new(0);
        slot.dispatch(1, 0, |_, _| {
            calls.set(calls.get() + 1);
            slot.dispatch(2, 0, |_, _| unreachable!()).unwrap();
            slot.close();
            slot.replace(99);
        })
        .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(slot.dispatch(3, 0, |_, _| unreachable!()), Ok(false));
        assert_eq!(slot.pending_bytes.get(), 0);
        assert_eq!(slot.pending.borrow().capacity(), 0);
    }

    #[test]
    fn recursive_single_item_work_is_bounded() {
        let slot = SerializedProvider::<(), ()>::new(Some(()));
        let calls = Cell::new(0);
        let result = slot.dispatch((), 0, |(), ()| {
            calls.set(calls.get() + 1);
            slot.dispatch((), 0, |(), ()| unreachable!()).unwrap();
        });
        assert_eq!(result, Err(DispatchError::DrainLimit));
        assert_eq!(calls.get(), MAX_DRAIN_WORK);
        assert_eq!(slot.dispatch((), 0, |(), ()| unreachable!()), Ok(false));
    }

    #[test]
    fn count_and_byte_limits_close_before_accepting_more_work() {
        for bytes in [0, MAX_PENDING_BYTES + 1, usize::MAX] {
            let slot = SerializedProvider::<(), ()>::new(Some(()));
            slot.dispatch((), 0, |(), ()| {
                if bytes == 0 {
                    for _ in 0..MAX_PENDING {
                        slot.dispatch((), 0, |(), ()| unreachable!()).unwrap();
                    }
                }
                assert_eq!(
                    slot.dispatch((), bytes, |(), ()| unreachable!()),
                    Err(DispatchError::QueueLimit)
                );
            })
            .unwrap();
            assert!(slot.closed.get());
            assert_eq!(slot.pending_bytes.get(), 0);
        }
    }

    #[test]
    fn exact_byte_budget_is_accepted_and_next_command_closes() {
        let slot = SerializedProvider::<(), ()>::new(Some(()));
        slot.dispatch((), 0, |(), ()| {
            let dynamic_bytes = MAX_PENDING_BYTES - std::mem::size_of::<((), usize)>();
            assert_eq!(
                slot.dispatch((), dynamic_bytes, |(), ()| unreachable!()),
                Ok(true)
            );
            assert_eq!(slot.pending_bytes.get(), MAX_PENDING_BYTES);
            assert_eq!(
                slot.dispatch((), 0, |(), ()| unreachable!()),
                Err(DispatchError::QueueLimit)
            );
        })
        .unwrap();
        assert!(slot.closed.get());
        assert_eq!(slot.pending_bytes.get(), 0);
    }

    #[test]
    fn retiring_provider_can_install_the_next_replacement() {
        struct Provider(u32, Option<Box<dyn FnOnce()>>);
        impl Drop for Provider {
            fn drop(&mut self) {
                if let Some(f) = self.1.take() {
                    f();
                }
            }
        }
        let slot = Rc::new(SerializedProvider::<Provider, u32>::new(None));
        let weak = Rc::downgrade(&slot);
        slot.replace(Provider(
            1,
            Some(Box::new(move || {
                weak.upgrade().unwrap().replace(Provider(3, None));
            })),
        ));
        let seen = RefCell::new(Vec::new());
        slot.dispatch(1, 0, |provider, command| {
            seen.borrow_mut().push((provider.0, command));
            if command == 1 {
                slot.replace(Provider(2, None));
                slot.dispatch(2, 0, |_, _| unreachable!()).unwrap();
            }
        })
        .unwrap();
        assert_eq!(*seen.borrow(), [(1, 1), (3, 2)]);
        slot.close();
    }

    #[test]
    fn callback_panic_closes_slot_and_drops_pending_work() {
        let slot = SerializedProvider::<(), ()>::new(Some(()));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            slot.dispatch((), 0, |(), ()| {
                slot.dispatch((), 0, |(), ()| unreachable!()).unwrap();
                panic!("provider panic");
            })
        }));
        result.unwrap_err();
        assert!(slot.closed.get());
        assert_eq!(slot.pending.borrow().capacity(), 0);
    }

    #[test]
    fn provider_destructors_can_replace_and_close_without_a_borrow_panic() {
        struct Provider(Option<Box<dyn FnOnce()>>);
        impl Drop for Provider {
            fn drop(&mut self) {
                if let Some(f) = self.0.take() {
                    f();
                }
            }
        }
        let slot = Rc::new(SerializedProvider::<Provider, ()>::new(None));
        let weak = Rc::downgrade(&slot);
        slot.replace(Provider(Some(Box::new(move || {
            let slot = weak.upgrade().unwrap();
            slot.replace(Provider(None));
            slot.close();
        }))));
        slot.replace(Provider(None));
        assert!(slot.closed.get());
    }
}
