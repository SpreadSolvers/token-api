use std::error::Error;

use crate::{erc20::ERC20, schema::evm_tokens};
use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
};
use diesel::prelude::*;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tap_caip::{AccountId, ChainId};

#[derive(Queryable, Insertable)]
#[diesel(table_name = evm_tokens)]
pub struct DbEvmToken {
    pub id: String,
    // TODO: change to u64
    pub chain_id: i32,
    pub address: String,
    pub symbol: String,
    pub decimals: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvmToken {
    pub id: AccountId,
    pub chain_id: u64,
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
    pub name: String,
}

pub async fn get_token_data_from_chain(
    chain_id: i32,
    address: Address,
    rpc_url: String,
) -> Result<EvmToken, Box<dyn Error>> {
    let provider = ProviderBuilder::new().connect(&rpc_url).await?;

    let chain_id_from_provider = provider.get_chain_id().await?;

    if chain_id_from_provider != chain_id as u64 {
        error!(
            "Chain ID mismatch: {:?} != {:?}",
            chain_id_from_provider, chain_id
        );
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Chain ID mismatch",
        )));
    }

    let token = ERC20::new(address, &provider);

    let multicall = provider
        .multicall()
        .add(token.name())
        .add(token.symbol())
        .add(token.decimals());

    let Ok((name, symbol, decimals)) = multicall.aggregate().await else {
        error!("Failed to get multicall result");
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get result",
        )));
    };

    let caip_chain_id =
        ChainId::new("eip155", &chain_id.to_string()).expect("Failed to create CAIP chain id");

    let account_id =
        AccountId::new(caip_chain_id, &address.to_string()).expect("Failed to create account id");

    let token: EvmToken = EvmToken {
        id: account_id,
        chain_id: chain_id as u64,
        address: address,
        symbol,
        decimals,
        name,
    };

    Ok(token)
}

pub fn save_evm_token(
    connection: &mut SqliteConnection,
    token: &EvmToken,
) -> Result<(), Box<dyn Error>> {
    use crate::schema::evm_tokens;
    use diesel::prelude::*;

    info!("Saving EVM token with id: {:?}", token.id);

    let new_token: DbEvmToken = DbEvmToken {
        id: token.id.to_string(),
        chain_id: token.chain_id as i32,
        address: token.address.to_string(),
        symbol: token.symbol.clone(),
        decimals: token.decimals as i32,
        name: token.name.clone(),
    };

    let write = diesel::insert_into(evm_tokens::table)
        .values(&new_token)
        .execute(connection);

    match write {
        Ok(_) => {
            info!("EVM token saved successfully");
            return Ok(());
        }
        Err(e) => {
            error!("Error saving EVM token: {:?}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Error saving EVM token",
            )));
        }
    };
}

pub fn find_token_by_id(
    connection: &mut SqliteConnection,
    id: AccountId,
) -> Result<EvmToken, diesel::result::Error> {
    debug!("Finding EVM token by id: {:?}", id.to_string());

    let token = evm_tokens::table
        .find(id.to_string())
        .first::<DbEvmToken>(connection)
        .optional()?;

    match token {
        Some(token) => {
            return Ok(EvmToken {
                id,
                chain_id: token.chain_id as u64,
                address: token
                    .address
                    .parse::<Address>()
                    .expect("Failed to create address"),
                symbol: token.symbol,
                decimals: token.decimals as u8,
                name: token.name,
            });
        }
        None => {
            debug!("Token not found by id: {:?}", id.to_string());
            return Err(diesel::result::Error::NotFound);
        }
    };
}
