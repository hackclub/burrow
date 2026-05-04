use anyhow::Result;

pub mod components;
mod account_store;
mod daemon_api;

//  Generated using meson
mod config;

fn main() {
    if let Err(error) = daemon_api::configure_client_paths() {
        eprintln!("failed to configure Burrow daemon paths: {error}");
    }
    components::App::run();
}
