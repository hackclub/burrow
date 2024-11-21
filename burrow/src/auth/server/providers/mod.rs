pub mod slack;
pub use super::{db, grpc_defs};
use anyhow::Result;
use grpc_defs::JwtInfo;

#[derive(serde::Deserialize, Default, Debug)]
pub struct OpenIdUser {
    pub sub: String,
    pub name: String,
}

impl TryFrom<&JwtInfo> for OpenIdUser {
    type Error = anyhow::Error;

    fn try_from(jwt_info: &JwtInfo) -> Result<Self> {
        todo!()
    }
}
