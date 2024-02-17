use anyhow::Result;
use reqwest::Url;

use super::{client_id, hackclub_teamid};

pub async fn login() -> Result<()> {
    let state = "vt :P";
    let nonce = "no";

    let mut url = Url::parse("https://slack.com/openid/connect/authorize")?;
    let mut q = url.query_pairs_mut();
    q.append_pair("response_type", "code");
    q.append_pair("scope", "openid profile email");
    q.append_pair("client_id", client_id);
    q.append_pair("state", state);
    q.append_pair("team", hackclub_teamid);
    q.append_pair("nonce", nonce);
    q.append_pair("redirect_uri", "https://burrow.rs/callback");
    drop(q);

    println!("Continue auth in your browser:\n{}", url.as_str());

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    Ok(())
}
