use std::mem;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use std::os::fd::FromRawFd;

use anyhow::{Context, Result};
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use burrow::retrieve;
use clap::{Args, Parser, Subcommand};
use tracing::instrument;
use tracing_log::LogTracer;
use tracing_oslog::OsLogger;
use tracing_subscriber::{prelude::*, FmtSubscriber};
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

    burrow::ensureroot::ensure_root();
    let iface2 = retrieve();
    tracing::info!("{}", iface2);
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
    tracing::info!("Platform: {}", std::env::consts::OS);

    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(..) => {
            try_start().await.unwrap();
            tracing::info!("FINISHED");
        }
        Commands::Retrieve(..) => {
            try_retrieve().await.unwrap();
            tracing::info!("FINISHED");
        }
        Commands::Stop => {
            try_stop().await.unwrap();
        }
        Commands::Daemon(_) => daemon::daemon_main().await?,
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
    Ok(Some(OsLogger::new("com.hackclub.burrow", "default")))
}
