use std::sync::Once;

use tracing::{error, info};
use tracing_subscriber::{
    layer::{Layer, SubscriberExt},
    EnvFilter,
    Registry,
};

static TRACING: Once = Once::new();

pub fn initialize() {
    TRACING.call_once(|| {
        if let Err(e) = tracing_log::LogTracer::init() {
            error!("Failed to initialize LogTracer: {}", e);
        }

        #[cfg(target_os = "windows")]
        let system_log = Some(tracing_subscriber::fmt::layer());

        #[cfg(target_os = "linux")]
        let system_log = match tracing_journald::layer() {
            Ok(layer) => Some(layer),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    error!("Failed to initialize journald: {}", e);
                }
                None
            }
        };

        #[cfg(target_vendor = "apple")]
        let system_log = Some(tracing_oslog::OsLogger::new(
            "com.hackclub.burrow",
            "tracing",
        ));

        let stderr = (console::user_attended_stderr() || system_log.is_none()).then(|| {
            tracing_subscriber::fmt::layer()
                .with_level(true)
                .with_writer(std::io::stderr)
                .compact()
                .with_filter(EnvFilter::from_default_env())
        });

        let subscriber = Registry::default().with(stderr).with(system_log);

        #[cfg(feature = "tokio-console")]
        let subscriber = subscriber.with(
            console_subscriber::spawn().with_filter(
                EnvFilter::from_default_env()
                    .add_directive("tokio=trace".parse().unwrap())
                    .add_directive("runtime=trace".parse().unwrap()),
            ),
        );

        if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
            error!("Failed to initialize logging: {}", e);
        }

        info!("Initialized logging")
    });
}
