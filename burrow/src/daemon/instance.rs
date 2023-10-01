use tracing::{debug, info, warn};
use DaemonResponse;
use crate::daemon::response::{DaemonResponseData, ServerConfig, ServerInfo};
use super::*;

pub struct DaemonInstance {
    rx: async_channel::Receiver<DaemonCommand>,
    sx: async_channel::Sender<DaemonResponse>,
    tun_interface: Option<TunInterface>,
}

impl DaemonInstance {
    pub fn new(rx: async_channel::Receiver<DaemonCommand>, sx: async_channel::Sender<DaemonResponse>) -> Self {
        Self {
            rx,
            sx,
            tun_interface: None,
        }
    }

    async fn proc_command(&mut self, command: DaemonCommand) -> Result<DaemonResponseData> {
        info!("Daemon got command: {:?}", command);
        match command {
            DaemonCommand::Start(st) => {
                if self.tun_interface.is_none() {
                    debug!("Daemon attempting start tun interface.");
                    self.tun_interface = Some(st.tun.open()?);
                    info!("Daemon started tun interface");
                } else {
                    warn!("Got start, but tun interface already up.");
                }
                Ok(DaemonResponseData::None)
            }
            DaemonCommand::ServerInfo => {
                match &self.tun_interface {
                    None => {Ok(DaemonResponseData::None)}
                    Some(ti) => {
                        info!("{:?}", ti);
                        Ok(
                            DaemonResponseData::ServerInfo(
                                ServerInfo::try_from(ti)?
                            )
                        )
                    }
                }
            }
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
