use std::sync::Arc;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use tokio::sync::RwLock;

use crate::{
    chainlist::{CHAINLIST_API_URL, Chain, fetch_chains},
    types::ChainId,
};

#[derive(Clone)]
pub struct ChainlistService {
    inner: Arc<Inner>,
}

struct Inner {
    client: reqwest::Client,
    chains_url: String,
    ttl: ChronoDuration,
    cache: RwLock<Option<CacheEntry>>,
}

struct CacheEntry {
    chains: Arc<Vec<Chain>>,
    fetched_at: DateTime<Utc>,
}

impl ChainlistService {
    pub fn new(ttl: std::time::Duration) -> Self {
        Self::with_client_and_url(ttl, reqwest::Client::new(), CHAINLIST_API_URL)
    }

    pub fn with_client_and_url(
        ttl: std::time::Duration,
        client: reqwest::Client,
        chains_url: impl Into<String>,
    ) -> Self {
        let ttl = ChronoDuration::from_std(ttl).expect("TTL must fit in chrono::Duration");
        Self {
            inner: Arc::new(Inner {
                client,
                chains_url: chains_url.into(),
                ttl,
                cache: RwLock::new(None),
            }),
        }
    }

    pub async fn chains(&self) -> Result<Vec<Chain>, reqwest::Error> {
        self.chains_shared().await.map(|v| (*v).clone())
    }

    pub async fn chains_shared(&self) -> Result<Arc<Vec<Chain>>, reqwest::Error> {
        let inner = self.inner.as_ref();

        {
            let guard = inner.cache.read().await;
            if let Some(entry) = guard.as_ref() {
                if Self::is_fresh(entry, inner.ttl) {
                    return Ok(Arc::clone(&entry.chains));
                }
            }
        }

        let mut guard = inner.cache.write().await;
        if let Some(entry) = guard.as_ref() {
            if Self::is_fresh(entry, inner.ttl) {
                return Ok(Arc::clone(&entry.chains));
            }
        }

        let list = fetch_chains(&inner.client, inner.chains_url.as_str()).await?;
        let chains = Arc::new(list);
        *guard = Some(CacheEntry {
            chains: Arc::clone(&chains),
            fetched_at: Utc::now(),
        });
        Ok(chains)
    }

    pub async fn get_chain_data(&self, chain_id: ChainId) -> Result<Option<Chain>, reqwest::Error> {
        let chains = self.chains_shared().await?;
        Ok(chains.iter().find(|c| c.chain_id == chain_id).cloned())
    }

    /// Trimmed, non-empty RPC URLs from Chainlist for `chain_id` (no liveness checks).
    pub async fn rpc_urls_for_chain(
        &self,
        chain_id: ChainId,
    ) -> Result<Option<Vec<String>>, reqwest::Error> {
        let Some(chain) = self.get_chain_data(chain_id).await? else {
            return Ok(None);
        };
        Ok(Some(trimmed_rpc_urls(chain)))
    }

    fn is_fresh(entry: &CacheEntry, ttl: ChronoDuration) -> bool {
        Utc::now().signed_duration_since(entry.fetched_at) < ttl
    }
}

fn trimmed_rpc_urls(chain: Chain) -> Vec<String> {
    chain
        .rpc
        .into_iter()
        .map(|r| r.url.trim().to_owned())
        .filter(|u| !u.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    fn sample_chain_json() -> serde_json::Value {
        json!([{
            "name": "Test Net",
            "chain": "TEST",
            "chainId": 42,
            "rpc": [{ "url": "https://rpc.test" }],
        }])
    }

    #[tokio::test]
    async fn chains_shared_fetches_once_while_cache_fresh() {
        let server = MockServer::start().await;
        let url = format!("{}/rpcs.json", server.uri());
        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_chain_json()))
            .expect(1)
            .mount(&server)
            .await;

        let svc = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            url,
        );
        let a = svc.chains_shared().await.unwrap();
        let b = svc.chains_shared().await.unwrap();
        assert_eq!(a.len(), 1);
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[tokio::test]
    async fn chains_shared_refetches_after_ttl() {
        let server = MockServer::start().await;
        let url = format!("{}/rpcs.json", server.uri());
        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_chain_json()))
            .expect(2)
            .mount(&server)
            .await;

        let svc = ChainlistService::with_client_and_url(
            Duration::from_millis(20),
            reqwest::Client::new(),
            url,
        );
        svc.chains_shared().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        svc.chains_shared().await.unwrap();
    }

    #[tokio::test]
    async fn get_chain_data_returns_matching_chain() {
        let server = MockServer::start().await;
        let url = format!("{}/rpcs.json", server.uri());
        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_chain_json()))
            .mount(&server)
            .await;

        let svc = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            url,
        );
        let c = svc.get_chain_data(42).await.unwrap().expect("chain 42");
        assert_eq!(c.chain_id, 42);
        assert_eq!(c.name, "Test Net");
    }

    #[tokio::test]
    async fn get_chain_data_returns_none_for_unknown_id() {
        let server = MockServer::start().await;
        let url = format!("{}/rpcs.json", server.uri());
        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_chain_json()))
            .mount(&server)
            .await;

        let svc = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            url,
        );
        assert!(svc.get_chain_data(999).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn rpc_urls_for_chain_trims_and_filters_empty() {
        let server = MockServer::start().await;
        let chain_json = json!([{
            "name": "Trim",
            "chain": "TRM",
            "chainId": 42,
            "rpc": [
                { "url": "   " },
                { "url": " https://a.test " },
                { "url": "https://b.test" },
            ],
        }]);
        let url = format!("{}/rpcs.json", server.uri());
        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(chain_json))
            .mount(&server)
            .await;

        let svc = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            url,
        );
        let urls = svc.rpc_urls_for_chain(42).await.unwrap().expect("urls");
        assert_eq!(
            urls,
            vec!["https://a.test".to_string(), "https://b.test".to_string()]
        );
    }

    #[tokio::test]
    async fn rpc_urls_for_chain_none_when_missing_chain() {
        let server = MockServer::start().await;
        let url = format!("{}/rpcs.json", server.uri());
        Mock::given(method("GET"))
            .and(path("/rpcs.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_chain_json()))
            .mount(&server)
            .await;

        let svc = ChainlistService::with_client_and_url(
            Duration::from_secs(3600),
            reqwest::Client::new(),
            url,
        );
        assert!(svc.rpc_urls_for_chain(999).await.unwrap().is_none());
    }
}
