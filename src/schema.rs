// @generated automatically by Diesel CLI.

diesel::table! {
    evm_tokens (id) {
        id -> Text,
        chain_id -> Integer,
        address -> Text,
        symbol -> Text,
        decimals -> Integer,
        name -> Text,
    }
}
