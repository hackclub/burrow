use super::*;
use adw::prelude::*;
use burrow::{DaemonClient, DaemonCommand, DaemonResponseData};
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

mod app;
mod auth_screen;
mod settings;
mod settings_screen;
mod switch_screen;

pub use app::*;
pub use settings::{DaemonGroupMsg, DiagGroupMsg};
