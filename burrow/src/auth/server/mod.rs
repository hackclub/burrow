pub mod db;
pub mod providers;

use anyhow::Result;
use axum::{http::StatusCode, routing::post, Router};
use providers::slack::auth;

pub async fn start_server() -> Result<()> {
    db::init_db()?;

    let app = Router::new()
        .route("/slack-auth", post(auth))
        .route("/device/new", post(device_new));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4225").await.unwrap();
    log::info!("Starting auth server on port 4225");
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn device_new() -> StatusCode {
    StatusCode::OK
}

// mod db {
//     use rusqlite::{Connection, Result};

//     #[derive(Debug)]
//     struct User {
//         id: i32,
//         created_at: String,
//     }
// }
