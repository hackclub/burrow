use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod daemon;
pub(crate) mod tracing;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use daemon::{DaemonClient, DaemonCommand, DaemonStartOptions};
use tun::TunOptions;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use crate::daemon::DaemonResponseData;

pub mod database;

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
    /// Stop Burrow daemon
    Stop,
    /// Start Burrow daemon
    Daemon(DaemonArgs),
    /// Server Info
    ServerInfo,
    /// Server config
    ServerConfig,
}

#[derive(Args)]
struct StartArgs {}

#[derive(Args)]
struct DaemonArgs {}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_start() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    client
        .send_command(DaemonCommand::Start(DaemonStartOptions {
            tun: TunOptions::new().address(vec!["10.13.13.2", "::2"]),
        }))
        .await
        .map(|_| ())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_stop() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    client.send_command(DaemonCommand::Stop).await?;
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_serverinfo() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    let res = client.send_command(DaemonCommand::ServerInfo).await?;
    match res.result {
        Ok(DaemonResponseData::ServerInfo(si)) => {
            println!("Got Result! {:?}", si);
        }
        Ok(DaemonResponseData::None) => {
            println!("Server not started.")
        }
        Ok(res) => {
            println!("Unexpected Response: {:?}", res)
        }
        Err(e) => {
            println!("Error when retrieving from server: {}", e)
        }
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_serverconfig() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    let res = client.send_command(DaemonCommand::ServerConfig).await?;
    match res.result {
        Ok(DaemonResponseData::ServerConfig(cfig)) => {
            println!("Got Result! {:?}", cfig);
        }
        Ok(DaemonResponseData::None) => {
            println!("Server not started.")
        }
        Ok(res) => {
            println!("Unexpected Response: {:?}", res)
        }
        Err(e) => {
            println!("Error when retrieving from server: {}", e)
        }
    }
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
async fn try_start() -> Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
async fn try_stop() -> Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
async fn try_serverinfo() -> Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
async fn try_serverconfig() -> Result<()> {
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing::initialize();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(..) => try_start().await?,
        Commands::Stop => try_stop().await?,
        Commands::Daemon(_) => daemon::daemon_main(None, None).await?,
        Commands::ServerInfo => try_serverinfo().await?,
        Commands::ServerConfig => try_serverconfig().await?,
    }

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
pub fn main() {
    eprintln!("This platform is not supported currently.")
}
