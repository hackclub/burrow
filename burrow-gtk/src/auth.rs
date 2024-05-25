use super::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct SlackToken {
    slack_token: String,
}

