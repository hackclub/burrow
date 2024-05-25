use anyhow::Result;

mod auth;
pub mod components;
mod diag;

//  Generated using meson
mod config;

fn main() {
    components::App::run();
}
