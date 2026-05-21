use anyhow::{Context, Result};
use clap::Parser;
use transfer_music_favourites::{
    api::{ApiConfig, fetch_starred_songs},
    cli::Args,
    config::{ConflictStrategy, ConversionPriority, load_config},
    sync::{SyncConfig, sync_songs},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = load_config(args.config).context("Could not load config file")?;

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
    let transfer_lyric_files = config.transfer_lyric_files.unwrap_or(false);

    if whitelist.is_some() && blacklist.is_some() {
        anyhow::bail!("Cannot use both whitelist and blacklist at the same time.");
    }

    println!("Fetching liked songs from Navidrome...");

    let api_config = ApiConfig {
        url,
        user,
        password,
    };
    let songs = fetch_starred_songs(&api_config).await?;

    let sync_config = SyncConfig {
        navidrome_dir,
        local_dir,
        dest_dir,
        format,
        on_conflict,
        priority,
        whitelist,
        blacklist,
        transfer_lyric_files,
    };
    sync_songs(&sync_config, songs).await?;

    Ok(())
}
