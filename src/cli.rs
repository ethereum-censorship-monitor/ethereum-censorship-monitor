use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use clap::{Parser, Subcommand};
use color_eyre::{eyre::WrapErr, Result};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

/// Monitor Ethereum for validators not including valid transactions.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the config file
    #[arg(short, long = "config")]
    pub config_path: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the monitor
    Run,
    /// Delete all data from the database
    TruncateDB,
    /// Check if a transaction could have been included earlier than it was
    Check {
        txhash: String,
        /// Number of blocks to check
        #[arg(short, default_value_t = 5)]
        n: usize,
    },
    /// Compare the pending transaction streams sent by the different providers.
    CompareProviders,
    /// Print gas information about a transaction and a block.
    CheckGas { txhash: String, slot: u64 },
    /// Run the REST API server
    Api,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_log_config")]
    pub log: String,

    pub execution_http_url: url::Url,
    pub main_execution_ws_url: url::Url,
    pub secondary_execution_ws_urls: Vec<url::Url>,
    pub consensus_http_url: url::Url,

    #[serde(default = "default_sync_check_enabled")]
    pub sync_check_enabled: bool,

    #[serde(default)]
    pub db_enabled: bool,
    #[serde(default)]
    pub db_connection: String,

    #[serde(default = "default_metrics_endpoint")]
    pub metrics_endpoint: SocketAddr,

    #[serde(default = "default_nonce_cache_size")]
    pub nonce_cache_size: usize,

    #[serde(default)]
    pub api_db_connection: String,
    pub api_host: String,
    pub api_port: u16,
    #[serde(default = "default_api_max_response_rows")]
    pub api_max_response_rows: usize,
}

impl Config {
    pub fn load(config_path: Option<&String>) -> Result<Self> {
        let mut config = Figment::new();
        if let Some(config_path) = config_path {
            config = config.merge(Toml::file(config_path));
        }
        config
            .merge(Env::prefixed("MONITOR_"))
            .extract()
            .wrap_err("error loading config")
    }

    pub fn execution_ws_urls(&self) -> Vec<url::Url> {
        let mut urls = vec![self.main_execution_ws_url.clone()];
        urls.extend(self.secondary_execution_ws_urls.clone());
        urls
    }
}

fn default_log_config() -> String {
    String::from("info,monitor=debug")
}

fn default_sync_check_enabled() -> bool {
    true
}

fn default_metrics_endpoint() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
}

fn default_nonce_cache_size() -> usize {
    1000
}

fn default_api_max_response_rows() -> usize {
    3
}
