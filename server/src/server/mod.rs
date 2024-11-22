pub mod db;
pub mod grpc_defs;
mod grpc_server;
pub mod providers;
pub mod settings;

use anyhow::Result;
use grpc_defs::burrow_web_server::BurrowWebServer;
use grpc_server::BurrowGrpcServer;
use tokio::signal;
use tonic::transport::Server;

pub async fn serve() -> Result<()> {
    db::init_db()?;
    let addr = "[::1]:8080".parse()?;
    log::info!("Starting auth server on port 8080");
    let burrow_grpc_server = BurrowGrpcServer::new()?;
    let svc = BurrowWebServer::new(burrow_grpc_server);
    Server::builder()
        .accept_http1(true)
        .add_service(tonic_web::enable(svc))
        .serve(addr)
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

// mod db {
//     use rusqlite::{Connection, Result};

//     #[derive(Debug)]
//     struct User {
//         id: i32,
//         created_at: String,
//     }
// }
