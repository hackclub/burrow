use anyhow::Result;
use hyper_util::rt::TokioIo;
use std::path::Path;
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
        Self::from_uds_path(get_socket_path()).await
    }

    #[cfg(any(target_os = "linux", target_vendor = "apple"))]
    pub async fn from_uds_path(path: impl AsRef<Path>) -> Result<Self> {
        let socket_path = path.as_ref().to_owned();
        let channel = Endpoint::try_from("http://[::]:50051")? // NOTE: this is a hack(?)
            .connect_with_connector(service_fn(move |_: Uri| {
                let socket_path = socket_path.clone();
                async move {
                    Ok::<_, std::io::Error>(TokioIo::new(UnixStream::connect(&socket_path).await?))
                }
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
