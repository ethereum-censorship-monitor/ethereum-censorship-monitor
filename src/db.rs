use hex::ToHex;

use crate::analyze::Analysis;

type Pool = sqlx::Pool<sqlx::Postgres>;

pub async fn connect(s: &str) -> Result<Pool, sqlx::Error> {
    log::debug!("connecting to db at {}", s);
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(s)
        .await
}

pub async fn migrate(pool: &Pool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn truncate(pool: &Pool) -> Result<(), sqlx::Error> {
    log::debug!("truncating db");
    sqlx::query!(
        r#"
        TRUNCATE miss, transaction, beacon_block RESTART IDENTITY;
        "#
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_analysis_into_db(analysis: &Analysis, pool: &Pool) -> Result<(), sqlx::Error> {
    log::debug!("persisting analysis for block {}", analysis.beacon_block);

    let mut tx = pool.begin().await?;
    let block = &analysis.beacon_block;
    let exec = &block.body.execution_payload;
    let beacon_root_str = encode_hex_prefixed(block.root);

    sqlx::query!(
        r#"
        INSERT INTO data.beacon_block (
            root,
            slot,
            proposer_index,
            execution_block_hash,
            execution_block_number,
            proposal_time
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6
        ) ON CONFLICT DO NOTHING;
        "#,
        beacon_root_str,
        block.slot.as_u64() as i64,
        block.proposer_index.as_u64() as i64,
        encode_hex_prefixed(exec.block_hash),
        exec.block_number.as_u64() as i64,
        block.proposal_time().naive_utc(),
    )
    .execute(&mut tx)
    .await?;

    for missing_transaction in analysis.missing_transactions.values() {
        let transaction_hash_str = encode_hex_prefixed(missing_transaction.transaction.hash);
        let queries = [
            sqlx::query!(
                r#"
            INSERT INTO data.transaction (
                hash,
                sender,
                first_seen,
                quorum_reached
            ) VALUES (
                $1,
                $2,
                $3,
                $4
            ) ON CONFLICT DO NOTHING;
            "#,
                transaction_hash_str,
                encode_hex_prefixed(missing_transaction.transaction.from),
                missing_transaction.first_seen.naive_utc(),
                missing_transaction.quorum_reached.naive_utc(),
            ),
            sqlx::query!(
                r#"
            INSERT INTO data.miss (
                transaction_hash,
                beacon_block_root,
                proposal_time,
                tip
            ) VALUES (
                $1,
                $2,
                $3,
                $4
            ) ON CONFLICT DO NOTHING;
            "#,
                transaction_hash_str,
                beacon_root_str,
                analysis.beacon_block.proposal_time().naive_utc(),
                missing_transaction.tip,
            ),
        ];
        for query in queries {
            query.execute(&mut tx).await?;
        }
    }
    tx.commit().await?;
    log::debug!("persisted analysis in db");
    Ok(())
}

fn encode_hex_prefixed<T: ToHex>(v: T) -> String {
    String::from("0x") + v.encode_hex::<String>().as_str()
}
