use crate::{api::ApiConfig, constants::CONFIG_FILE_NAME, sync::SyncConfig};
use anyhow::{Context, Result};
use config::{Config as ConfigBuilder, Environment, File};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Mp3,
    Opus,
    Ogg,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStrategy {
    Overwrite,
    Ignore,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversionPriority {
    Quality,
    Balance,
    Compression,
}

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    pub url: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub navidrome_dir: Option<String>,
    pub local_dir: Option<PathBuf>,
    pub dest_dir: Option<PathBuf>,
    pub format: Option<Format>,
    pub on_conflict: Option<ConflictStrategy>,
    pub priority: Option<ConversionPriority>,
    pub whitelist: Option<Vec<String>>,
    pub blacklist: Option<Vec<String>>,
}

pub fn get_configs(config_path: Option<PathBuf>) -> Result<(ApiConfig, SyncConfig)> {
    let config = load_config_file(config_path).context("Could not load config file")?;

    let url = config.url.context("Missing required parameter: url")?;
    let user = config.user.context("Missing required parameter: user")?;
    let password = config
        .password
        .context("Missing required parameter: password")?;
    let navidrome_dir = config
        .navidrome_dir
        .context("Missing required parameter: navidrome_dir")?;
    let local_dir = config
        .local_dir
        .context("Missing required parameter: local_dir")?;
    let dest_dir = config
        .dest_dir
        .context("Missing required parameter: dest_dir")?;
    let format = config.format;
    let on_conflict = config.on_conflict.unwrap_or(ConflictStrategy::Overwrite);
    let priority = config.priority.unwrap_or(ConversionPriority::Balance);
    let whitelist = config.whitelist;
    let blacklist = config.blacklist;

    if whitelist.is_some() && blacklist.is_some() {
        anyhow::bail!("Cannot use both whitelist and blacklist at the same time.");
    }

    let api_config = ApiConfig {
        url,
        user,
        password,
    };

    let sync_config = SyncConfig {
        navidrome_dir,
        local_dir,
        dest_dir,
        format,
        on_conflict,
        priority,
        whitelist,
        blacklist,
    };

    Ok((api_config, sync_config))
}

fn load_config_file(config_path: Option<PathBuf>) -> Result<Config> {
    let mut builder = ConfigBuilder::builder();

    if let Some(config_dir) = dirs::config_dir() {
        let default_config_path = config_dir.join(CONFIG_FILE_NAME);
        if default_config_path.exists() {
            builder = builder.add_source(File::from(default_config_path).required(false));
        }
    }

    let local_config_path = PathBuf::from(CONFIG_FILE_NAME);
    if local_config_path.exists() {
        builder = builder.add_source(File::from(local_config_path).required(false));
    }

    if let Some(path) = config_path {
        builder = builder.add_source(File::from(path).required(true));
    }

    builder = builder.add_source(
        Environment::with_prefix("NAVIDROME")
            .try_parsing(true)
            .separator("_")
            .list_separator(" "),
    );

    let config = builder.build()?.try_deserialize()?;
    Ok(config)
}
