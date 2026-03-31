use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::{
    control::TailnetConfig,
    daemon::rpc::grpc_defs::{
        Network as RPCNetwork, NetworkDeleteRequest, NetworkReorderRequest, NetworkType,
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
    conn.execute_batch(CREATE_NETWORK_TABLE)?;
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
    let conn = Connection::open(p)?;
    initialize_tables(&conn)?;
    Ok(conn)
}

pub fn add_network(conn: &Connection, network: &RPCNetwork) -> Result<()> {
    validate_network_payload(network)?;
    let mut stmt = conn.prepare("INSERT INTO network (id, type, payload) VALUES (?, ?, ?)")?;
    stmt.execute(params![
        network.id,
        network.r#type().as_str_name(),
        &network.payload
    ])?;
    Ok(())
}

pub fn list_networks(conn: &Connection) -> Result<Vec<RPCNetwork>> {
    let mut stmt = conn.prepare("SELECT id, type, payload FROM network ORDER BY idx, id")?;
    let networks: Vec<RPCNetwork> = stmt
        .query_map([], |row| {
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
    let mut ordered_ids = ordered_network_ids(conn)?;
    let Some(current_idx) = ordered_ids.iter().position(|id| *id == req.id) else {
        return Err(anyhow::anyhow!("No such network exists"));
    };

    let target_idx = usize::try_from(req.index)
        .map_err(|_| anyhow::anyhow!("Network index must be non-negative"))?;

    let moved_id = ordered_ids.remove(current_idx);
    let target_idx = target_idx.min(ordered_ids.len());
    ordered_ids.insert(target_idx, moved_id);

    renumber_networks(conn, &ordered_ids)
}

pub fn delete_network(conn: &Connection, req: NetworkDeleteRequest) -> Result<()> {
    let mut stmt = conn.prepare("DELETE FROM network WHERE id = ?")?;
    let res = stmt.execute(params![req.id])?;
    if res == 0 {
        return Err(anyhow::anyhow!("No such network exists"));
    }
    let ordered_ids = ordered_network_ids(conn)?;
    renumber_networks(conn, &ordered_ids)
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

fn validate_network_payload(network: &RPCNetwork) -> Result<()> {
    match network.r#type() {
        NetworkType::WireGuard => {
            let payload_str = String::from_utf8(network.payload.clone())?;
            Config::from_content_fmt(&payload_str, "ini")?;
        }
        NetworkType::Tailnet => {
            TailnetConfig::from_slice(&network.payload)?;
        }
    }
    Ok(())
}

fn ordered_network_ids(conn: &Connection) -> Result<Vec<i32>> {
    let mut stmt = conn.prepare("SELECT id FROM network ORDER BY idx, id")?;
    let ids = stmt
        .query_map([], |row| row.get::<_, i32>(0))?
        .collect::<rusqlite::Result<Vec<i32>>>()?;
    Ok(ids)
}

fn renumber_networks(conn: &Connection, ordered_ids: &[i32]) -> Result<()> {
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| -> Result<()> {
        let mut stmt = conn.prepare("UPDATE network SET idx = ? WHERE id = ?")?;
        for (idx, id) in ordered_ids.iter().enumerate() {
            stmt.execute(params![idx as i32, id])?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT")?;
            Ok(())
        }
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_wireguard_payload() -> Vec<u8> {
        br#"[Interface]
PrivateKey = OEPVdomeLTxTIBvv3TYsJRge0Hp9NMiY0sIrhT8OWG8=
Address = 10.13.13.2/24
ListenPort = 51820

[Peer]
PublicKey = 8GaFjVO6c4luCHG4ONO+1bFG8tO+Zz5/Gy+Geht1USM=
PresharedKey = ha7j4BjD49sIzyF9SNlbueK0AMHghlj6+u0G3bzC698=
AllowedIPs = 0.0.0.0/0, 8.8.8.8/32
Endpoint = wg.burrow.rs:51820
"#
        .to_vec()
    }

    fn sample_wireguard_payload_with_address(address: &str, mtu: u16) -> Vec<u8> {
        format!(
            "[Interface]\nPrivateKey = OEPVdomeLTxTIBvv3TYsJRge0Hp9NMiY0sIrhT8OWG8=\nAddress = {address}\nListenPort = 51820\nMTU = {mtu}\n\n[Peer]\nPublicKey = 8GaFjVO6c4luCHG4ONO+1bFG8tO+Zz5/Gy+Geht1USM=\nPresharedKey = ha7j4BjD49sIzyF9SNlbueK0AMHghlj6+u0G3bzC698=\nAllowedIPs = 0.0.0.0/0\nEndpoint = wg.burrow.rs:51820\n"
        )
        .into_bytes()
    }

    fn sample_tailnet_payload() -> Vec<u8> {
        br#"{
  "provider":"tailscale",
  "account":"default",
  "identity":"apple",
  "tailnet":"example.ts.net",
  "hostname":"burrow-phone"
}"#
        .to_vec()
    }

    #[test]
    fn test_db() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_tables(&conn).unwrap();
        let config = Config::default();
        dump_interface(&conn, &config).unwrap();
        let loaded = load_interface(&conn, "1").unwrap();
        assert_eq!(config, loaded);
    }

    #[test]
    fn add_network_validates_payloads() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_tables(&conn).unwrap();

        add_network(
            &conn,
            &RPCNetwork {
                id: 1,
                r#type: NetworkType::WireGuard.into(),
                payload: sample_wireguard_payload(),
            },
        )
        .unwrap();

        add_network(
            &conn,
            &RPCNetwork {
                id: 2,
                r#type: NetworkType::Tailnet.into(),
                payload: sample_tailnet_payload(),
            },
        )
        .unwrap();

        add_network(
            &conn,
            &RPCNetwork {
                id: 3,
                r#type: NetworkType::WireGuard.into(),
                payload: sample_wireguard_payload_with_address("10.42.0.2/32", 1380),
            },
        )
        .unwrap();

        assert!(add_network(
            &conn,
            &RPCNetwork {
                id: 4,
                r#type: NetworkType::WireGuard.into(),
                payload: b"not-a-config".to_vec(),
            },
        )
        .is_err());

        assert!(add_network(
            &conn,
            &RPCNetwork {
                id: 5,
                r#type: NetworkType::Tailnet.into(),
                payload: b"not-a-tailnet-config".to_vec(),
            },
        )
        .is_err());

        let ids: Vec<i32> = list_networks(&conn)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn reorder_and_delete_networks_keep_priority_stable() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_tables(&conn).unwrap();

        for (id, address, mtu) in [
            (1, "10.42.0.2/32", 1380),
            (2, "10.42.0.3/32", 1381),
            (3, "10.42.0.4/32", 1382),
        ] {
            add_network(
                &conn,
                &RPCNetwork {
                    id,
                    r#type: NetworkType::WireGuard.into(),
                    payload: sample_wireguard_payload_with_address(address, mtu),
                },
            )
            .unwrap();
        }

        reorder_network(&conn, NetworkReorderRequest { id: 3, index: 0 }).unwrap();
        let ids: Vec<i32> = list_networks(&conn)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec![3, 1, 2]);

        delete_network(&conn, NetworkDeleteRequest { id: 1 }).unwrap();
        let ids: Vec<i32> = list_networks(&conn)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec![3, 2]);
    }

    #[test]
    fn get_connection_does_not_seed_a_default_interface() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("burrow.sqlite3");

        let conn = get_connection(Some(db_path.as_path())).unwrap();

        let interface_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM wg_interface", [], |row| row.get(0))
            .unwrap();
        let network_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM network", [], |row| row.get(0))
            .unwrap();

        assert_eq!(interface_count, 0);
        assert_eq!(network_count, 0);
    }
}
