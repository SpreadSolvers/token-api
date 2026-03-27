use actix_web::{HttpResponse, Responder, get};
use alloy::primitives::Address;
use jsonrpc_v2::Params;
use log::{debug, error};
use serde::Deserialize;

use crate::{
    services::evm::EvmTokenService,
    token::{EvmTokenDetails, Token},
};

#[derive(Deserialize)]
pub struct GetEvmTokenMetadata {
    chain_id: i32,
    address: String,
}

pub async fn get_evm_token_metadata_with_default_rpc_url(
    Params(params): Params<GetEvmTokenMetadata>,
    evm_token_service: jsonrpc_v2::Data<EvmTokenService>,
) -> Result<Token<EvmTokenDetails>, jsonrpc_v2::Error> {
    let rpc_url = "https://virginia.rpc.blxrbdn.com".to_string();
    get_evm_token_metadata(Params((params, rpc_url).into()), evm_token_service).await
}

#[derive(Deserialize)]
pub struct GetEvmTokenMetadataParamsWithRpcUrl {
    chain_id: i32,
    address: String,
    rpc_url: String,
}

impl From<(GetEvmTokenMetadata, String)> for GetEvmTokenMetadataParamsWithRpcUrl {
    fn from((params, rpc_url): (GetEvmTokenMetadata, String)) -> Self {
        Self {
            chain_id: params.chain_id,
            address: params.address,
            rpc_url: rpc_url,
        }
    }
}

pub async fn get_evm_token_metadata(
    Params(params): Params<GetEvmTokenMetadataParamsWithRpcUrl>,
    evm_token_service: jsonrpc_v2::Data<EvmTokenService>,
) -> Result<Token<EvmTokenDetails>, jsonrpc_v2::Error> {
    let chain_id = params.chain_id;
    let evm_address = params.address;
    let rpc_url = params.rpc_url;

    debug!("RPC URL: {:?}", rpc_url);
    debug!("Chain ID: {:?}", chain_id);
    debug!("EVM address: {:?}", evm_address);

    let Ok(checked_address) = evm_address.parse::<Address>() else {
        return Err("Invalid EVM address".into());
    };

    let token = evm_token_service
        .get_or_fetch_token(chain_id, checked_address, rpc_url)
        .await;

    return match token {
        Ok(token) => Ok(token),

        Err(e) => {
            error!("Error getting EVM token: {:?}", e);
            return Err(e.into());
        }
    };
}

#[get("/")]
pub async fn hello_world() -> impl Responder {
    HttpResponse::Ok().body("Hello, TokenAPI!")
}
