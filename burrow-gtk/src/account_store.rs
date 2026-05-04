use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: String,
    pub kind: AccountKind,
    pub title: String,
    pub authority: Option<String>,
    pub account: String,
    pub identity: String,
    pub hostname: Option<String>,
    pub tailnet: Option<String>,
    pub note: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    WireGuard,
    Tor,
    Tailnet,
}

impl AccountKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::WireGuard => "WireGuard",
            Self::Tor => "Tor",
            Self::Tailnet => "Tailnet",
        }
    }

    fn sort_rank(self) -> u8 {
        match self {
            Self::Tailnet => 0,
            Self::Tor => 1,
            Self::WireGuard => 2,
        }
    }
}

pub fn load() -> Result<Vec<AccountRecord>> {
    let path = storage_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data =
        std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&data).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn upsert(mut record: AccountRecord) -> Result<Vec<AccountRecord>> {
    let mut accounts = load()?;
    let now = timestamp();
    record.updated_at = now;
    if record.created_at == 0 {
        record.created_at = now;
    }

    if let Some(index) = accounts.iter().position(|account| account.id == record.id) {
        accounts[index] = record;
    } else {
        accounts.push(record);
    }
    accounts.sort_by(|lhs, rhs| {
        lhs.kind
            .sort_rank()
            .cmp(&rhs.kind.sort_rank())
            .then_with(|| lhs.title.to_lowercase().cmp(&rhs.title.to_lowercase()))
    });
    persist(&accounts)?;
    Ok(accounts)
}

pub fn new_record(
    kind: AccountKind,
    title: String,
    authority: Option<String>,
    account: String,
    identity: String,
    hostname: Option<String>,
    tailnet: Option<String>,
    note: Option<String>,
) -> AccountRecord {
    let now = timestamp();
    AccountRecord {
        id: format!("{}-{now}", kind.title().to_ascii_lowercase()),
        kind,
        title,
        authority,
        account,
        identity,
        hostname,
        tailnet,
        note,
        created_at: now,
        updated_at: now,
    }
}

fn persist(accounts: &[AccountRecord]) -> Result<()> {
    let path = storage_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let data = serde_json::to_vec_pretty(accounts).context("failed to encode account store")?;
    std::fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))
}

fn storage_path() -> Result<PathBuf> {
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(data_home)
            .join("burrow")
            .join("accounts.json"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("burrow")
            .join("accounts.json"));
    }
    Ok(std::env::temp_dir().join("burrow-accounts.json"))
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
