-- Your SQL goes here
CREATE TABLE evm_tokens (
    id VARCHAR(255) NOT NULL PRIMARY KEY,
    chain_id INT NOT NULL,
    address VARCHAR(255) NOT NULL,
    symbol VARCHAR(255) NOT NULL,
    decimals TINYINT NOT NULL,
    name VARCHAR(255) NOT NULL
);