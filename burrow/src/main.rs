use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use tracing::instrument;
use tracing_log::LogTracer;
use tracing_oslog::OsLogger;
use tracing_subscriber::{prelude::*, EnvFilter, FmtSubscriber};
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use tun::TunInterface;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod daemon;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use daemon::{DaemonClient, DaemonCommand, DaemonStartOptions};
use tun::TunOptions;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use crate::daemon::DaemonResponseData;

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
    /// Server Info
    ServerInfo,
    /// Server config
    ServerConfig,
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
        .send_command(DaemonCommand::Start(DaemonStartOptions {
            tun: TunOptions::new().address("10.13.13.2"),
        }))
        .await
        .map(|_| ())
}

#[cfg(target_vendor = "apple")]
#[instrument]
async fn try_retrieve() -> Result<()> {
    LogTracer::init()
        .context("Failed to initialize LogTracer")
        .unwrap();

    if cfg!(target_os = "linux") || cfg!(target_vendor = "apple") {
        let maybe_layer = system_log().unwrap();
        if let Some(layer) = maybe_layer {
            let logger = layer.with_subscriber(FmtSubscriber::new());
            tracing::subscriber::set_global_default(logger)
                .context("Failed to set the global tracing subscriber")
                .unwrap();
        }
    }

    let iface2 = TunInterface::retrieve().ok_or(anyhow::anyhow!("No interface found"))?;
    tracing::info!("{:?}", iface2);
    Ok(())
}

async fn initialize_tracing() -> Result<()> {
    LogTracer::init().context("Failed to initialize LogTracer")?;

    #[cfg(any(target_os = "linux", target_vendor = "apple"))]
    {
        let maybe_layer = system_log()?;
        if let Some(layer) = maybe_layer {
            let logger = layer.with_subscriber(
                FmtSubscriber::builder()
                    .with_line_number(true)
                    .with_env_filter(EnvFilter::from_default_env())
                    .finish(),
            );
            tracing::subscriber::set_global_default(logger)
                .context("Failed to set the global tracing subscriber")?;
        }
    }

    Ok(())
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

#[cfg(not(target_vendor = "apple"))]
async fn try_retrieve() -> Result<()> {
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
    initialize_tracing().await?;
    tracing::info!("Platform: {}", std::env::consts::OS);

    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(..) => {
            try_start().await?;
            tracing::info!("FINISHED");
        }
        Commands::Retrieve(..) => {
            try_retrieve().await?;
            tracing::info!("FINISHED");
        }
        Commands::Stop => {
            try_stop().await?;
        }
        Commands::Daemon(_) => daemon::daemon_main(None).await?,
        Commands::ServerInfo => try_serverinfo().await?,
        Commands::ServerConfig => try_serverconfig().await?,
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn system_log() -> Result<Option<tracing_journald::Layer>> {
    let maybe_journald = tracing_journald::layer();
    match maybe_journald {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::trace!("journald not found");
            Ok(None)
        }
        _ => Ok(Some(maybe_journald?)),
    }
}

#[cfg(target_vendor = "apple")]
fn system_log() -> Result<Option<OsLogger>> {
    Ok(Some(OsLogger::new("com.hackclub.burrow", "burrow-cli")))
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
pub fn main() {
    eprintln!("This platform is not supported currently.")
}
