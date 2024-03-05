use std::{path::Path, sync::Arc};

pub mod apple;
mod command;
mod instance;
mod net;
mod response;

use anyhow::Result;
pub use command::{DaemonCommand, DaemonStartOptions};
use instance::DaemonInstance;
pub use net::{DaemonClient, Listener};
pub use response::{DaemonResponse, DaemonResponseData, ServerInfo};
use tokio::sync::{Notify, RwLock};
use tracing::{error, info};
use crate::database::{get_connection, load_interface};

use crate::wireguard::{Config, Interface};

pub async fn daemon_main(path: Option<&Path>, notify_ready: Option<Arc<Notify>>) -> Result<()> {
    let (commands_tx, commands_rx) = async_channel::unbounded();
    let (response_tx, response_rx) = async_channel::unbounded();

    let listener = if let Some(path) = path {
        info!("Creating listener... {:?}", path);
        Listener::new_with_path(commands_tx, response_rx, path)
    } else {
        info!("Creating listener...");
        Listener::new(commands_tx, response_rx)
    };
    if let Some(n) = notify_ready {
        n.notify_one()
    }
    let listener = listener?;

    let conn = get_connection()?;
    let config = load_interface(&conn, "0".into())?;
    let iface: Interface = config.try_into()?;
    let mut instance = DaemonInstance::new(commands_rx, response_tx, Arc::new(RwLock::new(iface)));

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
