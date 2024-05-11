use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use tokio::{sync::RwLock, task::JoinHandle};
use tracing::{debug, info, warn};
use tun::tokio::TunInterface;

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

enum RunState {
    Running(JoinHandle<Result<()>>),
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
                    RunState::Running(_) => {
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
                        self.wg_state = RunState::Running(run_task);
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
            DaemonCommand::AddConfigToml(interface_id, config_toml) => {
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
