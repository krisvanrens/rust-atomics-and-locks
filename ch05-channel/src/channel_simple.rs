use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

pub struct Channel<T> {
    queue: Mutex<VecDeque<T>>,
    ready: Condvar,
}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
        }
    }

    pub fn send(&self, value: T) {
        self.queue.lock().unwrap().push_back(value);
        self.ready.notify_one();
    }

    pub fn receive(&self) -> T {
        let mut g = self.queue.lock().unwrap();
        loop {
            if let Some(value) = g.pop_front() {
                return value;
            }

            g = self.ready.wait(g).unwrap();
        }
    }
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}
