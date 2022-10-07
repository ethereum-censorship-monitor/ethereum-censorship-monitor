mod db;
mod history;
mod pool;
mod types;
mod visibility;
mod watch;

use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

const HTTP_URL: &str = "https://mainnet.infura.io/v3/cb47771bf3324acc895994de6752654b";
const WS_URL: &str = "wss://mainnet.infura.io/ws/v3/cb47771bf3324acc895994de6752654b";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx): (Sender<watch::Event>, Receiver<watch::Event>) = mpsc::channel(100);

    tokio::spawn(async move {
        while let Some(i) = rx.recv().await {
            println!("{:?}", i);
        }
    });

    let node_config = watch::NodeConfig {
        http_url: url::Url::parse(HTTP_URL)?,
        ws_url: url::Url::parse(WS_URL)?,
    };
    watch::watch(node_config, tx).await.unwrap();

    Ok(())
}
