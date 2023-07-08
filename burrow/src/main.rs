use clap::{Args, Parser, Subcommand};
use tokio::io::Result;
use tracing_log::LogTracer;
use tracing_oslog::OsLogger;
use tracing_subscriber::prelude::*;
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

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    LogTracer::init().expect("first thing in main - a logger shouldn't have been set yet");

    let logger = tracing_subscriber::registry().with(init_logger_layer());
    tracing::subscriber::set_global_default(logger).expect("Logger shouldn't be set already");

    println!("Platform: {}", std::env::consts::OS);

    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(..) => {
            try_main().await.unwrap();
        }
    }
}

#[cfg(target_os = "linux")]
fn init_logger_layer() ->  tracing_journald::Layer {
    tracing_journald::layer().expect("Couldn't open journald socket - are you using systemd?").with_syslog_identifier("burrow".to_string())
}

#[cfg(target_os="macos")]
fn init_logger_layer() -> _ {
    OsLogger::new("com.hackclub.burrow", "default")
}
