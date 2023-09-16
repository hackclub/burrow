mod iface;
mod noise;
mod pcb;
mod peer;

pub use iface::Interface;
pub use pcb::PeerPcb;
pub use peer::Peer;
pub use x25519_dalek::{PublicKey, StaticSecret};

const WIREGUARD_CONFIG: &str = r#"
[Interface]
# Device: Gentle Tomcat
PrivateKey = sIxpokQPnWctJKNaQ3DRdcQbL2S5OMbUrvr4bbsvTHw=
Address = 10.68.136.199/32,fc00:bbbb:bbbb:bb01::5:88c6/128
DNS = 10.64.0.1

[Peer]
PublicKey = EKZXvHlSDeqAjfC/m9aQR0oXfQ6Idgffa9L0DH5yaCo=
AllowedIPs = 0.0.0.0/0,::0/0
Endpoint = 146.70.173.66:51820
"#;
