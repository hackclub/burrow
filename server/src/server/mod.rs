pub mod db;
pub mod grpc_defs;
mod grpc_server;
pub mod providers;
pub mod settings;

use anyhow::Result;
use providers::slack::auth;
use tokio::signal;

pub async fn serve() -> Result<()> {
    db::init_db()?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    log::info!("Starting auth server on port 8080");
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
