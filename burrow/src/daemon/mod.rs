use super::*;
use tokio::sync::mpsc;

mod command;
mod instance;
mod net;
mod response;

use instance::DaemonInstance;
use net::listen;

pub use command::{DaemonCommand, DaemonStartOptions};
pub use net::DaemonClient;

#[cfg(target_vendor = "apple")]
pub use net::start_srv;

pub use response::{DaemonResponseData, DaemonResponse, ServerInfo};

pub async fn daemon_main() -> Result<()> {
    let (commands_tx, commands_rx) = async_channel::unbounded();
    let (response_tx, response_rx) = async_channel::unbounded();
    let mut inst = DaemonInstance::new(commands_rx, response_tx);

    tokio::try_join!(inst.run(), listen(commands_tx, response_rx)).map(|_| ())
}
