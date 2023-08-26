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

pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

// UnsafeCell is not Sync (it is Send only). However, if 'T' is Send, we can treat the spin lock as Sync.
unsafe impl<T> Sync for SpinLock<T> where T: Send {}

pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}

// The Guard, essentially a reference to a spin lock for value type 'T' access, can only be Sync if 'T' is as well.
unsafe impl<T> Sync for Guard<'_, T> where T: Sync {}

impl<T> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
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

    pub fn lock(&self) -> Guard<T> {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }

        Guard { lock: self }
    }
}

#[test]
fn test_spin_lock() {
    let l = SpinLock::new(42);

    thread::scope(|s| {
        s.spawn(|| {
            let mut g = l.lock();
            *g = 23;
            thread::sleep(Duration::from_millis(50));
        });

        s.spawn(|| {
            thread::sleep(Duration::from_millis(50));
            let g = l.lock();
            assert_eq!(*g, 23);

            thread::scope(|ss| {
                ss.spawn(|| {
                    println!("{}", *g);
                });
                ss.spawn(|| {
                    println!("{}", *g);
                });
            });
        });
    });

    let g = l.lock();
    drop(g); // Explicitly dropping the guard consumes it.
}
