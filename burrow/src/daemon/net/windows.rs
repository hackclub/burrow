use super::*;

pub async fn listen(_cmd_tx: async_channel::Sender<DaemonCommand>, _rsp_rx: async_channel::Receiver<DaemonResponse>) -> Result<()> {
    unimplemented!("This platform does not currently support daemon mode.")
}

pub struct DaemonClient;

impl DaemonClient {
    pub async fn new() -> Result<Self> {
        unimplemented!("This platform does not currently support daemon mode.")
    }

    pub async fn send_command(&mut self, _: DaemonCommand) -> Result<()> {
        unimplemented!("This platform does not currently support daemon mode.")
    }
}
