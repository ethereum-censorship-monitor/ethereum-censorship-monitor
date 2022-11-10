mod analyze;
mod consensus_api;
mod db;
mod head_history;
mod nonce_cache;
mod pool;
mod state;
mod types;
mod visibility;
mod watch;

use clap::Parser;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_postgres::NoTls;

/// Monitor Ethereum for validators not including valid transactions.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the config file
    #[arg(short, long = "config")]
    config_path: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Config {
    execution_http_url: String,
    execution_ws_url: String,
    consensus_http_url: String,

    #[serde(default)]
    db_enabled: bool,
    db_connection: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();
    let mut config = Figment::new();
    if let Some(config_path) = args.config_path {
        config = config.merge(Toml::file(config_path));
    }
    let config: Config = config.merge(Env::prefixed("MONITOR_")).extract()?;

    let node_config = watch::NodeConfig {
        execution_http_url: url::Url::parse(&config.execution_http_url)?,
        execution_ws_url: url::Url::parse(&config.execution_ws_url)?,
        consensus_http_url: url::Url::parse(&config.consensus_http_url)?,
    };
    let mut state = state::State::new(&node_config);

    let (event_tx, mut event_rx): (Sender<watch::Event>, Receiver<watch::Event>) =
        mpsc::channel(100);
    let (analysis_tx, mut analysis_rx): (Sender<analyze::Analysis>, Receiver<analyze::Analysis>) =
        mpsc::channel(100);

    if let Err(e) = node_config.test_connection().await {
        log::error!("failed to connect to Ethereum node: {}", e);
        return Ok(());
    }
    log::info!("node connection is up");

    let process_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let analyses = state.process_event(event).await;
            for analysis in analyses {
                log::info!("{}", analysis.summary());
                analysis_tx.send(analysis).await.unwrap();
            }
        }
    });

    let db_handle = tokio::spawn(async move {
        if !config.db_enabled {
            log::warn!("db is disabled, analysises will not be persisted");
            while let Some(_) = analysis_rx.recv().await {}
        }
        log::info!("spawning db task");

        log::debug!(
            "connecting to db at {}",
            config.db_connection.as_ref().unwrap()
        );
        let (client, connection) =
            tokio_postgres::connect(config.db_connection.unwrap().as_str(), NoTls)
                .await
                .unwrap();

        let connection_handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("db connection error: {}", e);
            }
        });

        let insert_handle = tokio::spawn(async move {
            while let Some(analysis) = analysis_rx.recv().await {
                db::insert_analysis_into_db(&analysis, &client)
                    .await
                    .unwrap();
            }
        });

        tokio::select! {
            _ = connection_handle => {},
            _ = insert_handle => {},
        }
    });

    let watch_handle = tokio::spawn(async move {
        log::info!("spawning watch task");
        watch::watch(&node_config, event_tx).await.unwrap()
    });

    tokio::select! {
        _ = process_handle => {},
        _ = db_handle => {},
        _ = watch_handle => {},
    }

    Ok(())
}
