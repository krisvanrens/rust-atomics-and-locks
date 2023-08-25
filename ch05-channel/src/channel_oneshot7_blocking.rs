use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
    thread::Thread,
};

#[cfg(test)]
use std::thread;

pub struct Channel<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
    receiving_thread: Thread,
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
    _no_send: PhantomData<*const ()>,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    pub fn split(&mut self) -> (Sender<'_, T>, Receiver<'_, T>) {
        *self = Self::new();

        (
            //
            // Bind the sender::receiving_thread to the current thread. That is, make the assumption that the receiver
            //  will not be called in another thread. This implies we have to make sure the receiver is not Send. This
            //  is achieved by adding a 'PhantomData<*const ()>' field to the receiver. Kinda ugly, but we'll just have
            //  wait until "inverted traits" I guess.
            //
            Sender {
                channel: self,
                receiving_thread: std::thread::current(),
            },
            Receiver {
                channel: self,
                _no_send: PhantomData,
            },
        )
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
        self.receiving_thread.unpark();
    }
}

impl<'a, T> Receiver<'a, T> {
    pub fn receive(self) -> T {
        while !self.channel.ready.load(Ordering::Acquire) {
            std::thread::park();
        }

        unsafe { (*self.channel.value.get()).assume_init_read() }
    }
}

#[test]
fn test_channel() {
    let mut c = Channel::<i32>::new();

    thread::scope(|sc| {
        let (s, r) = c.split();

        sc.spawn(|| {
            s.send(42);
        });

        assert_eq!(r.receive(), 42);
    });
}
