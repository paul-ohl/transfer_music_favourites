use crate::models::Song;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

pub struct SyncConfig {
    pub navidrome_dir: String,
    pub local_dir: PathBuf,
    pub dest_dir: PathBuf,
}

pub async fn sync_songs(config: &SyncConfig, songs: Vec<Song>) -> Result<()> {
    if songs.is_empty() {
        println!("No liked songs found.");
        return Ok(());
    }

    println!("Found {} liked songs. Starting copy...", songs.len());

    let pb = ProgressBar::new(songs.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")?
            .progress_chars("##-"),
    );

    let navidrome_dir_path = Path::new(&config.navidrome_dir);

    for song in songs {
        // Handle paths whether they are absolute (from Navidrome's view) or already relative
        let song_path = Path::new(&song.path);
        let rel_path = song_path
            .strip_prefix(navidrome_dir_path)
            .unwrap_or(song_path);
        let rel_path = rel_path.strip_prefix("/").unwrap_or(rel_path);

        let source_path = config.local_dir.join(rel_path);
        let dest_path = config.dest_dir.join(rel_path);

        pb.set_message(song.title.clone());

        if dest_path.exists() {
            pb.inc(1);
            continue;
        }

        if !source_path.exists() {
            pb.println(format!("Warning: Source file not found: {:?}", source_path));
            pb.inc(1);
            continue;
        }

        if let Some(parent) = dest_path.parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            pb.println(format!("Error creating directory {:?}: {}", parent, e));
            pb.inc(1);
            continue;
        }

        if let Err(e) = tokio::fs::copy(&source_path, &dest_path).await {
            pb.println(format!(
                "Error copying {:?} to {:?}: {}",
                source_path, dest_path, e
            ));
        }

        pb.inc(1);
    }

    pb.finish_with_message("Done!");

    Ok(())
}
