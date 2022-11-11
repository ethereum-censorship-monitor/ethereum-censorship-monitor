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
use color_eyre::{Report, Result};
use eyre::{eyre, WrapErr};
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

#[derive(Deserialize, Debug, Clone)]
struct Config {
    #[serde(default = "default_log_config")]
    log: String,

    execution_http_url: url::Url,
    execution_ws_url: url::Url,
    consensus_http_url: url::Url,

    #[serde(default)]
    db_enabled: bool,
    #[serde(default)]
    db_connection: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let args = Args::parse();
        let mut config = Figment::new();
        if let Some(config_path) = args.config_path {
            config = config.merge(Toml::file(config_path));
        }
        config
            .merge(Env::prefixed("MONITOR_"))
            .extract()
            .wrap_err("error loading config")
    }

    pub fn node_config(&self) -> watch::NodeConfig {
        watch::NodeConfig {
            execution_http_url: self.execution_http_url.clone(),
            execution_ws_url: self.execution_ws_url.clone(),
            consensus_http_url: self.consensus_http_url.clone(),
        }
    }
}

fn default_log_config() -> String {
    String::from("info,monitor=debug")
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let config = Config::load()?;

    env_logger::Builder::new()
        .parse_filters(config.log.as_str())
        .try_init()?;

    let mut state = state::State::new(&config.node_config());

    let (event_tx, mut event_rx): (Sender<watch::Event>, Receiver<watch::Event>) =
        mpsc::channel(100);
    let (analysis_tx, mut analysis_rx): (Sender<analyze::Analysis>, Receiver<analyze::Analysis>) =
        mpsc::channel(100);

    config
        .node_config()
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

    let db_connection = config.db_connection.clone();
    let db_handle = tokio::spawn(async move {
        if !config.db_enabled {
            log::warn!("db is disabled, analyses will not be persisted");
            while let Some(_) = analysis_rx.recv().await {}
        }
        log::info!("spawning db task");

        log::debug!("connecting to db at {}", db_connection);
        let (client, connection) = tokio_postgres::connect(db_connection.as_str(), NoTls)
            .await
            .wrap_err_with(|| format!("error connecting to db at {}", db_connection))?;

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

    let node_config = config.node_config();
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
