ALTER TABLE
  data.beacon_block ADD proposal_time timestamp without time zone;

UPDATE
  data.beacon_block
SET
  proposal_time = to_timestamp (1606824023 + slot * 12) AT TIME ZONE 'UTC';

ALTER TABLE
  data.miss ADD proposal_time timestamp without time zone;

UPDATE
  data.miss
SET
  proposal_time = (
    SELECT
      proposal_time
    FROM
      data.beacon_block
    WHERE
      data.beacon_block.root = beacon_block_root
  );
