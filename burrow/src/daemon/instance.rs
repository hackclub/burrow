use std::sync::Arc;

use anyhow::Result;
use tokio::{sync::RwLock, task::JoinHandle};
use tracing::{debug, info, warn};
use tun::tokio::TunInterface;

use crate::{
    daemon::{
        command::DaemonCommand,
        response::{DaemonResponse, DaemonResponseData, ServerConfig, ServerInfo},
    },
    wireguard::Interface,
};

enum RunState {
    Running(JoinHandle<Result<()>>),
    Idle,
}

pub struct DaemonInstance {
    rx: async_channel::Receiver<DaemonCommand>,
    sx: async_channel::Sender<DaemonResponse>,
    tun_interface: Option<Arc<RwLock<TunInterface>>>,
    wg_interface: Arc<RwLock<Interface>>,
    wg_state: RunState,
}

impl DaemonInstance {
    pub fn new(
        rx: async_channel::Receiver<DaemonCommand>,
        sx: async_channel::Sender<DaemonResponse>,
        wg_interface: Arc<RwLock<Interface>>,
    ) -> Self {
        Self {
            rx,
            sx,
            wg_interface,
            tun_interface: None,
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
                        let tun_if = Arc::new(RwLock::new(st.tun.open()?));

                        debug!("Setting tun_interface");
                        self.tun_interface = Some(tun_if.clone());
                        debug!("tun_interface set: {:?}", self.tun_interface);

                        debug!("Setting tun on wg_interface");
                        self.wg_interface.write().await.set_tun(tun_if);
                        debug!("tun set on wg_interface");

                        debug!("Cloning wg_interface");
                        let tmp_wg = self.wg_interface.clone();
                        debug!("wg_interface cloned");

                        debug!("Spawning run task");
                        let run_task = tokio::spawn(async move {
                            debug!("Running wg_interface");
                            let twlock = tmp_wg.read().await;
                            debug!("wg_interface read lock acquired");
                            twlock.run().await
                        });
                        debug!("Run task spawned: {:?}", run_task);

                        debug!("Setting wg_state to Running");
                        self.wg_state = RunState::Running(run_task);
                        debug!("wg_state set to Running");

                        info!("Daemon started tun interface");
                    }
                }
                Ok(DaemonResponseData::None)
            }
            DaemonCommand::ServerInfo => match &self.tun_interface {
                None => Ok(DaemonResponseData::None),
                Some(ti) => {
                    info!("{:?}", ti);
                    Ok(DaemonResponseData::ServerInfo(ServerInfo::try_from(
                        ti.read().await.inner.get_ref(),
                    )?))
                }
            },
            DaemonCommand::Stop => {
                if self.tun_interface.is_some() {
                    self.tun_interface = None;
                    info!("Daemon stopping tun interface.");
                } else {
                    warn!("Got stop, but tun interface is not up.")
                }
                Ok(DaemonResponseData::None)
            }
            DaemonCommand::ServerConfig => {
                Ok(DaemonResponseData::ServerConfig(ServerConfig::default()))
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
