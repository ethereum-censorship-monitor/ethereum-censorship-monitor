use tokio_postgres::{error::Error as PostgresError, Client};

use crate::analyze::Analysis;

pub async fn insert_analysis_into_db(
    analysis: &Analysis,
    client: &Client,
) -> Result<(), PostgresError> {
    log::debug!("persisting analysis for block {}", analysis.beacon_block);

    let insert_transaction_statement = client
        .prepare("INSERT INTO transaction VALUES ($1) ON CONFLICT DO NOTHING;")
        .await?;
    let insert_beacon_block_statement = client
        .prepare("INSERT INTO beacon_block VALUES ($1) ON CONFLICT DO NOTHING;")
        .await?;
    let insert_miss_statement = client
        .prepare("INSERT INTO miss VALUES ($1, $2) ON CONFLICT DO NOTHING;")
        .await?;

    let beacon_root = &analysis.beacon_block.root.to_string();
    client
        .execute(&insert_beacon_block_statement, &[&beacon_root.to_string()])
        .await?;

    for (_, tx) in &analysis.missing_transactions {
        let tx_hash = &tx.hash.to_string();
        client
            .execute(&insert_transaction_statement, &[tx_hash])
            .await?;
        client
            .execute(&insert_miss_statement, &[tx_hash, beacon_root])
            .await?;
    }
    Ok(())
}
