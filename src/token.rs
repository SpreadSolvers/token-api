use alloy::primitives::Address;
use serde::Serialize;
use tap_caip::{AccountId, ChainId};

pub type TokenId = AccountId;

// Universal data structure for all tokens (EVM, Solana, etc.)
// chain_id is EVM specific data
// address is Ecosystem specific data, so address MUST be typed differently for each ecosystem
#[derive(Debug, Clone, Serialize)]
pub struct Token<T> {
    pub id: TokenId,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub details: T,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvmTokenDetails {
    pub chain_id: ChainId,
    pub address: Address,
}

pub struct SolanaTokenDetails {
    address: String, // base58 encoded address
}
