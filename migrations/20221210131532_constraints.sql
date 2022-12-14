ALTER TABLE
  data.transaction
ALTER COLUMN
  sender
SET
  NOT NULL,
ALTER COLUMN
  first_seen
SET
  NOT NULL,
ALTER COLUMN
  quorum_reached
SET
  NOT NULL;

ALTER TABLE
  data.beacon_block
ALTER COLUMN
  slot
SET
  NOT NULL,
ALTER COLUMN
  proposer_index
SET
  NOT NULL,
ALTER COLUMN
  execution_block_hash
SET
  NOT NULL,
ALTER COLUMN
  execution_block_number
SET
  NOT NULL,
ALTER COLUMN
  proposal_time
SET
  NOT NULL;

ALTER TABLE
  data.miss
ALTER COLUMN
  proposal_time
SET
  NOT NULL;
