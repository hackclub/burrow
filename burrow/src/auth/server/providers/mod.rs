pub mod slack;
pub use super::db;

#[derive(serde::Deserialize, Default, Debug)]
pub struct OpenIdUser {
    pub sub: String,
    pub name: String,
}
