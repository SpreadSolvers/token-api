use std::error::Error;

use crate::{erc20::ERC20, schema::evm_tokens};
use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
};
use diesel::prelude::*;
use log::error;
use serde::{Deserialize, Serialize};
use tap_caip::{AccountId, ChainId};

#[derive(Queryable, Insertable)]
#[diesel(table_name = evm_tokens)]
pub struct DbEvmToken {
    id: String,
    chain_id: i32,
    address: String,
    symbol: String,
    decimals: i32,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvmToken {
    pub id: AccountId,
    pub chain_id: i32,
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
    pub name: String,
}

pub async fn get_token_data_from_chain(
    chain_id: i32,
    address: Address,
) -> Result<EvmToken, Box<dyn Error>> {
    let rpc_url = "https://eth.llamarpc.com";

    let Ok(provider) = ProviderBuilder::new().connect(rpc_url).await else {
        error!("Failed to connect to RPC");
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to connect to RPC",
        )));
    };

    let token = ERC20::new(address, &provider);

    let multicall = provider
        .multicall()
        .add(token.name())
        .add(token.symbol())
        .add(token.decimals());

    let Ok((name, symbol, decimals)) = multicall.aggregate().await else {
        error!("Failed to get result");
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get result",
        )));
    };

    let caip_chain_id = ChainId::new("eip155", &chain_id.to_string()).unwrap();
    let account_id =
        AccountId::new(caip_chain_id, &address.to_string()).expect("Failed to create asset id");

    let token: EvmToken = EvmToken {
        id: account_id,
        chain_id: chain_id,
        address: address,
        symbol,
        decimals,
        name,
    };

    Ok(token)
}
