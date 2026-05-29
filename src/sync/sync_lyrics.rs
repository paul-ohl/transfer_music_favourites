#![allow(unused)]

use std::{path::Path, sync::Arc};

use crate::{
    models::{Song, SyncConfig},
    sync::utils::setup_progress_bar,
};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use std::thread;

pub async fn sync_lyrics(config: &SyncConfig, songs: Vec<Song>) -> Result<()> {
    if songs.is_empty() {
        println!("No liked songs found matching the criteria.");
        return Ok(());
    }

    let progress_bar = setup_progress_bar(songs.len() as u64)?;
    progress_bar.println(format!("Start transfering {} songs", songs.len()));

    let workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    stream::iter(songs)
        .for_each_concurrent(workers, |song| {
            let config = &config;
            let progress_bar = Arc::clone(&progress_bar);

            async move {
                progress_bar.set_message(song.title.clone());

                let rel_path = song
                    .path
                    .strip_prefix(config.navidrome_dir.to_str().unwrap())
                    .unwrap_or(&song.path);
                let rel_path = rel_path.strip_prefix("/").unwrap_or(rel_path);

                let source_path = config.local_dir.join(rel_path);
                let mut dest_path = config.dest_dir.join(rel_path);

                if let Err(e) = transfer_lyric_file(&source_path, &dest_path).await {
                    progress_bar.println(format!(
                        "Warning: Could not transfer lyric file for '{}': {}",
                        song.title, e
                    ));
                };
                progress_bar.inc(1);
            }
        })
        .await;

    progress_bar.finish_with_message("Done transfering songs!");
    Ok(())
}

async fn transfer_lyric_file(original_song_path: &Path, dest_song_path: &Path) -> Result<()> {
    for ext in ["lrc", "txt"] {
        let original_lyric_path = original_song_path.with_extension(ext);
        if original_lyric_path.exists() {
            let dest_lyric_path = dest_song_path.with_extension(ext);
            if let Some(parent) = dest_lyric_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&original_lyric_path, &dest_lyric_path).await?;
        }
    }
    Ok(())
}
