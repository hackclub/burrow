use std::{net::IpAddr, ops::Deref, sync::Arc};

use anyhow::Error;
use fehler::throws;
use futures::future::join_all;
use ip_network_table::IpNetworkTable;
use tokio::sync::{Notify, RwLock};
use tracing::{debug, error};
use tun::tokio::TunInterface;

use super::{noise::Tunnel, Peer, PeerPcb};

pub struct IndexedPcbs {
    pcbs: Vec<Arc<PeerPcb>>,
    allowed_ips: IpNetworkTable<usize>,
}

impl Default for IndexedPcbs {
    fn default() -> Self {
        Self::new()
    }
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
            self.allowed_ips.insert(*allowed_ip, idx);
        }
        self.pcbs.insert(idx, Arc::new(pcb));
    }

    pub fn find(&self, addr: IpAddr) -> Option<usize> {
        let (_, &idx) = self.allowed_ips.longest_match(addr)?;
        Some(idx)
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

enum IfaceStatus {
    Running,
    Idle,
}

pub struct Interface {
    pub tun: Arc<RwLock<Option<TunInterface>>>,
    pub pcbs: Arc<IndexedPcbs>,
    status: Arc<RwLock<IfaceStatus>>,
    stop_notifier: Arc<Notify>,
}

async fn is_running(status: Arc<RwLock<IfaceStatus>>) -> bool {
    let st = status.read().await;
    matches!(st.deref(), IfaceStatus::Running)
}

impl Interface {
    #[throws]
    pub fn new<I: IntoIterator<Item = Peer>>(peers: I) -> Self {
        let pcbs: IndexedPcbs = peers
            .into_iter()
            .map(PeerPcb::new)
            .collect::<Result<_, _>>()?;

        let pcbs = Arc::new(pcbs);
        Self {
            pcbs,
            tun: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(IfaceStatus::Idle)),
            stop_notifier: Arc::new(Notify::new()),
        }
    }

    pub async fn set_tun(&self, tun: TunInterface) {
        debug!("Setting tun interface");
        self.tun.write().await.replace(tun);
        let mut st = self.status.write().await;
        *st = IfaceStatus::Running;
    }

    pub fn get_tun(&self) -> Arc<RwLock<Option<TunInterface>>> {
        self.tun.clone()
    }

    pub async fn remove_tun(&self) {
        let mut st = self.status.write().await;
        self.stop_notifier.notify_waiters();
        *st = IfaceStatus::Idle;
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let pcbs = self.pcbs.clone();
        let tun = self.tun.clone();
        let status = self.status.clone();
        let stop_notifier = self.stop_notifier.clone();
        log::info!("Starting interface");

        let outgoing = async move {
            while is_running(status.clone()).await {
                let mut buf = [0u8; 3000];

                let src = {
                    let t = tun.read().await;
                    let Some(_tun) = t.as_ref() else {
                        continue;
                    };
                    tokio::select! {
                        _ = stop_notifier.notified() => continue,
                        pkg = _tun.recv(&mut buf[..]) => match pkg {
                            Ok(len) => &buf[..len],
                            Err(e) => {
                                error!("Failed to read from interface: {}", e);
                                continue
                            }
                        },
                    }
                };

                let dst_addr = match Tunnel::dst_address(src) {
                    Some(addr) => addr,
                    None => {
                        debug!("No destination found");
                        continue
                    }
                };

                debug!("Routing packet to {}", dst_addr);

                let Some(idx) = pcbs.find(dst_addr) else {
                    continue
                };

                debug!("Found peer:{}", idx);

                match pcbs.pcbs[idx].send(src).await {
                    Ok(..) => {
                        let addr = pcbs.pcbs[idx].endpoint;
                        debug!("Sent packet to peer {}", addr);
                    }
                    Err(e) => {
                        log::error!("Failed to send packet {}", e);
                        continue
                    }
                };
            }
        };

        let mut tsks = vec![];
        let tun = self.tun.clone();
        let outgoing = tokio::task::spawn(outgoing);
        tsks.push(outgoing);
        debug!("preparing to spawn read tasks");

        {
            let pcbs = &self.pcbs;
            for i in 0..pcbs.pcbs.len() {
                debug!("spawning read task for peer {}", i);
                let pcb = pcbs.pcbs[i].clone();
                let tun = tun.clone();
                let main_tsk = async move {
                    if let Err(e) = pcb.open_if_closed().await {
                        log::error!("failed to open pcb: {}", e);
                        return
                    }
                    let r2 = pcb.run(tun).await;
                    if let Err(e) = r2 {
                        log::error!("failed to run pcb: {}", e);
                    } else {
                        debug!("pcb ran successfully");
                    }
                };

                let pcb = pcbs.pcbs[i].clone();
                let status = self.status.clone();
                let update_timers_tsk = async move {
                    let mut buf = [0u8; 65535];
                    while is_running(status.clone()).await {
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                        match pcb.update_timers(&mut buf).await {
                            Ok(..) => (),
                            Err(e) => {
                                error!("Failed to update timers: {}", e);
                                return
                            }
                        }
                    }
                };

                let pcb = pcbs.pcbs[i].clone();
                let status = self.status.clone();
                let reset_rate_limiter_tsk = async move {
                    while is_running(status.clone()).await {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        pcb.reset_rate_limiter().await;
                    }
                };
                tsks.extend(vec![
                    tokio::spawn(main_tsk),
                    tokio::spawn(update_timers_tsk),
                    tokio::spawn(reset_rate_limiter_tsk),
                ]);
                debug!("task made..");
            }
            debug!("spawned read tasks");
        }
        debug!("preparing to join..");
        join_all(tsks).await;
        debug!("joined!");
        Ok(())
    }
}
