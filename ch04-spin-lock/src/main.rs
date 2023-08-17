use ch04_spin_lock::spin_lock_safe::SpinLock as SpinLockSafe;
use ch04_spin_lock::spin_lock_unsafe::SpinLock as SpinLockUnsafe;

use std::{thread, time};

fn main() {
    let s1 = SpinLockSafe::new();
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
}
