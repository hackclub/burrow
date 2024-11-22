use std::sync::Arc;

use jwt_simple::prelude::Ed25519KeyPair;
use tonic::{Request, Response, Status};

use super::providers::{KeypairT, OpenIdUser};

use super::{
    grpc_defs::{
        burrowwebrpc::burrow_web_server::BurrowWeb, CreateDeviceRequest, CreateDeviceResponse,
        Empty, JwtInfo, ListDevicesResponse, SlackAuthRequest,
    },
    providers::slack::auth,
    settings::BurrowAuthServerConfig,
};

struct BurrowGrpcServer {
    config: Arc<BurrowAuthServerConfig>,
    jwt_keypair: Arc<KeypairT>,
}

impl BurrowGrpcServer {
    pub fn new() -> anyhow::Result<Self> {
        let config = BurrowAuthServerConfig::new_dotenv()?;
        let jwt_keypair = Ed25519KeyPair::from_pem(&config.jwt_pem)?;
        Ok(Self {
            config: Arc::new(config),
            jwt_keypair: Arc::new(jwt_keypair),
        })
    }
}

#[tonic::async_trait]
impl BurrowWeb for BurrowGrpcServer {
    async fn slack_auth(
        &self,
        request: Request<SlackAuthRequest>,
    ) -> Result<Response<JwtInfo>, Status> {
        auth(request, &self.jwt_keypair).await
    }

    async fn create_device(
        &self,
        request: Request<CreateDeviceRequest>,
    ) -> Result<Response<CreateDeviceResponse>, Status> {
        let req = request.into_inner();
        let jwt = req
            .jwt
            .ok_or(Status::invalid_argument("JWT Not existent!"))?;
        let oid_user = OpenIdUser::try_from_jwt(&jwt, &self.jwt_keypair)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        todo!()
    }

    async fn delete_device(&self, request: Request<JwtInfo>) -> Result<Response<Empty>, Status> {
        todo!()
    }

    async fn list_devices(
        &self,
        request: Request<JwtInfo>,
    ) -> Result<Response<ListDevicesResponse>, Status> {
        todo!()
    }
}
