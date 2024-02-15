use anyhow::Result;
use axum::{
    extract::Query,
    http::StatusCode,
    routing::{get, post},
    Json,
    Router,
};
use serde::{Deserialize, Serialize};

pub async fn start_server() -> Result<()> {
    let app = Router::new()
        .route("/callback", get(callback))
        .route("/users", post(create_user));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4225").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn callback(Query(code): Query<String>) -> StatusCode {
    let req = reqwest::get("https://slack.com/openid.connect.token").await;
    if req.is_err() {
        // Couldn't fetch
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let res = req.unwrap().json::<CodeExchangeResponse>().await;
    if res.is_err() {
        // Couldn't parse the returned JSON document.
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let data = res.unwrap();

    if !data.ok {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    let payload = decode_jwt(data);

    StatusCode::CREATED
}

fn decode_jwt(input: CodeExchangeResponse) -> SlackUserPayload {
    SlackUserPayload::default()
}

#[derive(Deserialize)]
struct CodeExchangeResponse {
    ok: bool,
    access_token: String,
    token_type: String,
    id_token: String,
}

#[derive(Deserialize, Default)]
struct SlackUserPayload {
    iss: String,
    sub: String,
    aud: String,
    exp: i64,
    iat: i64,
    auth_time: i64,
    nonce: String,
    at_hash: String,
    #[serde(rename(deserialize = "https://slack.com/team_id"))]
    team_id: String,
    #[serde(rename(deserialize = "https://slack.com/user_id"))]
    user_id: String,
    email: String,
    email_verified: bool,
    date_email_verified: i64,
    locale: String,
    name: String,
    given_name: String,
    family_name: String,
    #[serde(rename(deserialize = "https://slack.com/team_image_230"))]
    team_image_230: String,
    #[serde(rename(deserialize = "https://slack.com/team_image_default"))]
    team_image_default: bool,
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
