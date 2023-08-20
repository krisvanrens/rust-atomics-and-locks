use ch05_channel::channel_simple::Channel as ChannelSimple;
use ch05_channel::channel_oneshot_option::Channel as ChannelOneshotOption;

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
}
