use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use std::{thread, time::Duration};

///
/// Pros:
///   - Simple to implement and use.
///
/// Cons:
///   - The user must still manually keep/control the lock and the value which is error-prone.
///
/// Notes:
///   - By using release memory ordering on the store in 'unlock', and acquire memory ordering in the load part of the
///      swap operation in 'lock', we assure there is a happens-before relation on 'lock'/'unlock'.
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

#[test]
fn test_spin_lock() {
    let l = SpinLock::new();

    thread::scope(|s| {
        s.spawn(|| {
            l.lock();
            thread::sleep(Duration::from_millis(50));
            l.unlock();
        });

        s.spawn(|| {
            l.lock();
            l.unlock();
        });
    });
}
