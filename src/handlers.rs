use actix_web::{HttpResponse, Responder, get};
use alloy::primitives::Address;
use alloy::rpc::client::RpcClient;
use jsonrpc_v2::Params;
use log::{debug, error};
use serde::Deserialize;

use crate::{
    services::{
        evm::EvmTokenService,
        provider::{ProviderService, ProviderServiceError},
    },
    token::{EvmTokenDetails, Token},
};

#[derive(Deserialize)]
pub struct GetEvmTokenMetadata {
    chain_id: i64,
    address: String,
}

pub async fn get_evm_token_metadata(
    Params(params): Params<GetEvmTokenMetadata>,
    evm_token_service: jsonrpc_v2::Data<EvmTokenService>,
    provider_service: jsonrpc_v2::Data<ProviderService>,
) -> Result<Token<EvmTokenDetails>, jsonrpc_v2::Error> {
    let rpc = provider_service
        .rpc_client_for_chain(params.chain_id)
        .await
        .map_err(provider_error_to_jsonrpc)?
        .ok_or_else(|| format!("No RPC URLs for chain {}", params.chain_id))?;

    get_evm_token_metadata_with_rpc_client(params, rpc, evm_token_service).await
}

#[derive(Deserialize)]
pub struct GetEvmTokenMetadataParamsWithRpcUrl {
    chain_id: i64,
    address: String,
    rpc_url: String,
}

pub async fn get_evm_token_metadata_with_rpc_url(
    Params(params): Params<GetEvmTokenMetadataParamsWithRpcUrl>,
    evm_token_service: jsonrpc_v2::Data<EvmTokenService>,
) -> Result<Token<EvmTokenDetails>, jsonrpc_v2::Error> {
    let url = params
        .rpc_url
        .parse::<reqwest::Url>()
        .map_err(|e| e.to_string())?;

    let rpc = RpcClient::new_http(url);

    get_evm_token_metadata_with_rpc_client(
        GetEvmTokenMetadata {
            chain_id: params.chain_id,
            address: params.address,
        },
        rpc,
        evm_token_service,
    )
    .await
}

async fn get_evm_token_metadata_with_rpc_client(
    params: GetEvmTokenMetadata,
    rpc: RpcClient,
    evm_token_service: jsonrpc_v2::Data<EvmTokenService>,
) -> Result<Token<EvmTokenDetails>, jsonrpc_v2::Error> {
    let chain_id = params.chain_id;
    let evm_address = params.address;

    debug!("Chain ID: {:?}", chain_id);
    debug!("EVM address: {:?}", evm_address);

    let Ok(checked_address) = evm_address.parse::<Address>() else {
        return Err("Invalid EVM address".into());
    };

    let token = evm_token_service
        .get_or_fetch_token(chain_id, checked_address, rpc)
        .await;

    match token {
        Ok(token) => Ok(token),
        Err(e) => {
            error!("Error getting EVM token: {:?}", e);
            Err(e.into())
        }
    }
}

fn provider_error_to_jsonrpc(e: ProviderServiceError) -> jsonrpc_v2::Error {
    e.to_string().into()
}

#[get("/")]
pub async fn hello_world() -> impl Responder {
    HttpResponse::Ok().body("Hello, TokenAPI!")
}
