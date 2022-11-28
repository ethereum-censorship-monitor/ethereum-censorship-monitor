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
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_log_config")]
    pub log: String,

    pub execution_http_url: url::Url,
    pub execution_ws_url: url::Url,
    pub consensus_http_url: url::Url,

    #[serde(default)]
    pub db_enabled: bool,
    #[serde(default)]
    pub db_connection: String,
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
}

fn default_log_config() -> String {
    String::from("info,monitor=debug")
}
