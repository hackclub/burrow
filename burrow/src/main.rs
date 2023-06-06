use clap::Parser;
use tokio::io::Result;
use tun::TunInterface;

#[derive(Parser)]
#[command(name = "Burrow")]
#[command(author = "Hack Club <team@hackclub.com>")]
#[command(version = "0.1")]
#[command(about = "Burrow is a tool for burrowing through firewalls, built by teenagers at Hack Club.", long_about = None)]

struct Cli {}

async fn try_main() -> Result<()> {
    let iface = TunInterface::new()?;
    println!("{:?}", iface.name());

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _cli = Cli::parse();
    try_main().await.unwrap();
}
