use std::env::var;

use anyhow::Result;
use reqwest::Url;

pub async fn login() -> Result<()> {
    let state = "vt :P";
    let nonce = "no";

    let mut url = Url::parse("https://slack.com/openid/connect/authorize")?;
    let mut q = url.query_pairs_mut();
    q.append_pair("response_type", "code");
    q.append_pair("scope", "openid profile email");
    q.append_pair("client_id", &var("CLIENT_ID")?);
    q.append_pair("state", state);
    q.append_pair("team", &var("SLACK_TEAM_ID")?);
    q.append_pair("nonce", nonce);
    q.append_pair("redirect_uri", "https://burrow.rs/callback");
    drop(q);

    println!("Continue auth in your browser:\n{}", url.as_str());

    Ok(())
}
