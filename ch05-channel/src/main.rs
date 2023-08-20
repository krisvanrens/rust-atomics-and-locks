use ch05_channel::channel_oneshot_option::Channel as ChannelOneshotOption;
use ch05_channel::channel_oneshot_unsafe::Channel as ChannelOneshotUnsafe;
use ch05_channel::channel_simple::Channel as ChannelSimple;

use std::{thread, time::Duration};

fn main() {
    let c1 = ChannelSimple::<i32>::new();

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_millis(100));
            c1.send(42);
        });

        s.spawn(|| {
            println!("c1 received: {}", c1.receive());
        });
    });

    let c2 = ChannelOneshotOption::<i32>::new();

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_millis(100));
            c2.send(42);
        });

        s.spawn(|| {
            println!("c2 received: {}", c2.receive());
        });
    });

    let c3 = ChannelOneshotUnsafe::<i32>::new();

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_millis(100));
            unsafe { c3.send(42) };
        });

        s.spawn(|| {
            while !c3.is_ready() {
                thread::sleep(Duration::from_millis(10));
            }

            println!("c3 received: {}", unsafe { c3.receive() });
        });
    });
}
