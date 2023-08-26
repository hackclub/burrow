use axum::{body::Bytes, error_handling::HandleErrorLayer, extract::{DefaultBodyLimit, Path, State}, handler::Handler, http::StatusCode, response::IntoResponse, routing::{delete, get}, Router, Json, debug_handler};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use std::net::{Ipv4Addr, SocketAddr};
use axum::handler::HandlerWithoutStateExt;
use serde_json::json;
use tun::TunInterface; // TODO: refactor to tokio TunInterface, which doesn't implement `Send`

type SharedState = Arc<RwLock<TunInterface>>;

pub async fn serve(ti: TunInterface){
    let state = Arc::new(RwLock::new(
        ti
    ));
    let app_router = Router::new()
        .route("/info", get(network_settings))
        .with_state(state);
    let port = std::env::var("BURROW_PORT").unwrap_or("3000".to_string());
    let sock_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port.parse().unwrap());
    axum::Server::bind(&sock_addr)
        .serve(app_router.into_make_service())
        .await
        .unwrap();
}

#[debug_handler]
async fn network_settings(State(state): State<SharedState>) -> impl IntoResponse{
    let st = state.read().unwrap();
    let name = st.name().unwrap();
    let mtu = st.mtu().unwrap();
    let netmask = st.netmask().unwrap();
    let res = Json(json!({
        "name": name,
        "mtu": mtu,
        "netmask": netmask,
    }));
    res
}