use std::mem;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use std::os::fd::FromRawFd;

use clap::{Args, Parser, Subcommand};
use tokio::io::Result;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use burrow::retrieve;
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
    /// Retrieve the file descriptor of the tun interface
    Retrieve(RetrieveArgs),
}

#[derive(Args)]
struct StartArgs {}

#[derive(Args)]
struct RetrieveArgs {}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_start() -> Result<()> {
    burrow::ensureroot::ensure_root();
    let iface = TunInterface::new()?;
    println!("{:?}", iface.name());
    let iface2 = retrieve();
    println!("{}", iface2);
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_retrieve() -> Result<()> {
    burrow::ensureroot::ensure_root();
    let iface2 = retrieve();
    println!("{}", iface2);
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
async fn try_start() -> Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
async fn try_retrieve() -> Result<()> {
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("Platform: {}", std::env::consts::OS);

    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(..) => {
            try_start().await.unwrap();
            println!("FINISHED");
        }
        Commands::Retrieve(..) => {
            try_retrieve().await.unwrap();
            println!("FINISHED");
        }
    }
}
