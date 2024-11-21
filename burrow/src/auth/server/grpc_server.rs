use tonic::{Request, Response, Status};

use crate::auth::server::providers::OpenIdUser;

use super::{
    grpc_defs::{
        burrowwebrpc::burrow_web_server::{BurrowWeb, BurrowWebServer},
        CreateDeviceRequest, CreateDeviceResponse, Empty, JwtInfo, ListDevicesResponse,
        SlackAuthRequest,
    },
    providers::slack::auth,
};

#[derive(Debug)]
struct BurrowGrpcServer;

#[tonic::async_trait]
impl BurrowWeb for BurrowGrpcServer {
    async fn slack_auth(
        &self,
        request: Request<SlackAuthRequest>,
    ) -> Result<Response<JwtInfo>, Status> {
        auth(request).await
    }

    async fn create_device(
        &self,
        request: Request<CreateDeviceRequest>,
    ) -> Result<Response<CreateDeviceResponse>, Status> {
        let req = request.into_inner();
        let jwt = req
            .jwt
            .ok_or(Status::invalid_argument("JWT Not existent!"))?;
        let oid_user =
            OpenIdUser::try_from(&jwt).map_err(|e| Status::invalid_argument(e.to_string()))?;
        unimplemented!()
    }

    async fn delete_device(&self, request: Request<JwtInfo>) -> Result<Response<Empty>, Status> {
        unimplemented!()
    }

    async fn list_devices(
        &self,
        request: Request<JwtInfo>,
    ) -> Result<Response<ListDevicesResponse>, Status> {
        unimplemented!()
    }
}
