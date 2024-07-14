use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use rusqlite::Connection;
use tokio::sync::{mpsc, watch, Notify, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status as RspStatus};
use tracing::{debug, info, warn};
use tun::{tokio::TunInterface, TunOptions};

use super::rpc::grpc_defs::{
    networks_server::Networks,
    tunnel_server::Tunnel,
    Empty,
    Network,
    NetworkDeleteRequest,
    NetworkListResponse,
    NetworkReorderRequest,
    State as RPCTunnelState,
    TunnelConfigurationResponse,
    TunnelStatusResponse,
};
use crate::{
    daemon::rpc::{
        DaemonCommand,
        DaemonNotification,
        DaemonResponse,
        DaemonResponseData,
        ServerConfig,
        ServerInfo,
    },
    database::{
        add_network,
        delete_network,
        get_connection,
        list_networks,
        load_interface,
        reorder_network,
    },
    wireguard::{Config, Interface},
};

#[derive(Debug, Clone)]
enum RunState {
    Running,
    Idle,
}

impl RunState {
    pub fn to_rpc(&self) -> RPCTunnelState {
        match self {
            RunState::Running => RPCTunnelState::Running,
            RunState::Idle => RPCTunnelState::Stopped,
        }
    }
}

#[derive(Clone)]
pub struct DaemonRPCServer {
    tun_interface: Arc<RwLock<Option<TunInterface>>>,
    wg_interface: Arc<RwLock<Interface>>,
    config: Arc<RwLock<Config>>,
    db_path: Option<PathBuf>,
    wg_state_chan: (watch::Sender<RunState>, watch::Receiver<RunState>),
    network_update_chan: (watch::Sender<()>, watch::Receiver<()>),
}

impl DaemonRPCServer {
    pub fn new(
        wg_interface: Arc<RwLock<Interface>>,
        config: Arc<RwLock<Config>>,
        db_path: Option<&Path>,
    ) -> Result<Self> {
        Ok(Self {
            tun_interface: Arc::new(RwLock::new(None)),
            wg_interface,
            config,
            db_path: db_path.map(|p| p.to_owned()),
            wg_state_chan: watch::channel(RunState::Idle),
            network_update_chan: watch::channel(()),
        })
    }

    pub fn get_connection(&self) -> Result<Connection, RspStatus> {
        get_connection(self.db_path.as_deref()).map_err(proc_err)
    }

    async fn set_wg_state(&self, state: RunState) -> Result<(), RspStatus> {
        self.wg_state_chan.0.send(state).map_err(proc_err)
    }

    async fn get_wg_state(&self) -> RunState {
        self.wg_state_chan.1.borrow().to_owned()
    }

    async fn notify_network_update(&self) -> Result<(), RspStatus> {
        self.network_update_chan.0.send(()).map_err(proc_err)
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
        tokio::spawn(async move {
            let serv_config = ServerConfig::default();
            tx.send(Ok(TunnelConfigurationResponse {
                mtu: serv_config.mtu.unwrap_or(1000),
                addresses: serv_config.address,
            }))
            .await
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn tunnel_start(&self, _request: Request<Empty>) -> Result<Response<Empty>, RspStatus> {
        let wg_state = self.get_wg_state().await;
        match wg_state {
            RunState::Idle => {
                let tun_if = TunOptions::new().open()?;
                debug!("Setting tun on wg_interface");
                self.tun_interface.write().await.replace(tun_if);
                self.wg_interface
                    .write()
                    .await
                    .set_tun_ref(self.tun_interface.clone())
                    .await;
                debug!("tun set on wg_interface");

                debug!("Setting tun_interface");
                debug!("tun_interface set: {:?}", self.tun_interface);

                debug!("Cloning wg_interface");
                let tmp_wg = self.wg_interface.clone();
                let run_task = tokio::spawn(async move {
                    let twlock = tmp_wg.read().await;
                    twlock.run().await
                });
                self.set_wg_state(RunState::Running).await?;
            }

            RunState::Running => {
                warn!("Got start, but tun interface already up.");
            }
        }

        return Ok(Response::new(Empty {}));
    }

    async fn tunnel_stop(&self, _request: Request<Empty>) -> Result<Response<Empty>, RspStatus> {
        self.wg_interface.write().await.remove_tun().await;
        self.set_wg_state(RunState::Idle).await?;
        return Ok(Response::new(Empty {}));
    }

    async fn tunnel_status(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::TunnelStatusStream>, RspStatus> {
        let (tx, rx) = mpsc::channel(10);
        let mut state_rx = self.wg_state_chan.1.clone();
        tokio::spawn(async move {
            let cur = state_rx.borrow_and_update().to_owned();
            tx.send(Ok(status_rsp(cur))).await;
            loop {
                state_rx.changed().await.unwrap();
                let cur = state_rx.borrow().to_owned();
                let res = tx.send(Ok(status_rsp(cur))).await;
                if res.is_err() {
                    eprintln!("Tunnel status channel closed");
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
        Ok(Response::new(Empty {}))
    }

    async fn network_list(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::NetworkListStream>, RspStatus> {
        debug!("Mock network_list called");
        let (tx, rx) = mpsc::channel(10);
        let conn = self.get_connection()?;
        let mut sub = self.network_update_chan.1.clone();
        tokio::spawn(async move {
            loop {
                let networks = list_networks(&conn)
                    .map(|res| NetworkListResponse { network: res })
                    .map_err(proc_err);
                let res = tx.send(networks).await;
                if res.is_err() {
                    eprintln!("Network list channel closed");
                    break;
                }
                sub.changed().await.unwrap();
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
        Ok(Response::new(Empty {}))
    }

    async fn network_delete(
        &self,
        request: Request<NetworkDeleteRequest>,
    ) -> Result<Response<Empty>, RspStatus> {
        let conn = self.get_connection()?;
        delete_network(&conn, request.into_inner()).map_err(proc_err)?;
        self.notify_network_update().await?;
        Ok(Response::new(Empty {}))
    }
}

fn proc_err(err: impl ToString) -> RspStatus {
    RspStatus::internal(err.to_string())
}

fn status_rsp(state: RunState) -> TunnelStatusResponse {
    TunnelStatusResponse {
        state: state.to_rpc().into(),
        start: None, // TODO: Add timestamp
    }
}
