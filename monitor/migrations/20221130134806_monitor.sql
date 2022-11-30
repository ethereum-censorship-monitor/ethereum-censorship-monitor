ALTER TABLE
  beacon_block
ADD COLUMN
  slot integer,
ADD COLUMN
  proposer_index integer,
ADD COLUMN
  execution_block_hash char(66),
ADD COLUMN
  execution_block_number integer;

ALTER TABLE
  transaction
ADD COLUMN
  sender char(42),
ADD COLUMN
  first_seen integer;
