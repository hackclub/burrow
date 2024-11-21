pub mod slack;
pub use super::{db, grpc_defs};

#[derive(serde::Deserialize, Default, Debug)]
pub struct OpenIdUser {
    pub sub: String,
    pub name: String,
}
