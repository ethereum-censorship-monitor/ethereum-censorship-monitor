CREATE INDEX ON data.beacon_block (proposal_time);

CREATE INDEX ON data.miss (proposal_time);

CREATE INDEX ON data.transaction (first_seen);
