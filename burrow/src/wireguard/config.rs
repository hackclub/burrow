use std::{net::ToSocketAddrs, str::FromStr};

use anyhow::{anyhow, Error, Result};
use base64::{engine::general_purpose, Engine};
use fehler::throws;
use ip_network::IpNetwork;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::wireguard::{Interface as WgInterface, Peer as WgPeer};

#[throws]
fn parse_key(string: &str) -> [u8; 32] {
    let value = general_purpose::STANDARD.decode(string)?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&value[..]);
    key
}

#[throws]
fn parse_secret_key(string: &str) -> StaticSecret {
    let key = parse_key(string)?;
    StaticSecret::from(key)
}

#[throws]
fn parse_public_key(string: &str) -> PublicKey {
    let key = parse_key(string)?;
    PublicKey::from(key)
}

/// A raw version of Peer Config that can be used later to reflect configuration files.
/// This should be later converted to a `WgPeer`.
/// Refers to https://github.com/pirate/wireguard-docs?tab=readme-ov-file#overview
pub struct Peer {
    pub public_key: String,
    pub preshared_key: Option<String>,
    pub allowed_ips: Vec<String>,
    pub endpoint: String,
    pub persistent_keepalive: Option<u32>,
    pub name: Option<String>,
}

pub struct Interface {
    pub private_key: String,
    pub address: Vec<String>,
    pub listen_port: u32,
    pub dns: Vec<String>,
    pub mtu: Option<u32>,
}

pub struct Config {
    pub peers: Vec<Peer>,
    pub interface: Interface, // Support for multiple interfaces?
}

impl TryFrom<Config> for WgInterface {
    type Error = anyhow::Error;

    fn try_from(cfig: Config) -> Result<Self, Error> {
        let sk = parse_secret_key(&cfig.interface.private_key)?;
        let wg_peers: Vec<WgPeer> = cfig
            .peers
            .iter()
            .map(|p| {
                Ok(WgPeer {
                    private_key: sk.clone(),
                    public_key: parse_public_key(&p.public_key)?,
                    endpoint: p
                        .endpoint
                        .to_socket_addrs()?
                        .find(|sock| sock.is_ipv4())
                        .ok_or(anyhow!("DNS Lookup Fails!"))?,
                    preshared_key: match &p.preshared_key {
                        None => Ok(None),
                        Some(k) => parse_key(k).map(Some),
                    }?,
                    allowed_ips: p
                        .allowed_ips
                        .iter()
                        .map(|ip_addr| {
                            IpNetwork::from_str(ip_addr)
                                .map_err(|e| anyhow!("Error parsing IP Network {}: {}", ip_addr, e))
                        })
                        .collect::<Result<Vec<IpNetwork>>>()?,
                })
            })
            .collect::<Result<Vec<WgPeer>>>()?;
        WgInterface::new(wg_peers)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interface: Interface {
                private_key: "OEPVdomeLTxTIBvv3TYsJRge0Hp9NMiY0sIrhT8OWG8=".into(),
                address: vec!["10.13.13.2/24".into()],
                listen_port: 51820,
                dns: Default::default(),
                mtu: Default::default(),
            },
            peers: vec![Peer {
                endpoint: "wg.burrow.rs:51820".into(),
                allowed_ips: vec!["8.8.8.8/32".into(), "0.0.0.0/0".into()],
                public_key: "8GaFjVO6c4luCHG4ONO+1bFG8tO+Zz5/Gy+Geht1USM=".into(),
                preshared_key: Some("ha7j4BjD49sIzyF9SNlbueK0AMHghlj6+u0G3bzC698=".into()),
                persistent_keepalive: Default::default(),
                name: Default::default(),
            }],
        }
    }
}
