use anyhow::Result;
use clap::Parser;

use transfer_music_favourites::{api, cli::Args, sync};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Fetching liked songs from Navidrome...");
    let songs = api::fetch_starred_songs(&args).await?;

    sync::sync_songs(&args, songs).await?;

    Ok(())
}
