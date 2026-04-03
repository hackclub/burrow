use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

use super::TailnetProvider;

pub const TAILNET_DISCOVERY_REL: &str = "https://burrow.net/rel/tailnet-control-server";
const TAILNET_DISCOVERY_PATH: &str = "/.well-known/burrow-tailnet";
const WEBFINGER_PATH: &str = "/.well-known/webfinger";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TailnetDiscovery {
    pub domain: String,
    pub provider: TailnetProvider,
    pub authority: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oidc_issuer: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct WebFingerDocument {
    #[serde(default)]
    links: Vec<WebFingerLink>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct WebFingerLink {
    #[serde(default)]
    rel: String,
    #[serde(default)]
    href: Option<String>,
}

pub async fn discover_tailnet(email: &str) -> Result<TailnetDiscovery> {
    let domain = email_domain(email)?;
    let base_url = Url::parse(&format!("https://{domain}"))
        .with_context(|| format!("invalid discovery domain {domain}"))?;
    let client = Client::builder()
        .user_agent("burrow-tailnet-discovery")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("failed to build tailnet discovery client")?;
    discover_tailnet_at(&client, email, &base_url).await
}

pub async fn discover_tailnet_at(
    client: &Client,
    email: &str,
    base_url: &Url,
) -> Result<TailnetDiscovery> {
    let domain = email_domain(email)?;

    if let Some(discovery) = discover_well_known(client, base_url).await? {
        return Ok(TailnetDiscovery { domain, ..discovery });
    }

    if let Some(authority) = discover_webfinger(client, email, base_url).await? {
        return Ok(TailnetDiscovery {
            domain,
            provider: TailnetProvider::Headscale,
            authority,
            oidc_issuer: None,
        });
    }

    Err(anyhow!("no tailnet discovery metadata found for {domain}"))
}

pub fn email_domain(email: &str) -> Result<String> {
    let trimmed = email.trim();
    let (_, domain) = trimmed
        .rsplit_once('@')
        .ok_or_else(|| anyhow!("email address must include a domain"))?;
    let domain = domain.trim().trim_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        return Err(anyhow!("email address must include a domain"));
    }
    Ok(domain)
}

async fn discover_well_known(client: &Client, base_url: &Url) -> Result<Option<TailnetDiscovery>> {
    let url = base_url
        .join(TAILNET_DISCOVERY_PATH)
        .context("failed to build tailnet discovery URL")?;
    let response = client
        .get(url)
        .header("accept", "application/json")
        .send()
        .await
        .context("tailnet well-known request failed")?;

    match response.status() {
        StatusCode::OK => response
            .json::<TailnetDiscovery>()
            .await
            .context("invalid tailnet discovery document")
            .map(Some),
        StatusCode::NOT_FOUND => Ok(None),
        status => Err(anyhow!("tailnet well-known lookup failed with HTTP {status}")),
    }
}

async fn discover_webfinger(client: &Client, email: &str, base_url: &Url) -> Result<Option<String>> {
    let mut url = base_url
        .join(WEBFINGER_PATH)
        .context("failed to build webfinger URL")?;
    url.query_pairs_mut()
        .append_pair("resource", &format!("acct:{email}"))
        .append_pair("rel", TAILNET_DISCOVERY_REL);

    let response = client
        .get(url)
        .header("accept", "application/jrd+json, application/json")
        .send()
        .await
        .context("tailnet webfinger request failed")?;

    match response.status() {
        StatusCode::OK => {
            let document = response
                .json::<WebFingerDocument>()
                .await
                .context("invalid webfinger document")?;
            Ok(document
                .links
                .into_iter()
                .find(|link| link.rel == TAILNET_DISCOVERY_REL)
                .and_then(|link| link.href)
                .filter(|href| !href.trim().is_empty()))
        }
        StatusCode::NOT_FOUND => Ok(None),
        status => Err(anyhow!("tailnet webfinger lookup failed with HTTP {status}")),
    }
}

#[cfg(test)]
mod tests {
    use axum::{routing::get, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn extracts_domain_from_email() {
        assert_eq!(email_domain("Contact@Burrow.net").unwrap(), "burrow.net");
        assert!(email_domain("contact").is_err());
    }

    #[tokio::test]
    async fn discovers_from_well_known_document() -> Result<()> {
        let router = Router::new().route(
            TAILNET_DISCOVERY_PATH,
            get(|| async {
                axum::Json(json!({
                    "domain": "burrow.net",
                    "provider": "headscale",
                    "authority": "https://ts.burrow.net",
                    "oidc_issuer": "https://auth.burrow.net/application/o/ts/"
                }))
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base_url = Url::parse(&format!("http://{}", listener.local_addr()?))?;
        let server = tokio::spawn(async move { axum::serve(listener, router).await });

        let client = Client::builder().build()?;
        let discovery = discover_tailnet_at(&client, "contact@burrow.net", &base_url).await?;
        assert_eq!(discovery.provider, TailnetProvider::Headscale);
        assert_eq!(discovery.authority, "https://ts.burrow.net");
        assert_eq!(discovery.domain, "burrow.net");

        server.abort();
        Ok(())
    }

    #[tokio::test]
    async fn falls_back_to_webfinger_authority() -> Result<()> {
        let router = Router::new()
            .route(
                TAILNET_DISCOVERY_PATH,
                get(|| async { (StatusCode::NOT_FOUND, "") }),
            )
            .route(
                WEBFINGER_PATH,
                get(|| async {
                    axum::Json(json!({
                        "subject": "acct:contact@burrow.net",
                        "links": [
                            {
                                "rel": TAILNET_DISCOVERY_REL,
                                "href": "https://ts.burrow.net"
                            }
                        ]
                    }))
                }),
            );

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base_url = Url::parse(&format!("http://{}", listener.local_addr()?))?;
        let server = tokio::spawn(async move { axum::serve(listener, router).await });

        let client = Client::builder().build()?;
        let discovery = discover_tailnet_at(&client, "contact@burrow.net", &base_url).await?;
        assert_eq!(discovery.provider, TailnetProvider::Headscale);
        assert_eq!(discovery.authority, "https://ts.burrow.net");

        server.abort();
        Ok(())
    }
}
