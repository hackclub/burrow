use tokio::io::Result;
use tun::TunInterface;

async fn lol() -> Result<()> {
    let iface = TunInterface::new()?;
    println!("{:?}", iface.name());

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    lol().await.unwrap();
}
