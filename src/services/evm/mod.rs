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
    rpc::client::RpcClient,
};
use tap_caip::{AccountId, ChainId as CaipChainId};

use crate::{
    repositories::sqlite::evm_token::SqliteEvmTokenRepository,
    token::{Token, TokenId},
    types::ChainId,
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
        chain_id: ChainId,
        address: Address,
        rpc: RpcClient,
    ) -> Result<Token, EvmTokenServiceError> {
        let token_id: TokenId = TokenId::new(
            CaipChainId::new(EVM_NAMESPACE, &chain_id.to_string()).unwrap(),
            &address.to_string(),
        )?;

        // Run potentially blocking repository access on a blocking thread pool.
        // Clone the repository so we don't capture &self into the closure.
        let repo = self.repository.clone();
        let token = web::block(move || repo.get(token_id)).await??;

        if let Some(token) = token {
            return Ok(token);
        }

        let token = Self::fetch_token(chain_id, address, rpc).await?;

        self.repository.save(&token)?;

        Ok(token)
    }

    async fn fetch_token(
        chain_id: ChainId,
        address: Address,
        rpc: RpcClient,
    ) -> Result<Token, EvmTokenServiceError> {
        let provider = ProviderBuilder::new().connect_client(rpc);

        let chain_id_from_provider: u64 = provider.get_chain_id().await?;

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

        let chain_id = CaipChainId::new(EVM_NAMESPACE, &chain_id.to_string())
            .expect("Failed to create CAIP chain id")
            .clone();

        let account_id = AccountId::new(chain_id.clone(), &address.to_string())
            .expect("Failed to create account id");

        let token: Token = Token {
            id: account_id,
            name,
            symbol,
            decimals,
        };

        Ok(token)
    }
}
