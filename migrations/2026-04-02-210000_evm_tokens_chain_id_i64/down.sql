CREATE TABLE evm_tokens_old (
    id VARCHAR(255) NOT NULL PRIMARY KEY,
    chain_id INT NOT NULL CHECK (chain_id > 0),
    address VARCHAR(255) NOT NULL CHECK (LENGTH(address) = 42),
    symbol VARCHAR(255) NOT NULL,
    decimals INT NOT NULL CHECK (decimals BETWEEN 0 AND 255),
    name VARCHAR(255) NOT NULL
);

INSERT INTO evm_tokens_old (id, chain_id, address, symbol, decimals, name)
SELECT id, chain_id, address, symbol, decimals, name
FROM evm_tokens
WHERE chain_id <= 2147483647;

DROP TABLE evm_tokens;

ALTER TABLE evm_tokens_old RENAME TO evm_tokens;
