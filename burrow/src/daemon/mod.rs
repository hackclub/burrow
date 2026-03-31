use std::{path::Path, sync::Arc};

pub mod apple;
mod instance;
mod net;
pub mod rpc;
mod runtime;

use anyhow::{Error as AhError, Result};
use instance::DaemonRPCServer;
pub use net::{get_socket_path, DaemonClient};
pub use rpc::{DaemonCommand, DaemonResponseData, DaemonStartOptions};
use tokio::{net::UnixListener, sync::Notify};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tracing::info;

use crate::{
    daemon::rpc::grpc_defs::{networks_server::NetworksServer, tunnel_server::TunnelServer},
    database::get_connection,
};

pub async fn daemon_main(
    socket_path: Option<&Path>,
    db_path: Option<&Path>,
    notify_ready: Option<Arc<Notify>>,
) -> Result<()> {
    let _conn = get_connection(db_path)?;
    let burrow_server = DaemonRPCServer::new(db_path)?;
    let spp = socket_path.clone();
    let tmp = get_socket_path();
    let sock_path = spp.unwrap_or(Path::new(tmp.as_str()));
    if sock_path.exists() {
        std::fs::remove_file(sock_path)?;
    }
    let uds = UnixListener::bind(sock_path)?;
    let serve_job = tokio::spawn(async move {
        let uds_stream = UnixListenerStream::new(uds);
        let _srv = Server::builder()
            .add_service(TunnelServer::new(burrow_server.clone()))
            .add_service(NetworksServer::new(burrow_server))
            .serve_with_incoming(uds_stream)
            .await?;
        Ok::<(), AhError>(())
    });

    if let Some(n) = notify_ready {
        n.notify_one();
    }

    info!("Starting daemon...");

    tokio::try_join!(serve_job)
        .map(|_| ())
        .map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use anyhow::{anyhow, Result};
    use tokio::time::{timeout, Duration};

    use super::*;
    use crate::daemon::rpc::{
        client::BurrowClient,
        grpc_defs::{
            Empty, Network, NetworkListResponse, NetworkReorderRequest, NetworkType,
            TunnelConfigurationResponse, TunnelStatusResponse,
        },
    };

    #[tokio::test]
    async fn daemon_tracks_network_priority_via_grpc() -> Result<()> {
        let socket_path = temp_path("sock");
        let db_path = temp_path("sqlite3");
        let ready = Arc::new(Notify::new());

        let daemon_ready = ready.clone();
        let daemon_socket_path = socket_path.clone();
        let daemon_db_path = db_path.clone();
        let daemon_task = tokio::spawn(async move {
            daemon_main(
                Some(daemon_socket_path.as_path()),
                Some(daemon_db_path.as_path()),
                Some(daemon_ready),
            )
            .await
        });

        timeout(Duration::from_secs(5), ready.notified()).await?;

        let mut client = timeout(
            Duration::from_secs(5),
            BurrowClient::from_uds_path(&socket_path),
        )
        .await??;
        let mut config_stream = client
            .tunnel_client
            .tunnel_configuration(Empty {})
            .await?
            .into_inner();
        let mut network_stream = client
            .networks_client
            .network_list(Empty {})
            .await?
            .into_inner();
        let mut status_stream = client
            .tunnel_client
            .tunnel_status(Empty {})
            .await?
            .into_inner();

        let initial_config = next_configuration(&mut config_stream).await?;
        assert!(initial_config.addresses.is_empty());
        assert_eq!(initial_config.mtu, 1500);

        let initial_networks = next_networks(&mut network_stream).await?;
        assert!(initial_networks.network.is_empty());

        let initial_status = next_status(&mut status_stream).await?;
        assert_eq!(
            initial_status.state(),
            crate::daemon::rpc::grpc_defs::State::Stopped
        );

        client.tunnel_client.tunnel_start(Empty {}).await?;

        let passthrough_status = next_status(&mut status_stream).await?;
        assert_eq!(
            passthrough_status.state(),
            crate::daemon::rpc::grpc_defs::State::Running
        );

        client.tunnel_client.tunnel_stop(Empty {}).await?;

        let stopped_status = next_status(&mut status_stream).await?;
        assert_eq!(
            stopped_status.state(),
            crate::daemon::rpc::grpc_defs::State::Stopped
        );

        client
            .networks_client
            .network_add(Network {
                id: 1,
                r#type: NetworkType::WireGuard.into(),
                payload: sample_wireguard_payload(),
            })
            .await?;

        let networks_after_wg = next_networks(&mut network_stream).await?;
        assert_eq!(
            network_ids(&networks_after_wg),
            vec![(1, NetworkType::WireGuard)]
        );

        let wireguard_config = next_configuration(&mut config_stream).await?;
        assert_eq!(
            wireguard_config.addresses,
            vec!["10.8.0.2/32", "fd00::2/128"]
        );
        assert_eq!(wireguard_config.mtu, 1420);

        client
            .networks_client
            .network_add(Network {
                id: 2,
                r#type: NetworkType::WireGuard.into(),
                payload: sample_wireguard_payload_with("10.77.0.2/32", 1380),
            })
            .await?;

        let networks_after_second_add = next_networks(&mut network_stream).await?;
        assert_eq!(
            network_ids(&networks_after_second_add),
            vec![(1, NetworkType::WireGuard), (2, NetworkType::WireGuard)]
        );

        let still_wireguard = next_configuration(&mut config_stream).await?;
        assert_eq!(still_wireguard.addresses, wireguard_config.addresses);

        client
            .networks_client
            .network_reorder(NetworkReorderRequest { id: 2, index: 0 })
            .await?;

        let networks_after_reorder = next_networks(&mut network_stream).await?;
        assert_eq!(
            network_ids(&networks_after_reorder),
            vec![(2, NetworkType::WireGuard), (1, NetworkType::WireGuard)]
        );

        let second_wireguard_config = next_configuration(&mut config_stream).await?;
        assert_eq!(second_wireguard_config.addresses, vec!["10.77.0.2/32"]);
        assert_eq!(second_wireguard_config.mtu, 1380);

        daemon_task.abort();
        let _ = daemon_task.await;
        cleanup_path(&socket_path);
        cleanup_path(&db_path);

        Ok(())
    }

    fn temp_path(ext: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("burrow-daemon-test-{now}.{ext}"))
    }

    fn cleanup_path(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    fn sample_wireguard_payload() -> Vec<u8> {
        br#"[Interface]
PrivateKey = OEPVdomeLTxTIBvv3TYsJRge0Hp9NMiY0sIrhT8OWG8=
Address = 10.8.0.2/32, fd00::2/128
ListenPort = 51820
MTU = 1420

[Peer]
PublicKey = 8GaFjVO6c4luCHG4ONO+1bFG8tO+Zz5/Gy+Geht1USM=
PresharedKey = ha7j4BjD49sIzyF9SNlbueK0AMHghlj6+u0G3bzC698=
AllowedIPs = 0.0.0.0/0, ::/0
Endpoint = wg.burrow.rs:51820
"#
        .to_vec()
    }

    fn sample_wireguard_payload_with(address: &str, mtu: u16) -> Vec<u8> {
        format!(
            "[Interface]\nPrivateKey = OEPVdomeLTxTIBvv3TYsJRge0Hp9NMiY0sIrhT8OWG8=\nAddress = {address}\nListenPort = 51820\nMTU = {mtu}\n\n[Peer]\nPublicKey = 8GaFjVO6c4luCHG4ONO+1bFG8tO+Zz5/Gy+Geht1USM=\nPresharedKey = ha7j4BjD49sIzyF9SNlbueK0AMHghlj6+u0G3bzC698=\nAllowedIPs = 0.0.0.0/0, ::/0\nEndpoint = wg.burrow.rs:51820\n"
        )
        .into_bytes()
    }

    async fn next_configuration(
        stream: &mut tonic::Streaming<TunnelConfigurationResponse>,
    ) -> Result<TunnelConfigurationResponse> {
        timeout(Duration::from_secs(5), stream.message())
            .await??
            .ok_or_else(|| anyhow!("configuration stream ended unexpectedly"))
    }

    async fn next_networks(
        stream: &mut tonic::Streaming<NetworkListResponse>,
    ) -> Result<NetworkListResponse> {
        timeout(Duration::from_secs(5), stream.message())
            .await??
            .ok_or_else(|| anyhow!("network stream ended unexpectedly"))
    }

    async fn next_status(
        stream: &mut tonic::Streaming<TunnelStatusResponse>,
    ) -> Result<TunnelStatusResponse> {
        timeout(Duration::from_secs(5), stream.message())
            .await??
            .ok_or_else(|| anyhow!("status stream ended unexpectedly"))
    }

    fn network_ids(response: &NetworkListResponse) -> Vec<(i32, NetworkType)> {
        response
            .network
            .iter()
            .map(|network| (network.id, network.r#type()))
            .collect()
    }
}
