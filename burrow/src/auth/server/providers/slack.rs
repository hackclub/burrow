use anyhow::Result;
use axum::{
    extract::Json,
    http::StatusCode,
    routing::{get, post},
};
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;

use super::db::store_connection;

#[derive(Deserialize)]
pub struct SlackToken {
    slack_token: String,
}
pub async fn auth(Json(payload): Json<SlackToken>) -> (StatusCode, String) {
    let slack_user = match fetch_slack_user(&payload.slack_token).await {
        Ok(user) => user,
        Err(e) => {
            log::error!("Failed to fetch Slack user: {:?}", e);
            return (StatusCode::UNAUTHORIZED, String::new());
        }
    };

    log::info!(
        "Slack user {} ({}) logged in.",
        slack_user.name,
        slack_user.sub
    );

    let conn = match store_connection(slack_user, "slack", &payload.slack_token, None) {
        Ok(user) => user,
        Err(e) => {
            log::error!("Failed to fetch Slack user: {:?}", e);
            return (StatusCode::UNAUTHORIZED, String::new());
        }
    };

    (StatusCode::OK, String::new())
}

async fn fetch_slack_user(access_token: &str) -> Result<super::OpenIdUser> {
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

// async fn fetch_save_slack_user_data(query: Query<CallbackQuery>) -> anyhow::Result<()> {
//     let client = reqwest::Client::new();
//     log::trace!("Code was {}", &query.code);
//     let mut url = Url::parse("https://slack.com/api/openid.connect.token")?;

//     {
//         let mut q = url.query_pairs_mut();
//         q.append_pair("client_id", &var("CLIENT_ID")?);
//         q.append_pair("client_secret", &var("CLIENT_SECRET")?);
//         q.append_pair("code", &query.code);
//         q.append_pair("grant_type", "authorization_code");
//         q.append_pair("redirect_uri", "https://burrow.rs/callback");
//     }

//     let data = client
//         .post(url)
//         .send()
//         .await?
//         .json::<slack::CodeExchangeResponse>()
//         .await?;

//     if !data.ok {
//         return Err(anyhow::anyhow!("Slack code exchange response not ok!"));
//     }

//     if let Some(access_token) = data.access_token {
//         log::trace!("Access token is {access_token}");
//         let user = slack::fetch_slack_user(&access_token)
//             .await
//             .map_err(|err| anyhow::anyhow!("Failed to fetch Slack user info {:#?}", err))?;

//         db::store_user(user, access_token, String::new())
//             .map_err(|_| anyhow::anyhow!("Failed to store user in db"))?;

//         Ok(())
//     } else {
//         Err(anyhow::anyhow!("Access token not found in response"))
//     }
// }
