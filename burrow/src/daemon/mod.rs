use std::{path::Path, sync::Arc};

pub mod apple;
mod instance;
mod net;
pub mod rpc;

use anyhow::{Error as AhError, Result};
use instance::{DaemonInstance, DaemonRPCServer};
pub use net::{get_socket_path, DaemonClient, Listener};
pub use rpc::{DaemonCommand, DaemonResponseData, DaemonStartOptions};
use tokio::{
    net::UnixListener,
    sync::{Notify, RwLock},
};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tracing::{error, info};

use crate::{
    daemon::rpc::grpc_defs::{networks_server::NetworksServer, tunnel_server::TunnelServer},
    database::{get_connection, load_interface},
    wireguard::Interface,
};

pub async fn daemon_main(
    socket_path: Option<&Path>,
    db_path: Option<&Path>,
    notify_ready: Option<Arc<Notify>>,
) -> Result<()> {
    let (commands_tx, commands_rx) = async_channel::unbounded();
    let (response_tx, response_rx) = async_channel::unbounded();
    let (subscribe_tx, subscribe_rx) = async_channel::unbounded();

    if let Some(n) = notify_ready {
        n.notify_one()
    }
    let conn = get_connection(db_path)?;
    let config = load_interface(&conn, "1")?;
    let iface: Interface = config.clone().try_into()?;
    let mut instance = DaemonInstance::new(
        commands_rx,
        response_tx,
        subscribe_tx,
        Arc::new(RwLock::new(iface)),
        Arc::new(RwLock::new(config.clone())),
        db_path.clone(),
    );
    let dbp = db_path.clone();
    let burrow_server = DaemonRPCServer::new(
        Arc::new(RwLock::new(config.clone().try_into()?)),
        Arc::new(RwLock::new(config)),
        dbp,
    )?;
    let spp = socket_path.clone();
    let tmp = get_socket_path();
    let sock_path = spp.unwrap_or(Path::new(tmp.as_str()));
    if sock_path.exists() {
        std::fs::remove_file(sock_path)?;
    }
    let uds = UnixListener::bind(sock_path)?;
    let serve_job = tokio::spawn(async move {
        let uds_stream = UnixListenerStream::new(uds);
        let _srv = Server::builder()
            .add_service(TunnelServer::new(burrow_server.clone()))
            .add_service(NetworksServer::new(burrow_server))
            .serve_with_incoming(uds_stream)
            .await?;
        Ok::<(), AhError>(())
    });

    info!("Starting daemon...");

    let main_job = tokio::spawn(async move {
        let result = instance.run().await;
        if let Err(e) = result.as_ref() {
            error!("Instance exited: {}", e);
        }
        result
    });

    tokio::try_join!(main_job, serve_job)
        .map(|_| ())
        .map_err(|e| e.into())
}
