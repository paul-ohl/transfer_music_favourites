use anyhow::{Context, Result};
use clap::Parser;
use transfer_music_favourites::{
    api::fetch_starred_songs,
    cli::Args,
    config::get_configs,
    sync::{sync_lyrics, sync_music},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let (api_config, sync_config) =
        get_configs(args.config).context("Could not load config file")?;

    println!("Fetching liked songs from Navidrome...");

    let songs = fetch_starred_songs(&api_config).await?;

    sync_music(&sync_config, songs.clone()).await?;
    sync_lyrics(&sync_config, songs).await?;

    Ok(())
}
