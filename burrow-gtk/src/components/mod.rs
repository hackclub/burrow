use super::*;
use adw::prelude::*;
use gtk::Align;
use relm4::{
    component::{
        AsyncComponent, AsyncComponentController, AsyncComponentParts, AsyncComponentSender,
        AsyncController,
    },
    prelude::*,
};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod burrow_rpc {
    tonic::include_proto!("burrow");
}
use burrow_rpc::tunnel_client;
use tonic::transport::Channel;

mod app;
mod settings;
mod settings_screen;
mod switch_screen;

pub use app::*;
pub use settings::{DaemonGroupMsg, DiagGroupMsg};
