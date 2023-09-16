use std::{net::IpAddr, rc::Rc};

use anyhow::Error;
use async_trait::async_trait;
use fehler::throws;
use ip_network_table::IpNetworkTable;
use tokio::{
    join,
    sync::Mutex,
    task::{self, JoinHandle},
};
use tun::tokio::TunInterface;

use super::{noise::Tunnel, pcb, Peer, PeerPcb};

#[async_trait]
pub trait PacketInterface {
    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, tokio::io::Error>;
    async fn send(&mut self, buf: &[u8]) -> Result<usize, tokio::io::Error>;
}

#[async_trait]
impl PacketInterface for tun::tokio::TunInterface {
    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, tokio::io::Error> {
        self.recv(buf).await
    }

    async fn send(&mut self, buf: &[u8]) -> Result<usize, tokio::io::Error> {
        self.send(buf).await
    }
}

struct IndexedPcbs {
    pcbs: Vec<PeerPcb>,
    allowed_ips: IpNetworkTable<usize>,
}

impl IndexedPcbs {
    pub fn new() -> Self {
        Self {
            pcbs: vec![],
            allowed_ips: IpNetworkTable::new(),
        }
    }

    pub fn insert(&mut self, pcb: PeerPcb) {
        let idx: usize = self.pcbs.len();
        for allowed_ip in pcb.allowed_ips.iter() {
            self.allowed_ips.insert(allowed_ip.clone(), idx);
        }
        self.pcbs.insert(idx, pcb);
    }

    pub fn find(&mut self, addr: IpAddr) -> Option<usize> {
        let (_, &idx) = self.allowed_ips.longest_match(addr)?;
        Some(idx)
    }

    pub fn connect(&mut self, idx: usize, handle: JoinHandle<()>) {
        self.pcbs[idx].handle = Some(handle);
    }
}

impl FromIterator<PeerPcb> for IndexedPcbs {
    fn from_iter<I: IntoIterator<Item = PeerPcb>>(iter: I) -> Self {
        iter.into_iter().fold(Self::new(), |mut acc, pcb| {
            acc.insert(pcb);
            acc
        })
    }
}

pub struct Interface {
    tun: Rc<Mutex<TunInterface>>,
    pcbs: Rc<Mutex<IndexedPcbs>>,
}

impl Interface {
    #[throws]
    pub fn new<I: IntoIterator<Item = Peer>>(tun: TunInterface, peers: I) -> Self {
        let pcbs: IndexedPcbs = peers
            .into_iter()
            .map(|peer| PeerPcb::new(peer))
            .collect::<Result<_, _>>()?;

        let tun = Rc::new(Mutex::new(tun));
        let pcbs = Rc::new(Mutex::new(pcbs));
        Self { tun, pcbs }
    }

    pub async fn run(self) {
        let pcbs = self.pcbs;
        let tun = self.tun;

        let outgoing = async move {
            loop {
                let mut buf = [0u8; 3000];

                let mut tun = tun.lock().await;
                let src = match tun.recv(&mut buf[..]).await {
                    Ok(len) => &buf[..len],
                    Err(e) => {
                        log::error!("failed reading from interface: {}", e);
                        continue
                    }
                };

                let mut pcbs = pcbs.lock().await;

                let dst_addr = match Tunnel::dst_address(src) {
                    Some(addr) => addr,
                    None => continue,
                };

                let Some(idx) = pcbs.find(dst_addr) else {
                    continue
                };
                match pcbs.pcbs[idx].send(src).await {
                    Ok(..) => {}
                    Err(e) => log::error!("failed to send packet {}", e),
                }
            }
        };

        task::LocalSet::new()
            .run_until(async move {
                let outgoing = task::spawn_local(outgoing);
                join!(outgoing);
            })
            .await;
    }
}
