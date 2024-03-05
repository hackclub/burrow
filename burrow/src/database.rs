use rusqlite::{Connection, params};
use anyhow::Result;
use crate::wireguard::config::{Config, Interface, Peer};

#[cfg(target_vendor = "apple")]
const DB_PATH: &str = "burrow.db";

#[cfg(not(target_vendor = "apple"))]
const DB_PATH: &str = "/var/lib/burrow/burrow.db";

pub fn prepare_db(conn: &Connection) -> Result<()> {
    conn.execute("CREATE TABLE IF NOT EXISTS wg_interface (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT,
        listen_port INTEGER,
        mtu INTEGER,
        private_key TEXT NOT NULL,
        address TEXT NOT NULL,
        dns TEXT NOT NULL
    )", [])?;
    conn.execute("CREATE TABLE IF NOT EXISTS wg_peer (
        interface_id INT REFERENCES wg_interface(id) ON UPDATE CASCADE,
        endpoint TEXT NOT NULL,
        public_key TEXT NOT NULL,
        allowed_ips TEXT NOT NULL,
        preshared_key TEXT
    )", [])?;
    conn.execute("CREATE TABLE IF NOT EXISTS network (
        interface_id INT REFERENCES wg_interface(id) ON UPDATE CASCADE
    )", [])?;
    Ok(())
}

pub fn load_interface(conn: &Connection, interface_id: String) -> Result<Config> {
    let iface = conn.query_row("SELECT private_key, dns, address, listen_port, mtu FROM wg_interface WHERE id = ?", [&interface_id], |row| {
        let dns_rw: String = row.get(1)?;
        let dns: Vec<String> = if dns_rw.len()>0 {
            dns_rw.split(',').map(|s| s.to_string()).collect()
        } else {
            vec![]
        };
        Ok(Interface {
            private_key: row.get(0)?,
            dns,
            address: row.get(2)?,
            mtu:row.get(4)?,
            listen_port: row.get(3)?,
        })
    })?;
    let mut peers_stmt = conn.prepare("SELECT public_key, preshared_key, allowed_ips, endpoint FROM wg_peer WHERE interface_id = ?")?;
    let peers = peers_stmt.query_map([&interface_id], |row| {
        let preshared_key: Option<String> = row.get(1)?;
        let allowed_ips_rw: String = row.get(2)?;
        let allowed_ips: Vec<String> = allowed_ips_rw.split(',').map(|s| s.to_string()).collect();
        Ok(Peer {
            public_key: row.get(0)?,
            preshared_key,
            allowed_ips,
            endpoint: row.get(3)?,
            persistent_keepalive: None,
            name: None,
        })
    })?.collect::<rusqlite::Result<Vec<Peer>>>()?;
    Ok(Config {
        interface: iface,
        peers,
    })
}

pub fn dump_interface(conn: &Connection, config: &Config) -> Result<()> {
    let mut stmt = conn.prepare("INSERT INTO wg_interface (private_key, dns, address, listen_port, mtu) VALUES (?, ?, ?, ?, ?)")?;
    let cif = &config.interface;
    stmt.execute(params![cif.private_key, cif.dns.join(","), cif.address, cif.listen_port, cif.mtu])?;
    let interface_id = conn.last_insert_rowid();
    let mut stmt = conn.prepare("INSERT INTO wg_peer (interface_id, public_key, preshared_key, allowed_ips, endpoint) VALUES (?, ?, ?, ?, ?)")?;
    for peer in &config.peers {
        stmt.execute(params![&interface_id, &peer.public_key, &peer.preshared_key, &peer.allowed_ips.join(","), &peer.endpoint])?;
    }
    Ok(())
}

pub fn get_connection() -> Result<Connection> {
    let p = std::path::Path::new(DB_PATH);
    Ok(Connection::open(p)?)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::*;

    #[test]
    fn test_db() {
        let p = Path::new(DB_PATH);
        let conn = Connection::open(p).unwrap();
        prepare_db(&conn).unwrap();
        let config = Config::default();
        dump_interface(&conn, &config).unwrap();
        let loaded = load_interface(&conn, "1".to_string()).unwrap();
        assert_eq!(config, loaded);
    }
}