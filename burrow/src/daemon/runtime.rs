use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::{sync::RwLock, task::JoinHandle};
use tun::{tokio::TunInterface, TunOptions};

use super::rpc::{
    grpc_defs::{Network, NetworkType},
    ServerConfig,
};
use crate::{
    control::TailnetConfig,
    wireguard::{Config, Interface as WireGuardInterface},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeIdentity {
    Passthrough,
    Network {
        id: i32,
        network_type: NetworkType,
        payload: Vec<u8>,
    },
}

#[derive(Clone, Debug)]
pub enum ResolvedTunnel {
    Passthrough {
        identity: RuntimeIdentity,
    },
    Tailnet {
        identity: RuntimeIdentity,
        config: TailnetConfig,
    },
    WireGuard {
        identity: RuntimeIdentity,
        config: Config,
    },
}

impl ResolvedTunnel {
    pub fn from_networks(networks: &[Network]) -> Result<Self> {
        let Some(network) = networks.first() else {
            return Ok(Self::Passthrough {
                identity: RuntimeIdentity::Passthrough,
            });
        };

        let identity = RuntimeIdentity::Network {
            id: network.id,
            network_type: network.r#type(),
            payload: network.payload.clone(),
        };

        match network.r#type() {
            NetworkType::Tailnet => {
                let config = TailnetConfig::from_slice(&network.payload)?;
                Ok(Self::Tailnet { identity, config })
            }
            NetworkType::WireGuard => {
                let payload = String::from_utf8(network.payload.clone())
                    .context("wireguard payload must be valid UTF-8")?;
                let config = Config::from_content_fmt(&payload, "ini")?;
                Ok(Self::WireGuard { identity, config })
            }
        }
    }

    pub fn identity(&self) -> &RuntimeIdentity {
        match self {
            Self::Passthrough { identity }
            | Self::Tailnet { identity, .. }
            | Self::WireGuard { identity, .. } => identity,
        }
    }

    pub fn server_config(&self) -> Result<ServerConfig> {
        match self {
            Self::Passthrough { .. } => Ok(ServerConfig {
                address: Vec::new(),
                name: None,
                mtu: Some(1500),
            }),
            Self::Tailnet { .. } => Ok(ServerConfig {
                address: Vec::new(),
                name: None,
                mtu: Some(1280),
            }),
            Self::WireGuard { config, .. } => ServerConfig::try_from(config),
        }
    }

    pub async fn start(
        self,
        tun_interface: Arc<RwLock<Option<TunInterface>>>,
    ) -> Result<ActiveTunnel> {
        match self {
            Self::Passthrough { identity } => Ok(ActiveTunnel::Passthrough { identity }),
            Self::Tailnet { config, .. } => Err(anyhow::anyhow!(
                "tailnet runtime is not wired in this checkout yet ({:?})",
                config.provider
            )),
            Self::WireGuard { identity, config } => {
                let tun = TunOptions::new().open()?;
                tun_interface.write().await.replace(tun);

                match start_wireguard_runtime(config, tun_interface.clone()).await {
                    Ok((interface, task)) => {
                        Ok(ActiveTunnel::WireGuard { identity, interface, task })
                    }
                    Err(err) => {
                        tun_interface.write().await.take();
                        Err(err)
                    }
                }
            }
        }
    }
}

pub enum ActiveTunnel {
    Passthrough {
        identity: RuntimeIdentity,
    },
    WireGuard {
        identity: RuntimeIdentity,
        interface: Arc<RwLock<WireGuardInterface>>,
        task: JoinHandle<Result<()>>,
    },
}

impl ActiveTunnel {
    pub fn identity(&self) -> &RuntimeIdentity {
        match self {
            Self::Passthrough { identity }
            | Self::WireGuard { identity, .. } => identity,
        }
    }

    pub async fn shutdown(self, tun_interface: &Arc<RwLock<Option<TunInterface>>>) -> Result<()> {
        match self {
            Self::Passthrough { .. } => Ok(()),
            Self::WireGuard { interface, task, .. } => {
                interface.read().await.remove_tun().await;
                let task_result = task.await;
                tun_interface.write().await.take();
                task_result??;
                Ok(())
            }
        }
    }
}

async fn start_wireguard_runtime(
    config: Config,
    tun_interface: Arc<RwLock<Option<TunInterface>>>,
) -> Result<(Arc<RwLock<WireGuardInterface>>, JoinHandle<Result<()>>)> {
    let mut interface: WireGuardInterface = config.try_into()?;
    interface.set_tun_ref(tun_interface).await;
    let interface = Arc::new(RwLock::new(interface));
    let run_interface = interface.clone();
    let task = tokio::spawn(async move {
        let guard = run_interface.read().await;
        guard.run().await
    });
    Ok((interface, task))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_networks_resolve_to_passthrough() {
        let resolved = ResolvedTunnel::from_networks(&[]).unwrap();
        assert_eq!(resolved.identity(), &RuntimeIdentity::Passthrough);
        assert_eq!(
            resolved.server_config().unwrap().address,
            Vec::<String>::new()
        );
    }
}
