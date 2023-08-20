use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Channel<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

// Tell the compiler our type is Sync as long as T is Send (required because UnsafeCell is neither).
unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    pub unsafe fn send(&self, value: T) {
        (*self.value.get()).write(value);
        self.ready.store(true, Ordering::Release);
    }

    pub unsafe fn receive(&self) -> T {
        (*self.value.get()).assume_init_read()
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}
