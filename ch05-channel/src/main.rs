use ch05_channel::channel_simple::Channel as ChannelSimple;

fn main() {
    let mut c1 = ChannelSimple::<i32>::new();
    c1.send(42);
    println!("c1 received: {}", c1.receive());
}
