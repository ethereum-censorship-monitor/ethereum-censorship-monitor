mod analyze;
mod check_transaction;
mod cli;
mod compare_providers;
mod consensus_api;
mod db;
mod head_history;
mod nonce_cache;
mod pool;
mod state;
mod types;
mod watch;

use core::str::FromStr;

use clap::Parser;
use color_eyre::{
    eyre::{eyre, WrapErr},
    Report, Result,
};
use tokio::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = cli::Cli::parse();
    let config = cli::Config::load(cli.config_path.as_ref())?;

    env_logger::Builder::new()
        .parse_filters(config.log.as_str())
        .try_init()?;

    match cli.command {
        cli::Commands::Run => run(config).await,
        cli::Commands::TruncateDB => truncate_db(config).await,
        cli::Commands::Check { txhash, n } => check(config, txhash, n).await,
        cli::Commands::CompareProviders => compare_providers(config).await,
    }
}

async fn run(config: cli::Config) -> Result<()> {
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

    if config.sync_check_enabled
        && node_config
            .is_syncing()
            .await
            .wrap_err("error connecting to Ethereum node")?
    {
        return Err::<(), Report>(eyre!("node is still syncing"));
    }

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
            while analysis_rx.recv().await.is_some() {}
        }
        log::info!("spawning db task");

        log::debug!("connecting to db at {}", config.db_connection);
        let pool = db::connect(config.db_connection.as_str()).await?;

        db::migrate(&pool)
            .await
            .wrap_err("failed to apply db migrations")?;

        while let Some(analysis) = analysis_rx.recv().await {
            db::insert_analysis_into_db(&analysis, &pool)
                .await
                .wrap_err_with(|| {
                    format!(
                        "failed to insert analysis for block {} into db",
                        analysis.beacon_block
                    )
                })?;
        }

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

async fn truncate_db(config: cli::Config) -> Result<()> {
    log::info!("drop all data from db at {}", config.db_connection);
    let pool = db::connect(config.db_connection.as_str())
        .await
        .wrap_err("failed to connect to db")?;

    db::migrate(&pool)
        .await
        .wrap_err("failed to apply db migrations")?;

    db::truncate(&pool)
        .await
        .wrap_err("failed to drop db tables")?;
    Ok(())
}

async fn check(config: cli::Config, tx_hash: String, n: usize) -> Result<()> {
    let hash = types::TxHash::from_str(tx_hash.as_str())?;
    check_transaction::check_transaction(hash, &config, n).await?;
    Ok(())
}

async fn compare_providers(config: cli::Config) -> Result<()> {
    compare_providers::compare_providers(&config).await
}
