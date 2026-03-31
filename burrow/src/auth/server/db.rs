use anyhow::{anyhow, Context, Result};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};

use crate::control::{
    DnsConfig, Hostinfo, LocalAuthResponse, MapRequest, MapResponse, Node, NodeCapMap,
    PacketFilter, PeerCapMap, RegisterRequest, UserProfile,
};

const CREATE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS auth_user (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    profile_pic_url TEXT,
    groups_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS auth_local_credential (
    user_id INTEGER PRIMARY KEY REFERENCES auth_user(id) ON DELETE CASCADE,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    rotated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS auth_session (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES auth_user(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL DEFAULT (datetime('now', '+7 days'))
);

CREATE TABLE IF NOT EXISTS control_node (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stable_id TEXT NOT NULL UNIQUE,
    user_id INTEGER NOT NULL REFERENCES auth_user(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    node_key TEXT NOT NULL UNIQUE,
    machine_key TEXT,
    disco_key TEXT,
    addresses_json TEXT NOT NULL,
    allowed_ips_json TEXT NOT NULL,
    endpoints_json TEXT NOT NULL,
    home_derp INTEGER,
    hostinfo_json TEXT,
    tags_json TEXT NOT NULL DEFAULT '[]',
    primary_routes_json TEXT NOT NULL DEFAULT '[]',
    cap_version INTEGER NOT NULL DEFAULT 1,
    cap_map_json TEXT NOT NULL DEFAULT '{}',
    peer_cap_map_json TEXT NOT NULL DEFAULT '{}',
    machine_authorized INTEGER NOT NULL DEFAULT 1,
    node_key_expired INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen TEXT,
    online INTEGER
);
"#;

#[derive(Clone, Debug)]
pub struct StoredUser {
    pub profile: UserProfile,
}

pub fn init_db(path: &str) -> Result<()> {
    let conn = Connection::open(path)?;
    conn.execute_batch(CREATE_SCHEMA)?;
    Ok(())
}

pub fn ensure_local_identity(
    path: &str,
    username: &str,
    email: &str,
    display_name: &str,
    password: &str,
) -> Result<UserProfile> {
    let conn = Connection::open(path)?;
    conn.execute(
        "INSERT INTO auth_user (email, display_name) VALUES (?, ?)
         ON CONFLICT(email) DO UPDATE SET display_name = excluded.display_name",
        params![email, display_name],
    )?;
    let user_id: i64 =
        conn.query_row("SELECT id FROM auth_user WHERE email = ?", [email], |row| {
            row.get(0)
        })?;

    let existing_hash: Option<String> = conn
        .query_row(
            "SELECT password_hash FROM auth_local_credential WHERE user_id = ?",
            [user_id],
            |row| row.get(0),
        )
        .optional()?;

    let password_hash = match existing_hash {
        Some(hash) if verify_password(password, &hash) => hash,
        _ => hash_password(password)?,
    };

    conn.execute(
        "INSERT INTO auth_local_credential (user_id, username, password_hash)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET username = excluded.username, password_hash = excluded.password_hash, rotated_at = datetime('now')",
        params![user_id, username, password_hash],
    )?;

    load_user_profile(&conn, user_id)
}

pub fn authenticate_local(
    path: &str,
    identifier: &str,
    password: &str,
) -> Result<Option<LocalAuthResponse>> {
    let conn = Connection::open(path)?;
    let record = conn
        .query_row(
            "SELECT u.id, u.email, u.display_name, u.profile_pic_url, u.groups_json, c.password_hash
             FROM auth_user u
             JOIN auth_local_credential c ON c.user_id = u.id
             WHERE c.username = ? OR u.email = ?",
            params![identifier, identifier],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;

    let Some((user_id, email, display_name, profile_pic_url, groups_json, password_hash)) = record
    else {
        return Ok(None);
    };

    if !verify_password(password, &password_hash) {
        return Ok(None);
    }

    let token = random_token();
    conn.execute(
        "INSERT INTO auth_session (id, user_id) VALUES (?, ?)",
        params![token, user_id],
    )?;

    Ok(Some(LocalAuthResponse {
        access_token: token,
        user: UserProfile {
            id: user_id,
            login_name: email,
            display_name,
            profile_pic_url,
            groups: parse_json(&groups_json)?,
        },
    }))
}

pub fn user_for_session(path: &str, token: &str) -> Result<Option<StoredUser>> {
    let conn = Connection::open(path)?;
    let user_id = conn
        .query_row(
            "SELECT user_id FROM auth_session WHERE id = ? AND expires_at > datetime('now')",
            [token],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    let Some(user_id) = user_id else {
        return Ok(None);
    };

    Ok(Some(load_user(&conn, user_id)?))
}

pub fn upsert_node(path: &str, user: &StoredUser, request: &RegisterRequest) -> Result<Node> {
    let conn = Connection::open(path)?;
    let existing = find_existing_node(&conn, user.profile.id, request)?;
    let name = Node::preferred_name(request);
    let allowed_ips = Node::normalized_allowed_ips(request);

    match existing {
        Some((node_id, stable_id, created_at)) => {
            conn.execute(
                "UPDATE control_node
                 SET name = ?, node_key = ?, machine_key = ?, disco_key = ?, addresses_json = ?, allowed_ips_json = ?,
                     endpoints_json = ?, home_derp = ?, hostinfo_json = ?, tags_json = ?, primary_routes_json = ?,
                     cap_version = ?, cap_map_json = ?, peer_cap_map_json = ?, updated_at = datetime('now'),
                     last_seen = datetime('now'), online = 1
                 WHERE id = ?",
                params![
                    name,
                    request.node_key,
                    request.machine_key,
                    request.disco_key,
                    to_json(&request.addresses)?,
                    to_json(&allowed_ips)?,
                    to_json(&request.endpoints)?,
                    request.home_derp,
                    optional_json(&request.hostinfo)?,
                    to_json(&request.tags)?,
                    to_json(&request.primary_routes)?,
                    request.version.max(1),
                    to_json(&request.cap_map)?,
                    to_json(&request.peer_cap_map)?,
                    node_id,
                ],
            )?;
            load_node(&conn, node_id, stable_id, Some(created_at))
        }
        None => {
            conn.execute(
                "INSERT INTO control_node (
                    stable_id, user_id, name, node_key, machine_key, disco_key, addresses_json, allowed_ips_json,
                    endpoints_json, home_derp, hostinfo_json, tags_json, primary_routes_json, cap_version,
                    cap_map_json, peer_cap_map_json, last_seen, online
                 ) VALUES ('', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), 1)",
                params![
                    user.profile.id,
                    name,
                    request.node_key,
                    request.machine_key,
                    request.disco_key,
                    to_json(&request.addresses)?,
                    to_json(&allowed_ips)?,
                    to_json(&request.endpoints)?,
                    request.home_derp,
                    optional_json(&request.hostinfo)?,
                    to_json(&request.tags)?,
                    to_json(&request.primary_routes)?,
                    request.version.max(1),
                    to_json(&request.cap_map)?,
                    to_json(&request.peer_cap_map)?,
                ],
            )?;
            let node_id = conn.last_insert_rowid();
            let stable_id = format!("bn-{node_id}");
            conn.execute(
                "UPDATE control_node SET stable_id = ? WHERE id = ?",
                params![stable_id, node_id],
            )?;
            load_node(&conn, node_id, stable_id, None)
        }
    }
}

pub fn map_for_node(
    path: &str,
    user: &StoredUser,
    request: &MapRequest,
    domain: &str,
) -> Result<MapResponse> {
    let conn = Connection::open(path)?;
    apply_map_request(&conn, user.profile.id, request)?;
    let self_row = conn
        .query_row(
            "SELECT id, stable_id, created_at FROM control_node WHERE user_id = ? AND node_key = ?",
            params![user.profile.id, request.node_key],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("node not registered"))?;

    let node = load_node(&conn, self_row.0, self_row.1, Some(self_row.2))?;
    let peers = load_peers(&conn, node.id)?;
    Ok(MapResponse {
        map_session_handle: Some(format!("map-{}", node.stable_id)),
        seq: Some(request.map_session_seq.unwrap_or(0) + 1),
        node,
        peers,
        domain: domain.to_owned(),
        dns: Some(DnsConfig {
            resolvers: vec!["1.1.1.1".to_owned(), "1.0.0.1".to_owned()],
            search_domains: vec![domain.to_owned()],
            magic_dns: true,
        }),
        packet_filters: vec![PacketFilter::default()],
    })
}

pub static PATH: &str = "./server.sqlite3";

fn apply_map_request(conn: &Connection, user_id: i64, request: &MapRequest) -> Result<()> {
    let current = conn
        .query_row(
            "SELECT id FROM control_node WHERE user_id = ? AND node_key = ?",
            params![user_id, request.node_key],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;

    let Some(node_id) = current else {
        return Ok(());
    };

    let hostinfo_json = optional_json(&request.hostinfo)?;
    let endpoints_json = to_json(&request.endpoints)?;
    conn.execute(
        "UPDATE control_node
         SET disco_key = COALESCE(?, disco_key),
             hostinfo_json = CASE WHEN ? IS NULL THEN hostinfo_json ELSE ? END,
             endpoints_json = CASE WHEN ? = '[]' THEN endpoints_json ELSE ? END,
             updated_at = datetime('now'),
             last_seen = datetime('now'),
             online = 1
         WHERE id = ?",
        params![
            request.disco_key,
            hostinfo_json,
            hostinfo_json,
            endpoints_json,
            endpoints_json,
            node_id,
        ],
    )?;
    Ok(())
}

fn find_existing_node(
    conn: &Connection,
    user_id: i64,
    request: &RegisterRequest,
) -> Result<Option<(i64, String, String)>> {
    let mut candidates = vec![request.node_key.as_str()];
    if let Some(old) = request.old_node_key.as_deref() {
        if old != request.node_key {
            candidates.push(old);
        }
    }

    for candidate in candidates {
        let hit = conn
            .query_row(
                "SELECT id, stable_id, created_at FROM control_node WHERE user_id = ? AND node_key = ?",
                params![user_id, candidate],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        if hit.is_some() {
            return Ok(hit);
        }
    }
    Ok(None)
}

fn load_peers(conn: &Connection, self_id: i64) -> Result<Vec<Node>> {
    let mut stmt = conn.prepare(
        "SELECT id, stable_id, created_at FROM control_node WHERE id != ? AND machine_authorized = 1 ORDER BY id",
    )?;
    let peers = stmt
        .query_map([self_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    peers
        .into_iter()
        .map(|(id, stable_id, created_at)| load_node(conn, id, stable_id, Some(created_at)))
        .collect()
}

fn load_node(
    conn: &Connection,
    id: i64,
    stable_id: String,
    created_at_hint: Option<String>,
) -> Result<Node> {
    let row = conn.query_row(
        "SELECT user_id, name, node_key, machine_key, disco_key, addresses_json, allowed_ips_json,
                endpoints_json, home_derp, hostinfo_json, tags_json, primary_routes_json, cap_version,
                cap_map_json, peer_cap_map_json, machine_authorized, node_key_expired,
                created_at, updated_at, last_seen, online
         FROM control_node WHERE id = ?",
        [id],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<i32>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, i32>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, i64>(15)?,
                row.get::<_, i64>(16)?,
                row.get::<_, String>(17)?,
                row.get::<_, String>(18)?,
                row.get::<_, Option<String>>(19)?,
                row.get::<_, Option<i64>>(20)?,
            ))
        },
    )?;
    Ok(Node {
        id,
        stable_id,
        user_id: row.0,
        name: row.1,
        node_key: row.2,
        machine_key: row.3,
        disco_key: row.4,
        addresses: parse_json(&row.5)?,
        allowed_ips: parse_json(&row.6)?,
        endpoints: parse_json(&row.7)?,
        home_derp: row.8,
        hostinfo: row.9.map(|raw| parse_json::<Hostinfo>(&raw)).transpose()?,
        tags: parse_json(&row.10)?,
        primary_routes: parse_json(&row.11)?,
        cap_version: row.12,
        cap_map: parse_json::<NodeCapMap>(&row.13)?,
        peer_cap_map: parse_json::<PeerCapMap>(&row.14)?,
        machine_authorized: row.15 != 0,
        node_key_expired: row.16 != 0,
        created_at: Some(created_at_hint.unwrap_or(row.17)),
        updated_at: Some(row.18),
        last_seen: row.19,
        online: row.20.map(|value| value != 0),
    })
}

fn load_user(conn: &Connection, user_id: i64) -> Result<StoredUser> {
    let profile = load_user_profile(conn, user_id)?;
    Ok(StoredUser { profile })
}

fn load_user_profile(conn: &Connection, user_id: i64) -> Result<UserProfile> {
    let row = conn.query_row(
        "SELECT email, display_name, profile_pic_url, groups_json FROM auth_user WHERE id = ?",
        [user_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;
    Ok(UserProfile {
        id: user_id,
        login_name: row.0,
        display_name: row.1,
        profile_pic_url: row.2,
        groups: parse_json(&row.3)?,
    })
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow!("failed to hash password: {err}"))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> bool {
    PasswordHash::new(password_hash)
        .ok()
        .and_then(|hash| {
            Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .ok()
        })
        .is_some()
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).context("failed to serialize json")
}

fn optional_json<T: serde::Serialize>(value: &Option<T>) -> Result<Option<String>> {
    value.as_ref().map(to_json).transpose()
}

fn parse_json<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value)
        .with_context(|| format!("failed to decode json payload from '{value}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{Hostinfo, RegisterRequest};
    use tempfile::TempDir;

    fn temp_db() -> Result<(TempDir, String)> {
        let dir = tempfile::tempdir()?;
        let db_path = dir.path().join("server.sqlite3");
        Ok((dir, db_path.to_string_lossy().to_string()))
    }

    #[test]
    fn local_auth_and_map_round_trip() -> Result<()> {
        let (_dir, db_path) = temp_db()?;
        init_db(&db_path)?;
        ensure_local_identity(
            &db_path,
            "contact",
            "contact@burrow.net",
            "Burrow Contact",
            "password-1",
        )?;

        let auth = authenticate_local(&db_path, "contact", "password-1")?
            .expect("expected login to succeed");
        let user =
            user_for_session(&db_path, &auth.access_token)?.expect("expected session to resolve");

        let node = upsert_node(
            &db_path,
            &user,
            &RegisterRequest {
                node_key: "nodekey:aaaa".to_owned(),
                machine_key: Some("machinekey:aaaa".to_owned()),
                disco_key: Some("discokey:aaaa".to_owned()),
                addresses: vec!["100.64.0.1/32".to_owned()],
                endpoints: vec!["203.0.113.10:41641".to_owned()],
                hostinfo: Some(Hostinfo {
                    hostname: Some("burrow-dev".to_owned()),
                    os: Some("linux".to_owned()),
                    os_version: Some("6.13".to_owned()),
                    services: vec!["ssh".to_owned()],
                    request_tags: vec!["tag:dev".to_owned()],
                }),
                ..RegisterRequest::default()
            },
        )?;
        assert_eq!(node.name, "burrow-dev");
        assert_eq!(node.allowed_ips, vec!["100.64.0.1/32"]);

        let map = map_for_node(
            &db_path,
            &user,
            &MapRequest {
                node_key: "nodekey:aaaa".to_owned(),
                stream: true,
                endpoints: vec!["203.0.113.10:41641".to_owned()],
                ..MapRequest::default()
            },
            "burrow.net",
        )?;
        assert_eq!(map.node.node_key, "nodekey:aaaa");
        assert_eq!(map.domain, "burrow.net");
        assert!(map.dns.expect("dns config").magic_dns);
        Ok(())
    }

    #[test]
    fn register_can_rotate_node_keys() -> Result<()> {
        let (_dir, db_path) = temp_db()?;
        init_db(&db_path)?;
        ensure_local_identity(
            &db_path,
            "contact",
            "contact@burrow.net",
            "Burrow Contact",
            "password-1",
        )?;
        let auth = authenticate_local(&db_path, "contact@burrow.net", "password-1")?
            .expect("expected login to succeed");
        let user =
            user_for_session(&db_path, &auth.access_token)?.expect("expected session to resolve");

        upsert_node(
            &db_path,
            &user,
            &RegisterRequest {
                node_key: "nodekey:old".to_owned(),
                addresses: vec!["100.64.0.2/32".to_owned()],
                ..RegisterRequest::default()
            },
        )?;

        let rotated = upsert_node(
            &db_path,
            &user,
            &RegisterRequest {
                node_key: "nodekey:new".to_owned(),
                old_node_key: Some("nodekey:old".to_owned()),
                addresses: vec!["100.64.0.3/32".to_owned()],
                ..RegisterRequest::default()
            },
        )?;
        assert_eq!(rotated.node_key, "nodekey:new");
        assert_eq!(rotated.addresses, vec!["100.64.0.3/32"]);
        Ok(())
    }
}
