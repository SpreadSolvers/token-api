use std::{
    collections::HashMap,
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::{
    rpc::client::RpcClient,
    transports::{http::Http, layers::FallbackLayer, utils::guess_local_url},
};
use thiserror::Error;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use url::Url;

use crate::{services::chainlist::ChainlistService, types::ChainId};

/// Parallel transport fan-out for FallbackLayer (Alloy ranks latency + stability).
const FALLBACK_ACTIVE_CAP: usize = 32;

#[derive(Clone)]
pub struct ProviderService {
    chainlist: ChainlistService,
    provider_ttl: Duration,
    cache: Arc<RwLock<HashMap<ChainId, CachedClient>>>,
}

struct CachedClient {
    created: Instant,
    client: RpcClient,
}

#[derive(Debug, Error)]
pub enum ProviderServiceError {
    #[error(transparent)]
    Chainlist(#[from] reqwest::Error),
    #[error("invalid RPC URL: {0}")]
    Url(#[from] url::ParseError),
}

impl ProviderService {
    pub fn new(chainlist: ChainlistService, provider_ttl: Duration) -> Self {
        Self {
            chainlist,
            provider_ttl,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Cached [`RpcClient`] over Chainlist RPCs using Alloy `FallbackLayer` (keeps transport rankings until TTL).
    pub async fn rpc_client_for_chain(
        &self,
        chain_id: ChainId,
    ) -> Result<Option<RpcClient>, ProviderServiceError> {
        {
            let guard = self.cache.read().await;
            if let Some(c) = guard.get(&chain_id) {
                if c.created.elapsed() < self.provider_ttl {
                    return Ok(Some(c.client.clone()));
                }
            }
        }

        let mut guard = self.cache.write().await;
        if let Some(c) = guard.get(&chain_id) {
            if c.created.elapsed() < self.provider_ttl {
                return Ok(Some(c.client.clone()));
            }
        }

        let Some(urls) = self.chainlist.rpc_urls_for_chain(chain_id).await? else {
            guard.remove(&chain_id);
            return Ok(None);
        };

        if urls.is_empty() {
            guard.remove(&chain_id);
            return Ok(None);
        }

        let client = build_fallback_rpc_client(&urls)?;
        let cloned = client.clone();

        guard.insert(
            chain_id,
            CachedClient {
                created: Instant::now(),
                client,
            },
        );
        Ok(Some(cloned))
    }
}

fn build_fallback_rpc_client(urls: &[String]) -> Result<RpcClient, ProviderServiceError> {
    let transports: Vec<Http<reqwest::Client>> = urls
        .iter()
        .map(|s| Url::parse(s).map(Http::new))
        .collect::<Result<_, url::ParseError>>()?;

    let active = NonZeroUsize::new(transports.len().min(FALLBACK_ACTIVE_CAP).max(1))
        .expect("Active transport count must be non-zero");
    let layer = FallbackLayer::default().with_active_transport_count(active);
    let transport = ServiceBuilder::new().layer(layer).service(transports);
    let is_local = urls.iter().any(|u| guess_local_url(u));
    Ok(RpcClient::builder().transport(transport, is_local))
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;
    use std::time::Duration;

    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    fn chainlist_with_rpc(url: &str, chain_id: ChainId) -> serde_json::Value {
        json!([{
            "name": "Test",
            "chain": "TST",
            "chainId": chain_id,
            "rpc": [{ "url": url }],
        }])
    }

    #[test]
    fn build_fallback_rejects_malformed_url() {
        let urls = vec!["https://ok.example".to_string(), ":::bad".to_string()];
        assert!(build_fallback_rpc_client(&urls).is_err());
    }

    #[tokio::test]
    async fn rpc_client_none_when_chain_unknown() {
        let list = MockServer::start().await;
        let body = chainlist_with_rpc("https://rpc.example/", 42);
        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&list)
            .await;

        let chainlist = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            format!("{}/rpcs.json", list.uri()),
        );
        let providers = ProviderService::new(chainlist, Duration::from_secs(3600));
        assert!(providers.rpc_client_for_chain(999).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn rpc_client_none_when_rpc_list_empty() {
        let list = MockServer::start().await;
        let body = json!([{
            "name": "Empty",
            "chain": "EMP",
            "chainId": 42,
            "rpc": [],
        }]);

        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&list)
            .await;

        let chainlist = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            format!("{}/rpcs.json", list.uri()),
        );
        let providers = ProviderService::new(chainlist, Duration::from_secs(3600));
        assert!(providers.rpc_client_for_chain(42).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn rpc_client_some_when_urls_valid() {
        let list = MockServer::start().await;
        let body = chainlist_with_rpc("https://ethereum.publicnode.com", 1);

        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&list)
            .await;

        let chainlist = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            format!("{}/rpcs.json", list.uri()),
        );
        let providers = ProviderService::new(chainlist, Duration::from_secs(3600));
        assert!(providers.rpc_client_for_chain(1).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn rpc_client_chainlist_http_fetched_once_while_provider_cache_warm() {
        let list = MockServer::start().await;
        let body = chainlist_with_rpc("https://rpc.cache.test/", 42);

        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .expect(1)
            .mount(&list)
            .await;

        let chainlist = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            format!("{}/rpcs.json", list.uri()),
        );
        let providers = ProviderService::new(chainlist, Duration::from_secs(3600));

        providers.rpc_client_for_chain(42).await.unwrap().unwrap();
        providers.rpc_client_for_chain(42).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn rpc_client_err_when_any_rpc_url_unparseable() {
        let list = MockServer::start().await;
        let body = json!([{
            "name": "Bad",
            "chain": "BAD",
            "chainId": 42,
            "rpc": [{ "url": ":::not-a-url" }],
        }]);

        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&list)
            .await;

        let chainlist = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            format!("{}/rpcs.json", list.uri()),
        );
        let providers = ProviderService::new(chainlist, Duration::from_secs(3600));

        let err = providers
            .rpc_client_for_chain(42)
            .await
            .expect_err("url parse");
        assert!(matches!(err, ProviderServiceError::Url(_)));
    }

    #[tokio::test]
    async fn rpc_client_rebuilds_after_provider_ttl() {
        let list = MockServer::start().await;
        let body = chainlist_with_rpc("https://rpc.ttl.test/", 42);

        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&list)
            .await;

        let chainlist = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            format!("{}/rpcs.json", list.uri()),
        );
        let providers = ProviderService::new(chainlist, Duration::from_millis(30));

        let a = providers.rpc_client_for_chain(42).await.unwrap().unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
        let b = providers.rpc_client_for_chain(42).await.unwrap().unwrap();

        assert_ne!(
            std::ptr::from_ref(a.deref()),
            std::ptr::from_ref(b.deref()),
            "expired provider TTL must allocate a new RpcClient"
        );
    }
}
