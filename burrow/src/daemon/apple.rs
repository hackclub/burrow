use std::{
    ffi::{c_char, CStr},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use once_cell::sync::{Lazy, OnceCell};
use tokio::{
    runtime::{Builder, Handle},
    sync::Notify,
};
use tracing::error;

use crate::daemon::daemon_main;

static BURROW_HANDLE: OnceCell<Handle> = OnceCell::new();
static BURROW_READY: OnceCell<()> = OnceCell::new();
static BURROW_SPAWN_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[no_mangle]
pub unsafe extern "C" fn spawn_in_process(path: *const c_char, db_path: *const c_char) {
    let path_buf = if path.is_null() {
        None
    } else {
        Some(PathBuf::from(CStr::from_ptr(path).to_str().unwrap()))
    };
    let db_path_buf = if db_path.is_null() {
        None
    } else {
        Some(PathBuf::from(CStr::from_ptr(db_path).to_str().unwrap()))
    };
    spawn_in_process_with_paths(path_buf, db_path_buf);
}

pub fn spawn_in_process_with_paths(path_buf: Option<PathBuf>, db_path_buf: Option<PathBuf>) {
    crate::tracing::initialize();

    let _guard = BURROW_SPAWN_LOCK.lock().unwrap();
    if BURROW_READY.get().is_some() {
        return;
    }

    let notify = Arc::new(Notify::new());
    let handle = BURROW_HANDLE.get_or_init(|| {
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
                let result = daemon_main(
                    path_buf.as_deref(),
                    db_path_buf.as_deref(),
                    Some(sender.clone()),
                )
                .await;
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
    let _ = BURROW_READY.set(());
}
