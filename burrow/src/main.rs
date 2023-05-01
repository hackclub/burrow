use std::net::Ipv4Addr;
use tokio::io::Result;
use tun::TunInterface;

async fn try_main() -> Result<()> {
    let interface = TunInterface::new()?;
    let interface_addr = Ipv4Addr::new(10, 1, 0, 1);
    let dest = Ipv4Addr::new(0, 0, 0, 0);
    burrow::reroute(interface, interface_addr, dest, interface_addr).await;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    try_main().await.unwrap();
}
