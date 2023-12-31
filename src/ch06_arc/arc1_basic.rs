use std::{
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

#[cfg(test)]
use std::thread;

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct Arc<T> {
    state: NonNull<ArcData<T>>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            state: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(1),
                data,
            }))),
        }
    }

    fn data(&self) -> &ArcData<T> {
        unsafe { &self.state.as_ref() }
    }

    //
    // Implement mutable dereference using an associated function. Using a mutable reference of the argument Arc, the
    //  lifetime of the associated instance is borrowed, making sure no other borrows occur during the lifetime of the
    //  (optional) return value.
    //
    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().ref_count.load(Ordering::Relaxed) == 1 {
            fence(Ordering::Acquire);
            unsafe { Some(&mut arc.state.as_mut().data) }
        } else {
            None
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }

        Arc { state: self.state }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe { drop(Box::from_raw(self.state.as_ptr())) };
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data().data
    }
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

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

    assert!(Arc::get_mut(&mut a2).is_none());

    let t = thread::spawn(move || {
        assert_eq!(a1.0, 42);
    });

    assert_eq!(a2.0, 42);

    assert!(Arc::get_mut(&mut a2).is_none());

    t.join().unwrap();

    assert!(Arc::get_mut(&mut a2).is_some());

    assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 0);

    drop(a2);

    assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 1);
}
