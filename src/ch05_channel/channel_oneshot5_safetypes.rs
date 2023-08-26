use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[cfg(test)]
use std::{thread, time::Duration};

struct Channel<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

// Tell the compiler our type is Sync as long as T is Send (required because UnsafeCell is Send only).
unsafe impl<T> Sync for Channel<T> where T: Send {}

pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}

pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    pub fn send(self, value: T) {
        unsafe {
            (*self.channel.value.get()).write(value);
        }
        self.channel.ready.store(true, Ordering::Release);
    }
}

impl<T> Receiver<T> {
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

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.value.get_mut().assume_init_drop() }
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(Channel {
        value: UnsafeCell::new(MaybeUninit::uninit()),
        ready: AtomicBool::new(false),
    });

    (Sender { channel: a.clone() }, Receiver { channel: a })
}

#[test]
fn test_channel() {
    let (s, r) = channel::<i32>();

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
