use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use rusqlite::Connection;
use tokio::sync::{mpsc, watch, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status as RspStatus};
use tracing::warn;
use tun::tokio::TunInterface;

use super::{
    rpc::grpc_defs::{
        networks_server::Networks, tunnel_server::Tunnel, Empty, Network, NetworkDeleteRequest,
        NetworkListResponse, NetworkReorderRequest, State as RPCTunnelState,
        TunnelConfigurationResponse, TunnelStatusResponse,
    },
    runtime::{ActiveTunnel, ResolvedTunnel},
};
use crate::{
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
}

impl DaemonRPCServer {
    pub fn new(db_path: Option<&Path>) -> Result<Self> {
        Ok(Self {
            tun_interface: Arc::new(RwLock::new(None)),
            db_path: db_path.map(Path::to_owned),
            wg_state_chan: watch::channel(RunState::Idle),
            network_update_chan: watch::channel(()),
            active_tunnel: Arc::new(RwLock::new(None)),
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

    async fn resolve_tunnel(&self) -> Result<Option<ResolvedTunnel>, RspStatus> {
        let conn = self.get_connection()?;
        let networks = list_networks(&conn).map_err(proc_err)?;
        ResolvedTunnel::from_networks(&networks).map_err(proc_err)
    }

    async fn current_tunnel_configuration(&self) -> Result<TunnelConfigurationResponse, RspStatus> {
        match self.resolve_tunnel().await? {
            Some(config) => {
                let config = config.server_config().map_err(proc_err)?;
                Ok(configuration_rsp(config))
            }
            None => Ok(empty_configuration_rsp()),
        }
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
        let active = desired
            .start(self.tun_interface.clone())
            .await
            .map_err(proc_err)?;
        self.active_tunnel.write().await.replace(active);
        self.set_wg_state(RunState::Running).await?;
        Ok(())
    }

    async fn reconcile_runtime(&self) -> Result<(), RspStatus> {
        let desired = self.resolve_tunnel().await?;
        let Some(desired) = desired else {
            let _ = self.stop_active_tunnel().await?;
            return Ok(());
        };
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
}

#[tonic::async_trait]
impl Tunnel for DaemonRPCServer {
    type TunnelConfigurationStream = ReceiverStream<Result<TunnelConfigurationResponse, RspStatus>>;
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

    async fn tunnel_start(&self, _request: Request<Empty>) -> Result<Response<Empty>, RspStatus> {
        let desired = self
            .resolve_tunnel()
            .await?
            .ok_or_else(|| RspStatus::failed_precondition("no stored network configured"))?;
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

fn proc_err(err: impl ToString) -> RspStatus {
    RspStatus::internal(err.to_string())
}

fn configuration_rsp(config: ServerConfig) -> TunnelConfigurationResponse {
    TunnelConfigurationResponse {
        mtu: config.mtu.unwrap_or(1000),
        addresses: config.address,
    }
}

fn empty_configuration_rsp() -> TunnelConfigurationResponse {
    TunnelConfigurationResponse {
        mtu: 1500,
        addresses: Vec::new(),
    }
}

fn status_rsp(state: RunState) -> TunnelStatusResponse {
    TunnelStatusResponse {
        state: state.to_rpc().into(),
        start: None, // TODO: Add timestamp
    }
}
