use crate::analyze::Analysis;

type Pool = sqlx::Pool<sqlx::Postgres>;

pub async fn connect(s: &str) -> Result<Pool, sqlx::Error> {
    log::debug!("connecting to db at {}", s);
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(s)
        .await
}

pub async fn create_db(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
    log::debug!("creating db tables");
    let queries = [
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS transaction (
                hash char(66) PRIMARY KEY
            );
            "#
        ),
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS beacon_block (
                root char(66) PRIMARY KEY
            );
            "#
        ),
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS miss (
                transaction_hash char(66),
                beacon_block_root char(66),
                PRIMARY KEY (transaction_hash, beacon_block_root),
                FOREIGN KEY (transaction_hash) REFERENCES transaction (hash),
                FOREIGN KEY (beacon_block_root) REFERENCES beacon_block (root)
            );
            "#
        ),
    ];

    let mut tx = pool.begin().await?;
    for query in queries {
        query.execute(&mut tx).await?;
    }
    tx.commit().await?;
    log::debug!("db tables created");
    Ok(())
}

pub async fn drop_db(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
    log::debug!("dropping db tables");
    let queries = [
        sqlx::query!(
            r#"
            DROP TABLE IF EXISTS miss;
            "#
        ),
        sqlx::query!(
            r#"
            DROP TABLE IF EXISTS transaction;
            "#
        ),
        sqlx::query!(
            r#"
            DROP TABLE IF EXISTS beacon_block;
            "#
        ),
    ];
    let mut tx = pool.begin().await?;
    for query in queries {
        query.execute(&mut tx).await?;
    }
    tx.commit().await?;
    log::debug!("db tables dropped");
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
