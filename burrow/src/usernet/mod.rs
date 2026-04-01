use std::{
    collections::HashMap,
    env,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    os::fd::{AsRawFd, FromRawFd, RawFd},
    os::unix::net::UnixStream as StdUnixStream,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::{Command as StdCommand, ExitStatus},
    str,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::ValueEnum;
use futures::{SinkExt, StreamExt};
use ipnetwork::IpNetwork;
use netstack_smoltcp::{
    StackBuilder, TcpListener as StackTcpListener, TcpStream as StackTcpStream,
    UdpSocket as StackUdpSocket,
};
use nix::{
    cmsg_space,
    fcntl::{fcntl, FcntlArg, FdFlag},
    sys::socket::{recvmsg, sendmsg, ControlMessage, ControlMessageOwned, MsgFlags},
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::copy_bidirectional,
    net::{TcpStream, UdpSocket},
    process::{Child, Command},
    sync::{mpsc, Mutex, RwLock},
    task::JoinSet,
};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tracing::{debug, warn};
use tun::{tokio::TunInterface as TokioTunInterface, TunOptions};

use crate::{
    tor::{bootstrap_client, dns::build_response as build_tor_dns_response, Config as TorConfig},
    wireguard::{Config as WireGuardConfig, Interface as WireGuardInterface},
};

const INNER_ENV: &str = "BURROW_USERNET_INNER";
const INNER_CONTROL_FD_ENV: &str = "BURROW_USERNET_CONTROL_FD";
const INNER_TUN_CONFIG_ENV: &str = "BURROW_USERNET_TUN_CONFIG";
const DEFAULT_MTU: u32 = 1500;
const DEFAULT_TUN_V4: &str = "100.64.0.2/24";
const DEFAULT_TUN_V6: &str = "fd00:64::2/64";
const UDP_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const READY_ACK: &[u8; 1] = b"1";

#[derive(Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum ExecBackendKind {
    Direct,
    Tor,
    Wireguard,
}

impl ExecBackendKind {
    fn cli_name(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Tor => "tor",
            Self::Wireguard => "wireguard",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExecInvocation {
    pub backend: ExecBackendKind,
    pub payload_path: Option<PathBuf>,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectConfig {
    #[serde(default)]
    pub address: Vec<String>,
    #[serde(default)]
    pub dns: Vec<String>,
    #[serde(default)]
    pub mtu: Option<u32>,
    #[serde(default)]
    pub tun_name: Option<String>,
}

impl DirectConfig {
    pub fn from_payload(payload: &[u8]) -> Result<Self> {
        if payload.is_empty() {
            return Ok(Self::default());
        }

        if let Ok(config) = serde_json::from_slice(payload) {
            return Ok(config);
        }

        let payload = str::from_utf8(payload).context("direct payload must be valid UTF-8")?;
        toml::from_str(payload).context("failed to parse direct payload as JSON or TOML")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TunNetworkConfig {
    tun_name: String,
    addresses: Vec<IpNetwork>,
    mtu: u32,
}

enum PreparedBackend {
    Socket {
        backend: SocketBackend,
        tun_config: TunNetworkConfig,
    },
    Wireguard {
        config: WireGuardConfig,
        tun_config: TunNetworkConfig,
    },
}

impl PreparedBackend {
    fn tun_config(&self) -> &TunNetworkConfig {
        match self {
            Self::Socket { tun_config, .. } => tun_config,
            Self::Wireguard { tun_config, .. } => tun_config,
        }
    }
}

struct NamespaceChild {
    child: Child,
    control: StdUnixStream,
}

#[derive(Clone)]
enum SocketBackend {
    Direct,
    Tor(Arc<arti_client::TorClient<tor_rtcompat::PreferredRuntime>>),
}

#[derive(Debug)]
struct UdpReply {
    payload: Vec<u8>,
    source: SocketAddr,
    destination: SocketAddr,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
struct UdpFlowKey {
    local: SocketAddr,
    remote: SocketAddr,
}

pub async fn run_exec(invocation: ExecInvocation) -> Result<i32> {
    if invocation.command.is_empty() {
        bail!("exec requires a command to run");
    }

    if env::var_os(INNER_ENV).is_some() {
        run_inner(invocation.command).await
    } else {
        run_supervisor(invocation).await
    }
}

async fn run_supervisor(invocation: ExecInvocation) -> Result<i32> {
    let prepared = prepare_backend(&invocation).await?;
    let mut child = spawn_namespaced_child(&invocation, prepared.tun_config())?;
    let tun = child.receive_tun().await?;

    match prepared {
        PreparedBackend::Socket { backend, .. } => run_socket_backend(backend, tun, child).await,
        PreparedBackend::Wireguard { config, .. } => {
            run_wireguard_backend(config, tun, child).await
        }
    }
}

async fn prepare_backend(invocation: &ExecInvocation) -> Result<PreparedBackend> {
    match invocation.backend {
        ExecBackendKind::Direct => {
            let payload = read_optional_payload(invocation.payload_path.as_deref()).await?;
            let config = DirectConfig::from_payload(&payload)?;
            let tun_config = socket_tun_config(
                &config.address,
                config.mtu,
                config.tun_name.as_deref(),
                "burrow-direct",
            )?;
            Ok(PreparedBackend::Socket {
                backend: SocketBackend::Direct,
                tun_config,
            })
        }
        ExecBackendKind::Tor => {
            let payload = read_required_payload(invocation.payload_path.as_deref(), "tor").await?;
            let mut config = TorConfig::from_payload(&payload)?;
            let (state_dir, cache_dir) = config.runtime_dirs(std::process::id() as i32);
            config.arti.state_dir = state_dir;
            config.arti.cache_dir = cache_dir;
            let tun_config = socket_tun_config(
                &config.address,
                config.mtu,
                config.tun_name.as_deref(),
                "burrow-tor",
            )?;
            let tor_client = bootstrap_client(&config).await?;
            Ok(PreparedBackend::Socket {
                backend: SocketBackend::Tor(tor_client),
                tun_config,
            })
        }
        ExecBackendKind::Wireguard => {
            let payload =
                read_required_payload(invocation.payload_path.as_deref(), "wireguard").await?;
            let config = parse_wireguard_payload(&payload, invocation.payload_path.as_deref())?;
            let tun_config = wireguard_tun_config(&config)?;
            Ok(PreparedBackend::Wireguard { config, tun_config })
        }
    }
}

fn spawn_namespaced_child(
    invocation: &ExecInvocation,
    tun_config: &TunNetworkConfig,
) -> Result<NamespaceChild> {
    ensure_tool("unshare")?;
    ensure_tool("ip")?;

    let (parent_control, child_control) =
        StdUnixStream::pair().context("failed to create namespace control socket")?;
    set_inheritable(child_control.as_raw_fd())?;

    let current_exe = env::current_exe().context("failed to locate current burrow binary")?;
    let mut cmd = Command::new("unshare");
    cmd.args([
        "--user",
        "--map-root-user",
        "--net",
        "--mount",
        "--pid",
        "--fork",
        "--kill-child",
        "--mount-proc",
    ]);
    cmd.env(INNER_ENV, "1");
    cmd.env(INNER_CONTROL_FD_ENV, child_control.as_raw_fd().to_string());
    cmd.env(
        INNER_TUN_CONFIG_ENV,
        serde_json::to_string(tun_config).context("failed to encode namespace tun config")?,
    );
    cmd.arg(current_exe);
    cmd.arg("exec");
    cmd.args(["--backend", invocation.backend.cli_name()]);
    if let Some(payload_path) = &invocation.payload_path {
        cmd.arg("--payload");
        cmd.arg(payload_path);
    }
    cmd.arg("--");
    cmd.args(&invocation.command);

    let child = cmd
        .spawn()
        .context("failed to enter unshared Linux namespace")?;
    drop(child_control);

    Ok(NamespaceChild { child, control: parent_control })
}

async fn run_inner(command: Vec<String>) -> Result<i32> {
    run_ip(["link", "set", "lo", "up"])?;
    let tun_config = read_inner_tun_config()?;
    let tun = open_tun_device(&tun_config)?;
    configure_tun_addresses(&tun, &tun_config.addresses, tun_config.mtu)?;
    let name = tun.name().context("failed to retrieve tun device name")?;
    run_ip(["link", "set", "dev", &name, "up"])?;
    install_default_routes(&name, &tun_config.addresses)?;

    let control_fd = env::var(INNER_CONTROL_FD_ENV)
        .context("missing namespace control fd")?
        .parse::<RawFd>()
        .context("invalid namespace control fd")?;
    send_tun_fd(control_fd, tun.as_raw_fd())?;
    await_parent_ready(control_fd).await?;
    drop(tun);

    let status = spawn_child(&command).await?;
    child_exit_code(status)
}

impl NamespaceChild {
    async fn receive_tun(&mut self) -> Result<TokioTunInterface> {
        let control = self
            .control
            .try_clone()
            .context("failed to clone namespace control socket")?;
        let fd = tokio::task::spawn_blocking(move || recv_tun_fd(&control))
            .await
            .context("failed to join namespace tun receive task")??;
        tokio_tun_from_fd(fd)
    }

    async fn signal_ready(&self) -> Result<()> {
        let mut control = self
            .control
            .try_clone()
            .context("failed to clone namespace control socket")?;
        tokio::task::spawn_blocking(move || -> Result<()> {
            std::io::Write::write_all(&mut control, READY_ACK)
                .context("failed to acknowledge namespace readiness")?;
            Ok(())
        })
        .await
        .context("failed to join namespace ready task")??;
        Ok(())
    }

    async fn wait(mut self) -> Result<ExitStatus> {
        self.child
            .wait()
            .await
            .context("failed to wait for namespace child")
    }
}

async fn run_socket_backend(
    backend: SocketBackend,
    tun: TokioTunInterface,
    child: NamespaceChild,
) -> Result<i32> {
    let tun = Arc::new(tun);
    let (stack, runner, udp_socket, tcp_listener) = StackBuilder::default()
        .stack_buffer_size(1024)
        .udp_buffer_size(1024)
        .tcp_buffer_size(1024)
        .enable_udp(true)
        .enable_tcp(true)
        .enable_icmp(true)
        .build()
        .context("failed to build userspace netstack")?;
    let (mut stack_sink, mut stack_stream) = stack.split();

    let mut tasks = JoinSet::new();
    if let Some(runner) = runner {
        tasks.spawn(async move { runner.await.map_err(anyhow::Error::from) });
    }

    {
        let tun = tun.clone();
        tasks.spawn(async move {
            let mut buf = vec![0u8; 65_535];
            loop {
                let len = tun
                    .recv(&mut buf)
                    .await
                    .context("failed to read packet from tun")?;
                if len == 0 {
                    continue;
                }
                stack_sink
                    .send(buf[..len].to_vec())
                    .await
                    .context("failed to send tun packet into userspace stack")?;
            }
            #[allow(unreachable_code)]
            Result::<()>::Ok(())
        });
    }

    {
        let tun = tun.clone();
        tasks.spawn(async move {
            while let Some(packet) = stack_stream.next().await {
                let packet = packet.context("failed to receive packet from userspace stack")?;
                tun.send(&packet)
                    .await
                    .context("failed to write userspace stack packet to tun")?;
            }
            Result::<()>::Ok(())
        });
    }

    if let Some(tcp_listener) = tcp_listener {
        let backend = backend.clone();
        tasks.spawn(async move { tcp_dispatch_loop(tcp_listener, backend).await });
    }

    if let Some(udp_socket) = udp_socket {
        tasks.spawn(async move { udp_dispatch_loop(udp_socket, backend).await });
    }

    child.signal_ready().await?;
    let status = child.wait().await?;

    tasks.abort_all();
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(err)) => debug!(?err, "usernet background task exited with error"),
            Err(err) if err.is_cancelled() => {}
            Err(err) => debug!(?err, "usernet background task panicked"),
        }
    }

    child_exit_code(status)
}

async fn run_wireguard_backend(
    config: WireGuardConfig,
    tun: TokioTunInterface,
    child: NamespaceChild,
) -> Result<i32> {
    let interface: WireGuardInterface = config.try_into()?;
    interface.set_tun(tun).await;
    let interface = Arc::new(interface);
    let runner = {
        let interface = interface.clone();
        tokio::spawn(async move { interface.run().await })
    };

    child.signal_ready().await?;
    let status = child.wait().await?;

    interface.remove_tun().await;
    match runner.await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => debug!(?err, "wireguard exec runtime exited with error"),
        Err(err) if err.is_cancelled() => {}
        Err(err) => debug!(?err, "wireguard exec runtime panicked"),
    }

    child_exit_code(status)
}

async fn tcp_dispatch_loop(mut listener: StackTcpListener, backend: SocketBackend) -> Result<()> {
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            Some(result) = tasks.join_next(), if !tasks.is_empty() => {
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => warn!(?err, "tcp bridge task failed"),
                    Err(err) if err.is_cancelled() => {}
                    Err(err) => warn!(?err, "tcp bridge task panicked"),
                }
            }
            next = listener.next() => match next {
                Some((stream, local_addr, remote_addr)) => {
                    debug!(%local_addr, %remote_addr, "accepted userspace tcp stream");
                    let backend = backend.clone();
                    tasks.spawn(async move {
                        bridge_tcp(backend, stream, local_addr, remote_addr).await
                    });
                }
                None => break,
            }
        }
    }

    tasks.abort_all();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => debug!(?err, "tcp bridge task exited during shutdown"),
            Err(err) if err.is_cancelled() => {}
            Err(err) => debug!(?err, "tcp bridge task panicked during shutdown"),
        }
    }
    Ok(())
}

async fn bridge_tcp(
    backend: SocketBackend,
    mut inbound: StackTcpStream,
    _local_addr: SocketAddr,
    remote_addr: SocketAddr,
) -> Result<()> {
    match backend {
        SocketBackend::Direct => {
            debug!(%remote_addr, "dialing direct outbound tcp");
            let mut outbound = TcpStream::connect(remote_addr)
                .await
                .with_context(|| format!("failed to connect to {remote_addr}"))?;
            copy_bidirectional(&mut inbound, &mut outbound)
                .await
                .with_context(|| format!("failed to bridge tcp stream for {remote_addr}"))?;
        }
        SocketBackend::Tor(tor_client) => {
            debug!(%remote_addr, "dialing tor outbound tcp");
            let tor_stream = tor_client
                .connect((remote_addr.ip().to_string(), remote_addr.port()))
                .await
                .with_context(|| format!("failed to connect to {remote_addr} over tor"))?;
            let mut tor_stream = tor_stream.compat();
            copy_bidirectional(&mut inbound, &mut tor_stream)
                .await
                .with_context(|| format!("failed to bridge tor stream for {remote_addr}"))?;
        }
    }
    Ok(())
}

async fn udp_dispatch_loop(socket: StackUdpSocket, backend: SocketBackend) -> Result<()> {
    let (mut udp_reader, mut udp_writer) = socket.split();
    let (reply_tx, mut reply_rx) = mpsc::channel::<UdpReply>(128);
    let direct_sessions = Arc::new(Mutex::new(
        HashMap::<UdpFlowKey, mpsc::Sender<Vec<u8>>>::new(),
    ));
    let mut session_tasks = JoinSet::new();

    loop {
        tokio::select! {
            Some(result) = session_tasks.join_next(), if !session_tasks.is_empty() => {
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => warn!(?err, "udp session task failed"),
                    Err(err) if err.is_cancelled() => {}
                    Err(err) => warn!(?err, "udp session task panicked"),
                }
            }
            maybe_reply = reply_rx.recv() => match maybe_reply {
                Some(reply) => {
                    udp_writer
                        .send((reply.payload, reply.source, reply.destination))
                        .await
                        .context("failed to write udp reply into userspace stack")?;
                }
                None => break,
            },
            maybe_datagram = udp_reader.next() => match maybe_datagram {
                Some((payload, local_addr, remote_addr)) => {
                    match &backend {
                        SocketBackend::Direct => {
                            dispatch_direct_udp(
                                payload,
                                local_addr,
                                remote_addr,
                                reply_tx.clone(),
                                direct_sessions.clone(),
                                &mut session_tasks,
                            ).await?;
                        }
                        SocketBackend::Tor(tor_client) => {
                            if remote_addr.port() != 53 {
                                debug!(%remote_addr, "dropping non-DNS UDP datagram for tor backend");
                                continue;
                            }
                            let response = build_tor_dns_response(&payload, tor_client.as_ref()).await?;
                            reply_tx
                                .send(UdpReply {
                                    payload: response,
                                    source: remote_addr,
                                    destination: local_addr,
                                })
                                .await
                                .context("failed to enqueue tor dns response")?;
                        }
                    }
                }
                None => break,
            }
        }
    }

    session_tasks.abort_all();
    while let Some(result) = session_tasks.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => debug!(?err, "udp session task exited during shutdown"),
            Err(err) if err.is_cancelled() => {}
            Err(err) => debug!(?err, "udp session task panicked during shutdown"),
        }
    }
    Ok(())
}

async fn dispatch_direct_udp(
    payload: Vec<u8>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    reply_tx: mpsc::Sender<UdpReply>,
    sessions: Arc<Mutex<HashMap<UdpFlowKey, mpsc::Sender<Vec<u8>>>>>,
    session_tasks: &mut JoinSet<Result<()>>,
) -> Result<()> {
    let key = UdpFlowKey {
        local: local_addr,
        remote: remote_addr,
    };
    let existing = { sessions.lock().await.get(&key).cloned() };
    if let Some(sender) = existing {
        if sender.send(payload.clone()).await.is_ok() {
            return Ok(());
        }
        sessions.lock().await.remove(&key);
    }

    let (tx, rx) = mpsc::channel::<Vec<u8>>(32);
    tx.send(payload)
        .await
        .context("failed to enqueue outbound udp payload")?;
    sessions.lock().await.insert(key.clone(), tx);

    session_tasks.spawn(async move { run_direct_udp_session(key, rx, reply_tx, sessions).await });
    Ok(())
}

async fn run_direct_udp_session(
    key: UdpFlowKey,
    mut outbound_rx: mpsc::Receiver<Vec<u8>>,
    reply_tx: mpsc::Sender<UdpReply>,
    sessions: Arc<Mutex<HashMap<UdpFlowKey, mpsc::Sender<Vec<u8>>>>>,
) -> Result<()> {
    let bind_addr = match key.remote {
        SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    };
    let socket = UdpSocket::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind udp socket for {}", key.remote))?;
    socket
        .connect(key.remote)
        .await
        .with_context(|| format!("failed to connect udp socket to {}", key.remote))?;

    let mut buf = vec![0u8; 65_535];
    loop {
        tokio::select! {
            maybe_payload = outbound_rx.recv() => match maybe_payload {
                Some(payload) => {
                    socket
                        .send(&payload)
                        .await
                        .with_context(|| format!("failed to send udp payload to {}", key.remote))?;
                }
                None => break,
            },
            recv = tokio::time::timeout(UDP_IDLE_TIMEOUT, socket.recv(&mut buf)) => match recv {
                Ok(Ok(len)) => {
                    reply_tx
                        .send(UdpReply {
                            payload: buf[..len].to_vec(),
                            source: key.remote,
                            destination: key.local,
                        })
                        .await
                        .context("failed to enqueue inbound udp reply")?;
                }
                Ok(Err(err)) => return Err(err).with_context(|| format!("failed to receive udp response from {}", key.remote)),
                Err(_) => break,
            }
        }
    }

    sessions.lock().await.remove(&key);
    Ok(())
}

fn wireguard_tun_config(config: &WireGuardConfig) -> Result<TunNetworkConfig> {
    parse_tun_config(
        &config.interface.address,
        config.interface.mtu,
        Some("burrow-wireguard"),
    )
}

fn socket_tun_config(
    addresses: &[String],
    mtu: Option<u32>,
    tun_name: Option<&str>,
    default_name: &str,
) -> Result<TunNetworkConfig> {
    let default_addresses;
    let addresses = if addresses.is_empty() {
        default_addresses = vec![DEFAULT_TUN_V4.to_string(), DEFAULT_TUN_V6.to_string()];
        default_addresses.as_slice()
    } else {
        addresses
    };
    parse_tun_config(addresses, mtu, Some(tun_name.unwrap_or(default_name)))
}

fn parse_tun_config(
    addresses: &[String],
    mtu: Option<u32>,
    tun_name: Option<&str>,
) -> Result<TunNetworkConfig> {
    let addresses = addresses
        .iter()
        .map(|addr| {
            addr.parse::<IpNetwork>()
                .with_context(|| format!("invalid tunnel address '{addr}'"))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(TunNetworkConfig {
        tun_name: tun_name.unwrap_or("burrow-exec").to_string(),
        addresses,
        mtu: mtu.unwrap_or(DEFAULT_MTU),
    })
}

fn open_tun_device(config: &TunNetworkConfig) -> Result<tun::TunInterface> {
    let tun = TunOptions::new()
        .name(&config.tun_name)
        .no_pi(true)
        .tun_excl(true)
        .open()
        .context("failed to create tun device")?;
    Ok(tun.inner.into_inner())
}

fn tokio_tun_from_fd(fd: RawFd) -> Result<TokioTunInterface> {
    let tun = unsafe { tun::TunInterface::from_raw_fd(fd) };
    TokioTunInterface::new(tun).context("failed to wrap tun fd in tokio interface")
}

fn read_inner_tun_config() -> Result<TunNetworkConfig> {
    let raw = env::var(INNER_TUN_CONFIG_ENV).context("missing namespace tun config")?;
    serde_json::from_str(&raw).context("invalid namespace tun config")
}

fn configure_tun_addresses(
    iface: &tun::TunInterface,
    networks: &[IpNetwork],
    mtu: u32,
) -> Result<()> {
    for network in networks {
        match network {
            IpNetwork::V4(net) => {
                iface.set_ipv4_addr(net.ip())?;
                let netmask = prefix_to_netmask_v4(net.prefix());
                iface.set_netmask(netmask)?;
                iface.set_broadcast_addr(broadcast_v4(net.ip(), netmask))?;
            }
            IpNetwork::V6(net) => iface.add_ipv6_addr(net.ip(), net.prefix())?,
        }
    }
    iface.set_mtu(mtu as i32)?;
    Ok(())
}

fn install_default_routes(name: &str, networks: &[IpNetwork]) -> Result<()> {
    if networks
        .iter()
        .any(|network| matches!(network, IpNetwork::V4(_)))
    {
        run_ip(["route", "replace", "default", "dev", name])?;
    }
    if networks
        .iter()
        .any(|network| matches!(network, IpNetwork::V6(_)))
    {
        run_ip(["-6", "route", "replace", "default", "dev", name])?;
    }
    Ok(())
}

fn run_ip<const N: usize>(args: [&str; N]) -> Result<()> {
    let status = StdCommand::new("ip")
        .args(args)
        .status()
        .context("failed to execute ip command")?;
    if !status.success() {
        bail!("ip {} failed with status {}", args.join(" "), status);
    }
    Ok(())
}

fn set_inheritable(fd: RawFd) -> Result<()> {
    let flags = FdFlag::from_bits_truncate(
        fcntl(fd, FcntlArg::F_GETFD).context("failed to query descriptor flags")?,
    );
    let flags = flags & !FdFlag::FD_CLOEXEC;
    fcntl(fd, FcntlArg::F_SETFD(flags)).context("failed to clear close-on-exec")?;
    Ok(())
}

async fn await_parent_ready(control_fd: RawFd) -> Result<()> {
    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut control = unsafe { StdUnixStream::from_raw_fd(control_fd) };
        let mut ack = [0u8; 1];
        std::io::Read::read_exact(&mut control, &mut ack)
            .context("failed to read namespace ready ack")?;
        if ack != *READY_ACK {
            bail!("unexpected namespace ready ack");
        }
        Ok(())
    })
    .await
    .context("failed to join namespace ready wait task")??;
    Ok(())
}

fn send_tun_fd(control_fd: RawFd, tun_fd: RawFd) -> Result<()> {
    let buf = [0u8; 1];
    let iov = [std::io::IoSlice::new(&buf)];
    let fds = [tun_fd];
    sendmsg::<()>(
        control_fd,
        &iov,
        &[ControlMessage::ScmRights(&fds)],
        MsgFlags::empty(),
        None,
    )
    .context("failed to send tun fd to parent")?;
    Ok(())
}

fn recv_tun_fd(control: &StdUnixStream) -> Result<RawFd> {
    let mut buf = [0u8; 1];
    let mut iov = [std::io::IoSliceMut::new(&mut buf)];
    let mut cmsgspace = cmsg_space!([RawFd; 1]);
    let msg = recvmsg::<()>(
        control.as_raw_fd(),
        &mut iov,
        Some(&mut cmsgspace),
        MsgFlags::empty(),
    )
    .context("failed to receive tun fd from namespace child")?;
    for cmsg in msg.cmsgs() {
        if let ControlMessageOwned::ScmRights(fds) = cmsg {
            if let Some(fd) = fds.first() {
                return Ok(*fd);
            }
        }
    }
    bail!("namespace child did not send a tun fd")
}

fn ensure_tool(tool: &str) -> Result<()> {
    let status = StdCommand::new("sh")
        .args(["-lc", &format!("command -v {tool} >/dev/null")])
        .status()
        .with_context(|| format!("failed to probe required tool '{tool}'"))?;
    if !status.success() {
        bail!("required host tool '{tool}' is not available");
    }
    Ok(())
}

async fn read_optional_payload(path: Option<&Path>) -> Result<Vec<u8>> {
    match path {
        Some(path) => tokio::fs::read(path)
            .await
            .with_context(|| format!("failed to read payload from {}", path.display())),
        None => Ok(Vec::new()),
    }
}

async fn read_required_payload(path: Option<&Path>, backend: &str) -> Result<Vec<u8>> {
    let path = path.ok_or_else(|| anyhow!("{backend} exec requires --payload"))?;
    tokio::fs::read(path)
        .await
        .with_context(|| format!("failed to read payload from {}", path.display()))
}

fn parse_wireguard_payload(payload: &[u8], path: Option<&Path>) -> Result<WireGuardConfig> {
    let payload = str::from_utf8(payload).context("wireguard payload must be valid UTF-8")?;
    if let Some(path) = path {
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            return WireGuardConfig::from_content_fmt(payload, ext);
        }
    }

    WireGuardConfig::from_toml(payload).or_else(|_| WireGuardConfig::from_ini(payload))
}

async fn spawn_child(command: &[String]) -> Result<ExitStatus> {
    let mut cmd = Command::new(&command[0]);
    if command.len() > 1 {
        cmd.args(&command[1..]);
    }
    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());
    cmd.kill_on_drop(true);
    cmd.status()
        .await
        .with_context(|| format!("failed to spawn '{}'", command[0]))
}

fn child_exit_code(status: ExitStatus) -> Result<i32> {
    if let Some(code) = status.code() {
        return Ok(code);
    }
    if let Some(signal) = status.signal() {
        return Ok(128 + signal);
    }
    bail!("child process terminated without an exit code");
}

fn prefix_to_netmask_v4(prefix: u8) -> Ipv4Addr {
    if prefix == 0 {
        Ipv4Addr::new(0, 0, 0, 0)
    } else {
        let mask = (!0u32) << (32 - prefix);
        Ipv4Addr::from(mask)
    }
}

fn broadcast_v4(ip: Ipv4Addr, netmask: Ipv4Addr) -> Ipv4Addr {
    let ip_u32 = u32::from(ip);
    let mask = u32::from(netmask);
    Ipv4Addr::from(ip_u32 | !mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_direct_json_payload() {
        let payload = br#"{"address":["10.0.0.2/24"],"mtu":1400,"tun_name":"burrow0"}"#;
        let config = DirectConfig::from_payload(payload).unwrap();
        assert_eq!(config.address, vec!["10.0.0.2/24"]);
        assert_eq!(config.mtu, Some(1400));
        assert_eq!(config.tun_name.as_deref(), Some("burrow0"));
    }

    #[test]
    fn socket_tun_config_uses_dual_stack_defaults() {
        let config = socket_tun_config(&[], None, None, "burrow-test").unwrap();
        assert_eq!(config.tun_name, "burrow-test");
        assert!(config
            .addresses
            .iter()
            .any(|network| matches!(network, IpNetwork::V4(_))));
        assert!(config
            .addresses
            .iter()
            .any(|network| matches!(network, IpNetwork::V6(_))));
    }
}
