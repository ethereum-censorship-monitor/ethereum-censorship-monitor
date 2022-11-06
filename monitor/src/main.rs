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

use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

const HTTP_URL: &str = "http://1.geth.mainnet.ethnodes.brainbot.com:8545/";
const WS_URL: &str = "ws://1.geth.mainnet.ethnodes.brainbot.com:8546/";
const CONSENSUS_HTTP_URL: &str = "http://1.geth.mainnet.ethnodes.brainbot.com:5052/";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let (event_tx, mut event_rx): (Sender<watch::Event>, Receiver<watch::Event>) =
        mpsc::channel(100);
    let (analysis_tx, mut analysis_rx): (Sender<analyze::Analysis>, Receiver<analyze::Analysis>) =
        mpsc::channel(100);

    let node_config = watch::NodeConfig {
        http_url: url::Url::parse(HTTP_URL)?,
        ws_url: url::Url::parse(WS_URL)?,
        consensus_http_url: url::Url::parse(CONSENSUS_HTTP_URL)?,
    };
    if let Err(e) = node_config.test_connection().await {
        log::error!("failed to connect to Ethereum node: {}", e);
        return Ok(());
    }
    log::info!("node connection is up");

    let mut state = state::State::new(&node_config);
    let mut db = db::memory::MemoryDB::new();

    let process_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let analysis = state.process_event(event).await;
            if let Some(analysis) = analysis {
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
