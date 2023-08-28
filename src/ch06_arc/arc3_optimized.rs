use std::{
    cell::UnsafeCell,
    mem::ManuallyDrop,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

#[cfg(test)]
use std::thread;

struct ArcData<T> {
    arc_ref_count: AtomicUsize,
    weak_ref_count: AtomicUsize,
    data: UnsafeCell<ManuallyDrop<T>>,
}

pub struct Arc<T> {
    state: NonNull<ArcData<T>>,
}

pub struct Weak<T> {
    state: NonNull<ArcData<T>>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            state: NonNull::from(Box::leak(Box::new(ArcData {
                arc_ref_count: AtomicUsize::new(1),
                weak_ref_count: AtomicUsize::new(1),
                data: UnsafeCell::new(ManuallyDrop::new(data)),
            }))),
        }
    }

    fn data(&self) -> &ArcData<T> {
        unsafe { self.state.as_ref() }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc
            .data()
            .weak_ref_count
            .compare_exchange(1, usize::MAX, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return None;
        }

        let is_unique = arc.data().arc_ref_count.load(Ordering::Relaxed) == 1;

        arc.data().weak_ref_count.store(1, Ordering::Release);

        if !is_unique {
            return None;
        }

        fence(Ordering::Acquire);
        unsafe { Some(&mut *arc.data().data.get()) }
    }

    pub fn downgrade(arc: &Self) -> Weak<T> {
        let mut ref_count = arc.data().weak_ref_count.load(Ordering::Relaxed);
        loop {
            if ref_count == usize::MAX {
                std::hint::spin_loop();
                ref_count = arc.data().weak_ref_count.load(Ordering::Relaxed);
                continue;
            }

            assert!(ref_count <= usize::MAX / 2);

            if let Err(e) = arc.data().weak_ref_count.compare_exchange_weak(
                ref_count,
                ref_count + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                ref_count = e;
                continue;
            }

            return Weak { state: arc.state };
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        if self.data().arc_ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }

        Arc { state: self.state }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().arc_ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);

            let data = self.data().data.get();
            unsafe { ManuallyDrop::drop(&mut *data) };

            drop(Weak { state: self.state });
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.data().data.get() }
    }
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T> Weak<T> {
    fn data(&self) -> &ArcData<T> {
        unsafe { self.state.as_ref() }
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        let mut ref_count = self.data().arc_ref_count.load(Ordering::Relaxed);
        loop {
            if ref_count == 0 {
                return None;
            }

            assert!(ref_count <= usize::MAX / 2);

            if let Err(e) = self.data().arc_ref_count.compare_exchange_weak(
                ref_count,
                ref_count + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                ref_count = e;
                continue;
            }

            return Some(Arc { state: self.state });
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
    let w1 = Arc::downgrade(&a1);
    let w2 = Arc::downgrade(&a1);

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

    let w3 = Arc::downgrade(&a2);

    drop(a2);

    assert!(w3.upgrade().is_none());
}
