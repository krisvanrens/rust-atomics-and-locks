use std::sync::{Condvar, Mutex};

#[cfg(test)]
use std::thread;

pub struct Channel<T> {
    value: Mutex<Option<T>>,
    ready: Condvar,
}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            value: Mutex::new(None),
            ready: Condvar::new(),
        }
    }

    pub fn send(&self, value: T) {
        self.value.lock().unwrap().replace(value);
        self.ready.notify_one();
    }

    pub fn receive(&self) -> T {
        let mut g = self.value.lock().unwrap();
        loop {
            if let Some(value) = g.take() {
                return value;
            }

            g = self.ready.wait(g).unwrap();
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
            assert_eq!(c.receive(), 42);
        });
    });
}
