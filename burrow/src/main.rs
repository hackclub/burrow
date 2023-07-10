use std::mem;
use std::os::fd::FromRawFd;

use clap::{Args, Parser, Subcommand};
use tokio::io::Result;
use tun::TunInterface;

#[derive(Parser)]
#[command(name = "Burrow")]
#[command(author = "Hack Club <team@hackclub.com>")]
#[command(version = "0.1")]
#[command(
    about = "Burrow is a tool for burrowing through firewalls, built by teenagers at Hack Club.",
    long_about = "Burrow is a 🚀 blazingly fast 🚀 tool designed to penetrate unnecessarily restrictive firewalls, providing teenagers worldwide with secure, less-filtered, and safe access to the internet!
It's being built by teenagers from Hack Club, in public! Check it out: https://github.com/hackclub/burrow
Spotted a bug? Please open an issue! https://github.com/hackclub/burrow/issues/new"
)]

struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start Burrow
    Start(StartArgs),
}

#[derive(Args)]
struct StartArgs {}

async fn try_main() -> Result<()> {
    burrow::ensureroot::ensure_root();

    let iface = TunInterface::new()?;
    println!("{:?}", iface.name());

    let iface2 = (1..100)
        .filter_map(|i| {
            let iface = unsafe { TunInterface::from_raw_fd(i) };
            match iface.name() {
                Ok(_name) => Some(iface),
                Err(_) => {
                    mem::forget(iface);
                    None
                }
            }
        })
        .next()
        .unwrap();
    println!("I am printing..");
    println!("{:?}", iface2.name());

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("Platform: {}", std::env::consts::OS);

    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(..) => {
            try_main().await.unwrap();
            println!("FINISHED");
        }
    }
}
