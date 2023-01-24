-- parameters:
-- $1: tmin
-- $2: tmax
-- $3: block_number
-- $4: proposer_index
-- $5: sender
-- $6: min_propagation_time
-- $7: min_tip
-- $8: is_order_ascending

SELECT
    tx_hash,
    block_hash,
    slot,
    block_number,
    proposal_time,
    proposer_index,
    tx_first_seen,
    tx_quorum_reached,
    sender,
    tip,
    proposal_time AS ref_time
FROM
    data.full_miss
WHERE
    ($1::timestamp IS NULL OR proposal_time >= $1) AND
    ($2::timestamp IS NULL OR proposal_time <= $2) AND
    ($3::integer IS NULL OR block_number = $3) AND
    ($4::integer IS NULL OR proposer_index = $4) AND
    ($5::char(42) IS NULL OR sender = $5) AND
    ($6::interval IS NULL OR proposal_time - tx_quorum_reached > $6) AND
    ($7::bigint IS NULL OR tip >= $7)
ORDER BY
    CASE WHEN $8 THEN
        proposal_time
    ELSE
        to_timestamp(0)
    END ASC,
    CASE WHEN $8 THEN
        tx_quorum_reached
    ELSE
        to_timestamp(0)
    END ASC,
    CASE WHEN $8 THEN
        to_timestamp(0)
    ELSE
        proposal_time
    END DESC,
    CASE WHEN $8 THEN
        to_timestamp(0)
    ELSE
        tx_quorum_reached
    END DESC
LIMIT $9;
