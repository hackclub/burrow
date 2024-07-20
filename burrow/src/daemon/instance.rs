use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status as RspStatus};
use tracing::{debug, info, warn};
use tun::{tokio::TunInterface, TunOptions};

use super::rpc::grpc_defs::{
    networks_server::Networks,
    tunnel_server::Tunnel,
    Empty,
    NetworkDeleteRequest,
    NetworkListResponse,
    NetworkReorderRequest,
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
    database::{get_connection, load_interface},
    wireguard::{Config, Interface},
};

#[derive(Debug, Clone)]
enum RunState {
    Running,
    Idle,
}

pub struct DaemonInstance {
    rx: async_channel::Receiver<DaemonCommand>,
    sx: async_channel::Sender<DaemonResponse>,
    subx: async_channel::Sender<DaemonNotification>,
    tun_interface: Arc<RwLock<Option<TunInterface>>>,
    wg_interface: Arc<RwLock<Interface>>,
    config: Arc<RwLock<Config>>,
    db_path: Option<PathBuf>,
    wg_state: RunState,
}

impl DaemonInstance {
    pub fn new(
        rx: async_channel::Receiver<DaemonCommand>,
        sx: async_channel::Sender<DaemonResponse>,
        subx: async_channel::Sender<DaemonNotification>,
        wg_interface: Arc<RwLock<Interface>>,
        config: Arc<RwLock<Config>>,
        db_path: Option<&Path>,
    ) -> Self {
        Self {
            rx,
            sx,
            subx,
            wg_interface,
            tun_interface: Arc::new(RwLock::new(None)),
            config,
            db_path: db_path.map(|p| p.to_owned()),
            wg_state: RunState::Idle,
        }
    }

    async fn proc_command(&mut self, command: DaemonCommand) -> Result<DaemonResponseData> {
        info!("Daemon got command: {:?}", command);
        match command {
            DaemonCommand::Start(st) => {
                match self.wg_state {
                    RunState::Running => {
                        warn!("Got start, but tun interface already up.");
                    }
                    RunState::Idle => {
                        let tun_if = st.tun.open()?;
                        debug!("Setting tun on wg_interface");
                        self.wg_interface.read().await.set_tun(tun_if).await;
                        debug!("tun set on wg_interface");

                        debug!("Setting tun_interface");
                        self.tun_interface = self.wg_interface.read().await.get_tun();
                        debug!("tun_interface set: {:?}", self.tun_interface);

                        debug!("Cloning wg_interface");
                        let tmp_wg = self.wg_interface.clone();
                        let run_task = tokio::spawn(async move {
                            let twlock = tmp_wg.read().await;
                            twlock.run().await
                        });
                        self.wg_state = RunState::Running;
                        info!("Daemon started tun interface");
                    }
                }
                Ok(DaemonResponseData::None)
            }
            DaemonCommand::ServerInfo => match &self.tun_interface.read().await.as_ref() {
                None => Ok(DaemonResponseData::None),
                Some(ti) => {
                    info!("{:?}", ti);
                    Ok(DaemonResponseData::ServerInfo(ServerInfo::try_from(
                        ti.inner.get_ref(),
                    )?))
                }
            },
            DaemonCommand::Stop => {
                self.wg_interface.read().await.remove_tun().await;
                self.wg_state = RunState::Idle;
                Ok(DaemonResponseData::None)
            }
            DaemonCommand::ServerConfig => {
                Ok(DaemonResponseData::ServerConfig(ServerConfig::default()))
            }
            DaemonCommand::ReloadConfig(interface_id) => {
                let conn = get_connection(self.db_path.as_deref())?;
                let cfig = load_interface(&conn, &interface_id)?;
                *self.config.write().await = cfig;
                self.subx
                    .send(DaemonNotification::ConfigChange(ServerConfig::try_from(
                        &self.config.read().await.to_owned(),
                    )?))
                    .await?;
                Ok(DaemonResponseData::None)
            }
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        while let Ok(command) = self.rx.recv().await {
            let response = self.proc_command(command).await;
            info!("Daemon response: {:?}", response);
            self.sx.send(DaemonResponse::new(response)).await?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct DaemonRPCServer {
    tun_interface: Arc<RwLock<Option<TunInterface>>>,
    wg_interface: Arc<RwLock<Interface>>,
    config: Arc<RwLock<Config>>,
    db_path: Option<PathBuf>,
    wg_state: Arc<RwLock<RunState>>,
}

impl DaemonRPCServer {
    pub fn new(
        wg_interface: Arc<RwLock<Interface>>,
        config: Arc<RwLock<Config>>,
        db_path: Option<&Path>,
    ) -> Self {
        Self {
            tun_interface: Arc::new(RwLock::new(None)),
            wg_interface,
            config,
            db_path: db_path.map(|p| p.to_owned()),
            wg_state: Arc::new(RwLock::new(RunState::Idle)),
        }
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
        match self.wg_state.read().await.deref() {
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
                let mut guard = self.wg_state.write().await;
                *guard = RunState::Running;
            }

            RunState::Running => {
                warn!("Got start, but tun interface already up.");
            }
        }

        return Ok(Response::new(Empty {}));
    }

    async fn tunnel_stop(&self, _request: Request<Empty>) -> Result<Response<Empty>, RspStatus> {
        return Ok(Response::new(Empty {}));
    }

    async fn tunnel_status(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::TunnelStatusStream>, RspStatus> {
        let (tx, rx) = mpsc::channel(10);
        tokio::spawn(async move {
            for _ in 0..1000 {
                tx.send(Ok(TunnelStatusResponse { ..Default::default() }))
                    .await;
                tokio::time::sleep(Duration::from_secs(100)).await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tonic::async_trait]
impl Networks for DaemonRPCServer {
    type NetworkListStream = ReceiverStream<Result<NetworkListResponse, RspStatus>>;

    async fn network_add(&self, _request: Request<Empty>) -> Result<Response<Empty>, RspStatus> {
        debug!("Mock network_add called");
        Ok(Response::new(Empty {}))
    }

    async fn network_list(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::NetworkListStream>, RspStatus> {
        debug!("Mock network_list called");
        let (tx, rx) = mpsc::channel(10);
        tokio::spawn(async move {
            tx.send(Ok(NetworkListResponse { ..Default::default() }))
                .await
                .unwrap();
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn network_reorder(
        &self,
        _request: Request<NetworkReorderRequest>,
    ) -> Result<Response<Empty>, RspStatus> {
        debug!("Mock network_reorder called");
        Ok(Response::new(Empty {}))
    }

    async fn network_delete(
        &self,
        _request: Request<NetworkDeleteRequest>,
    ) -> Result<Response<Empty>, RspStatus> {
        debug!("Mock network_delete called");
        Ok(Response::new(Empty {}))
    }
}
