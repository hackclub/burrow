use config::{Config, ConfigError, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BurrowAuthServerConfig {
    pub jwt_pem: String,
}

impl BurrowAuthServerConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(Environment::default())
            .build()?;
        s.try_deserialize()
    }

    /// Creates a new config that includes the dotenv
    pub fn new_dotenv() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        Self::new()
    }
}
