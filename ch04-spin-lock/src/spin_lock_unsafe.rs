use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

///
/// Pros:
///   - Actually wraps the value that is locked.
///
/// Cons:
///   - Unsafe interface.
///

pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&mut self) -> &mut T {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }

        unsafe { &mut *self.value.get() }
    }

    /// # Safety
    /// This function is unsafe, as there may be references to self.value outside of the critical section.
    pub unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}
