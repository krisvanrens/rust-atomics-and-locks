use std::sync::atomic::{AtomicBool, Ordering};

///
/// Pros:
///   - Simple to implement and use.
///
/// Cons:
///   - The user must still manually keep/control the lock and the value.
///

pub struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }

        // Also fine, and almost identical:
        //
        //while self.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
        //    std::hint::spin_loop();
        //}
    }

    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}
