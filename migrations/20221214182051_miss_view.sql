CREATE VIEW
  data.rich_miss AS
SELECT
  transaction_hash,
  transaction.sender AS sender,
  transaction.first_seen AS first_seen,
  transaction.quorum_reached AS quorum_reached,
  beacon_block_root,
  beacon_block.slot AS slot,
  beacon_block.proposer_index AS proposer_index,
  beacon_block.execution_block_hash AS execution_block_hash,
  beacon_block.execution_block_number AS execution_block_number,
  beacon_block.proposal_time AS proposal_time
FROM
  data.miss
  LEFT JOIN data.transaction ON transaction.hash = transaction_hash
  LEFT JOIN data.beacon_block ON beacon_block.root = beacon_block_root
ORDER BY
  slot DESC,
  beacon_block_root,
  first_seen DESC;
