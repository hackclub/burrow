use std::net::SocketAddr;

use tokio::sync::mpsc;

mod command;
mod instance;
mod net;

use anyhow::Error;
use base64::{engine::general_purpose, Engine as _};
use burrow::wireguard::{Interface, Peer, PublicKey, StaticSecret};
pub use command::{DaemonCommand, DaemonStartOptions};
use fehler::throws;
use instance::DaemonInstance;
pub use net::DaemonClient;

#[throws]
fn parse_secret_key(string: &str) -> StaticSecret {
    let value = general_purpose::STANDARD.decode(string)?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&value[..]);
    StaticSecret::from(key)
}

#[throws]
fn parse_public_key(string: &str) -> PublicKey {
    let value = general_purpose::STANDARD.decode(string)?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&value[..]);
    PublicKey::from(key)
}

pub async fn daemon_main() -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel(2);
    let mut inst = DaemonInstance::new(rx);
    // tokio::try_join!(inst.run(), listen(tx)).map(|_| ())

    let tun = tun::tokio::TunInterface::new(tun::TunInterface::new()?)?;

    let private_key = parse_secret_key("sIxpokQPnWctJKNaQ3DRdcQbL2S5OMbUrvr4bbsvTHw=")?;
    let public_key = parse_public_key("EKZXvHlSDeqAjfC/m9aQR0oXfQ6Idgffa9L0DH5yaCo=")?;
    let endpoint = "146.70.173.66:51820".parse::<SocketAddr>()?;
    let iface = Interface::new(tun, vec![Peer {
        endpoint,
        private_key,
        public_key,
        allowed_ips: vec![],
    }])?;

    iface.run().await;
    Ok(())
}
