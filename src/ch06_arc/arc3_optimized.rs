use std::{
    cell::UnsafeCell,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

#[cfg(test)]
use std::thread;

struct ArcData<T> {
    arc_ref_count: AtomicUsize,
    weak_ref_count: AtomicUsize,
    data: UnsafeCell<Option<T>>,
}

pub struct Arc<T> {
    weak: Weak<T>,
}

pub struct Weak<T> {
    state: NonNull<ArcData<T>>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            weak: Weak {
                state: NonNull::from(Box::leak(Box::new(ArcData {
                    arc_ref_count: AtomicUsize::new(1),
                    weak_ref_count: AtomicUsize::new(1),
                    data: UnsafeCell::new(Some(data)),
                }))),
            },
        }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.weak.data().weak_ref_count.load(Ordering::Relaxed) == 1 {
            fence(Ordering::Acquire);

            let state = unsafe { arc.weak.state.as_mut() };
            (*state.data.get_mut()).as_mut()
        } else {
            None
        }
    }

    pub fn downgrade(&self) -> Weak<T> {
        self.weak.clone()
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let weak = self.weak.clone();
        if weak.data().arc_ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }

        Arc { weak }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self
            .weak
            .data()
            .arc_ref_count
            .fetch_sub(1, Ordering::Release)
            == 1
        {
            fence(Ordering::Acquire);
            let data = self.weak.data().data.get();
            unsafe { (*data) = None }
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let data = self.weak.data().data.get();
        unsafe { (*data).as_ref().unwrap() }
    }
}

impl<T> Weak<T> {
    fn data(&self) -> &ArcData<T> {
        unsafe { &self.state.as_ref() }
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        let mut ref_count = self.data().arc_ref_count.load(Ordering::Relaxed);
        loop {
            if ref_count == 0 {
                return None;
            }

            assert!(ref_count < usize::MAX);

            if let Err(e) = self.data().arc_ref_count.compare_exchange_weak(
                ref_count,
                ref_count + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                ref_count = e;
                continue;
            }

            return Some(Arc { weak: self.clone() });
        }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        if self.data().weak_ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }

        Weak { state: self.state }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        if self.data().weak_ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe { drop(Box::from_raw(self.state.as_ptr())) };
        }
    }
}

unsafe impl<T: Send + Sync> Send for Weak<T> {}
unsafe impl<T: Send + Sync> Sync for Weak<T> {}

#[test]
fn test_arc() {
    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct DropCounter;

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }

    let a1 = Arc::new((42, DropCounter));
    let mut a2 = a1.clone();
    let w1 = a1.downgrade();
    let w2 = a1.downgrade();

    assert!(Arc::get_mut(&mut a2).is_none());

    let t = thread::spawn(move || {
        let arc = w1.upgrade().unwrap();
        assert_eq!(arc.0, 42);
    });

    assert_eq!(a1.0, 42);
    assert!(w2.upgrade().is_some());

    assert!(Arc::get_mut(&mut a2).is_none());

    t.join().unwrap();

    assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 0);

    drop(a1);

    assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 0);
    assert!(w2.upgrade().is_some());
    assert!(Arc::get_mut(&mut a2).is_none());

    drop(w2);

    assert!(Arc::get_mut(&mut a2).is_some());

    let w3 = a2.downgrade();

    drop(a2);

    assert!(w3.upgrade().is_none());
}
