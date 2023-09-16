use std::net::SocketAddr;

use anyhow::Error;
use fehler::throws;
use ip_network::IpNetwork;
use tokio::{net::UdpSocket, task::JoinHandle};

use super::{
    iface::PacketInterface,
    noise::{TunnResult, Tunnel},
    Peer,
};

#[derive(Debug)]
pub struct PeerPcb {
    pub endpoint: SocketAddr,
    pub allowed_ips: Vec<IpNetwork>,
    pub handle: Option<JoinHandle<()>>,
    socket: Option<UdpSocket>,
    tunnel: Tunnel,
}

impl PeerPcb {
    #[throws]
    pub fn new(peer: Peer) -> Self {
        let tunnel = Tunnel::new(peer.private_key, peer.public_key, None, None, 1, None)
            .map_err(|s| anyhow::anyhow!("{}", s))?;

        Self {
            endpoint: peer.endpoint,
            allowed_ips: peer.allowed_ips,
            handle: None,
            socket: None,
            tunnel,
        }
    }

    async fn open_if_closed(&mut self) -> Result<(), Error> {
        if self.socket.is_none() {
            let socket = UdpSocket::bind("0.0.0.0:0").await?;
            socket.connect(self.endpoint).await?;
            self.socket = Some(socket);
        }
        Ok(())
    }

    pub async fn run(&self, interface: Box<&dyn PacketInterface>) -> Result<(), Error> {
        let mut buf = [0u8; 3000];
        loop {
            let Some(socket) = self.socket.as_ref() else {
                continue
            };

            let packet = match socket.recv(&mut buf).await {
                Ok(s) => &buf[..s],
                Err(e) => {
                    tracing::error!("eror receiving on peer socket: {}", e);
                    continue
                }
            };

            let (len, addr) = socket.recv_from(&mut buf).await?;

            tracing::debug!("received {} bytes from {}", len, addr);
        }
    }

    pub async fn socket(&mut self) -> Result<&UdpSocket, Error> {
        self.open_if_closed().await?;
        Ok(self.socket.as_ref().expect("socket was just opened"))
    }

    pub async fn send(&mut self, src: &[u8]) -> Result<(), Error> {
        let mut dst_buf = [0u8; 3000];
        match self.tunnel.encapsulate(src, &mut dst_buf[..]) {
            TunnResult::Done => {}
            TunnResult::Err(e) => {
                tracing::error!(message = "Encapsulate error", error = ?e)
            }
            TunnResult::WriteToNetwork(packet) => {
                let socket = self.socket().await?;
                socket.send(packet).await?;
            }
            _ => panic!("Unexpected result from encapsulate"),
        };
        Ok(())
    }
}
