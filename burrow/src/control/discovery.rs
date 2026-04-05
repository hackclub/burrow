use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::TailnetProvider;

pub const TAILNET_DISCOVERY_REL: &str = "https://burrow.net/rel/tailnet-control-server";
const TAILNET_DISCOVERY_PATH: &str = "/.well-known/burrow-tailnet";
const WEBFINGER_PATH: &str = "/.well-known/webfinger";
const MANAGED_TAILSCALE_AUTHORITY: &str = "controlplane.tailscale.com";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TailnetDiscovery {
    pub domain: String,
    pub provider: TailnetProvider,
    pub authority: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oidc_issuer: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TailnetAuthorityProbe {
    pub authority: String,
    pub status_code: i32,
    pub summary: String,
    pub detail: String,
    pub reachable: bool,
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
    info!(%email, %domain, "tailnet discovery requested");
    let base_url = Url::parse(&format!("https://{domain}"))
        .with_context(|| format!("invalid discovery domain {domain}"))?;
    let client = Client::builder()
        .user_agent("burrow-tailnet-discovery")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("failed to build tailnet discovery client")?;
    discover_tailnet_at(&client, email, &base_url).await
}

pub fn normalize_authority(authority: &str) -> String {
    let trimmed = authority.trim();
    if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    }
}

pub fn is_managed_tailscale_authority(authority: &str) -> bool {
    let normalized = normalize_authority(authority)
        .trim_end_matches('/')
        .to_ascii_lowercase();
    normalized == format!("https://{MANAGED_TAILSCALE_AUTHORITY}")
        || normalized == format!("http://{MANAGED_TAILSCALE_AUTHORITY}")
}

pub async fn probe_tailnet_authority(authority: &str) -> Result<TailnetAuthorityProbe> {
    let authority = normalize_authority(authority);
    if is_managed_tailscale_authority(&authority) {
        return Ok(TailnetAuthorityProbe {
            authority,
            status_code: 200,
            summary: "Tailscale-managed control plane".to_owned(),
            detail: "Using Tailscale's default login server.".to_owned(),
            reachable: true,
        });
    }

    let base_url =
        Url::parse(&authority).with_context(|| format!("invalid tailnet authority {authority}"))?;
    let client = Client::builder()
        .user_agent("burrow-tailnet-probe")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("failed to build tailnet authority probe client")?;

    if let Some(status) =
        probe_url(&client, base_url.join("/health")?, &authority, "Tailnet server reachable").await?
    {
        return Ok(status);
    }

    if let Some(status) = probe_url(
        &client,
        base_url.clone(),
        &authority,
        "Tailnet server reachable",
    )
    .await?
    {
        return Ok(status);
    }

    Err(anyhow!("could not connect to the server"))
}

pub async fn discover_tailnet_at(
    client: &Client,
    email: &str,
    base_url: &Url,
) -> Result<TailnetDiscovery> {
    let domain = email_domain(email)?;
    debug!(%email, %domain, base_url = %base_url, "starting tailnet domain discovery");

    if let Some(discovery) = discover_well_known(client, base_url).await? {
        info!(
            %email,
            %domain,
            authority = %discovery.authority,
            provider = ?discovery.provider,
            "resolved tailnet discovery from well-known document"
        );
        return Ok(TailnetDiscovery { domain, ..discovery });
    }

    if let Some(authority) = discover_webfinger(client, email, base_url).await? {
        info!(%email, %domain, %authority, "resolved tailnet discovery from webfinger");
        return Ok(TailnetDiscovery {
            domain,
            provider: inferred_provider(Some(&authority), None),
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

pub fn inferred_provider(
    authority: Option<&str>,
    explicit: Option<&TailnetProvider>,
) -> TailnetProvider {
    if matches!(explicit, Some(TailnetProvider::Burrow)) {
        return TailnetProvider::Burrow;
    }
    if authority.is_some_and(is_managed_tailscale_authority) {
        return TailnetProvider::Tailscale;
    }
    TailnetProvider::Headscale
}

async fn discover_well_known(client: &Client, base_url: &Url) -> Result<Option<TailnetDiscovery>> {
    let url = base_url
        .join(TAILNET_DISCOVERY_PATH)
        .context("failed to build tailnet discovery URL")?;
    debug!(%url, "requesting tailnet well-known document");
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
    debug!(%email, url = %url, "requesting tailnet webfinger document");

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

async fn probe_url(
    client: &Client,
    url: Url,
    authority: &str,
    summary: &str,
) -> Result<Option<TailnetAuthorityProbe>> {
    let response = match client
        .get(url)
        .header("accept", "application/json")
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return Ok(None),
    };

    let status = response.status();
    if !status.is_success() {
        return Ok(None);
    }

    let detail = response.text().await.unwrap_or_default().trim().to_owned();
    Ok(Some(TailnetAuthorityProbe {
        authority: authority.to_owned(),
        status_code: i32::from(status.as_u16()),
        summary: summary.to_owned(),
        detail,
        reachable: true,
    }))
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

    #[test]
    fn detects_managed_tailscale_authority() {
        assert!(is_managed_tailscale_authority("controlplane.tailscale.com"));
        assert!(is_managed_tailscale_authority("https://controlplane.tailscale.com/"));
        assert!(!is_managed_tailscale_authority("https://ts.burrow.net"));
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

    #[tokio::test]
    async fn probes_custom_authority() -> Result<()> {
        let router = Router::new().route("/health", get(|| async { "ok" }));
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let authority = format!("http://{}", listener.local_addr()?);
        let server = tokio::spawn(async move { axum::serve(listener, router).await });

        let status = probe_tailnet_authority(&authority).await?;
        assert_eq!(status.authority, authority);
        assert_eq!(status.status_code, 200);
        assert!(status.reachable);

        server.abort();
        Ok(())
    }
}
