// @generated automatically by Diesel CLI.

diesel::table! {
    evm_tokens (id) {
        id -> Text,
        chain_id -> BigInt,
        address -> Text,
        symbol -> Text,
        decimals -> Integer,
        name -> Text,
    }
}
