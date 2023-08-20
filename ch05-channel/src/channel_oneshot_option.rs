use std::sync::{Condvar, Mutex};

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
