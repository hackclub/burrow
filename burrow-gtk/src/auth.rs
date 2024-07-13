use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tokio::{
    io::AsyncWriteExt,
    io::{AsyncBufReadExt, BufReader},
    net::TcpListener,
};
use url::Url;

const SLACK_CLIENT_ID: &str = "2210535565.6884042183125";
const SLACK_CLIENT_SECRET: &str = "2793c8a5255cae38830934c664eeb62d";
const SLACK_REDIRECT_URI: &str = "https://burrow.rs/callback/oauth2";

pub async fn slack_auth() {
    let url = Url::parse_with_params(
        "https://slack.com/openid/connect/authorize",
        &[
            ("response_type", "code"),
            ("scope", "openid profile"),
            ("client_id", SLACK_CLIENT_ID),
            ("redirect_uri", SLACK_REDIRECT_URI),
        ],
    )
    .unwrap();
    Command::new("xdg-open").arg(url.as_str()).spawn().unwrap();
    let listener = TcpListener::bind("127.0.0.1:1024").await.unwrap();

    let (mut stream, _) = listener.accept().await.unwrap();

    let buf_reader = BufReader::new(&mut stream);

    let mut lines = buf_reader.lines();
    let mut http_request = vec![];
    while let Some(line) = lines.next_line().await.unwrap() {
        if !line.is_empty() {
            http_request.push(line);
        } else {
            break;
        }
    }

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes()).await.unwrap();

    let code = http_request
        .iter()
        .filter_map(|field| {
            if field.starts_with("GET ") {
                Some(
                    field
                        .replace("GET /?code=", "")
                        .replace(" HTTP/1.1", "")
                        .to_owned(),
                )
            } else {
                None
            }
        })
        .next()
        .unwrap();

    #[derive(Debug, Clone, Deserialize)]
    struct TokenRes {
        ok: bool,
        access_token: Option<String>,
        token_type: Option<String>,
        id_token: Option<String>,
    }

    let client = Client::builder().build().unwrap();
    let res = client
        .request(Method::POST, "https://slack.com/api/openid.connect.token")
        .query(&[
            ("client_id", SLACK_CLIENT_ID),
            ("client_secret", SLACK_CLIENT_SECRET),
            ("code", &code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", SLACK_REDIRECT_URI),
        ])
        .send()
        .await
        .unwrap()
        .json::<TokenRes>()
        .await
        .unwrap();

    #[derive(Debug, Clone, Serialize)]
    struct SlackAuthReq {
        slack_token: String,
    }

    let res = client
        .post("https://burrow-hidden-pine-3298.fly.dev/slack-auth")
        .json(&SlackAuthReq {
            slack_token: res.id_token.unwrap(),
        })
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    println!("{:?}", res);
}
