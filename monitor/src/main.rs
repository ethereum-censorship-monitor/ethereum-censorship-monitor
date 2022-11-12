mod analyze;
mod config;
mod consensus_api;
mod db;
mod head_history;
mod nonce_cache;
mod pool;
mod state;
mod types;
mod watch;

use color_eyre::{Report, Result};
use eyre::{eyre, WrapErr};
use tokio::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
};
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let config = config::Config::load()?;

    env_logger::Builder::new()
        .parse_filters(config.log.as_str())
        .try_init()?;

    run(config).await
}

async fn run(config: config::Config) -> Result<()> {
    let node_config = watch::NodeConfig::from(&config);
    let mut state = state::State::new(&node_config);

    let (event_tx, mut event_rx): (Sender<watch::Event>, Receiver<watch::Event>) =
        mpsc::channel(100);
    let (analysis_tx, mut analysis_rx): (Sender<analyze::Analysis>, Receiver<analyze::Analysis>) =
        mpsc::channel(100);

    node_config
        .test_connection()
        .await
        .wrap_err("error connecting to Ethereum node")?;
    log::info!("node connection is up");

    let process_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let analyses = state.process_event(event).await;
            for analysis in analyses {
                log::info!("{}", analysis.summary());
                analysis_tx.send(analysis).await?;
            }
        }
        Err::<(), Report>(eyre!("process task ended unexpectedly"))
    });

    let db_handle = tokio::spawn(async move {
        if !config.db_enabled {
            log::warn!("db is disabled, analyses will not be persisted");
            while let Some(_) = analysis_rx.recv().await {}
        }
        log::info!("spawning db task");

        log::debug!("connecting to db at {}", config.db_connection);
        let (client, connection) = tokio_postgres::connect(config.db_connection.as_str(), NoTls)
            .await
            .wrap_err_with(|| format!("error connecting to db at {}", config.db_connection))?;

        let connection_handle = tokio::spawn(async move {
            connection.await.wrap_err("db connection error")?;
            Err::<(), Report>(eyre!("db connection task ended unexpectedly"))
        });

        let insert_handle = tokio::spawn(async move {
            while let Some(analysis) = analysis_rx.recv().await {
                db::insert_analysis_into_db(&analysis, &client)
                    .await
                    .wrap_err_with(|| {
                        format!(
                            "failed to insert analysis for block {} into db",
                            analysis.beacon_block
                        )
                    })?;
            }
            Err::<(), Report>(eyre!("db insert task ended unexpectedly"))
        });

        tokio::select! {
            r = connection_handle => r,
            r = insert_handle => r,
        }??;
        Err::<(), Report>(eyre!("db task ended unexpectedly"))
    });

    let watch_handle = tokio::spawn(async move {
        log::info!("spawning watch task");
        watch::watch(&node_config, event_tx)
            .await
            .wrap_err("watch task failed")?;
        Err::<(), Report>(eyre!("watch task ended unexpectedly"))
    });

    tokio::select! {
        r = process_handle => r,
        r = db_handle => r,
        r = watch_handle => r,
    }??;

    Ok(())
}
