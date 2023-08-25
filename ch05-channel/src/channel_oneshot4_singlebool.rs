use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

#[cfg(test)]
use std::{thread, time::Duration};

const EMPTY: u8 = 0;
const WRITING: u8 = 1;
const READY: u8 = 2;
const READING: u8 = 3;

pub struct Channel<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
}

// Tell the compiler our type is Sync as long as T is Send (required because UnsafeCell is Send only).
unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            state: AtomicU8::new(EMPTY),
        }
    }

    pub fn send(&self, value: T) {
        //
        // State: EMPTY --> WRITING
        //
        // ...
        //
        if self
            .state
            .compare_exchange(EMPTY, WRITING, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            panic!("calling send on an occupied channel");
        }

        unsafe { (*self.value.get()).write(value) };

        //
        // State: WRITING --> READY
        //
        // ...
        //
        self.state.store(READY, Ordering::Release);
    }

    pub fn receive(&self) -> T {
        //
        // State: READY --> READING
        //
        // ...
        //
        if self
            .state
            .compare_exchange(READY, READING, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("calling receive on an empty channel");
        }

        unsafe { (*self.value.get()).assume_init_read() }

        //
        // State: READING --> EMPTY
        //
        // ...
        //
    }

    pub fn is_ready(&self) -> bool {
        self.state.load(Ordering::Relaxed) == READY
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.state.get_mut() == READY {
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
