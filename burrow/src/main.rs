use tokio::io::Result;
use tun::TunInterface;

async fn try_main() -> Result<()> {
    burrow::ensure_root();

    let iface = TunInterface::new()?;
    println!("{:?}", iface.name());

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    try_main().await.unwrap();
}
