use anyhow::Result;
use fehler::throws;

use super::DaemonCommand;
use crate::daemon::DaemonResponse;

pub struct Listener;

impl Listener {
    pub fn new_with_path(
        cmd_tx: async_channel::Sender<DaemonCommand>,
        rsp_rx: async_channel::Receiver<DaemonResponse>,
        path: &Path,
    ) -> Self {
        Self
    }

    pub async fn run(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct DaemonClient;

impl DaemonClient {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }

    pub async fn send_command(&mut self, command: DaemonCommand) -> Result<DaemonResponse> {
        unimplemented!("This platform does not currently support daemon mode.")
    }
}
