-- parameters:
-- $1: min_proposal_time
-- $2: min_tx_quorum_reached
-- $3: max_proposal_time
-- $4: max_tx_quorum_reached
-- $5: block_number
-- $6: proposer_index
-- $7: sender
-- $8: min_propagation_time
-- $9: min_tip
-- $10: is_order_ascending
-- $11: limit

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
    tip
FROM
    data.full_miss
WHERE
    ($1::timestamp IS NULL OR
        (proposal_time > $1 OR
            (proposal_time = $1 AND ($2::timestamp IS NULL OR tx_quorum_reached >= $2)))) AND
    ($3::timestamp IS NULL OR
        (proposal_time < $3 OR
            (proposal_time = $3 AND ($4::timestamp IS NULL OR tx_quorum_reached <= $4)))) AND
    ($5::integer IS NULL OR block_number = $5) AND
    ($6::integer IS NULL OR proposer_index = $6) AND
    ($7::char(42) IS NULL OR sender = $7) AND
    ($8::interval IS NULL OR proposal_time - tx_quorum_reached > $8) AND
    ($9::bigint IS NULL OR tip >= $9)
ORDER BY
    CASE WHEN $10 THEN
        proposal_time
    ELSE
        to_timestamp(0)
    END ASC,
    CASE WHEN $10 THEN
        tx_quorum_reached
    ELSE
        to_timestamp(0)
    END ASC,
    CASE WHEN $10 THEN
        to_timestamp(0)
    ELSE
        proposal_time
    END DESC,
    CASE WHEN $10 THEN
        to_timestamp(0)
    ELSE
        tx_quorum_reached
    END DESC
LIMIT $11;
