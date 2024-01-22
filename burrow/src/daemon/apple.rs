use std::{
    ffi::{c_char, CStr},
    path::PathBuf,
    sync::Arc,
    thread,
};

use once_cell::sync::OnceCell;
use tokio::{
    runtime::{Builder, Handle},
    sync::Notify,
};
use tracing::error;

use crate::daemon::daemon_main;

static BURROW_NOTIFY: OnceCell<Arc<Notify>> = OnceCell::new();
static BURROW_HANDLE: OnceCell<Handle> = OnceCell::new();

#[no_mangle]
pub unsafe extern "C" fn spawn_in_process(path: *const c_char) {
    crate::tracing::initialize();

    let notify = BURROW_NOTIFY.get_or_init(|| Arc::new(Notify::new()));
    let handle = BURROW_HANDLE.get_or_init(|| {
        let path_buf = if path.is_null() {
            None
        } else {
            Some(PathBuf::from(CStr::from_ptr(path).to_str().unwrap()))
        };
        let sender = notify.clone();

        let (handle_tx, handle_rx) = tokio::sync::oneshot::channel();
        thread::spawn(move || {
            let runtime = Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .thread_name("burrow-worker")
                .build()
                .unwrap();
            handle_tx.send(runtime.handle().clone()).unwrap();
            runtime.block_on(async {
                let result = daemon_main(path_buf.as_deref(), Some(sender.clone())).await;
                if let Err(error) = result.as_ref() {
                    error!("Burrow thread exited: {}", error);
                }
                result
            })
        });
        handle_rx.blocking_recv().unwrap()
    });

    let receiver = notify.clone();
    handle.block_on(async move { receiver.notified().await });
}
