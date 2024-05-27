use std::{borrow::Cow, path::PathBuf};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use crate::daemon::rpc::request::AddConfigOptions;

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

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
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
    /// Reload Config
    ReloadConfig(ReloadConfigArgs),
    /// Add Server Config
    AddConfig(AddServerConfigArgs),
}

#[derive(Args)]
struct AddServerConfigArgs {
    #[clap(short, long)]
    path: PathBuf,
    #[clap(short, long)]
    interface_id: Option<i64>,
}

#[derive(Args)]
struct ReloadConfigArgs {
    #[clap(long, short)]
    interface_id: String,
}

#[derive(Args)]
struct StartArgs {}

#[derive(Args)]
struct DaemonArgs {
    #[clap(long, short)]
    socket_path: Option<PathBuf>,
    #[clap(long, short)]
    db_path: Option<PathBuf>,
}

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
fn handle_unexpected(res: Result<DaemonResponseData, String>) {
    match res {
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
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_serverinfo() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    let res = client.send_command(DaemonCommand::ServerInfo).await?;
    if let Ok(DaemonResponseData::ServerInfo(si)) = res.result {
        println!("Got Result! {:?}", si);
    } else {
        handle_unexpected(res.result);
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_serverconfig() -> Result<()> {
    let mut client = DaemonClient::new().await?;
    let res = client.send_command(DaemonCommand::ServerConfig).await?;
    if let Ok(DaemonResponseData::ServerConfig(cfig)) = res.result {
        println!("Got Result! {:?}", cfig);
    } else {
        handle_unexpected(res.result);
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_reloadconfig(interface_id: String) -> Result<()> {
    let mut client = DaemonClient::new().await?;
    let res = client
        .send_command(DaemonCommand::ReloadConfig(interface_id))
        .await?;
    if let Ok(DaemonResponseData::ServerConfig(cfig)) = res.result {
        println!("Got Result! {:?}", cfig);
    } else {
        handle_unexpected(res.result);
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_add_server_config(path: &PathBuf, interface_id: Option<i64>) -> Result<()> {
    let mut client = DaemonClient::new().await?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy())
        .unwrap_or_else(|| Cow::Borrowed("toml"));
    let content = std::fs::read_to_string(path)?;
    let res = client
        .send_command(DaemonCommand::AddConfig(AddConfigOptions {
            content,
            fmt: ext.to_string(),
            interface_id,
        }))
        .await?;
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing::initialize();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(_) => try_start().await?,
        Commands::Stop => try_stop().await?,
        Commands::Daemon(daemon_args) => {
            daemon::daemon_main(
                daemon_args.socket_path.as_ref().map(|p| p.as_path()),
                daemon_args.db_path.as_ref().map(|p| p.as_path()),
                None,
            )
            .await?
        }
        Commands::ServerInfo => try_serverinfo().await?,
        Commands::ServerConfig => try_serverconfig().await?,
        Commands::ReloadConfig(args) => try_reloadconfig(args.interface_id.clone()).await?,
        Commands::AddConfig(args) => try_add_server_config(&args.path, args.interface_id).await?,
    }

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
pub fn main() {
    eprintln!("This platform is not supported currently.")
}
