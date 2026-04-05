use std::{path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::UnixStream,
    sync::{broadcast, mpsc, RwLock},
    task::JoinHandle,
    time::{sleep, Duration},
};
use tun::{tokio::TunInterface, TunOptions};

use super::rpc::{
    grpc_defs::{Network, NetworkType},
    ServerConfig,
};
use crate::{
    auth::server::tailscale::{
        default_hostname, packet_socket_path, spawn_tailscale_helper, TailscaleHelperProcess,
        TailscaleLoginStartRequest, TailscaleLoginStatus,
    },
    control::{discovery, TailnetConfig},
    wireguard::{Config, Interface as WireGuardInterface},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeIdentity {
    Passthrough,
    Network {
        id: i32,
        network_type: NetworkType,
        payload: Vec<u8>,
    },
}

#[derive(Clone, Debug)]
pub enum ResolvedTunnel {
    Passthrough {
        identity: RuntimeIdentity,
    },
    Tailnet {
        identity: RuntimeIdentity,
        config: TailnetConfig,
    },
    WireGuard {
        identity: RuntimeIdentity,
        config: Config,
    },
}

impl ResolvedTunnel {
    pub fn from_networks(networks: &[Network]) -> Result<Self> {
        let Some(network) = networks.first() else {
            return Ok(Self::Passthrough {
                identity: RuntimeIdentity::Passthrough,
            });
        };

        let identity = RuntimeIdentity::Network {
            id: network.id,
            network_type: network.r#type(),
            payload: network.payload.clone(),
        };

        match network.r#type() {
            NetworkType::Tailnet => {
                let config = TailnetConfig::from_slice(&network.payload)?;
                Ok(Self::Tailnet { identity, config })
            }
            NetworkType::WireGuard => {
                let payload = String::from_utf8(network.payload.clone())
                    .context("wireguard payload must be valid UTF-8")?;
                let config = Config::from_content_fmt(&payload, "ini")?;
                Ok(Self::WireGuard { identity, config })
            }
        }
    }

    pub fn identity(&self) -> &RuntimeIdentity {
        match self {
            Self::Passthrough { identity }
            | Self::Tailnet { identity, .. }
            | Self::WireGuard { identity, .. } => identity,
        }
    }

    pub fn server_config(&self) -> Result<ServerConfig> {
        match self {
            Self::Passthrough { .. } => Ok(ServerConfig {
                address: Vec::new(),
                routes: Vec::new(),
                dns_servers: Vec::new(),
                search_domains: Vec::new(),
                include_default_route: false,
                name: None,
                mtu: Some(1500),
            }),
            Self::Tailnet { .. } => Ok(ServerConfig {
                address: Vec::new(),
                routes: tailnet_routes(),
                dns_servers: tailnet_dns_servers(),
                search_domains: Vec::new(),
                include_default_route: false,
                name: None,
                mtu: Some(1280),
            }),
            Self::WireGuard { config, .. } => ServerConfig::try_from(config),
        }
    }

    pub async fn start(
        self,
        tun_interface: Arc<RwLock<Option<TunInterface>>>,
        tailnet_helper: Option<Arc<TailscaleHelperProcess>>,
    ) -> Result<ActiveTunnel> {
        match self {
            Self::Passthrough { identity } => Ok(ActiveTunnel::Passthrough {
                identity,
                server_config: ServerConfig {
                    address: Vec::new(),
                    routes: Vec::new(),
                    dns_servers: Vec::new(),
                    search_domains: Vec::new(),
                    include_default_route: false,
                    name: None,
                    mtu: Some(1500),
                },
            }),
            Self::Tailnet { identity, config } => {
                let (helper, shutdown_helper_on_stop) = match tailnet_helper {
                    Some(helper) => (helper, false),
                    None => {
                        let helper_request = tailnet_helper_request(&identity, &config);
                        let helper = Arc::new(spawn_tailscale_helper(&helper_request).await?);
                        (helper, true)
                    }
                };
                let status = wait_for_tailnet_ready(helper.as_ref()).await?;
                let server_config = tailnet_server_config(&status);
                let packet_socket = helper
                    .packet_socket()
                    .map(PathBuf::from)
                    .ok_or_else(|| anyhow::anyhow!("tailnet helper did not report a packet socket"))?;
                let packet_bridge = connect_tailnet_packet_bridge(packet_socket).await?;
                #[cfg(target_vendor = "apple")]
                let tun_task = None;
                #[cfg(not(target_vendor = "apple"))]
                let tun_task = {
                    let tun = TunOptions::new().open()?;
                    tun_interface.write().await.replace(tun);
                    Some(tokio::spawn(run_tailnet_tun_bridge(
                        tun_interface.clone(),
                        packet_bridge.outbound_sender(),
                        packet_bridge.subscribe(),
                    )))
                };

                Ok(ActiveTunnel::Tailnet {
                    identity,
                    server_config,
                    helper,
                    shutdown_helper_on_stop,
                    packet_bridge,
                    tun_task,
                })
            }
            Self::WireGuard { identity, config } => {
                let server_config = ServerConfig::try_from(&config)?;
                let tun = TunOptions::new().open()?;
                tun_interface.write().await.replace(tun);

                match start_wireguard_runtime(config, tun_interface.clone()).await {
                    Ok((interface, task)) => Ok(ActiveTunnel::WireGuard {
                        identity,
                        server_config,
                        interface,
                        task,
                    }),
                    Err(err) => {
                        tun_interface.write().await.take();
                        Err(err)
                    }
                }
            }
        }
    }
}

pub enum ActiveTunnel {
    Passthrough {
        identity: RuntimeIdentity,
        server_config: ServerConfig,
    },
    Tailnet {
        identity: RuntimeIdentity,
        server_config: ServerConfig,
        helper: Arc<TailscaleHelperProcess>,
        shutdown_helper_on_stop: bool,
        packet_bridge: TailnetPacketBridge,
        tun_task: Option<JoinHandle<Result<()>>>,
    },
    WireGuard {
        identity: RuntimeIdentity,
        server_config: ServerConfig,
        interface: Arc<RwLock<WireGuardInterface>>,
        task: JoinHandle<Result<()>>,
    },
}

impl ActiveTunnel {
    pub fn identity(&self) -> &RuntimeIdentity {
        match self {
            Self::Passthrough { identity, .. }
            | Self::Tailnet { identity, .. }
            | Self::WireGuard { identity, .. } => identity,
        }
    }

    pub fn server_config(&self) -> &ServerConfig {
        match self {
            Self::Passthrough { server_config, .. }
            | Self::Tailnet { server_config, .. }
            | Self::WireGuard { server_config, .. } => server_config,
        }
    }

    pub fn packet_stream(
        &self,
    ) -> Option<(mpsc::Sender<Vec<u8>>, broadcast::Receiver<Vec<u8>>)> {
        match self {
            Self::Tailnet { packet_bridge, .. } => Some((
                packet_bridge.outbound_sender(),
                packet_bridge.subscribe(),
            )),
            _ => None,
        }
    }

    pub async fn shutdown(self, tun_interface: &Arc<RwLock<Option<TunInterface>>>) -> Result<()> {
        match self {
            Self::Passthrough { .. } => Ok(()),
            Self::Tailnet {
                helper,
                shutdown_helper_on_stop,
                packet_bridge,
                tun_task,
                ..
            } => {
                if let Some(tun_task) = tun_task {
                    tun_task.abort();
                    match tun_task.await {
                        Ok(Ok(())) => {}
                        Ok(Err(err)) => return Err(err),
                        Err(err) if err.is_cancelled() => {}
                        Err(err) => return Err(err.into()),
                    }
                }
                packet_bridge.task.abort();
                match packet_bridge.task.await {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => return Err(err),
                    Err(err) if err.is_cancelled() => {}
                    Err(err) => return Err(err.into()),
                }
                tun_interface.write().await.take();
                if shutdown_helper_on_stop {
                    helper.shutdown().await?;
                }
                Ok(())
            }
            Self::WireGuard {
                interface,
                task,
                ..
            } => {
                interface.read().await.remove_tun().await;
                let task_result = task.await;
                tun_interface.write().await.take();
                task_result??;
                Ok(())
            }
        }
    }
}

pub struct TailnetPacketBridge {
    outbound: mpsc::Sender<Vec<u8>>,
    inbound: broadcast::Sender<Vec<u8>>,
    task: JoinHandle<Result<()>>,
}

impl TailnetPacketBridge {
    fn outbound_sender(&self) -> mpsc::Sender<Vec<u8>> {
        self.outbound.clone()
    }

    fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.inbound.subscribe()
    }
}

async fn start_wireguard_runtime(
    config: Config,
    tun_interface: Arc<RwLock<Option<TunInterface>>>,
) -> Result<(Arc<RwLock<WireGuardInterface>>, JoinHandle<Result<()>>)> {
    let mut interface: WireGuardInterface = config.try_into()?;
    interface.set_tun_ref(tun_interface).await;
    let interface = Arc::new(RwLock::new(interface));
    let run_interface = interface.clone();
    let task = tokio::spawn(async move {
        let guard = run_interface.read().await;
        guard.run().await
    });
    Ok((interface, task))
}

pub(crate) fn tailnet_helper_request(
    identity: &RuntimeIdentity,
    config: &TailnetConfig,
) -> TailscaleLoginStartRequest {
    let account_name = config
        .account
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("default")
        .to_owned();
    let identity_name = config
        .identity
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| match identity {
            RuntimeIdentity::Network { id, .. } => format!("network-{id}"),
            RuntimeIdentity::Passthrough => "apple".to_owned(),
        });
    let control_url = config.authority.as_deref().and_then(|authority| {
        let authority = discovery::normalize_authority(authority);
        (!discovery::is_managed_tailscale_authority(&authority)).then_some(authority)
    });

    let mut request = TailscaleLoginStartRequest {
        account_name,
        identity_name,
        hostname: config.hostname.clone(),
        control_url,
        packet_socket: None,
    };
    request.packet_socket = Some(packet_socket_path(&request).display().to_string());
    if request
        .hostname
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        request.hostname = Some(default_hostname(&request));
    }
    request
}

async fn wait_for_tailnet_ready(helper: &TailscaleHelperProcess) -> Result<TailscaleLoginStatus> {
    let mut last_status = None;
    for _ in 0..120 {
        let status = helper.status().await?;
        if status.running && !status.tailscale_ips.is_empty() {
            return Ok(status);
        }
        if status.needs_login || status.auth_url.is_some() {
            bail!("tailnet runtime requires a completed login before the tunnel can start");
        }
        last_status = Some(status);
        sleep(Duration::from_millis(250)).await;
    }

    if let Some(status) = last_status {
        bail!(
            "tailnet helper never became ready (backend_state={})",
            status.backend_state
        );
    }
    bail!("tailnet helper never produced a status update")
}

fn tailnet_server_config(status: &TailscaleLoginStatus) -> ServerConfig {
    let mut search_domains = Vec::new();
    if let Some(suffix) = status.magic_dns_suffix.as_deref() {
        let suffix = suffix.trim().trim_end_matches('.');
        if !suffix.is_empty() {
            search_domains.push(suffix.to_owned());
        }
    }

    ServerConfig {
        address: status
            .tailscale_ips
            .iter()
            .map(|ip| tailnet_cidr(ip))
            .collect(),
        routes: tailnet_routes(),
        dns_servers: tailnet_dns_servers(),
        search_domains,
        include_default_route: false,
        name: status.self_dns_name.clone(),
        mtu: Some(1280),
    }
}

fn tailnet_routes() -> Vec<String> {
    vec!["100.64.0.0/10".to_owned(), "fd7a:115c:a1e0::/48".to_owned()]
}

fn tailnet_dns_servers() -> Vec<String> {
    vec!["100.100.100.100".to_owned()]
}

fn tailnet_cidr(ip: &str) -> String {
    if ip.contains('/') {
        return ip.to_owned();
    }
    if ip.contains(':') {
        format!("{ip}/128")
    } else {
        format!("{ip}/32")
    }
}

async fn connect_tailnet_packet_bridge(packet_socket: PathBuf) -> Result<TailnetPacketBridge> {
    let mut last_error = None;
    let mut stream = None;
    for _ in 0..50 {
        match UnixStream::connect(&packet_socket).await {
            Ok(connected) => {
                stream = Some(connected);
                break;
            }
            Err(err) => {
                last_error = Some(err);
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
    let stream = if let Some(stream) = stream {
        stream
    } else {
        return Err(last_error
            .context("failed to connect to tailnet helper packet socket")?
            .into());
    };

    let (outbound_tx, outbound_rx) = mpsc::channel(128);
    let (inbound_tx, _) = broadcast::channel(128);
    let task = tokio::spawn(run_tailnet_socket_bridge(
        stream,
        outbound_rx,
        inbound_tx.clone(),
    ));

    Ok(TailnetPacketBridge {
        outbound: outbound_tx,
        inbound: inbound_tx,
        task,
    })
}

async fn run_tailnet_socket_bridge(
    stream: UnixStream,
    mut outbound_rx: mpsc::Receiver<Vec<u8>>,
    inbound_tx: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    let (mut reader, mut writer) = stream.into_split();

    let inbound = tokio::spawn(async move {
        loop {
            let packet = read_packet_frame(&mut reader).await?;
            tracing::debug!(
                "tailnet packet bridge received {} bytes from helper socket",
                packet.len()
            );
            let _ = inbound_tx.send(packet);
        }
        #[allow(unreachable_code)]
        Result::<()>::Ok(())
    });

    let outbound = tokio::spawn(async move {
        while let Some(packet) = outbound_rx.recv().await {
            tracing::debug!(
                "tailnet packet bridge writing {} bytes to helper socket",
                packet.len()
            );
            write_packet_frame(&mut writer, &packet).await?;
        }
        Result::<()>::Ok(())
    });

    let (inbound_result, outbound_result) = tokio::try_join!(inbound, outbound)?;
    inbound_result?;
    outbound_result?;
    Ok(())
}

#[cfg(not(target_vendor = "apple"))]
async fn run_tailnet_tun_bridge(
    tun_interface: Arc<RwLock<Option<TunInterface>>>,
    outbound_tx: mpsc::Sender<Vec<u8>>,
    mut inbound_rx: broadcast::Receiver<Vec<u8>>,
) -> Result<()> {
    let inbound_tun = tun_interface.clone();
    let inbound = tokio::spawn(async move {
        loop {
            let packet = match inbound_rx.recv().await {
                Ok(packet) => packet,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            };
            let guard = inbound_tun.read().await;
            let Some(tun) = guard.as_ref() else {
                bail!("tailnet tun interface unavailable");
            };
            tun.send(&packet)
                .await
                .context("failed to write tailnet packet to tun")?;
        }
        Result::<()>::Ok(())
    });

    let outbound_tun = tun_interface.clone();
    let outbound = tokio::spawn(async move {
        let mut buf = vec![0u8; 65_535];
        loop {
            let len = {
                let guard = outbound_tun.read().await;
                let Some(tun) = guard.as_ref() else {
                    bail!("tailnet tun interface unavailable");
                };
                tun.recv(&mut buf)
                    .await
                    .context("failed to read packet from tailnet tun")?
            };
            outbound_tx
                .send(buf[..len].to_vec())
                .await
                .context("failed to forward packet to tailnet helper")?;
        }
        #[allow(unreachable_code)]
        Result::<()>::Ok(())
    });

    let (inbound_result, outbound_result) = tokio::try_join!(inbound, outbound)?;
    inbound_result?;
    outbound_result?;
    Ok(())
}

async fn read_packet_frame<R>(reader: &mut R) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .context("failed to read tailnet packet frame length")?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut packet = vec![0u8; len];
    reader
        .read_exact(&mut packet)
        .await
        .context("failed to read tailnet packet frame payload")?;
    Ok(packet)
}

async fn write_packet_frame<W>(writer: &mut W, packet: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    writer
        .write_all(&(packet.len() as u32).to_be_bytes())
        .await
        .context("failed to write tailnet packet frame length")?;
    writer
        .write_all(packet)
        .await
        .context("failed to write tailnet packet frame payload")?;
    writer
        .flush()
        .await
        .context("failed to flush tailnet packet frame")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_networks_resolve_to_passthrough() {
        let resolved = ResolvedTunnel::from_networks(&[]).unwrap();
        assert_eq!(resolved.identity(), &RuntimeIdentity::Passthrough);
        assert_eq!(
            resolved.server_config().unwrap().address,
            Vec::<String>::new()
        );
    }

    #[test]
    fn tailnet_server_config_uses_host_prefixes() {
        let status = TailscaleLoginStatus {
            running: true,
            tailscale_ips: vec!["100.101.102.103".to_owned(), "fd7a:115c:a1e0::123".to_owned()],
            ..Default::default()
        };
        let config = tailnet_server_config(&status);
        assert_eq!(
            config.address,
            vec!["100.101.102.103/32", "fd7a:115c:a1e0::123/128"]
        );
        assert_eq!(config.mtu, Some(1280));
    }
}
