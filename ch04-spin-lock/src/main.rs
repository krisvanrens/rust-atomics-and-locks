use ch04_spin_lock::spin_lock_guard::SpinLock as SpinLockGuard;
use ch04_spin_lock::spin_lock_simple::SpinLock as SpinLockSimple;
use ch04_spin_lock::spin_lock_unsafe::SpinLock as SpinLockUnsafe;

use std::{thread, time};

fn main() {
    let s1 = SpinLockSimple::new();
    s1.lock();
    thread::sleep(time::Duration::from_millis(100));
    s1.unlock();

    let mut s2 = SpinLockUnsafe::new(42);
    let v2 = s2.lock();
    thread::sleep(time::Duration::from_millis(100));
    *v2 = 23;
    println!("v2 = {v2}");
    unsafe {
        s2.unlock();
    }

    let mut s3 = SpinLockGuard::new(42);
    {
        let mut g3 = s3.lock();
        thread::sleep(time::Duration::from_millis(100));
        let v3 = &mut *g3;
        *v3 = 23;
        println!("v3 = {v3}");
    }
    {
        let g4 = s3.lock();
        println!("v3 = {}", *g4);
    }
}
