use std::env::var;

use anyhow::Result;
use axum::{extract::Query, http::StatusCode, routing::get, Router};
use reqwest::Url;
use serde::Deserialize;

pub async fn start_server() -> Result<()> {
    let app = Router::new()
        .route("/", get(root_callback))
        .route("/callback", get(code_callback));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4225").await.unwrap();
    log::info!("Starting auth server on port 4225");
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn root_callback() -> String {
    String::from("Did you mean /callback?")
}

#[derive(Deserialize, Debug)]
struct CallbackQuery {
    code: String,
}
async fn code_callback(query: Query<CallbackQuery>) -> StatusCode {
    match fetch_save_slack_user_data(query).await {
        Ok(_) => StatusCode::CREATED,
        Err(err) => {
            log::error!("{err}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
async fn fetch_save_slack_user_data(query: Query<CallbackQuery>) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    log::trace!("Code was {}", &query.code);
    let mut url = Url::parse("https://slack.com/api/openid.connect.token")?;

    {
        let mut q = url.query_pairs_mut();
        q.append_pair("client_id", &var("CLIENT_ID")?);
        q.append_pair("client_secret", &var("CLIENT_SECRET")?);
        q.append_pair("code", &query.code);
        q.append_pair("grant_type", "authorization_code");
        q.append_pair("redirect_uri", "https://burrow.rs/callback");
    }

    let data = client
        .post(url)
        .send()
        .await?
        .json::<slack::CodeExchangeResponse>()
        .await?;

    if !data.ok {
        return Err(anyhow::anyhow!("Slack code exchange response not ok!"));
    }

    if let Some(access_token) = data.access_token {
        log::trace!("Access token is {access_token}");
        let user = slack::fetch_slack_user(&access_token)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to fetch Slack user info {:#?}", err))?;

        db::store_user(user, access_token, String::new())
            .map_err(|_| anyhow::anyhow!("Failed to store user in db"))?;

        Ok(())
    } else {
        Err(anyhow::anyhow!("Access token not found in response"))
    }
}

mod slack {
    use anyhow::Result;
    use reqwest::header::AUTHORIZATION;
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

    #[derive(Deserialize, Default, Debug)]
    pub struct User {
        pub sub: String,
        pub name: String,
    }

    pub async fn fetch_slack_user(access_token: &str) -> Result<User> {
        let client = reqwest::Client::new();
        let res = client
            .get("https://slack.com/api/openid.connect.userInfo")
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let res_ok = res
            .get("ok")
            .and_then(|v| v.as_bool())
            .ok_or(anyhow::anyhow!("Slack user object not ok!"))?;

        if !res_ok {
            return Err(anyhow::anyhow!("Slack user object not ok!"));
        }

        Ok(serde_json::from_value(res)?)
    }
}

mod db {
    use rusqlite::{Connection, Result};

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
        log::debug!("Storing openid user {:#?}", openid_user);
        let conn = Connection::open_in_memory().unwrap();

        init_db(&conn).unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO user (id, created_at) VALUES (?, datetime('now'))",
            (&openid_user.sub,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO user_connection (user_id, openid_provider, openid_user_id, openid_user_name, access_token, refresh_token) VALUES (
            	(SELECT id FROM user WHERE id = ?),
             	'slack',
              	?,
               	?,
                ?,
                ?
            )",
            (&openid_user.sub, &openid_user.sub, &openid_user.name, access_token, refresh_token),
        ).unwrap();

        Ok(())
    }

    fn init_db(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE user (
            id PRIMARY KEY,
            created_at TEXT NOT NULL
        )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE user_connection (
                user_id INT REFERENCES user(id) ON DELETE CASCADE,
                openid_provider TEXT NOT NULL,
                openid_user_id TEXT NOT NULL,
                openid_user_name TEXT NOT NULL,
                access_token TEXT NOT NULL,
                refresh_token TEXT NOT NULL,
                PRIMARY KEY (openid_provider, openid_user_id)
            )",
            (),
        )
        .unwrap();

        Ok(())
    }
}
