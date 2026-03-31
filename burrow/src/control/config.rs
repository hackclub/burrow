use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TailnetProvider {
    Tailscale,
    Headscale,
    Burrow,
}

impl Default for TailnetProvider {
    fn default() -> Self {
        Self::Tailscale
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TailnetConfig {
    #[serde(default)]
    pub provider: TailnetProvider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tailnet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}

impl TailnetConfig {
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let payload = std::str::from_utf8(bytes).context("tailnet payload must be valid UTF-8")?;
        Self::from_str(payload)
    }

    pub fn from_str(payload: &str) -> Result<Self> {
        let trimmed = payload.trim();
        if trimmed.starts_with('{') {
            return serde_json::from_str(trimmed).context("invalid tailnet JSON payload");
        }
        toml::from_str(trimmed).context("invalid tailnet TOML payload")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_payload() {
        let config = TailnetConfig::from_str(
            r#"{
                "provider":"tailscale",
                "account":"default",
                "identity":"apple",
                "tailnet":"example.ts.net",
                "hostname":"burrow-phone"
            }"#,
        )
        .unwrap();
        assert_eq!(config.provider, TailnetProvider::Tailscale);
        assert_eq!(config.account.as_deref(), Some("default"));
        assert_eq!(config.identity.as_deref(), Some("apple"));
    }

    #[test]
    fn parses_toml_payload() {
        let config = TailnetConfig::from_str(
            r#"
provider = "headscale"
authority = "https://headscale.example.com"
account = "default"
identity = "apple"
"#,
        )
        .unwrap();
        assert_eq!(config.provider, TailnetProvider::Headscale);
        assert_eq!(
            config.authority.as_deref(),
            Some("https://headscale.example.com")
        );
    }
}
