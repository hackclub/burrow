use axum::{body::Bytes, error_handling::HandleErrorLayer, extract::{DefaultBodyLimit, Path, State}, handler::Handler, http::StatusCode, response::IntoResponse, routing::{delete, get}, Router, Json, debug_handler};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use tokio::runtime::Runtime;
use std::net::{Ipv4Addr, SocketAddr};
use axum::handler::HandlerWithoutStateExt;
use serde_json::json;
use tun::tokio::TunInterface; // TODO: refactor to tokio TunInterface, which doesn't implement `Send`
use std::thread;
use crate::get_iface;
use tracing::{info, debug, error};

type SharedState = Arc<RwLock<TunInterface>>;

#[no_mangle]
pub extern "C" fn spawn_server(){
    info!("Spawning server");
    let ti = get_iface().unwrap();
    debug!("Got interface");
    let rt = Runtime::new().unwrap();
    let _handle = thread::spawn(move || {
        rt.spawn(async {
            service(ti).await;
        });
    });
    debug!("Spawned thread: finish spawn server");
}

async fn service(ti: crate::TunInterface){
    info!("Spawning service");
    let shared_state = Arc::new(RwLock::new(TunInterface::new(ti).unwrap()));
    info!("Created shared state");
    let state_cl= shared_state.clone();
    let lp = tokio::spawn(
        async move {
            burrow_loop(state_cl).await;
        }
    );
    let srv = tokio::spawn(
        async move {
            serve(shared_state).await;
        }
    );
    info!("Created threads");
    tokio::join!(lp, srv);
}

async fn burrow_loop(state: SharedState){
    debug!("loop called");
    let mut buf = [0u8; 1504];
    loop {
        let n = state.write().await.read(&mut buf[..]).await.unwrap();
        // do something with the data
        info!("read {} bytes", n);
    }
}

async fn serve(state: SharedState){
    debug!("serve called");
    let app_router = Router::new()
        .route("/info", get(network_settings))
        .with_state(state);
    let port = std::env::var("BURROW_PORT").unwrap_or("3000".to_string());
    let sock_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port.parse().unwrap());
    info!("Listening on {}...", sock_addr);
    axum::Server::bind(&sock_addr)
        .serve(app_router.into_make_service())
        .await
        .unwrap();
}

#[debug_handler]
async fn network_settings(State(state): State<SharedState>) -> impl IntoResponse {
    let st = state.read().await;
    let name = st.name().await.unwrap();
    let mtu = st.mtu().await.unwrap();
    let netmask = st.netmask().await.unwrap();
    let res = Json(json!({
        "name": name,
        "mtu": mtu,
        "netmask": netmask,
    }));
    res
}