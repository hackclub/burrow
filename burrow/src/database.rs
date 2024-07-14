use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::{
    daemon::rpc::grpc_defs::{
        Network as RPCNetwork,
        NetworkDeleteRequest,
        NetworkReorderRequest,
        NetworkType,
    },
    wireguard::config::{Config, Interface, Peer},
};

#[cfg(target_vendor = "apple")]
const DB_PATH: &str = "burrow.db";

#[cfg(not(target_vendor = "apple"))]
const DB_PATH: &str = "/var/lib/burrow/burrow.db";

const CREATE_WG_INTERFACE_TABLE: &str = "CREATE TABLE IF NOT EXISTS wg_interface (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    listen_port INTEGER,
    mtu INTEGER,
    private_key TEXT NOT NULL,
    address TEXT NOT NULL,
    dns TEXT NOT NULL
)";

const CREATE_WG_PEER_TABLE: &str = "CREATE TABLE IF NOT EXISTS wg_peer (
    interface_id INT REFERENCES wg_interface(id) ON UPDATE CASCADE,
    endpoint TEXT NOT NULL,
    public_key TEXT NOT NULL,
    allowed_ips TEXT NOT NULL,
    preshared_key TEXT
)";

const CREATE_NETWORK_TABLE: &str = "CREATE TABLE IF NOT EXISTS network (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,
    payload BLOB,
    idx INTEGER,
    interface_id INT REFERENCES wg_interface(id) ON UPDATE CASCADE
);
CREATE TRIGGER IF NOT EXISTS increment_network_idx
AFTER INSERT ON network
BEGIN
    UPDATE network
    SET idx = (SELECT COALESCE(MAX(idx), 0) + 1 FROM network)
    WHERE id = NEW.id;
END;
";

pub fn initialize_tables(conn: &Connection) -> Result<()> {
    conn.execute(CREATE_WG_INTERFACE_TABLE, [])?;
    conn.execute(CREATE_WG_PEER_TABLE, [])?;
    conn.execute(CREATE_NETWORK_TABLE, [])?;
    Ok(())
}

pub fn load_interface(conn: &Connection, interface_id: &str) -> Result<Config> {
    let iface = conn.query_row(
        "SELECT private_key, dns, address, listen_port, mtu FROM wg_interface WHERE id = ?",
        [&interface_id],
        |row| {
            let dns_rw: String = row.get(1)?;
            let dns = parse_lst(&dns_rw);
            let address_rw: String = row.get(2)?;
            let address = parse_lst(&address_rw);
            Ok(Interface {
                private_key: row.get(0)?,
                dns,
                address,
                mtu: row.get(4)?,
                listen_port: row.get(3)?,
            })
        },
    )?;
    let mut peers_stmt = conn.prepare("SELECT public_key, preshared_key, allowed_ips, endpoint FROM wg_peer WHERE interface_id = ?")?;
    let peers = peers_stmt
        .query_map([&interface_id], |row| {
            let preshared_key: Option<String> = row.get(1)?;
            let allowed_ips_rw: String = row.get(2)?;
            let allowed_ips: Vec<String> =
                allowed_ips_rw.split(',').map(|s| s.to_string()).collect();
            Ok(Peer {
                public_key: row.get(0)?,
                preshared_key,
                allowed_ips,
                endpoint: row.get(3)?,
                persistent_keepalive: None,
                name: None,
            })
        })?
        .collect::<rusqlite::Result<Vec<Peer>>>()?;
    Ok(Config { interface: iface, peers })
}

pub fn dump_interface(conn: &Connection, config: &Config) -> Result<()> {
    let mut stmt = conn.prepare("INSERT INTO wg_interface (private_key, dns, address, listen_port, mtu) VALUES (?, ?, ?, ?, ?)")?;
    let cif = &config.interface;
    stmt.execute(params![
        cif.private_key,
        to_lst(&cif.dns),
        to_lst(&cif.address),
        cif.listen_port.unwrap_or(51820),
        cif.mtu
    ])?;
    let interface_id = conn.last_insert_rowid();
    let mut stmt = conn.prepare("INSERT INTO wg_peer (interface_id, public_key, preshared_key, allowed_ips, endpoint) VALUES (?, ?, ?, ?, ?)")?;
    for peer in &config.peers {
        stmt.execute(params![
            &interface_id,
            &peer.public_key,
            &peer.preshared_key,
            &peer.allowed_ips.join(","),
            &peer.endpoint
        ])?;
    }
    Ok(())
}

pub fn get_connection(path: Option<&Path>) -> Result<Connection> {
    let p = path.unwrap_or_else(|| std::path::Path::new(DB_PATH));
    if !p.exists() {
        let conn = Connection::open(p)?;
        initialize_tables(&conn)?;
        dump_interface(&conn, &Config::default())?;
        return Ok(conn);
    }
    Ok(Connection::open(p)?)
}

pub fn add_network(conn: &Connection, network: &RPCNetwork) -> Result<()> {
    let mut stmt = conn.prepare("INSERT INTO network (id, type, payload) VALUES (?, ?, ?)")?;
    stmt.execute(params![
        network.id,
        network.r#type().as_str_name(),
        &network.payload
    ])?;
    if network.r#type() == NetworkType::WireGuard {
        let payload_str = String::from_utf8(network.payload.clone())?;
        let wg_config = Config::from_content_fmt(&payload_str, "ini")?;
        dump_interface(conn, &wg_config)?;
    }
    Ok(())
}

pub fn list_networks(conn: &Connection) -> Result<Vec<RPCNetwork>> {
    let mut stmt = conn.prepare("SELECT id, type, payload FROM network ORDER BY idx")?;
    let networks: Vec<RPCNetwork> = stmt
        .query_map([], |row| {
            println!("row: {:?}", row);
            let network_id: i32 = row.get(0)?;
            let network_type: String = row.get(1)?;
            let network_type = NetworkType::from_str_name(network_type.as_str())
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let payload: Vec<u8> = row.get(2)?;
            Ok(RPCNetwork {
                id: network_id,
                r#type: network_type.into(),
                payload: payload.into(),
            })
        })?
        .collect::<Result<Vec<RPCNetwork>, rusqlite::Error>>()?;
    Ok(networks)
}

pub fn reorder_network(conn: &Connection, req: NetworkReorderRequest) -> Result<()> {
    let mut stmt = conn.prepare("UPDATE network SET idx = ? WHERE id = ?")?;
    let res = stmt.execute(params![req.index, req.id])?;
    if res == 0 {
        return Err(anyhow::anyhow!("No such network exists"));
    }
    Ok(())
}

pub fn delete_network(conn: &Connection, req: NetworkDeleteRequest) -> Result<()> {
    let mut stmt = conn.prepare("DELETE FROM network WHERE id = ?")?;
    let res = stmt.execute(params![req.id])?;
    if res == 0 {
        return Err(anyhow::anyhow!("No such network exists"));
    }
    Ok(())
}

fn parse_lst(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    s.split(',').map(|s| s.to_string()).collect()
}

fn to_lst<T: ToString>(v: &Vec<T>) -> String {
    v.iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_tables(&conn).unwrap();
        let config = Config::default();
        dump_interface(&conn, &config).unwrap();
        let loaded = load_interface(&conn, "1").unwrap();
        assert_eq!(config, loaded);
    }
}
