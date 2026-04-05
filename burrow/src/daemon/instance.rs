use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use rusqlite::Connection;
use tokio::sync::{mpsc, watch, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status as RspStatus};
use tracing::{debug, info, warn};
use tun::tokio::TunInterface;

use super::{
    rpc::grpc_defs::{
        networks_server::Networks, tailnet_control_server::TailnetControl, tunnel_server::Tunnel,
        Empty, Network, NetworkDeleteRequest, NetworkListResponse, NetworkReorderRequest,
        State as RPCTunnelState, TailnetDiscoverRequest, TailnetDiscoverResponse,
        TailnetProbeRequest, TailnetProbeResponse, TunnelConfigurationResponse, TunnelPacket,
        TunnelStatusResponse,
    },
    runtime::{tailnet_helper_request, ActiveTunnel, ResolvedTunnel},
};
use crate::{
    auth::server::tailscale::{
        packet_socket_path, TailscaleBridgeManager,
        TailscaleLoginStartRequest as BridgeLoginStartRequest, TailscaleLoginStatus,
    },
    control::discovery,
    daemon::rpc::ServerConfig,
    database::{add_network, delete_network, get_connection, list_networks, reorder_network},
};

#[derive(Debug, Clone)]
enum RunState {
    Running,
    Idle,
}

impl RunState {
    fn to_rpc(&self) -> RPCTunnelState {
        match self {
            Self::Running => RPCTunnelState::Running,
            Self::Idle => RPCTunnelState::Stopped,
        }
    }
}

#[derive(Clone)]
pub struct DaemonRPCServer {
    tun_interface: Arc<RwLock<Option<TunInterface>>>,
    db_path: Option<PathBuf>,
    wg_state_chan: (watch::Sender<RunState>, watch::Receiver<RunState>),
    network_update_chan: (watch::Sender<()>, watch::Receiver<()>),
    active_tunnel: Arc<RwLock<Option<ActiveTunnel>>>,
    tailnet_login: TailscaleBridgeManager,
}

impl DaemonRPCServer {
    pub fn new(db_path: Option<&Path>) -> Result<Self> {
        Ok(Self {
            tun_interface: Arc::new(RwLock::new(None)),
            db_path: db_path.map(Path::to_owned),
            wg_state_chan: watch::channel(RunState::Idle),
            network_update_chan: watch::channel(()),
            active_tunnel: Arc::new(RwLock::new(None)),
            tailnet_login: TailscaleBridgeManager::default(),
        })
    }

    fn get_connection(&self) -> Result<Connection, RspStatus> {
        get_connection(self.db_path.as_deref()).map_err(proc_err)
    }

    async fn set_wg_state(&self, state: RunState) -> Result<(), RspStatus> {
        self.wg_state_chan.0.send(state).map_err(proc_err)
    }

    async fn notify_network_update(&self) -> Result<(), RspStatus> {
        self.network_update_chan.0.send(()).map_err(proc_err)
    }

    async fn resolve_tunnel(&self) -> Result<ResolvedTunnel, RspStatus> {
        let conn = self.get_connection()?;
        let networks = list_networks(&conn).map_err(proc_err)?;
        ResolvedTunnel::from_networks(&networks).map_err(proc_err)
    }

    async fn current_tunnel_configuration(&self) -> Result<TunnelConfigurationResponse, RspStatus> {
        let config = {
            let active = self.active_tunnel.read().await;
            active
                .as_ref()
                .map(|tunnel| tunnel.server_config().clone())
        };
        let config = match config {
            Some(config) => config,
            None => self
                .resolve_tunnel()
                .await?
                .server_config()
                .map_err(proc_err)?,
        };
        Ok(configuration_rsp(config))
    }

    async fn stop_active_tunnel(&self) -> Result<bool, RspStatus> {
        let current = { self.active_tunnel.write().await.take() };
        let Some(current) = current else {
            return Ok(false);
        };

        current
            .shutdown(&self.tun_interface)
            .await
            .map_err(proc_err)?;
        self.set_wg_state(RunState::Idle).await?;
        Ok(true)
    }

    async fn replace_active_tunnel(&self, desired: ResolvedTunnel) -> Result<(), RspStatus> {
        let _ = self.stop_active_tunnel().await?;
        let tailnet_helper = match &desired {
            ResolvedTunnel::Tailnet { identity, config } => Some(
                self.tailnet_login
                    .ensure_session(tailnet_helper_request(identity, config))
                    .await
                    .map_err(proc_err)?
                    .helper,
            ),
            _ => None,
        };
        let active = desired
            .start(self.tun_interface.clone(), tailnet_helper)
            .await
            .map_err(proc_err)?;
        self.active_tunnel.write().await.replace(active);
        self.set_wg_state(RunState::Running).await?;
        Ok(())
    }

    async fn reconcile_runtime(&self) -> Result<(), RspStatus> {
        let desired = self.resolve_tunnel().await?;
        let needs_restart = {
            let guard = self.active_tunnel.read().await;
            guard
                .as_ref()
                .map(|active| active.identity() != desired.identity())
                .unwrap_or(false)
        };

        if needs_restart {
            self.replace_active_tunnel(desired).await?;
        }

        Ok(())
    }

    fn tailnet_bridge_request(
        account_name: String,
        identity_name: String,
        hostname: String,
        authority: String,
    ) -> BridgeLoginStartRequest {
        let mut request = BridgeLoginStartRequest {
            account_name,
            identity_name,
            hostname: (!hostname.trim().is_empty()).then_some(hostname),
            control_url: Self::tailnet_control_url(&authority),
            packet_socket: None,
        };
        request.packet_socket = Some(packet_socket_path(&request).display().to_string());
        request
    }

    fn tailnet_control_url(authority: &str) -> Option<String> {
        let authority = discovery::normalize_authority(authority);
        (!discovery::is_managed_tailscale_authority(&authority)).then_some(authority)
    }
}

#[tonic::async_trait]
impl Tunnel for DaemonRPCServer {
    type TunnelConfigurationStream = ReceiverStream<Result<TunnelConfigurationResponse, RspStatus>>;
    type TunnelPacketsStream = ReceiverStream<Result<TunnelPacket, RspStatus>>;
    type TunnelStatusStream = ReceiverStream<Result<TunnelStatusResponse, RspStatus>>;

    async fn tunnel_configuration(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::TunnelConfigurationStream>, RspStatus> {
        let (tx, rx) = mpsc::channel(10);
        let server = self.clone();
        let mut sub = self.network_update_chan.1.clone();

        tokio::spawn(async move {
            loop {
                let response = server.current_tunnel_configuration().await;
                if tx.send(response).await.is_err() {
                    break;
                }
                if sub.changed().await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn tunnel_packets(
        &self,
        request: Request<tonic::Streaming<TunnelPacket>>,
    ) -> Result<Response<Self::TunnelPacketsStream>, RspStatus> {
        let (packet_tx, mut packet_rx) = {
            let guard = self.active_tunnel.read().await;
            let Some(active) = guard.as_ref() else {
                return Err(RspStatus::failed_precondition("no active tunnel"));
            };
            active.packet_stream().ok_or_else(|| {
                RspStatus::failed_precondition(
                    "active tunnel does not support packet streaming",
                )
            })?
        };

        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            loop {
                match packet_rx.recv().await {
                    Ok(payload) => {
                        if tx.send(Ok(TunnelPacket { payload })).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        let mut inbound = request.into_inner();
        tokio::spawn(async move {
            loop {
                match inbound.message().await {
                    Ok(Some(packet)) => {
                        debug!(
                            "daemon tunnel packet stream received {} bytes from client",
                            packet.payload.len()
                        );
                        if packet_tx.send(packet.payload).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        warn!("tailnet packet stream receive error: {error}");
                        break;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn tunnel_start(&self, _request: Request<Empty>) -> Result<Response<Empty>, RspStatus> {
        let desired = self.resolve_tunnel().await?;
        let already_running = {
            let guard = self.active_tunnel.read().await;
            guard
                .as_ref()
                .map(|active| active.identity() == desired.identity())
                .unwrap_or(false)
        };

        if already_running {
            warn!("Got start, but active tunnel already matches desired network.");
            return Ok(Response::new(Empty {}));
        }

        self.replace_active_tunnel(desired).await?;
        Ok(Response::new(Empty {}))
    }

    async fn tunnel_stop(&self, _request: Request<Empty>) -> Result<Response<Empty>, RspStatus> {
        let _ = self.stop_active_tunnel().await?;
        Ok(Response::new(Empty {}))
    }

    async fn tunnel_status(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::TunnelStatusStream>, RspStatus> {
        let (tx, rx) = mpsc::channel(10);
        let mut state_rx = self.wg_state_chan.1.clone();
        tokio::spawn(async move {
            let cur = state_rx.borrow_and_update().to_owned();
            if tx.send(Ok(status_rsp(cur))).await.is_err() {
                return;
            }

            loop {
                if state_rx.changed().await.is_err() {
                    break;
                }
                let cur = state_rx.borrow().to_owned();
                if tx.send(Ok(status_rsp(cur))).await.is_err() {
                    break;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tonic::async_trait]
impl Networks for DaemonRPCServer {
    type NetworkListStream = ReceiverStream<Result<NetworkListResponse, RspStatus>>;

    async fn network_add(&self, request: Request<Network>) -> Result<Response<Empty>, RspStatus> {
        let conn = self.get_connection()?;
        let network = request.into_inner();
        add_network(&conn, &network).map_err(proc_err)?;
        self.notify_network_update().await?;
        self.reconcile_runtime().await?;
        Ok(Response::new(Empty {}))
    }

    async fn network_list(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::NetworkListStream>, RspStatus> {
        let (tx, rx) = mpsc::channel(10);
        let conn = self.get_connection()?;
        let mut sub = self.network_update_chan.1.clone();
        tokio::spawn(async move {
            loop {
                let networks = list_networks(&conn)
                    .map(|res| NetworkListResponse { network: res })
                    .map_err(proc_err);
                if tx.send(networks).await.is_err() {
                    break;
                }
                if sub.changed().await.is_err() {
                    break;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn network_reorder(
        &self,
        request: Request<NetworkReorderRequest>,
    ) -> Result<Response<Empty>, RspStatus> {
        let conn = self.get_connection()?;
        reorder_network(&conn, request.into_inner()).map_err(proc_err)?;
        self.notify_network_update().await?;
        self.reconcile_runtime().await?;
        Ok(Response::new(Empty {}))
    }

    async fn network_delete(
        &self,
        request: Request<NetworkDeleteRequest>,
    ) -> Result<Response<Empty>, RspStatus> {
        let conn = self.get_connection()?;
        delete_network(&conn, request.into_inner()).map_err(proc_err)?;
        self.notify_network_update().await?;
        self.reconcile_runtime().await?;
        Ok(Response::new(Empty {}))
    }
}

#[tonic::async_trait]
impl TailnetControl for DaemonRPCServer {
    async fn discover(
        &self,
        request: Request<TailnetDiscoverRequest>,
    ) -> Result<Response<TailnetDiscoverResponse>, RspStatus> {
        let request = request.into_inner();
        info!(email = %request.email, "daemon tailnet discover RPC received");
        let discovery = discovery::discover_tailnet(&request.email)
            .await
            .map_err(proc_err)?;
        info!(
            email = %request.email,
            authority = %discovery.authority,
            provider = ?discovery.provider,
            "daemon tailnet discover RPC resolved"
        );

        Ok(Response::new(TailnetDiscoverResponse {
            domain: discovery.domain,
            authority: discovery.authority.clone(),
            oidc_issuer: discovery.oidc_issuer.unwrap_or_default(),
            managed: matches!(
                discovery::inferred_provider(Some(&discovery.authority), Some(&discovery.provider)),
                crate::control::TailnetProvider::Tailscale
            ),
        }))
    }

    async fn probe(
        &self,
        request: Request<TailnetProbeRequest>,
    ) -> Result<Response<TailnetProbeResponse>, RspStatus> {
        let request = request.into_inner();
        let status = discovery::probe_tailnet_authority(&request.authority)
            .await
            .map_err(proc_err)?;

        Ok(Response::new(TailnetProbeResponse {
            authority: status.authority,
            status_code: status.status_code,
            summary: status.summary,
            detail: status.detail,
            reachable: status.reachable,
        }))
    }

    async fn login_start(
        &self,
        request: Request<super::rpc::grpc_defs::TailnetLoginStartRequest>,
    ) -> Result<Response<super::rpc::grpc_defs::TailnetLoginStatusResponse>, RspStatus> {
        let request = request.into_inner();
        info!(
            account = %request.account_name,
            identity = %request.identity_name,
            authority = %request.authority,
            "daemon tailnet login start RPC received"
        );
        let response = self
            .tailnet_login
            .start_login(Self::tailnet_bridge_request(
                request.account_name,
                request.identity_name,
                request.hostname,
                request.authority,
            ))
            .await
            .map_err(proc_err)?;

        info!(
            session_id = %response.session_id,
            backend_state = %response.status.backend_state,
            running = response.status.running,
            needs_login = response.status.needs_login,
            auth_url = ?response.status.auth_url,
            "daemon tailnet login start RPC resolved"
        );

        Ok(Response::new(tailnet_login_rsp(
            response.session_id,
            response.status,
        )))
    }

    async fn login_status(
        &self,
        request: Request<super::rpc::grpc_defs::TailnetLoginStatusRequest>,
    ) -> Result<Response<super::rpc::grpc_defs::TailnetLoginStatusResponse>, RspStatus> {
        let request = request.into_inner();
        info!(session_id = %request.session_id, "daemon tailnet login status RPC received");
        let status = self
            .tailnet_login
            .status(&request.session_id)
            .await
            .map_err(proc_err)?;
        let Some(status) = status else {
            return Err(RspStatus::not_found("tailnet login session not found"));
        };
        info!(
            session_id = %request.session_id,
            backend_state = %status.backend_state,
            running = status.running,
            needs_login = status.needs_login,
            auth_url = ?status.auth_url,
            "daemon tailnet login status RPC resolved"
        );
        Ok(Response::new(tailnet_login_rsp(request.session_id, status)))
    }

    async fn login_cancel(
        &self,
        request: Request<super::rpc::grpc_defs::TailnetLoginCancelRequest>,
    ) -> Result<Response<Empty>, RspStatus> {
        let request = request.into_inner();
        let canceled = self
            .tailnet_login
            .cancel(&request.session_id)
            .await
            .map_err(proc_err)?;
        if !canceled {
            return Err(RspStatus::not_found("tailnet login session not found"));
        }
        Ok(Response::new(Empty {}))
    }
}

fn proc_err(err: impl ToString) -> RspStatus {
    RspStatus::internal(err.to_string())
}

fn configuration_rsp(config: ServerConfig) -> TunnelConfigurationResponse {
    TunnelConfigurationResponse {
        addresses: config.address,
        mtu: config.mtu.unwrap_or(1000),
        routes: config.routes,
        dns_servers: config.dns_servers,
        search_domains: config.search_domains,
        include_default_route: config.include_default_route,
    }
}

fn status_rsp(state: RunState) -> TunnelStatusResponse {
    TunnelStatusResponse {
        state: state.to_rpc().into(),
        start: None, // TODO: Add timestamp
    }
}

fn tailnet_login_rsp(
    session_id: String,
    status: TailscaleLoginStatus,
) -> super::rpc::grpc_defs::TailnetLoginStatusResponse {
    super::rpc::grpc_defs::TailnetLoginStatusResponse {
        session_id,
        backend_state: status.backend_state,
        auth_url: status.auth_url.unwrap_or_default(),
        running: status.running,
        needs_login: status.needs_login,
        tailnet_name: status.tailnet_name.unwrap_or_default(),
        magic_dns_suffix: status.magic_dns_suffix.unwrap_or_default(),
        self_dns_name: status.self_dns_name.unwrap_or_default(),
        tailnet_ips: status.tailscale_ips,
        health: status.health,
    }
}
