DROP TABLE
  IF EXISTS miss;

DROP TABLE
  IF EXISTS transaction;

DROP TABLE
  IF EXISTS beacon_block;

CREATE TABLE
  transaction (hash char(66) PRIMARY KEY);

CREATE TABLE
  beacon_block (root char(66) PRIMARY KEY);

CREATE TABLE
  miss (
    transaction_hash char(66),
    beacon_block_root char(66),
    PRIMARY KEY (transaction_hash, beacon_block_root),
    FOREIGN KEY (transaction_hash) REFERENCES transaction (hash),
    FOREIGN KEY (beacon_block_root) REFERENCES beacon_block (root)
  );
