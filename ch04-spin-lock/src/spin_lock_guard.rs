use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use std::{thread, time::Duration};

///
/// Pros:
///   - Fully safe interface.
///   - Wraps value to be locked.
///   - No unlocking required due to implementation of Drop trait.
///
/// Cons:
///   - ...
///

#[derive(Debug)]
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

#[derive(Debug)]
pub struct Guard<'a, T> {
    lock: &'a mut SpinLock<T>,
}

impl<T> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.lock.value.get_mut()
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&mut self) -> Guard<T> {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }

        Guard { lock: self }
    }
}

#[test]
fn test_spin_lock() {
    let mut s = SpinLock::new(42);
    {
        let mut ga = s.lock();
        thread::sleep(Duration::from_millis(100));
        let v = &mut *ga;
        *v = 23;
        assert_eq!(*ga, 23);
    }
    {
        let gb = s.lock();
        assert_eq!(*gb, 23);
    }

    let gc = s.lock();
    drop(gc); // Explicitly dropping the guard consumes it.
}
