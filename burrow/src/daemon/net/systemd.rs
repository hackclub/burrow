use super::*;
use std::os::fd::IntoRawFd;

pub async fn listen(cmd_tx: mpsc::Sender<DaemonCommand>) -> Result<()> {
    if !libsystemd::daemon::booted() || listen_with_systemd(cmd_tx.clone()).await.is_err() {
        unix::listen(cmd_tx).await?;
    }
    Ok(())
}

async fn listen_with_systemd(cmd_tx: mpsc::Sender<DaemonCommand>) -> Result<()> {
    let fds = libsystemd::activation::receive_descriptors(false).unwrap();
    super::unix::listen_with_optional_fd(cmd_tx, Some(fds[0].clone().into_raw_fd())).await
}

pub type DaemonClient = unix::DaemonClient;
