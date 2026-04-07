mod erc20;
pub mod error;

#[cfg(test)]
mod igra_tests;

use crate::{
    repositories::Repository,
    services::evm::{
        erc20::ERC20::{self, decimalsCall, nameCall, symbolCall},
        error::EvmTokenServiceError,
    },
};
use actix_web::web;
use alloy::{
    primitives::{Address, B256, b256, keccak256},
    providers::{MULTICALL3_ADDRESS, Provider, ProviderBuilder},
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

/// Keccak-256 of the canonical Multicall3 **deployed bytecode** (matches `codeHash` from `eth_getAccount`).
const MULTICALL3_DEPLOYED_CODE_HASH: B256 =
    b256!("0xd5c15df687b16f2ff992fc8d767b4216323184a2bbc6ee2f9c398c318e770891");

/// True if Multicall3 is deployed at [`MULTICALL3_ADDRESS`]: prefer `eth_getAccount.codeHash`, fall
/// back to `keccak256(eth_getCode)` when the node disallows or omits `eth_getAccount`.
async fn multicall3_matches_canonical_deployment<P: Provider>(
    provider: &P,
) -> Result<bool, EvmTokenServiceError> {
    match provider.get_account(MULTICALL3_ADDRESS).await {
        Ok(acc) => Ok(acc.code_hash == MULTICALL3_DEPLOYED_CODE_HASH),
        Err(_) => {
            let code = provider
                .get_code_at(MULTICALL3_ADDRESS)
                .await
                .map_err(EvmTokenServiceError::Chain)?;
            Ok(!code.is_empty() && keccak256(&code) == MULTICALL3_DEPLOYED_CODE_HASH)
        }
    }
}

fn require_decoded<T, E: std::fmt::Display>(
    result: Result<T, E>,
    field: &'static str,
) -> Result<T, EvmTokenServiceError> {
    result.map_err(|e| {
        EvmTokenServiceError::Multicall(format!("Failed to fetch and decode {field} call: {e}"))
    })
}

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
        let provider = ProviderBuilder::new().connect_client(rpc.clone());

        let chain_id_from_provider: u64 = provider.get_chain_id().await?;

        if chain_id_from_provider != chain_id as u64 {
            return Err(EvmTokenServiceError::ChainIdMismatch(
                chain_id_from_provider,
                chain_id as u64,
            ));
        }

        let token_contract = ERC20::new(address, &provider);
        let multicall = provider
            .multicall()
            .add(token_contract.name())
            .add(token_contract.symbol())
            .add(token_contract.decimals());

        let (name, symbol, decimals) = match multicall.aggregate().await {
            Ok(fields) => fields,
            Err(multicall_err) => {
                if multicall3_matches_canonical_deployment(&provider).await? {
                    return Err(EvmTokenServiceError::Multicall(multicall_err.to_string()));
                }
                Self::fetch_token_metadata_with_rpc_batch(address, &provider).await?
            }
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

    /// Reads ERC-20 metadata via three parallel `eth_call`s (no Multicall3).
    async fn fetch_token_metadata_with_rpc_batch<P: Provider>(
        token_address: Address,
        provider: &P,
    ) -> Result<(String, String, u8), EvmTokenServiceError> {
        let token_contract = ERC20::new(token_address, provider);

        let name_call = token_contract.name().into_transaction_request();
        let symbol_call = token_contract.symbol().into_transaction_request();
        let decimals_call = token_contract.decimals().into_transaction_request();

        let (name_result, symbol_result, decimals_result) = tokio::try_join!(
            provider.call(name_call).decode_resp::<nameCall>(),
            provider.call(symbol_call).decode_resp::<symbolCall>(),
            provider.call(decimals_call).decode_resp::<decimalsCall>(),
        )?;

        let (name, symbol, decimals) = (
            require_decoded(name_result, "name")?,
            require_decoded(symbol_result, "symbol")?,
            require_decoded(decimals_result, "decimals")?,
        );

        Ok((name, symbol, decimals))
    }
}
