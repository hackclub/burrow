use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod daemon;
pub(crate) mod tracing;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod auth;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use daemon::{DaemonClient, DaemonCommand};

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use crate::daemon::DaemonResponseData;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub mod database;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use crate::daemon::rpc::{grpc_defs::Empty, BurrowClient};

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
    /// Authentication server
    AuthServer,
    /// Server Status
    ServerStatus,
    /// Tunnel Config
    TunnelConfig,
    /// Add Network
    NetworkAdd(NetworkAddArgs),
    /// List Networks
    NetworkList,
    /// Reorder Network
    NetworkReorder(NetworkReorderArgs),
    /// Delete Network
    NetworkDelete(NetworkDeleteArgs),
}

#[derive(Args)]
struct ReloadConfigArgs {
    #[clap(long, short)]
    interface_id: String,
}

#[derive(Args)]
struct StartArgs {}

#[derive(Args)]
struct DaemonArgs {}

#[derive(Args)]
struct NetworkAddArgs {
    id: i32,
    network_type: i32,
    payload_path: String,
}

#[derive(Args)]
struct NetworkReorderArgs {
    id: i32,
    index: i32,
}

#[derive(Args)]
struct NetworkDeleteArgs {
    id: i32,
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_start() -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    let res = client.tunnel_client.tunnel_start(Empty {}).await?;
    println!("Got results! {:?}", res);
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_stop() -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    let res = client.tunnel_client.tunnel_stop(Empty {}).await?;
    println!("Got results! {:?}", res);
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_serverstatus() -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    let mut res = client
        .tunnel_client
        .tunnel_status(Empty {})
        .await?
        .into_inner();
    if let Some(st) = res.message().await? {
        println!("Server Status: {:?}", st);
    } else {
        println!("Server Status is None");
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_tun_config() -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    let mut res = client
        .tunnel_client
        .tunnel_configuration(Empty {})
        .await?
        .into_inner();
    if let Some(config) = res.message().await? {
        println!("Tunnel Config: {:?}", config);
    } else {
        println!("Tunnel Config is None");
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_network_add(id: i32, network_type: i32, payload_path: &str) -> Result<()> {
    use tokio::{fs::File, io::AsyncReadExt};

    use crate::daemon::rpc::grpc_defs::Network;

    let mut file = File::open(payload_path).await?;
    let mut payload = Vec::new();
    file.read_to_end(&mut payload).await?;

    let mut client = BurrowClient::from_uds().await?;
    let network = Network {
        id,
        r#type: network_type,
        payload,
    };
    let res = client.networks_client.network_add(network).await?;
    println!("Network Add Response: {:?}", res);
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_network_list() -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    let mut res = client
        .networks_client
        .network_list(Empty {})
        .await?
        .into_inner();
    while let Some(network_list) = res.message().await? {
        println!("Network List: {:?}", network_list);
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_network_reorder(id: i32, index: i32) -> Result<()> {
    use crate::daemon::rpc::grpc_defs::NetworkReorderRequest;

    let mut client = BurrowClient::from_uds().await?;
    let reorder_request = NetworkReorderRequest { id, index };
    let res = client
        .networks_client
        .network_reorder(reorder_request)
        .await?;
    println!("Network Reorder Response: {:?}", res);
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_network_delete(id: i32) -> Result<()> {
    use crate::daemon::rpc::grpc_defs::NetworkDeleteRequest;

    let mut client = BurrowClient::from_uds().await?;
    let delete_request = NetworkDeleteRequest { id };
    let res = client
        .networks_client
        .network_delete(delete_request)
        .await?;
    println!("Network Delete Response: {:?}", res);
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
#[tokio::main]
async fn main() -> Result<()> {
    tracing::initialize();
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Start(..) => try_start().await?,
        Commands::Stop => try_stop().await?,
        Commands::Daemon(_) => daemon::daemon_main(None, None, None).await?,
        Commands::ServerInfo => try_serverinfo().await?,
        Commands::ServerConfig => try_serverconfig().await?,
        Commands::ReloadConfig(args) => try_reloadconfig(args.interface_id.clone()).await?,
        Commands::AuthServer => crate::auth::server::serve().await?,
        Commands::ServerStatus => try_serverstatus().await?,
        Commands::TunnelConfig => try_tun_config().await?,
        Commands::NetworkAdd(args) => {
            try_network_add(args.id, args.network_type, &args.payload_path).await?
        }
        Commands::NetworkList => try_network_list().await?,
        Commands::NetworkReorder(args) => try_network_reorder(args.id, args.index).await?,
        Commands::NetworkDelete(args) => try_network_delete(args.id).await?,
    }

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
pub fn main() {
    eprintln!("This platform is not supported")
}
