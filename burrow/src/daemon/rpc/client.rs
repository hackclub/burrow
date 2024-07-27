use anyhow::Result;
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

use super::grpc_defs::{networks_client::NetworksClient, tunnel_client::TunnelClient};
use crate::daemon::get_socket_path;

pub struct BurrowClient<T> {
    pub networks_client: NetworksClient<T>,
    pub tunnel_client: TunnelClient<T>,
}

impl BurrowClient<tonic::transport::Channel> {
    #[cfg(any(target_os = "linux", target_vendor = "apple"))]
    pub async fn from_uds() -> Result<Self> {
        let channel = Endpoint::try_from("http://[::]:50051")? // NOTE: this is a hack(?)
            .connect_with_connector(service_fn(|_: Uri| async {
                let sock_path = get_socket_path();
                Ok::<_, std::io::Error>(TokioIo::new(UnixStream::connect(sock_path).await?))
            }))
            .await?;
        let nw_client = NetworksClient::new(channel.clone());
        let tun_client = TunnelClient::new(channel.clone());
        Ok(BurrowClient {
            networks_client: nw_client,
            tunnel_client: tun_client,
        })
    }
}
