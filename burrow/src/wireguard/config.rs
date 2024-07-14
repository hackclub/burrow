use std::{net::ToSocketAddrs, str::FromStr};

use anyhow::{anyhow, Error, Result};
use base64::{engine::general_purpose, Engine};
use fehler::throws;
use ini::{Ini, Properties};
use ip_network::IpNetwork;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

use super::inifield::IniField;
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
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Peer {
    pub public_key: String,
    pub preshared_key: Option<String>,
    pub allowed_ips: Vec<String>,
    pub endpoint: String,
    pub persistent_keepalive: Option<u32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    pub private_key: String,
    pub address: Vec<String>,
    pub listen_port: Option<u32>,
    pub dns: Vec<String>,
    pub mtu: Option<u32>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "Peer")]
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
                listen_port: Some(51820),
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

fn props_get<T>(props: &Properties, key: &str) -> Result<T>
where
    T: TryFrom<IniField, Error = anyhow::Error>,
{
    IniField::try_from(props.get(key))?.try_into()
}

impl TryFrom<&Properties> for Interface {
    type Error = anyhow::Error;

    fn try_from(props: &Properties) -> Result<Self, Error> {
        Ok(Self {
            private_key: props_get(props, "PrivateKey")?,
            address: props_get(props, "Address")?,
            listen_port: props_get(props, "ListenPort")?,
            dns: props_get(props, "DNS")?,
            mtu: props_get(props, "MTU")?,
        })
    }
}

impl TryFrom<&Properties> for Peer {
    type Error = anyhow::Error;

    fn try_from(props: &Properties) -> Result<Self, Error> {
        Ok(Self {
            public_key: props_get(props, "PublicKey")?,
            preshared_key: props_get(props, "PresharedKey")?,
            allowed_ips: props_get(props, "AllowedIPs")?,
            endpoint: props_get(props, "Endpoint")?,
            persistent_keepalive: props_get(props, "PersistentKeepalive")?,
            name: props_get(props, "Name")?,
        })
    }
}

impl Config {
    pub fn from_toml(toml: &str) -> Result<Self> {
        toml::from_str(toml).map_err(Into::into)
    }

    pub fn from_ini(ini: &str) -> Result<Self> {
        let ini = Ini::load_from_str(ini)?;
        let interface = ini
            .section(Some("Interface"))
            .ok_or(anyhow!("Interface section not found"))?;
        let peers = ini.section_all(Some("Peer"));
        Ok(Self {
            interface: Interface::try_from(interface)?,
            peers: peers
                .into_iter()
                .map(|v| Peer::try_from(v))
                .collect::<Result<Vec<Peer>>>()?,
        })
    }

    pub fn from_content_fmt(content: &str, fmt: &str) -> Result<Self> {
        match fmt {
            "toml" => Self::from_toml(content),
            "ini" | "conf" => Self::from_ini(content),
            _ => Err(anyhow::anyhow!("Unsupported format: {}", fmt)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tst_config_toml() {
        let cfig = Config::default();
        let toml = toml::to_string(&cfig).unwrap();
        println!("{}", &toml);
        insta::assert_snapshot!(toml);
        let cfig2: Config = toml::from_str(&toml).unwrap();
        assert_eq!(cfig, cfig2);
    }
}
