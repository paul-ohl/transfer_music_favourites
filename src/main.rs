use anyhow::Result;
use clap::Parser;

use transfer_music_favourites::{api, cli::Args, sync};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Fetching liked songs from Navidrome...");

    let api_config = api::ApiConfig {
        url: args.url,
        user: args.user,
        password: args.password,
    };
    let songs = api::fetch_starred_songs(&api_config).await?;

    let sync_config = sync::SyncConfig {
        navidrome_dir: args.navidrome_dir,
        local_dir: args.local_dir,
        dest_dir: args.dest_dir,
        format: args.format,
        on_conflict: args.on_conflict,
        priority: args.priority,
        delete_unliked: !args.keep_unliked,
    };
    sync::sync_songs(
        &sync_config,
        songs
            .into_iter()
            .filter(|s| s.path.ends_with("mp3"))
            .take(1)
            .collect(),
    )
    .await?;

    Ok(())
}
