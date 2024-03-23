use std::{path::Path, sync::Arc};

pub mod apple;
mod instance;
mod net;
mod rpc;

use anyhow::Result;
use instance::DaemonInstance;
pub use net::{DaemonClient, Listener};
pub use rpc::{DaemonCommand, DaemonResponse, DaemonResponseData, DaemonStartOptions, ServerInfo};
use tokio::sync::{Notify, RwLock};
use tracing::{error, info};

use crate::{
    database::{get_connection, load_interface},
    wireguard::Interface,
};

pub async fn daemon_main(path: Option<&Path>, notify_ready: Option<Arc<Notify>>) -> Result<()> {
    let (commands_tx, commands_rx) = async_channel::unbounded();
    let (response_tx, response_rx) = async_channel::unbounded();
    let (subscribe_tx, subscribe_rx) = async_channel::unbounded();

    let listener = if let Some(path) = path {
        info!("Creating listener... {:?}", path);
        Listener::new_with_path(commands_tx, response_rx, subscribe_rx, path)
    } else {
        info!("Creating listener...");
        Listener::new(commands_tx, response_rx, subscribe_rx)
    };
    if let Some(n) = notify_ready {
        n.notify_one()
    }
    let listener = listener?;

    let conn = get_connection()?;
    let config = load_interface(&conn, "1")?;
    let iface: Interface = config.clone().try_into()?;
    let mut instance = DaemonInstance::new(
        commands_rx,
        response_tx,
        subscribe_tx,
        Arc::new(RwLock::new(iface)),
        Arc::new(RwLock::new(config)),
    );

    info!("Starting daemon...");

    let main_job = tokio::spawn(async move {
        let result = instance.run().await;
        if let Err(e) = result.as_ref() {
            error!("Instance exited: {}", e);
        }
        result
    });

    let listener_job = tokio::spawn(async move {
        let result = listener.run().await;
        if let Err(e) = result.as_ref() {
            error!("Listener exited: {}", e);
        }
        result
    });

    tokio::try_join!(main_job, listener_job)
        .map(|_| ())
        .map_err(|e| e.into())
}
