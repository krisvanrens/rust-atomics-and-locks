use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(test)]
use std::{thread, time::Duration};

pub struct Channel<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

// Tell the compiler our type is Sync as long as T is Send (required because UnsafeCell is Send only).
//
// Obviously we need Sync to be able to refer to each side of the channel from different thread contexts. Also 'T' needs
//  to be Send because it will be transmitted over the channel, potentially to another thread context. Under the
//  assumption that the user is a good user, and first checks 'is_ready' before receiving the data from the channel, all
//  is fine (otherwise uninitialized data may be read). We don't require Sync for T, because we only allow one thread
//  context at a time to access the receiving end of the channel.
//
unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    /// # Safety
    /// This function dereferences a raw pointer, sorry.
    pub unsafe fn send(&self, value: T) {
        (*self.value.get()).write(value);
        self.ready.store(true, Ordering::Release);
    }

    /// # Safety
    /// This function dereferences a raw pointer, sorry.
    pub unsafe fn receive(&self) -> T {
        (*self.value.get()).assume_init_read()
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}

#[test]
fn test_channel() {
    let c = Channel::<i32>::new();

    thread::scope(|s| {
        s.spawn(|| {
            unsafe { c.send(42) };
        });

        s.spawn(|| {
            while !c.is_ready() {
                thread::sleep(Duration::from_millis(10));
            }

            assert_eq!(unsafe { c.receive() }, 42);
        });
    });
}
