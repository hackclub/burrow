use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod control;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod daemon;
pub(crate) mod tracing;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod auth;
#[cfg(target_os = "linux")]
mod tor;
#[cfg(target_os = "linux")]
mod usernet;

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
    /// Discover a Tailnet authority through the daemon
    TailnetDiscover(TailnetDiscoverArgs),
    /// Probe a Tailnet authority through the daemon
    TailnetProbe(TailnetProbeArgs),
    /// Send an ICMP echo probe through the active Tailnet tunnel over daemon packet streaming
    TailnetPing(TailnetPingArgs),
    /// Send a UDP echo probe through the active Tailnet tunnel over daemon packet streaming
    TailnetUdpEcho(TailnetUdpEchoArgs),
    #[cfg(target_os = "linux")]
    /// Run a command in an unshared Linux namespace using a Burrow backend
    Exec(ExecArgs),
    #[cfg(target_os = "linux")]
    /// Run a command in a Linux user namespace with Tor-backed networking
    TorExec(TorExecArgs),
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

#[derive(Args)]
struct TailnetDiscoverArgs {
    email: String,
}

#[derive(Args)]
struct TailnetProbeArgs {
    authority: String,
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[derive(Args)]
struct TailnetPingArgs {
    remote: String,
    #[arg(long, default_value = "burrow-tailnet-smoke")]
    payload: String,
    #[arg(long, default_value_t = 5000)]
    timeout_ms: u64,
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[derive(Args)]
struct TailnetUdpEchoArgs {
    remote: String,
    #[arg(long, default_value = "burrow-tailnet-smoke")]
    message: String,
    #[arg(long, default_value_t = 5000)]
    timeout_ms: u64,
}

#[cfg(target_os = "linux")]
#[derive(Args)]
struct TorExecArgs {
    payload_path: String,
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
    command: Vec<String>,
}

#[cfg(target_os = "linux")]
#[derive(Args)]
struct ExecArgs {
    #[arg(long, value_enum)]
    backend: usernet::ExecBackendKind,
    #[arg(long)]
    payload: Option<String>,
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
    command: Vec<String>,
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
async fn try_tailnet_discover(email: &str) -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    let response = client
        .tailnet_client
        .discover(crate::daemon::rpc::grpc_defs::TailnetDiscoverRequest { email: email.to_owned() })
        .await?
        .into_inner();
    println!("Tailnet Discover Response: {:?}", response);
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_tailnet_probe(authority: &str) -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    let response = client
        .tailnet_client
        .probe(crate::daemon::rpc::grpc_defs::TailnetProbeRequest {
            authority: authority.to_owned(),
        })
        .await?
        .into_inner();
    println!("Tailnet Probe Response: {:?}", response);
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_tailnet_ping(remote: &str, payload: &str, timeout_ms: u64) -> Result<()> {
    use std::net::IpAddr;

    use anyhow::Context;
    use rand::Rng;
    use tokio::{
        sync::mpsc,
        time::{timeout, Duration},
    };
    use tokio_stream::wrappers::ReceiverStream;

    use crate::daemon::rpc::grpc_defs::{Empty, TunnelPacket};

    let remote_ip: IpAddr = remote
        .parse()
        .with_context(|| format!("invalid remote IP address {remote}"))?;
    let message = payload.as_bytes().to_vec();

    let mut client = BurrowClient::from_uds().await?;
    client.tunnel_client.tunnel_start(Empty {}).await?;

    let mut config_stream = client
        .tunnel_client
        .tunnel_configuration(Empty {})
        .await?
        .into_inner();
    let config = config_stream
        .message()
        .await?
        .context("tunnel configuration stream ended before yielding a config")?;
    let local_ip = select_tailnet_local_ip(&config.addresses, remote_ip)?;

    let identifier = rand::thread_rng().gen::<u16>();
    let sequence = 1_u16;
    let packet = build_icmp_echo_request(local_ip, remote_ip, identifier, sequence, &message)?;

    let (outbound_tx, outbound_rx) = mpsc::channel::<TunnelPacket>(128);
    let mut tunnel_packets = client
        .tunnel_client
        .tunnel_packets(ReceiverStream::new(outbound_rx))
        .await?
        .into_inner();

    outbound_tx
        .send(TunnelPacket { payload: packet })
        .await
        .context("failed to send ICMP echo probe into daemon packet stream")?;
    log::debug!(
        "tailnet ping probe queued from {local_ip} to {remote_ip} identifier={identifier} sequence={sequence}"
    );
    drop(outbound_tx);

    let reply = timeout(Duration::from_millis(timeout_ms), async {
        loop {
            let packet = tunnel_packets
                .message()
                .await
                .context("failed to read packet from daemon packet stream")?
                .context("daemon packet stream ended before returning a reply")?;
            log::debug!(
                "tailnet ping received {} bytes from daemon packet stream",
                packet.payload.len()
            );
            if let Some(reply) =
                parse_icmp_echo_reply(&packet.payload, local_ip, remote_ip, identifier, sequence)?
            {
                break Ok::<_, anyhow::Error>(reply);
            }
        }
    })
    .await
    .with_context(|| format!("timed out waiting for ICMP echo reply from {remote_ip}"))??;

    println!("Tailnet Ping Source: {}", reply.source);
    println!("Tailnet Ping Destination: {}", reply.destination);
    println!(
        "Tailnet Ping Payload: {}",
        String::from_utf8_lossy(&reply.payload)
    );
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
async fn try_tailnet_udp_echo(remote: &str, message: &str, timeout_ms: u64) -> Result<()> {
    use std::net::SocketAddr;

    use anyhow::{bail, Context};
    use futures::{SinkExt, StreamExt};
    use netstack_smoltcp::StackBuilder;
    use tokio::{
        sync::mpsc,
        time::{timeout, Duration},
    };
    use tokio_stream::wrappers::ReceiverStream;

    use crate::daemon::rpc::grpc_defs::{Empty, TunnelPacket};

    let remote_addr: SocketAddr = remote
        .parse()
        .with_context(|| format!("invalid remote socket address {remote}"))?;

    let mut client = BurrowClient::from_uds().await?;
    client.tunnel_client.tunnel_start(Empty {}).await?;

    let mut config_stream = client
        .tunnel_client
        .tunnel_configuration(Empty {})
        .await?
        .into_inner();
    let config = config_stream
        .message()
        .await?
        .context("tunnel configuration stream ended before yielding a config")?;
    let local_addr = select_tailnet_local_socket(&config.addresses, remote_addr.ip())?;

    let (stack, runner, udp_socket, _) = StackBuilder::default()
        .enable_udp(true)
        .enable_tcp(true)
        .build()
        .context("failed to build userspace UDP stack")?;
    let runner = runner.context("userspace UDP stack runner unavailable")?;
    let udp_socket = udp_socket.context("userspace UDP stack socket unavailable")?;
    let (mut stack_sink, mut stack_stream) = stack.split();
    let (mut udp_reader, mut udp_writer) = udp_socket.split();

    let (outbound_tx, outbound_rx) = mpsc::channel::<TunnelPacket>(128);
    let mut tunnel_packets = client
        .tunnel_client
        .tunnel_packets(ReceiverStream::new(outbound_rx))
        .await?
        .into_inner();

    let ingress_task = tokio::spawn(async move {
        loop {
            match tunnel_packets.message().await? {
                Some(packet) => {
                    log::debug!(
                        "tailnet udp echo received {} bytes from daemon packet stream",
                        packet.payload.len()
                    );
                    stack_sink
                        .send(packet.payload)
                        .await
                        .context("failed to feed inbound tailnet packet into userspace stack")?;
                }
                None => break,
            }
        }
        Result::<()>::Ok(())
    });

    let egress_task = tokio::spawn(async move {
        while let Some(packet) = stack_stream.next().await {
            let payload = packet.context("failed to read outbound packet from userspace stack")?;
            log::debug!(
                "tailnet udp echo sending {} bytes into daemon packet stream",
                payload.len()
            );
            outbound_tx
                .send(TunnelPacket { payload })
                .await
                .context("failed to forward outbound tailnet packet to daemon")?;
        }
        Result::<()>::Ok(())
    });

    let runner_task = tokio::spawn(async move { runner.await.map_err(anyhow::Error::from) });

    udp_writer
        .send((message.as_bytes().to_vec(), local_addr, remote_addr))
        .await
        .context("failed to send UDP echo probe into userspace stack")?;
    log::debug!("tailnet udp echo probe queued from {local_addr} to {remote_addr}");

    let response = timeout(Duration::from_millis(timeout_ms), udp_reader.next())
        .await
        .with_context(|| format!("timed out waiting for UDP echo from {remote_addr}"))?
        .context("userspace UDP stack ended before returning a reply")?;
    let (payload, reply_source, reply_destination) = response;
    let response_text = String::from_utf8_lossy(&payload);

    ingress_task.abort();
    egress_task.abort();
    runner_task.abort();

    if reply_source != remote_addr {
        bail!("received UDP reply from unexpected source {reply_source}");
    }
    if reply_destination != local_addr {
        bail!("received UDP reply for unexpected local socket {reply_destination}");
    }
    if payload != message.as_bytes() {
        bail!("UDP echo payload mismatch");
    }

    println!("Tailnet UDP Echo Source: {reply_source}");
    println!("Tailnet UDP Echo Destination: {reply_destination}");
    println!("Tailnet UDP Echo Payload: {response_text}");
    Ok(())
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn select_tailnet_local_ip(
    addresses: &[String],
    remote_ip: std::net::IpAddr,
) -> Result<std::net::IpAddr> {
    use anyhow::Context;

    let family_is_v4 = remote_ip.is_ipv4();
    addresses
        .iter()
        .filter_map(|cidr| cidr.split('/').next())
        .filter_map(|ip| ip.parse::<std::net::IpAddr>().ok())
        .find(|ip| ip.is_ipv4() == family_is_v4)
        .with_context(|| {
            format!(
                "no local {} tailnet address found in daemon config {:?}",
                if family_is_v4 { "IPv4" } else { "IPv6" },
                addresses
            )
        })
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn select_tailnet_local_socket(
    addresses: &[String],
    remote_ip: std::net::IpAddr,
) -> Result<std::net::SocketAddr> {
    use rand::Rng;

    let local_ip = select_tailnet_local_ip(addresses, remote_ip)?;
    let port = rand::thread_rng().gen_range(40000..50000);
    Ok(std::net::SocketAddr::new(local_ip, port))
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
struct IcmpEchoReply {
    source: std::net::IpAddr,
    destination: std::net::IpAddr,
    payload: Vec<u8>,
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn build_icmp_echo_request(
    source: std::net::IpAddr,
    destination: std::net::IpAddr,
    identifier: u16,
    sequence: u16,
    payload: &[u8],
) -> Result<Vec<u8>> {
    use anyhow::bail;

    let (source, destination) = match (source, destination) {
        (std::net::IpAddr::V4(source), std::net::IpAddr::V4(destination)) => (source, destination),
        _ => bail!("tailnet ping currently supports IPv4 only"),
    };

    let mut icmp = Vec::with_capacity(8 + payload.len());
    icmp.push(8);
    icmp.push(0);
    icmp.extend_from_slice(&[0, 0]);
    icmp.extend_from_slice(&identifier.to_be_bytes());
    icmp.extend_from_slice(&sequence.to_be_bytes());
    icmp.extend_from_slice(payload);
    let icmp_checksum = internet_checksum(&icmp);
    icmp[2..4].copy_from_slice(&icmp_checksum.to_be_bytes());

    let total_len = 20 + icmp.len();
    let mut packet = Vec::with_capacity(total_len);
    packet.push(0x45);
    packet.push(0);
    packet.extend_from_slice(&(total_len as u16).to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.push(64);
    packet.push(1);
    packet.extend_from_slice(&[0, 0]);
    packet.extend_from_slice(&source.octets());
    packet.extend_from_slice(&destination.octets());
    let header_checksum = internet_checksum(&packet);
    packet[10..12].copy_from_slice(&header_checksum.to_be_bytes());
    packet.extend_from_slice(&icmp);
    Ok(packet)
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn parse_icmp_echo_reply(
    packet: &[u8],
    local_ip: std::net::IpAddr,
    remote_ip: std::net::IpAddr,
    identifier: u16,
    sequence: u16,
) -> Result<Option<IcmpEchoReply>> {
    use anyhow::bail;

    let (local_ip, remote_ip) = match (local_ip, remote_ip) {
        (std::net::IpAddr::V4(local_ip), std::net::IpAddr::V4(remote_ip)) => (local_ip, remote_ip),
        _ => bail!("tailnet ping currently supports IPv4 only"),
    };

    if packet.len() < 20 {
        return Ok(None);
    }
    let version = packet[0] >> 4;
    if version != 4 {
        return Ok(None);
    }
    let ihl = (packet[0] & 0x0f) as usize * 4;
    if packet.len() < ihl + 8 {
        return Ok(None);
    }
    if packet[9] != 1 {
        return Ok(None);
    }

    let source = std::net::Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]);
    let destination = std::net::Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]);
    if source != remote_ip || destination != local_ip {
        return Ok(None);
    }

    let icmp = &packet[ihl..];
    if icmp[0] != 0 || icmp[1] != 0 {
        return Ok(None);
    }
    let reply_identifier = u16::from_be_bytes([icmp[4], icmp[5]]);
    let reply_sequence = u16::from_be_bytes([icmp[6], icmp[7]]);
    if reply_identifier != identifier || reply_sequence != sequence {
        return Ok(None);
    }

    Ok(Some(IcmpEchoReply {
        source: std::net::IpAddr::V4(source),
        destination: std::net::IpAddr::V4(destination),
        payload: icmp[8..].to_vec(),
    }))
}

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn internet_checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&last) = chunks.remainder().first() {
        sum += (last as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(target_os = "linux")]
async fn try_tor_exec(payload_path: &str, command: Vec<String>) -> Result<()> {
    let exit_code = usernet::run_exec(usernet::ExecInvocation {
        backend: usernet::ExecBackendKind::Tor,
        payload_path: Some(payload_path.into()),
        command,
    })
    .await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn try_exec(
    backend: usernet::ExecBackendKind,
    payload: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    let exit_code = usernet::run_exec(usernet::ExecInvocation {
        backend,
        payload_path: payload.map(Into::into),
        command,
    })
    .await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
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
        Commands::TailnetDiscover(args) => try_tailnet_discover(&args.email).await?,
        Commands::TailnetProbe(args) => try_tailnet_probe(&args.authority).await?,
        Commands::TailnetPing(args) => {
            try_tailnet_ping(&args.remote, &args.payload, args.timeout_ms).await?
        }
        Commands::TailnetUdpEcho(args) => {
            try_tailnet_udp_echo(&args.remote, &args.message, args.timeout_ms).await?
        }
        #[cfg(target_os = "linux")]
        Commands::Exec(args) => {
            try_exec(
                args.backend.clone(),
                args.payload.clone(),
                args.command.clone(),
            )
            .await?
        }
        #[cfg(target_os = "linux")]
        Commands::TorExec(args) => try_tor_exec(&args.payload_path, args.command.clone()).await?,
    }

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
pub fn main() {
    eprintln!("This platform is not supported")
}
