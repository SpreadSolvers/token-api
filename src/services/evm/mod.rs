mod erc20;
pub mod error;

use crate::{
    repositories::Repository,
    services::evm::{erc20::ERC20, error::EvmTokenServiceError},
};
use actix_web::web;
use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
};
use tap_caip::{AccountId, ChainId};

use crate::{
    repositories::sqlite::evm_token::SqliteEvmTokenRepository,
    token::{EvmTokenDetails, Token, TokenId},
};

#[derive(Clone)]
pub struct EvmTokenService {
    repository: SqliteEvmTokenRepository,
}

const EVM_NAMESPACE: &str = "eip155";

impl EvmTokenService {
    pub fn new(repository: SqliteEvmTokenRepository) -> Self {
        Self { repository }
    }

    pub async fn get_or_fetch_token(
        &self,
        chain_id: i32,
        address: Address,
        rpc_url: String,
    ) -> Result<Token<EvmTokenDetails>, EvmTokenServiceError> {
        let token_id: TokenId = TokenId::new(
            ChainId::new(EVM_NAMESPACE, &chain_id.to_string()).unwrap(),
            &address.to_string(),
        )?;

        // Run potentially blocking repository access on a blocking thread pool.
        // Clone the repository so we don't capture &self into the closure.
        let repo = self.repository.clone();
        let token = web::block(move || repo.get(token_id)).await??;

        if let Some(token) = token {
            return Ok(token);
        }

        let token = Self::fetch_token(chain_id, address, rpc_url).await?;

        self.repository.save(&token)?;

        Ok(token)
    }

    async fn fetch_token(
        chain_id: i32,
        address: Address,
        rpc_url: String,
    ) -> Result<Token<EvmTokenDetails>, EvmTokenServiceError> {
        let provider = ProviderBuilder::new().connect(&rpc_url).await?;

        let chain_id_from_provider = provider.get_chain_id().await?;

        if chain_id_from_provider != chain_id as u64 {
            return Err(EvmTokenServiceError::ChainIdMismatch(
                chain_id_from_provider,
                chain_id as u64,
            ));
        }

        let token = ERC20::new(address, &provider);

        let multicall = provider
            .multicall()
            .add(token.name())
            .add(token.symbol())
            .add(token.decimals());

        let Ok((name, symbol, decimals)) = multicall.aggregate().await else {
            return Err(EvmTokenServiceError::Multicall(
                "Failed to get multicall result".to_string(),
            ));
        };

        let chain_id = ChainId::new(EVM_NAMESPACE, &chain_id.to_string())
            .expect("Failed to create CAIP chain id")
            .clone();

        let account_id = AccountId::new(chain_id.clone(), &address.to_string())
            .expect("Failed to create account id");

        let token: Token<EvmTokenDetails> = Token::<EvmTokenDetails> {
            id: account_id,
            details: EvmTokenDetails { chain_id, address },
            symbol,
            decimals,
            name,
        };

        Ok(token)
    }
}
