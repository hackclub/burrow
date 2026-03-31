use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::{sync::RwLock, task::JoinHandle};
use tun::{tokio::TunInterface, TunOptions};

use super::rpc::{
    grpc_defs::{Network, NetworkType},
    ServerConfig,
};
use crate::{
    mesh::iroh::{self as mesh_iroh, HackClubNetworkConfig, MeshHandle},
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
    WireGuard {
        identity: RuntimeIdentity,
        config: Config,
    },
    HackClub {
        identity: RuntimeIdentity,
        config: HackClubNetworkConfig,
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
            NetworkType::WireGuard => {
                let payload = String::from_utf8(network.payload.clone())
                    .context("wireguard payload must be valid UTF-8")?;
                let config = Config::from_content_fmt(&payload, "ini")?;
                Ok(Self::WireGuard { identity, config })
            }
            NetworkType::HackClub => {
                let config = HackClubNetworkConfig::from_payload(&network.payload)?;
                Ok(Self::HackClub { identity, config })
            }
        }
    }

    pub fn identity(&self) -> &RuntimeIdentity {
        match self {
            Self::Passthrough { identity }
            | Self::WireGuard { identity, .. }
            | Self::HackClub { identity, .. } => identity,
        }
    }

    pub fn server_config(&self) -> Result<ServerConfig> {
        match self {
            Self::Passthrough { .. } => Ok(ServerConfig {
                address: Vec::new(),
                name: None,
                mtu: Some(1500),
            }),
            Self::WireGuard { config, .. } => ServerConfig::try_from(config),
            Self::HackClub { config, .. } => Ok(ServerConfig {
                address: config.local_addresses.clone(),
                name: config.tun_name.clone(),
                mtu: config.mtu.map(i32::from),
            }),
        }
    }

    pub async fn start(
        self,
        tun_interface: Arc<RwLock<Option<TunInterface>>>,
    ) -> Result<ActiveTunnel> {
        match self {
            Self::Passthrough { identity } => Ok(ActiveTunnel::Passthrough { identity }),
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
            Self::HackClub { identity, config } => {
                let mut tun_opts = TunOptions::new();
                if let Some(name) = config.tun_name.as_deref() {
                    tun_opts = tun_opts.name(name);
                }

                let tun = tun_opts.open()?;
                tun_interface.write().await.replace(tun);

                match mesh_iroh::spawn_hackclub_tunnel(config, tun_interface.clone()).await {
                    Ok(handle) => Ok(ActiveTunnel::HackClub { identity, handle }),
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
    HackClub {
        identity: RuntimeIdentity,
        handle: MeshHandle,
    },
}

impl ActiveTunnel {
    pub fn identity(&self) -> &RuntimeIdentity {
        match self {
            Self::Passthrough { identity }
            | Self::WireGuard { identity, .. }
            | Self::HackClub { identity, .. } => identity,
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
            Self::HackClub { handle, .. } => {
                let result = handle.shutdown().await;
                tun_interface.write().await.take();
                result
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
