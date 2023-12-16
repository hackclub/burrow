use tracing::debug;
use tracing_oslog::OsLogger;
use tracing_subscriber::layer::SubscriberExt;

pub use crate::daemon::start_srv;

#[no_mangle]
pub extern "C" fn initialize_oslog() {
    let collector =
        tracing_subscriber::registry().with(OsLogger::new("com.hackclub.burrow", "backend"));
    tracing::subscriber::set_global_default(collector).unwrap();
    debug!("Initialized oslog tracing in libburrow rust FFI");
}
