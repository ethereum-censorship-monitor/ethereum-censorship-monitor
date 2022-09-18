CREATE TABLE blocks (
    block_number       INTEGER,
    validator          TEXT,
    hash               TEXT,
    timestamp          INTEGER
);

CREATE UNIQUE INDEX unique_block_number ON blocks(block_number);

CREATE TABLE transactions (
    hash               TEXT,
    first_seen         INTEGER,
    sender             TEXT
);

CREATE UNIQUE INDEX unique_hash ON transactions(hash);
