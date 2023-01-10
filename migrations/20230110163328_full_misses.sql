CREATE TABLE
  data.full_miss (
    block_hash char(66),
    tx_hash char(66),
    slot integer NOT NULL,
    block_number integer NOT NULL,
    proposal_time timestamp without time zone NOT NULL,
    proposer_index integer NOT NULL,
    tx_first_seen timestamp without time zone NOT NULL,
    tx_quorum_reached timestamp without time zone NOT NULL,
    sender char(42) NOT NULL,
    tip bigint,
    PRIMARY KEY (block_hash, tx_hash)
  );

CREATE INDEX ON data.full_miss (proposal_time);

CREATE INDEX ON data.full_miss (block_number);

CREATE INDEX ON data.full_miss (proposer_index);

CREATE INDEX ON data.full_miss (sender);

INSERT INTO
  data.full_miss (
    block_hash,
    tx_hash,
    slot,
    block_number,
    proposal_time,
    proposer_index,
    tx_first_seen,
    tx_quorum_reached,
    sender,
    tip
  )
SELECT
  execution_block_hash,
  transaction_hash,
  slot,
  execution_block_number,
  proposal_time,
  proposer_index,
  first_seen,
  quorum_reached,
  sender,
  tip
FROM
  data.rich_miss;
