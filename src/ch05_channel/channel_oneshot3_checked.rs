use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(test)]
use std::{thread, time::Duration};

pub struct Channel<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    occupied: AtomicBool,
    ready: AtomicBool,
}

// Tell the compiler our type is Sync as long as T is Send (required because UnsafeCell is Send only).
unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            occupied: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }

    //
    // A relaxed memory ordering here is fine, because 'send' is the only place where it is used. A relaxed memory
    //  ordering only guarantees total modification order of all operations. The only time a swap actually changes the
    //  value of 'occupied', is upon the first call of 'send'.
    //
    pub fn send(&self, value: T) {
        if self.occupied.swap(true, Ordering::Relaxed) {
            panic!("calling send on an occupied channel");
        }

        unsafe { (*self.value.get()).write(value) };
        self.ready.store(true, Ordering::Release);
    }

    pub fn receive(&self) -> T {
        if !self.ready.swap(false, Ordering::Acquire) {
            panic!("calling receive on an empty channel");
        }

        unsafe { (*self.value.get()).assume_init_read() }
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.value.get_mut().assume_init_drop() };
        }
    }
}

#[test]
fn test_channel() {
    let c = Channel::<i32>::new();

    thread::scope(|s| {
        s.spawn(|| {
            c.send(42);
        });

        s.spawn(|| {
            while !c.is_ready() {
                thread::sleep(Duration::from_millis(10));
            }

            assert_eq!(c.receive(), 42);
        });
    });
}
