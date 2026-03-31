use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use anyhow::{Context, Result};
use arti_client::TorClient;
use hickory_proto::{
    op::{Message, MessageType, ResponseCode},
    rr::{rdata::A, rdata::AAAA, RData, Record, RecordType},
};
use tokio::{net::UdpSocket, sync::watch, task::JoinError};
use tor_rtcompat::PreferredRuntime;
use tracing::{debug, warn};

const DNS_TTL_SECS: u32 = 60;

#[derive(Debug)]
pub struct TorDnsHandle {
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl TorDnsHandle {
    pub async fn shutdown(self) -> Result<()> {
        let _ = self.shutdown.send(true);
        match self.task.await {
            Ok(()) => Ok(()),
            Err(err) if err.is_cancelled() => Ok(()),
            Err(err) => Err(join_error(err)),
        }
    }
}

pub async fn spawn(
    bind_addr: SocketAddr,
    tor_client: Arc<TorClient<PreferredRuntime>>,
) -> Result<TorDnsHandle> {
    let socket = UdpSocket::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind tor dns proxy on {bind_addr}"))?;
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(async move {
        let mut buffer = [0u8; 4096];
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    match changed {
                        Ok(()) if *shutdown_rx.borrow() => break,
                        Ok(()) => continue,
                        Err(_) => break,
                    }
                }
                received = socket.recv_from(&mut buffer) => {
                    let (len, peer_addr) = match received {
                        Ok(value) => value,
                        Err(err) => {
                            warn!(?err, "tor dns proxy recv failed");
                            continue;
                        }
                    };

                    let response = match build_response(&buffer[..len], tor_client.as_ref()).await {
                        Ok(message) => message,
                        Err(err) => {
                            debug!(?err, "tor dns proxy failed to answer query");
                            continue;
                        }
                    };

                    if let Err(err) = socket.send_to(&response, peer_addr).await {
                        warn!(?err, "tor dns proxy send failed");
                    }
                }
            }
        }
    });

    Ok(TorDnsHandle {
        shutdown: shutdown_tx,
        task,
    })
}

async fn build_response(
    packet: &[u8],
    tor_client: &TorClient<PreferredRuntime>,
) -> Result<Vec<u8>> {
    let request = Message::from_vec(packet).context("failed to parse dns packet")?;
    let mut response = Message::new();
    response
        .set_id(request.id())
        .set_op_code(request.op_code())
        .set_message_type(MessageType::Response)
        .set_recursion_desired(request.recursion_desired())
        .set_recursion_available(true)
        .set_response_code(ResponseCode::NoError);

    for query in request.queries().iter().cloned() {
        response.add_query(query.clone());
        match query.query_type() {
            RecordType::A | RecordType::AAAA => {
                let hostname = query.name().to_utf8();
                let hostname = hostname.trim_end_matches('.');
                match tor_client.resolve(hostname).await {
                    Ok(addrs) => {
                        for addr in addrs {
                            if let Some(answer) =
                                record_for_address(query.name().clone(), query.query_type(), addr)
                            {
                                response.add_answer(answer);
                            }
                        }
                    }
                    Err(err) => {
                        debug!(hostname, ?err, "tor dns lookup failed");
                        response.set_response_code(ResponseCode::ServFail);
                    }
                }
            }
            _ => {
                response.set_response_code(ResponseCode::NotImp);
            }
        }
    }

    response.to_vec().context("failed to encode dns response")
}

fn record_for_address(
    name: hickory_proto::rr::Name,
    record_type: RecordType,
    addr: IpAddr,
) -> Option<Record> {
    match (record_type, addr) {
        (RecordType::A, IpAddr::V4(ip)) => {
            Some(Record::from_rdata(name, DNS_TTL_SECS, RData::A(A::from(ip))))
        }
        (RecordType::AAAA, IpAddr::V6(ip)) => Some(Record::from_rdata(
            name,
            DNS_TTL_SECS,
            RData::AAAA(AAAA::from(ip)),
        )),
        _ => None,
    }
}

fn join_error(err: JoinError) -> anyhow::Error {
    anyhow::anyhow!("tor dns task failed: {err}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::rr::Name;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn builds_a_record_for_ipv4_answer() {
        let record = record_for_address(
            Name::from_ascii("example.com.").unwrap(),
            RecordType::A,
            IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
        )
        .unwrap();
        assert_eq!(record.record_type(), RecordType::A);
    }

    #[test]
    fn skips_mismatched_record_type() {
        let record = record_for_address(
            Name::from_ascii("example.com.").unwrap(),
            RecordType::A,
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        );
        assert!(record.is_none());
    }
}
