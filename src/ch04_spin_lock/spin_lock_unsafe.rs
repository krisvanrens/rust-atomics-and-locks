use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use std::{thread, time::Duration};

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

//
// Convince the compiler that it is fine to refer to the spin lock from different thread contexts as long as 'T' can be
//  safely moved from one thread context to another. UnsafeCell does not implement Sync, as on its own, no assumptions
//  can be made as to whether it's safe to refer to from different thread contexts. As long as 'T' is Send (i.e. it can
//  be transferred to another thread context), and the access to the UnsafeCell is synchronized using the atomic bool,
//  we can assume all is fine. The unsafe interface is a nuisance, but that's all.
//
unsafe impl<T> Sync for SpinLock<T> where T: Send {}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> &mut T {
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

#[test]
fn test_spin_lock() {
    let l = SpinLock::new(42);

    thread::scope(|s| {
        s.spawn(|| {
            let v = l.lock();
            *v = 23;
            thread::sleep(Duration::from_millis(50));
            unsafe { l.unlock() };
        });

        s.spawn(|| {
            thread::sleep(Duration::from_millis(50));
            let v = l.lock();
            assert_eq!(*v, 23);
            unsafe { l.unlock() };
        });
    });
}
