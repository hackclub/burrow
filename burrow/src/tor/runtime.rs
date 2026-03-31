use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use arti_client::{config::TorClientConfigBuilder, TorClient};
use tokio::{
    sync::watch,
    task::{JoinError, JoinSet},
};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tor_rtcompat::PreferredRuntime;
use tracing::{debug, error, info, warn};

use super::{system::SystemTcpStackRuntime, Config, TcpStackConfig};

#[derive(Debug)]
pub struct TorHandle {
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl TorHandle {
    pub async fn shutdown(self) -> Result<()> {
        let _ = self.shutdown.send(true);
        match self.task.await {
            Ok(()) => Ok(()),
            Err(err) if err.is_cancelled() => Ok(()),
            Err(err) => Err(join_error(err)),
        }
    }
}

pub async fn bootstrap_client(config: &Config) -> Result<Arc<TorClient<PreferredRuntime>>> {
    let builder =
        TorClientConfigBuilder::from_directories(&config.arti.state_dir, &config.arti.cache_dir);
    let tor_config = builder.build().context("failed to build arti config")?;
    let tor_client = TorClient::create_bootstrapped(tor_config)
        .await
        .context("failed to bootstrap arti client")?;
    Ok(Arc::new(tor_client))
}

pub async fn spawn(config: Config) -> Result<TorHandle> {
    let tor_client = bootstrap_client(&config).await?;
    spawn_with_client(config, tor_client).await
}

pub async fn spawn_with_client(
    config: Config,
    tor_client: Arc<TorClient<PreferredRuntime>>,
) -> Result<TorHandle> {
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let task = match config.tcp_stack.clone() {
        TcpStackConfig::System(system_config) => tokio::spawn(async move {
            let stack = match SystemTcpStackRuntime::bind(&system_config).await {
                Ok(stack) => stack,
                Err(err) => {
                    error!(?err, "failed to bind system tcp stack listener");
                    return;
                }
            };
            info!(
                listen = %stack.local_addr(),
                "system tcp stack listener bound for tor transparent proxy"
            );

            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        match changed {
                            Ok(()) if *shutdown_rx.borrow() => break,
                            Ok(()) => continue,
                            Err(_) => break,
                        }
                    }
                    Some(res) = connections.join_next(), if !connections.is_empty() => {
                        match res {
                            Ok(Ok(())) => {}
                            Ok(Err(err)) => warn!(?err, "transparent proxy task failed"),
                            Err(err) => warn!(?err, "transparent proxy task panicked"),
                        }
                    }
                    accepted = stack.accept() => {
                        let (mut inbound, original_dst) = match accepted {
                            Ok(pair) => pair,
                            Err(err) => {
                                warn!(?err, "failed to accept transparent tcp connection");
                                tokio::time::sleep(Duration::from_millis(50)).await;
                                continue;
                            }
                        };

                        let tor_client = tor_client.clone();
                        connections.spawn(async move {
                            debug!(%original_dst, "accepted transparent tcp connection");
                            let tor_stream = tor_client
                                .connect((original_dst.ip().to_string(), original_dst.port()))
                                .await
                                .with_context(|| format!("failed to connect to {original_dst} over tor"))?;
                            let mut tor_stream = tor_stream.compat();
                            tokio::io::copy_bidirectional(&mut inbound, &mut tor_stream)
                                .await
                                .with_context(|| format!("failed to bridge tor stream for {original_dst}"))?;
                            Result::<()>::Ok(())
                        });
                    }
                }
            }

            connections.abort_all();
            while let Some(res) = connections.join_next().await {
                match res {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => debug!(?err, "transparent proxy task failed during shutdown"),
                    Err(err) => debug!(?err, "transparent proxy task exited during shutdown"),
                }
            }
        }),
    };

    Ok(TorHandle {
        shutdown: shutdown_tx,
        task,
    })
}

fn join_error(err: JoinError) -> anyhow::Error {
    anyhow::anyhow!("tor runtime task failed: {err}")
}
