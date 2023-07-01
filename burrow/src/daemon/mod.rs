use super::*;
use tokio::sync::mpsc;

mod command;
mod instance;
mod net;

use instance::DaemonInstance;
use net::listen;

pub use command::{DaemonCommand, DaemonStartOptions};
pub use net::DaemonClient;

pub async fn daemon_main() -> Result<()> {
    let (tx, rx) = mpsc::channel(2);
    let mut inst = DaemonInstance::new(rx);

    tokio::try_join!(inst.run(), listen(tx)).map(|_| ())
}
