use serde::Deserialize;

use crate::types::ChainId;

pub(crate) const CHAINLIST_API_URL: &str = "https://chainlist.org/rpcs.json";

#[derive(Deserialize, Debug, Clone)]
pub struct Chain {
    pub name: String,
    pub chain: String,
    #[serde(rename = "chainId")]
    pub chain_id: ChainId,
    pub rpc: Vec<Rpc>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Rpc {
    pub url: String,
}

pub async fn fetch_chains(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<Chain>, reqwest::Error> {
    let response = client.get(url).send().await?;
    let body = response.json::<Vec<Chain>>().await?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_chainlist_response() {
        let client = reqwest::Client::new();
        let response = fetch_chains(&client, CHAINLIST_API_URL).await;

        match response {
            Ok(response) => println!("{:?}", response),
            Err(e) => {
                eprintln!("Error: {:?}", e);
                assert!(false);
            }
        }
    }
}
