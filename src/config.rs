use crate::constants::CONFIG_FILE_NAME;
use anyhow::Result;
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
    pub transfer_lyric_files: Option<bool>,
}

pub fn load_config(config_path: Option<PathBuf>) -> Result<Config> {
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
