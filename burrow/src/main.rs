use clap::{Args, Parser, Subcommand};
use tokio::io::Result;
use tun::TunInterface;

#[derive(Parser)]
#[command(name = "Burrow")]
#[command(author = "Hack Club <team@hackclub.com>")]
#[command(version = "0.1")]
#[command(about = "Burrow is a tool for burrowing through firewalls, built by teenagers at Hack Club.", long_about = None)]

struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Start(StartArgs),
}

#[derive(Args)]
struct StartArgs {}

async fn try_main() -> Result<()> {
    let iface = TunInterface::new()?;
    println!("{:?}", iface.name());

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(..) => {
            try_main().await.unwrap();
        }
    }
}
