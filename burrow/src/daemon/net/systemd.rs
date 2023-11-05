use super::*;
use std::os::fd::IntoRawFd;

pub async fn listen(cmd_tx: async_channel::Sender<DaemonCommand>, rsp_rx: async_channel::Receiver<DaemonResponse>) -> Result<()> {
    if !libsystemd::daemon::booted() || listen_with_systemd(cmd_tx.clone(), rsp_rx.clone()).await.is_err() {
        unix::listen(cmd_tx, rsp_rx).await?;
    }
    Ok(())
}

async fn listen_with_systemd(cmd_tx: async_channel::Sender<DaemonCommand>, rsp_rx: async_channel::Receiver<DaemonResponse>) -> Result<()> {
    let fds = libsystemd::activation::receive_descriptors(false)?;
    super::unix::listen_with_optional_fd(cmd_tx, rsp_rx,Some(fds[0].clone().into_raw_fd())).await
}

pub type DaemonClient = unix::DaemonClient;
