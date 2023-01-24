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
    block_hash,
    min(slot) AS "slot!",
    min(proposal_time) AS "proposal_time!",
    min(block_number) AS "block_number!",
    min(proposer_index) AS "proposer_index!",
    min(ref_time) AS "ref_time!",
    count(tx_hash) AS "num_misses!",
    json_agg(json_build_object(
        'tx_hash', tx_hash,
        'tx_first_seen', tx_first_seen,
        'tx_quorum_reached', tx_quorum_reached,
        'sender', sender,
        'tip', tip
    )) AS "txs!"
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
        ref_miss.proposal_time AS ref_time
    FROM (
        SELECT
            tx_hash,
            block_hash,
            proposal_time,
            tx_quorum_reached
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
        LIMIT $9
    ) AS ref_miss
    INNER JOIN data.full_miss joined_miss ON joined_miss.block_hash = ref_miss.block_hash
    WHERE
        ($6::interval IS NULL OR joined_miss.proposal_time - joined_miss.tx_quorum_reached > $6) AND
        ($7::bigint IS NULL OR joined_miss.tip >= $7)
) AS miss
GROUP BY block_hash;
