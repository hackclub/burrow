use std::{mem::MaybeUninit, net::Ipv4Addr};

use tokio::io::Result;
use tun::TunInterface;
use tun::TunQueue;

async fn try_main() -> Result<()> {
    let iface = tun::create_interface();
    iface.set_ipv4(Ipv4Addr::new(10, 0, 0, 2))?;

    let queue = TunQueue::from(iface);

    loop {
        let mut buf = [MaybeUninit::<u8>::uninit(); 1500];
        let len = queue.recv(&mut buf)?;
        println!("Received {len} bytes");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    try_main().await.unwrap();
}
