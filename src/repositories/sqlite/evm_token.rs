use alloy::primitives::Address;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use log::{debug, info};
use tap_caip::{AccountId, ChainId};

use crate::{
    repositories::{RepoError, Repository},
    token::{EvmTokenDetails, Token},
};

#[derive(Queryable, Insertable)]
#[diesel(table_name = crate::schema::evm_tokens)]
pub struct DbEvmToken {
    pub id: String,
    pub chain_id: i32,
    pub address: String,
    pub symbol: String,
    pub decimals: i32,
    pub name: String,
}

#[derive(Clone)]
pub struct SqliteEvmTokenRepository {
    pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl SqliteEvmTokenRepository {
    pub fn new(database_url: String) -> Self {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);

        let pool = Pool::builder()
            .build(manager)
            .expect("Could not build connection pool");

        debug!("Connected to SQLite database");

        Self { pool }
    }
}

impl Repository<Token<EvmTokenDetails>> for SqliteEvmTokenRepository {
    fn get(&self, id: AccountId) -> Result<Option<Token<EvmTokenDetails>>, RepoError> {
        let mut connection: PooledConnection<ConnectionManager<SqliteConnection>> = self
            .pool
            .get()
            .map_err(|e| RepoError::Backend(e.to_string()))?;

        debug!("Finding EVM token by id: {:?}", id.to_string());

        let token: Option<DbEvmToken> = crate::schema::evm_tokens::table
            .find(id.to_string())
            .first::<DbEvmToken>(&mut connection)
            .optional()?;

        match token {
            Some(token) => {
                let id: AccountId = token
                    .id
                    .parse::<AccountId>()
                    .expect("Failed to create account id");
                let chain_id: ChainId = id.chain_id().clone();
                let address: Address = id
                    .address()
                    .parse::<Address>()
                    .expect("Failed to create address");

                let token: Token<EvmTokenDetails> = Token::<EvmTokenDetails> {
                    id,
                    details: EvmTokenDetails { chain_id, address },
                    symbol: token.symbol,
                    decimals: token.decimals as u8,
                    name: token.name,
                };

                return Ok(Some(token));
            }
            None => {
                debug!("Token not found by id: {:?}", id.to_string());
                return Ok(None);
            }
        };
    }

    fn save(&self, token: &Token<EvmTokenDetails>) -> Result<(), RepoError> {
        // Acquire a pooled connection for this operation
        let mut connection: PooledConnection<ConnectionManager<SqliteConnection>> = self
            .pool
            .get()
            .map_err(|e| RepoError::Backend(e.to_string()))?;

        use crate::schema::evm_tokens;
        use diesel::prelude::*;

        info!("Saving EVM token with id: {:?}", token.id);

        let chain_id = token
            .id
            .chain_id()
            .reference()
            .to_string()
            .parse::<i32>()
            .map_err(|e| {
                RepoError::Backend(format!("Failed to parse chain id: {}", e.to_string()))
            })?;

        let new_token: DbEvmToken = DbEvmToken {
            id: token.id.to_string(),
            chain_id,
            address: token.id.address().to_string(),
            symbol: token.symbol.clone(),
            decimals: token.decimals as i32,
            name: token.name.clone(),
        };

        diesel::insert_into(evm_tokens::table)
            .values(&new_token)
            .execute(&mut connection)?;

        Ok(())
    }
}
