use std::sync::Arc;
use std::thread;

use tokio::runtime::Runtime;
use tokio::sync::Notify;
use tracing::{error, info};

use crate::daemon::{daemon_main, DaemonClient};

#[no_mangle]
pub extern "C" fn start_srv() {
    info!("Starting server");
    let start_notify = Arc::new(Notify::new());
    let start_recv = start_notify.clone();
    let _handle = thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = daemon_main(Some(start_notify.clone())).await {
                error!("Error when starting rpc server: {}", e);
            }
        });
        start_notify.notify_one();
    });
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        start_recv.notified().await;
        match DaemonClient::new().await {
            Ok(..) => info!("Server successfully started"),
            Err(e) => error!("Could not connect to server: {}", e)
        }
    });
}
