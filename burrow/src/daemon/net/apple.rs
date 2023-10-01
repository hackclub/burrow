use std::thread;
use tokio::runtime::Runtime;
use tracing::error;
use crate::daemon::{daemon_main, DaemonClient};

#[no_mangle]
pub extern "C" fn start_srv(){
    let _handle = thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = daemon_main().await {
                error!("Error when starting rpc server: {}", e);
            }
        });
    });
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        loop {
            if let Ok(_) = DaemonClient::new().await{
                break
            }
        }
    });
}