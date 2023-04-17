use std::{mem::MaybeUninit, net::Ipv4Addr};

use tokio::io::Result;
use tun::TunInterface;
use tun_async::TunQueue;

async fn try_main() -> Result<()> {
    let iface = TunInterface::new()?;
    iface.set_ipv4_addr(Ipv4Addr::new(10, 0, 0, 2))?;
    println!("{:?}", iface.index()?);
    println!("{:?}", iface.ipv4_addr()?);

    let queue = TunQueue::from_queue(iface.into())?;

    loop {
        let mut buf = [MaybeUninit::<u8>::uninit(); 1500];
        let len = queue.try_recv(&mut buf).await?;
        println!("Received {len} bytes");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    try_main().await.unwrap();
}
