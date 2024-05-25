use super::*;
use reqwest::{Client, Method};
use serde::Deserialize;
use std::{
    io::{prelude::*, BufReader},
    net::TcpListener,
    process::Command,
};
use url::Url;

const SLACK_CLIENT_ID: &str = "2210535565.6884042183125";
const SLACK_CLIENT_SECRET: &str = "2793c8a5255cae38830934c664eeb62d";
const SLACK_REDIRECT_URI: &str = "https://burrow.rs/callback/oauth2";

pub struct AuthScreen {}

pub struct AuthScreenInit {}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthScreenMsg {
    SlackAuth,
}

#[relm4::component(pub, async)]
impl AsyncComponent for AuthScreen {
    type Init = AuthScreenInit;
    type Input = AuthScreenMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_valign: Align::Fill,

            gtk::Button {
                set_label: "Authenticate with Slack",
                connect_clicked => AuthScreenMsg::SlackAuth,
            },
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let widgets = view_output!();

        let model = AuthScreen {};

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AuthScreenMsg::SlackAuth => {
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
                let listener = TcpListener::bind("127.0.0.1:1024").unwrap();

                let stream = listener.incoming().next().unwrap();
                let mut stream = stream.unwrap();

                let buf_reader = BufReader::new(&mut stream);
                let http_request: Vec<_> = buf_reader
                    .lines()
                    .map(|result| result.unwrap())
                    .take_while(|line| !line.is_empty())
                    .collect();

                let response = "HTTP/1.1 200 OK\r\n\r\n";
                stream.write_all(response.as_bytes()).unwrap();

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

                eprintln!("{:?}", res);
            }
        };
    }
}
