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
    min(tx_first_seen) AS "tx_first_seen!",
    min(tx_quorum_reached) AS "tx_quorum_reached!",
    min(sender) AS "sender!",
    min(source_proposal_time) AS "source_proposal_time!",
    min(source_tx_quorum_reached) AS "source_tx_quorum_reached!",
    max(source_row_number) AS "source_row_number!",
    count(block_hash) AS "num_misses!",
    json_agg(json_build_object(
        'block_hash', block_hash,
        'slot', slot,
        'proposal_time', floor(extract(epoch from proposal_time)),
        'block_number', block_number,
        'proposer_index', proposer_index,
        'tip', tip
    )) AS "blocks!"
FROM (
    SELECT
        DISTINCT ON (tx_hash, block_hash)
        joined_miss.tx_hash AS tx_hash,
        joined_miss.block_hash AS block_hash,
        joined_miss.slot AS slot,
        joined_miss.block_number AS block_number,
        joined_miss.proposal_time AS proposal_time,
        joined_miss.proposer_index AS proposer_index,
        joined_miss.tx_first_seen AS tx_first_seen,
        joined_miss.tx_quorum_reached AS tx_quorum_reached,
        joined_miss.sender AS sender,
        joined_miss.tip AS tip,
        source_miss.proposal_time AS source_proposal_time,
        source_miss.tx_quorum_reached AS source_tx_quorum_reached,
        source_miss.row_number AS source_row_number
    FROM (
        SELECT
            tx_hash,
            block_hash,
            proposal_time,
            tx_quorum_reached,
            ROW_NUMBER () OVER () AS row_number
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
        LIMIT $11
    ) AS source_miss
    INNER JOIN data.full_miss joined_miss ON joined_miss.tx_hash = source_miss.tx_hash
    WHERE
        ($8::interval IS NULL OR joined_miss.proposal_time - joined_miss.tx_quorum_reached > $8) AND
        ($9::bigint IS NULL OR joined_miss.tip >= $9)
) AS miss
GROUP BY tx_hash;
