CREATE TABLE blocks (
    block_number       INTEGER,
    validator          INTEGER
);

INSERT INTO blocks DEFAULT VALUES;

CREATE UNIQUE INDEX unique_block_number ON blocks(block_number);

CREATE TABLE transactions (
hash                         INTEGER,
first_seen                   INTEGER,
sender                       INTEGER
);

CREATE UNIQUE INDEX unique_hash ON transactions(hash);
