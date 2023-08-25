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

pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    //
    // Here we exclusively borrow ourselves, to tie the lifetime of the channel to the existence of the sender and the
    //  receiver. This means it is not possible to borrow or move the channel as long as the sender/receiver exists.
    //
    pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
        //
        // To allow for channel re-use, we must reinitialize ourselves after use, just to be sure.
        //
        *self = Self::new();

        (Sender { channel: self }, Receiver { channel: self })
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.value.get_mut().assume_init_drop() }
        }
    }
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<'a, T> Sender<'a, T> {
    pub fn send(self, value: T) {
        unsafe {
            (*self.channel.value.get()).write(value);
        }
        self.channel.ready.store(true, Ordering::Release);
    }
}

impl<'a, T> Receiver<'a, T> {
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Ordering::Relaxed)
    }

    pub fn receive(self) -> T {
        if !self.channel.ready.swap(false, Ordering::Acquire) {
            panic!("calling receive on an empty channel");
        }

        unsafe { (*self.channel.value.get()).assume_init_read() }
    }
}

#[test]
fn test_channel() {
    let mut c = Channel::<i32>::new();
    let (s, r) = c.split();

    thread::scope(|sc| {
        sc.spawn(|| {
            s.send(42);
        });

        sc.spawn(|| {
            while !r.is_ready() {
                thread::sleep(Duration::from_millis(10));
            }

            assert_eq!(r.receive(), 42);
        });
    });
}
