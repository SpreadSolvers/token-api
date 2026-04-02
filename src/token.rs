use serde::Serialize;
use tap_caip::AccountId;

pub type TokenId = AccountId;

#[derive(Debug, Clone, Serialize)]
pub struct Token {
    pub id: TokenId,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}
