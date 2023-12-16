use std::{net::SocketAddr, sync::Arc};

use anyhow::Error;
use fehler::throws;
use ip_network::IpNetwork;
use rand::random;
use tokio::{net::UdpSocket, sync::RwLock, task::JoinHandle};
use tun::tokio::TunInterface;

use super::{
    noise::{TunnResult, Tunnel},
    Peer,
};

#[derive(Debug)]
pub struct PeerPcb {
    pub endpoint: SocketAddr,
    pub allowed_ips: Vec<IpNetwork>,
    pub handle: RwLock<Option<JoinHandle<()>>>,
    socket: RwLock<Option<UdpSocket>>,
    tunnel: RwLock<Tunnel>,
}

impl PeerPcb {
    #[throws]
    pub fn new(peer: Peer) -> Self {
        let tunnel = RwLock::new(
            Tunnel::new(
                peer.private_key,
                peer.public_key,
                peer.preshared_key,
                None,
                1,
                None,
            )
            .map_err(|s| anyhow::anyhow!("{}", s))?,
        );
        Self {
            endpoint: peer.endpoint,
            allowed_ips: peer.allowed_ips,
            handle: RwLock::new(None),
            socket: RwLock::new(None),
            tunnel,
        }
    }

    pub async fn open_if_closed(&self) -> Result<(), Error> {
        if self.socket.read().await.is_none() {
            let socket = UdpSocket::bind("0.0.0.0:0").await?;
            socket.connect(self.endpoint).await?;
            self.socket.write().await.replace(socket);
        }
        Ok(())
    }

    pub async fn run(&self, tun_interface: Arc<RwLock<TunInterface>>) -> Result<(), Error> {
        tracing::debug!("starting read loop for pcb... for {:?}", &self);
        let rid: i32 = random();
        let mut buf: [u8; 3000] = [0u8; 3000];
        tracing::debug!("start read loop {}", rid);
        loop {
            tracing::debug!("{}: waiting for packet", rid);
            let guard = self.socket.read().await;
            let Some(socket) = guard.as_ref() else {
                continue
            };
            let mut res_buf = [0; 1500];
            // tracing::debug!("{} : waiting for readability on {:?}", rid, socket);
            let len = match socket.recv(&mut res_buf).await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("{}: error reading from socket: {:?}", rid, e);
                    continue
                }
            };
            let mut res_dat = &res_buf[..len];
            tracing::debug!("{}: Decapsulating {} bytes", rid, len);
            tracing::debug!("{:?}", &res_dat);
            loop {
                match self
                    .tunnel
                    .write()
                    .await
                    .decapsulate(None, res_dat, &mut buf[..])
                {
                    TunnResult::Done => break,
                    TunnResult::Err(e) => {
                        tracing::error!(message = "Decapsulate error", error = ?e);
                        break
                    }
                    TunnResult::WriteToNetwork(packet) => {
                        tracing::debug!("WriteToNetwork: {:?}", packet);
                        self.open_if_closed().await?;
                        socket.send(packet).await?;
                        tracing::debug!("WriteToNetwork done");
                        res_dat = &[];
                        continue
                    }
                    TunnResult::WriteToTunnelV4(packet, addr) => {
                        tracing::debug!("WriteToTunnelV4: {:?}, {:?}", packet, addr);
                        tun_interface.read().await.send(packet).await?;
                        break
                    }
                    TunnResult::WriteToTunnelV6(packet, addr) => {
                        tracing::debug!("WriteToTunnelV6: {:?}, {:?}", packet, addr);
                        tun_interface.read().await.send(packet).await?;
                        break
                    }
                }
            }
        }
    }

    pub async fn send(&self, src: &[u8]) -> Result<(), Error> {
        let mut dst_buf = [0u8; 3000];
        match self.tunnel.write().await.encapsulate(src, &mut dst_buf[..]) {
            TunnResult::Done => {}
            TunnResult::Err(e) => {
                tracing::error!(message = "Encapsulate error", error = ?e)
            }
            TunnResult::WriteToNetwork(packet) => {
                self.open_if_closed().await?;
                let handle = self.socket.read().await;
                let Some(socket) = handle.as_ref() else {
                    tracing::error!("No socket for peer");
                    return Ok(())
                };
                tracing::debug!("Our Encapsulated packet: {:?}", packet);
                socket.send(packet).await?;
            }
            _ => panic!("Unexpected result from encapsulate"),
        };
        Ok(())
    }
}
