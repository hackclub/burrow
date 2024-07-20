use anyhow::Result;
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

const BURROW_RPC_SOCKET_PATH: &str = "/run/burrow.sock";

pub async fn daemon_connect() -> Result<Channel> {
    Ok(Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| async {
            Ok::<_, std::io::Error>(TokioIo::new(
                UnixStream::connect(BURROW_RPC_SOCKET_PATH).await?,
            ))
        }))
        .await?)
}
