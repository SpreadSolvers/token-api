-- Your SQL goes here
CREATE TABLE evm_tokens (
    id VARCHAR(255) NOT NULL PRIMARY KEY,
    chain_id INT NOT NULL CHECK (chain_id > 0),
    address VARCHAR(255) NOT NULL CHECK (LENGTH(address) = 42),
    symbol VARCHAR(255) NOT NULL,
    decimals INT NOT NULL CHECK (decimals BETWEEN 0 AND 255),
    name VARCHAR(255) NOT NULL
);