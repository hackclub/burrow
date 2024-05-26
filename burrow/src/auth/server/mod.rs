pub mod db;
pub mod providers;

use anyhow::Result;
use axum::{http::StatusCode, routing::post, Router};
use providers::slack::auth;
use tokio::signal;

pub async fn serve() -> Result<()> {
    db::init_db()?;

    let app = Router::new()
        .route("/slack-auth", post(auth))
        .route("/device/new", post(device_new));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    log::info!("Starting auth server on port 8080");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

async fn device_new() -> StatusCode {
    StatusCode::OK
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
