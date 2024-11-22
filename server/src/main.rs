pub mod client;
pub mod server;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use server::providers::gen_keypem;

#[derive(Parser)]
#[command(name = "Burrow Server")]
#[command(author = "Hack Club <team@hackclub.com>")]
#[command(version = "0.1")]
#[command(
    about = "Server for hosting auth logic of Burrow",
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
    StartServer,
    #[command(name = "genkeys")]
    GenKeys(GenKeyArgs),
}

#[derive(Args)]
pub struct GenKeyArgs {
    #[arg(short, long, default_value = "false")]
    pub raw: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::GenKeys(args) => {
            let pem = gen_keypem();
            if args.raw {
                println!(r"{pem:?}");
            } else {
                println!("Generated PEM:\n{pem}")
            }
        }
        Commands::StartServer => todo!(),
    };
    Ok(())
}
