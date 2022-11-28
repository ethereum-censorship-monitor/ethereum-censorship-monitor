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
    let beacon_root_str = &analysis.beacon_block.root.to_string();
    sqlx::query!(
        r#"
        INSERT INTO beacon_block (
            root
        ) VALUES (
            $1
        ) ON CONFLICT DO NOTHING;
        "#,
        beacon_root_str
    )
    .execute(&mut tx)
    .await?;

    for missing_transaction in analysis.missing_transactions.values() {
        let transaction_hash_str = missing_transaction.hash.to_string();
        let queries = [
            sqlx::query!(
                r#"
            INSERT INTO transaction (
                hash
            ) VALUES (
                $1
            ) ON CONFLICT DO NOTHING;
            "#,
                transaction_hash_str,
            ),
            sqlx::query!(
                r#"
            INSERT INTO miss (
                transaction_hash,
                beacon_block_root
            ) VALUES (
                $1,
                $2
            ) ON CONFLICT DO NOTHING;
            "#,
                transaction_hash_str,
                beacon_root_str,
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
