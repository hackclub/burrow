use std::sync::Arc;

mod command;
mod instance;
mod net;
mod response;

use anyhow::Result;
pub use command::{DaemonCommand, DaemonStartOptions};
use instance::DaemonInstance;
#[cfg(target_vendor = "apple")]
pub use net::start_srv;
pub use net::DaemonClient;
pub use response::{DaemonResponse, DaemonResponseData, ServerInfo};
use tokio::sync::{Notify, RwLock};

use crate::{
    daemon::net::listen,
    wireguard::{Config, Interface},
};

pub async fn daemon_main(notify_ready: Option<Arc<Notify>>) -> Result<()> {
    let (commands_tx, commands_rx) = async_channel::unbounded();
    let (response_tx, response_rx) = async_channel::unbounded();

    let config = Config::default();
    let iface: Interface = config.try_into()?;

    let mut inst: DaemonInstance =
        DaemonInstance::new(commands_rx, response_tx, Arc::new(RwLock::new(iface)));

    tracing::info!("Starting daemon jobs...");

    let inst_job = tokio::spawn(async move {
        let res = inst.run().await;
        if let Err(e) = res {
            tracing::error!("Error when running instance: {}", e);
        }
    });

    let listen_job = tokio::spawn(async move {
        let res = listen(commands_tx, response_rx, notify_ready).await;
        if let Err(e) = res {
            tracing::error!("Error when listening: {}", e);
        }
    });

    tokio::try_join!(inst_job, listen_job)
        .map(|_| ())
        .map_err(|e| e.into())
}
