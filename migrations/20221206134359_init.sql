CREATE SCHEMA IF NOT EXISTS data;

CREATE TABLE
  data.transaction (
    hash char(66) PRIMARY KEY,
    sender char(42),
    first_seen timestamp without time zone,
    quorum_reached timestamp without time zone
  );

CREATE TABLE
  data.beacon_block (
    root char(66) PRIMARY KEY,
    slot integer,
    proposer_index integer,
    execution_block_hash char(66),
    execution_block_number integer
  );

CREATE TABLE
  data.miss (
    transaction_hash char(66),
    beacon_block_root char(66),
    PRIMARY KEY (transaction_hash, beacon_block_root),
    FOREIGN KEY (transaction_hash) REFERENCES data.transaction (hash),
    FOREIGN KEY (beacon_block_root) REFERENCES data.beacon_block (root)
  );
