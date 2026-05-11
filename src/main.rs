use anyhow::{Context, Result};
use clap::Parser;
use transfer_music_favourites::{
    api,
    cli::Args,
    config,
    config::{ConflictStrategy, ConversionPriority},
    sync::{self, SyncConfig},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = config::load_config(args.config).context("Could not load config file")?;

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

    println!("Fetching liked songs from Navidrome...");

    let api_config = api::ApiConfig {
        url,
        user,
        password,
    };
    let songs = api::fetch_starred_songs(&api_config).await?;

    let sync_config = SyncConfig {
        navidrome_dir,
        local_dir,
        dest_dir,
        format,
        on_conflict,
        priority,
    };
    sync::sync_songs(&sync_config, songs).await?;

    Ok(())
}
