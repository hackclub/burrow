use anyhow::Result;

pub mod components;
mod daemon;
mod diag;

//  Generated using meson
mod config;

fn main() {
    components::App::run();
}
