use anyhow::Result;

pub static PATH: &str = "./server.sqlite3";

pub fn init_db() -> Result<()> {
    let conn = rusqlite::Connection::open(PATH)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user (
			id PRIMARY KEY,
			created_at TEXT NOT NULL
		)",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_connection (
			user_id INTEGER REFERENCES user(id) ON DELETE CASCADE,
			openid_provider TEXT NOT NULL,
			openid_user_id TEXT NOT NULL,
			openid_user_name TEXT NOT NULL,
			access_token TEXT NOT NULL,
			refresh_token TEXT,
			PRIMARY KEY (openid_provider, openid_user_id)
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS device (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            public_key TEXT NOT NULL,
            apns_token TEXT UNIQUE,
            user_id INT REFERENCES user(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL DEFAULT (datetime('now')) CHECK(created_at IS datetime(created_at)),
            ipv4 TEXT NOT NULL UNIQUE,
            ipv6 TEXT NOT NULL UNIQUE,
            access_token TEXT NOT NULL UNIQUE,
            refresh_token TEXT NOT NULL UNIQUE,
            expires_at TEXT NOT NULL DEFAULT (datetime('now', '+7 days')) CHECK(expires_at IS datetime(expires_at))
        )",
        ()
    ).unwrap();

    Ok(())
}

pub fn store_connection(
    openid_user: super::providers::OpenIdUser,
    openid_provider: &str,
    access_token: &str,
    refresh_token: Option<&str>,
) -> Result<()> {
    log::debug!("Storing openid user {:#?}", openid_user);
    let conn = rusqlite::Connection::open(PATH)?;

    conn.execute(
        "INSERT OR IGNORE INTO user (id, created_at) VALUES (?, datetime('now'))",
        (&openid_user.sub,),
    )?;
    conn.execute(
        "INSERT INTO user_connection (user_id, openid_provider, openid_user_id, openid_user_name, access_token, refresh_token) VALUES (
        	(SELECT id FROM user WHERE id = ?),
         	?,
          	?,
           	?,
            ?,
            ?
        )",
        (&openid_user.sub, &openid_provider, &openid_user.sub, &openid_user.name, access_token, refresh_token),
    )?;

    Ok(())
}

pub fn store_device(
    openid_user: super::providers::OpenIdUser,
    openid_provider: &str,
    access_token: &str,
    refresh_token: Option<&str>,
) -> Result<()> {
    log::debug!("Storing openid user {:#?}", openid_user);
    let conn = rusqlite::Connection::open(PATH)?;

    // TODO

    Ok(())
}
