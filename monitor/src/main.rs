mod analyzer;
mod db;
mod history;
mod head_history;
mod pool;
mod simple_pool;
mod types;
mod visibility;
mod watch;

use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

use watch::Event;

const HTTP_URL: &str = "https://mainnet.infura.io/v3/cb47771bf3324acc895994de6752654b";
const WS_URL: &str = "wss://mainnet.infura.io/ws/v3/cb47771bf3324acc895994de6752654b";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (event_tx, mut event_rx): (Sender<watch::Event>, Receiver<watch::Event>) =
        mpsc::channel(100);
    let (analysis_tx, mut analysis_rx): (Sender<analyzer::Analysis>, Receiver<analyzer::Analysis>) =
        mpsc::channel(100);

    let node_config = watch::NodeConfig {
        http_url: url::Url::parse(HTTP_URL)?,
        ws_url: url::Url::parse(WS_URL)?,
    };

    let mut pool = simple_pool::SimplePool::new();
    let mut head_history = head_history::HeadHistory::new();
    let mut db = db::memory::MemoryDB::new();

    let process_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                Event::NewTransaction {
                    hash: h,
                    timestamp: t,
                } => {
                    pool.pre_announce_transaction(h, t);
                }
                Event::NewHead {
                    block: b,
                    timestamp: t,
                } => {
                    head_history.observe(t, b.clone()).unwrap();
                    let analysis = analyzer::analyze(&b, &pool);
                    pool.set_head_hash(b.hash.unwrap());
                    analysis_tx.send(analysis).await.unwrap();
                }
                Event::TxpoolContent {
                    content: c,
                    timestamp: t,
                } => {
                    pool.update(&c, t);
                }
            }
        }
    });

    let db_handle = tokio::spawn(async move {
        println!("spawning db task");
        while let Some(analysis) = analysis_rx.recv().await {
            analyzer::insert_analysis_into_db(&analysis, &mut db).unwrap();
        }
    });

    let watch_handle = tokio::spawn(async move {
        println!("spawning watch task");
        watch::watch(node_config, event_tx).await.unwrap()
    });

    tokio::select! {
        _ = process_handle => {},
        _ = db_handle => {},
        _ = watch_handle => {},
    }

    Ok(())
}
