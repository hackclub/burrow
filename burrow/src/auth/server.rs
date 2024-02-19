use anyhow::Result;
use axum::{extract::Query, http::StatusCode, routing::get, Router};
use reqwest::Url;
use serde::Deserialize;

pub async fn start_server() -> Result<()> {
    let app = Router::new().route("/callback", get(callback));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4225").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

#[derive(Deserialize, Debug)]
struct CallbackQuery {
    code: String,
}
async fn callback(query: Query<CallbackQuery>) -> StatusCode {
    let client = reqwest::Client::new();

    let mut url = Url::parse("https://slack.com/api/openid.connect.token").unwrap();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("client_id", super::client_id);
        q.append_pair("client_secret", super::client_secret);
        q.append_pair("code", &query.code);
        q.append_pair("grant_type", "authorization_code");
        q.append_pair("redirect_uri", "https://burrow.rs/callback");
    }

    let req = client.post(url).send().await;
    if req.is_err() {
        println!("{:?}", req.err());
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let res = req.unwrap().json::<slack::CodeExchangeResponse>().await;
    if res.is_err() {
        println!("{:?}", res.err());
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let data = res.unwrap();

    if !data.ok {
        println!("not ok!");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if let Some(access_token) = data.access_token {
        // println!("Access token is {access_token}");
        // let user = slack::fetch_slack_user(&access_token).await;
        // if user.is_err() {
        //     println!("failed to fetch {:?}", user.err());
        //     return StatusCode::INTERNAL_SERVER_ERROR;
        // }
        // let user = user.unwrap();
        // db::store_user(user, access_token, String::new()).expect("failed to store user in db");

        StatusCode::CREATED
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

mod slack {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub struct CodeExchangeResponse {
        pub ok: bool,

        // Success
        pub access_token: Option<String>,
        token_type: Option<String>,
        id_token: Option<String>,

        // Failure
        error: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct User {
        pub sub: String,
        pub name: String,
    }

    pub async fn fetch_slack_user(access_token: &str) -> Result<User, reqwest::Error> {
        reqwest::get(format!(
            "https://slack.com/api/openid.connect.userInfo?token={access_token}"
        ))
        .await?
        .json::<User>()
        .await
    }
}

mod db {
    use rusqlite::{params, Connection, Result};

    #[derive(Debug)]
    struct User {
        id: i32,
        created_at: String,
    }

    pub fn store_user(
        openid_user: super::slack::User,
        access_token: String,
        refresh_token: String,
    ) -> Result<()> {
        let conn = Connection::open_in_memory()?;

        init_db(&conn)?;

        conn.execute(
            "INSERT INTO person (user_id, openid_provider, openid_user_id, openid_user_name, access_token, refresh_token) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (0, "slack", openid_user.sub, openid_user.name, access_token, refresh_token),
        )?;

        Ok(())
    }

    fn init_db(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE user (
				id PRIMARY KEY
				created_at TEXT NOT NULL
			)",
            (),
        )?;

        conn.execute(
            "CREATE TABLE user_connection (
			user_id INT REFERENCES user(id) ON DELETE CASCADE
			openid_provider TEXT NOT NULL
			openid_user_id TEXT NOT NULL
			openid_user_name TEXT NOT NULL
			access_token TEXT NOT NULL
			refresh_token TEXT NOT NULL

			PRIMARY KEY (openid_provider, openid_user_id)
		)",
            (),
        )?;

        Ok(())
    }
}
