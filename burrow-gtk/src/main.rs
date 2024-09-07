use anyhow::{Context, Result};
use log::error;

pub mod components;
mod daemon;
mod diag;

//  Generated using meson
mod config;

fn main() {
    colog::default_builder()
        .filter(None, log::LevelFilter::Error)
        .init();
    components::App::run();
}
