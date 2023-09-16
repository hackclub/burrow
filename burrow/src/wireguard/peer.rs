use std::{fmt, net::SocketAddr};

use anyhow::Error;
use fehler::throws;
use ip_network::IpNetwork;
use x25519_dalek::{PublicKey, StaticSecret};

pub struct Peer {
    pub endpoint: SocketAddr,
    pub private_key: StaticSecret,
    pub public_key: PublicKey,
    pub allowed_ips: Vec<IpNetwork>,
}

impl fmt::Debug for Peer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Peer")
            .field("endpoint", &self.endpoint)
            .field("public_key", &self.public_key)
            .field("allowed_ips", &self.allowed_ips)
            .finish()
    }
}
