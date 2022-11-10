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
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

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

    let (event_tx, mut event_rx): (Sender<watch::Event>, Receiver<watch::Event>) =
        mpsc::channel(100);
    let (analysis_tx, mut analysis_rx): (Sender<analyze::Analysis>, Receiver<analyze::Analysis>) =
        mpsc::channel(100);

    if let Err(e) = node_config.test_connection().await {
        log::error!("failed to connect to Ethereum node: {}", e);
        return Ok(());
    }
    log::info!("node connection is up");

    let mut state = state::State::new(&node_config);
    let mut db = db::memory::MemoryDB::new();

    let process_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let analyses = state.process_event(event).await;
            for analysis in analyses {
                log::info!("{}", analysis.summary());
            }
        }
    });

    let db_handle = tokio::spawn(async move {
        log::info!("spawning db task");
        while let Some(analysis) = analysis_rx.recv().await {
            analyze::insert_analysis_into_db(&analysis, &mut db).unwrap();
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
