-- parameters:
-- $1: min_proposal_time
-- $2: max_proposal_time
-- $3: block_number
-- $4: proposer_index
-- $5: sender
-- $6: min_propagation_time
-- $7: min_tip
-- $8: is_order_ascending
-- $9: limit
-- $10: offset

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
    filtered_miss_count AS "filtered_miss_count!",
    filtered_miss_row_by_proposal_time AS "filtered_miss_row_by_proposal_time!"
FROM (
    SELECT
        *,
        count(*) OVER () AS "filtered_miss_count",
        row_number() OVER (PARTITION BY proposal_time ORDER BY ord1, ord2) - 1 AS "filtered_miss_row_by_proposal_time"
    FROM (
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
            CASE WHEN $8 THEN
                1
            ELSE
                -1
            END * EXTRACT(EPOCH FROM proposal_time) AS "ord1",
            CASE WHEN $8 THEN
                1
            ELSE
                -1
            END * EXTRACT(EPOCH FROM tx_quorum_reached) AS "ord2"
        FROM
            data.full_miss
        WHERE
            ($1::timestamp IS NULL OR proposal_time > $1 OR
                ($8 AND proposal_time = $1)) AND
            ($2::timestamp IS NULL OR proposal_time < $2 OR
                (NOT $8 AND proposal_time = $2)) AND
            ($3::integer IS NULL OR block_number = $3) AND
            ($4::integer IS NULL OR proposer_index = $4) AND
            ($5::char(42) IS NULL OR sender = $5) AND
            ($6::interval IS NULL OR proposal_time - tx_quorum_reached > $6) AND
            ($7::bigint IS NULL OR tip >= $7)
        ORDER BY ord1, ord2
        LIMIT $9
        OFFSET $10
    ) AS uncounted_miss
    ORDER BY ord1, ord2
) AS miss
ORDER BY ord1, ord2;
