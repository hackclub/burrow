use std::mem;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use std::os::fd::FromRawFd;

use clap::{Args, Parser, Subcommand};
use tokio::io::Result;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use burrow::retrieve;
use tun::TunInterface;

mod daemon;

use daemon::{DaemonClient, DaemonCommand, DaemonStartOptions};

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
    /// Stop Burrow daemon
    Stop,
    /// Start Burrow daemon
    Daemon(DaemonArgs),
}

#[derive(Args)]
struct StartArgs {}

#[derive(Args)]
struct RetrieveArgs {}

#[derive(Args)]
struct DaemonArgs {}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_start() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    client
        .send_command(DaemonCommand::Start(DaemonStartOptions::default()))
        .await
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_retrieve() -> Result<()> {
    burrow::ensureroot::ensure_root();
    let iface2 = retrieve();
    println!("{}", iface2);
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_stop() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    client.send_command(DaemonCommand::Stop).await?;
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

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
async fn try_stop() -> Result<()> {
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
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
        Commands::Stop => {
            try_stop().await.unwrap();
        }
        Commands::Daemon(_) => daemon::daemon_main().await?,
    }

    Ok(())
}
