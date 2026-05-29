use crate::models::{
    ConflictStrategy, ConversionPriority, Format, Song, SongToConvert, SyncConfig,
};
use crate::sync::utils::{check_ffmpeg_installed, needs_conversion, setup_progress_bar};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use std::path::Path;
use std::sync::Arc;
use std::thread;
use tokio::process::Command;

pub async fn sync_music(config: &SyncConfig, songs: Vec<Song>) -> Result<()> {
    if songs.is_empty() {
        println!("No liked songs found matching the criteria.");
        return Ok(());
    }

    if config.format.is_some() {
        check_ffmpeg_installed().await?;
    }

    println!("Found {} liked songs.", songs.len());

    let songs_to_transfer: Vec<SongToConvert> = songs
        .into_iter()
        .filter_map(|song| should_transfer_song(config, &song, &config.navidrome_dir))
        .collect();

    let progress_bar = setup_progress_bar(songs_to_transfer.len() as u64)?;
    progress_bar.println(format!(
        "Start transfering {} songs",
        songs_to_transfer.len()
    ));

    let workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    stream::iter(songs_to_transfer)
        .for_each_concurrent(workers, |song| {
            let config = &config;
            let progress_bar = Arc::clone(&progress_bar);

            async move {
                progress_bar.set_message(song.title.clone());

                if let Err(e) = process_song(config, &song).await {
                    progress_bar.println(format!(
                        "Warning: Could not process '{}': {}",
                        song.title, e
                    ));
                }

                progress_bar.inc(1);
            }
        })
        .await;

    progress_bar.finish_with_message("Done transfering songs!");

    Ok(())
}

/// This function simply checks if the final path the music file will have already exists or not.
///
/// It first converts the navidrome file path to a dest dir file path, and replaces the extension if
/// needed. Afterwards, it simply checks if the file with that path and extension already exists or
/// not.
fn should_transfer_song(
    config: &SyncConfig,
    song: &Song,
    navidrome_dir_path: &Path,
) -> Option<SongToConvert> {
    let song_path = Path::new(&song.path);
    let rel_path = song_path
        .strip_prefix(navidrome_dir_path)
        .unwrap_or(song_path);
    let rel_path = rel_path.strip_prefix("/").unwrap_or(rel_path);

    let source_path = config.local_dir.join(rel_path);
    let mut dest_path = config.dest_dir.join(rel_path);

    let needs_conversion = needs_conversion(config, &source_path);

    if needs_conversion && let Some(format) = &config.format {
        dest_path.set_extension(format.as_ref());
    }

    if dest_path.exists() {
        None
    } else {
        Some(SongToConvert {
            title: song.title.clone(),
            navidrome_path: song.path.clone(),
            src_path: source_path,
            dst_path: dest_path,
        })
    }
}

async fn process_song(config: &SyncConfig, song: &SongToConvert) -> Result<()> {
    if !song.src_path.exists() {
        anyhow::bail!("Source file not found: {:?}", song.src_path);
    }

    if let Some(parent) = song.dst_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
        if handle_conflicts(config, &song.dst_path).await? {
            return Ok(());
        }
    }

    if needs_conversion(config, &song.src_path) {
        convert_song(config, &song.src_path, &song.dst_path).await?;
    } else {
        tokio::fs::copy(&song.src_path, &song.dst_path).await?;
    }

    Ok(())
}

async fn handle_conflicts(config: &SyncConfig, dest_path: &Path) -> Result<bool> {
    let parent = dest_path.parent().unwrap();
    let file_stem = dest_path.file_stem().unwrap_or_default();
    let mut conflict_found = false;

    if let Ok(mut entries) = tokio::fs::read_dir(parent).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let p = entry.path();
            if p.is_file() && p.file_stem() == Some(file_stem) && p != dest_path {
                conflict_found = true;
                if config.on_conflict == ConflictStrategy::Overwrite {
                    let _ = tokio::fs::remove_file(&p).await;
                }
            }
        }
    }

    if conflict_found && config.on_conflict == ConflictStrategy::Ignore {
        return Ok(true);
    }

    Ok(false)
}

async fn convert_song(config: &SyncConfig, source_path: &Path, dest_path: &Path) -> Result<()> {
    let file_name = dest_path.file_name().unwrap_or_default().to_string_lossy();
    let tmp_dest = dest_path.with_file_name(format!(".tmp.{}", file_name));

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i").arg(source_path);
    cmd.arg("-vn");

    match config.format.as_ref().unwrap() {
        Format::Mp3 => {
            cmd.arg("-c:a").arg("libmp3lame");
            let q = match config.priority {
                ConversionPriority::Quality => "0",
                ConversionPriority::Balance => "2",
                ConversionPriority::Compression => "5",
            };
            cmd.arg("-q:a").arg(q);
        }
        Format::Opus => {
            cmd.arg("-c:a").arg("libopus");
            let b = match config.priority {
                ConversionPriority::Quality => "192k",
                ConversionPriority::Balance => "128k",
                ConversionPriority::Compression => "64k",
            };
            cmd.arg("-b:a").arg(b);
        }
        Format::Ogg => {
            cmd.arg("-c:a").arg("libvorbis");
            let q = match config.priority {
                ConversionPriority::Quality => "8",
                ConversionPriority::Balance => "5",
                ConversionPriority::Compression => "2",
            };
            cmd.arg("-q:a").arg(q);
        }
    }

    cmd.arg("-y").arg(&tmp_dest);

    let output = cmd.output().await?;

    if output.status.success() {
        tokio::fs::rename(&tmp_dest, dest_path).await?;
        Ok(())
    } else {
        let _ = tokio::fs::remove_file(&tmp_dest).await;
        anyhow::bail!("FFmpeg error: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_needs_conversion_mp3() {
        let config = SyncConfig {
            navidrome_dir: PathBuf::from("/music"),
            local_dir: PathBuf::from("/local"),
            dest_dir: PathBuf::from("/dest"),
            format: Some(Format::Mp3),
            on_conflict: ConflictStrategy::Ignore,
            priority: ConversionPriority::Balance,
            whitelist: None,
            blacklist: None,
        };

        // Needs conversion
        assert!(needs_conversion(&config, Path::new("song.flac")));
        assert!(needs_conversion(&config, Path::new("song.wav")));
        assert!(needs_conversion(&config, Path::new("song.opus")));
        assert!(needs_conversion(&config, Path::new("song.ogg")));

        // Doesn't need conversion
        assert!(!needs_conversion(&config, Path::new("song.mp3")));
        assert!(!needs_conversion(&config, Path::new("song.MP3")));
    }

    #[test]
    fn test_needs_conversion_opus() {
        let config = SyncConfig {
            navidrome_dir: PathBuf::from("/music"),
            local_dir: PathBuf::from("/local"),
            dest_dir: PathBuf::from("/dest"),
            format: Some(Format::Opus),
            on_conflict: ConflictStrategy::Ignore,
            priority: ConversionPriority::Balance,
            whitelist: None,
            blacklist: None,
        };

        // Needs conversion
        assert!(needs_conversion(&config, Path::new("song.flac")));
        assert!(needs_conversion(&config, Path::new("song.wav")));
        assert!(needs_conversion(&config, Path::new("song.mp3")));
        assert!(needs_conversion(&config, Path::new("song.ogg")));

        // Doesn't need conversion
        assert!(!needs_conversion(&config, Path::new("song.opus")));
        assert!(!needs_conversion(&config, Path::new("song.OPUS")));
    }

    #[test]
    fn test_needs_conversion_none() {
        let config = SyncConfig {
            navidrome_dir: PathBuf::from("/music"),
            local_dir: PathBuf::from("/local"),
            dest_dir: PathBuf::from("/dest"),
            format: None,
            on_conflict: ConflictStrategy::Ignore,
            priority: ConversionPriority::Balance,
            whitelist: None,
            blacklist: None,
        };

        // Never needs conversion if no format is specified
        assert!(!needs_conversion(&config, Path::new("song.flac")));
        assert!(!needs_conversion(&config, Path::new("song.wav")));
        assert!(!needs_conversion(&config, Path::new("song.mp3")));
        assert!(!needs_conversion(&config, Path::new("song.opus")));
        assert!(!needs_conversion(&config, Path::new("song.ogg")));
    }

    #[test]
    fn test_needs_conversion_ogg() {
        let config = SyncConfig {
            navidrome_dir: PathBuf::from("/music"),
            local_dir: PathBuf::from("/local"),
            dest_dir: PathBuf::from("/dest"),
            format: Some(Format::Ogg),
            on_conflict: ConflictStrategy::Ignore,
            priority: ConversionPriority::Balance,
            whitelist: None,
            blacklist: None,
        };

        // Needs conversion
        assert!(needs_conversion(&config, Path::new("song.flac")));
        assert!(needs_conversion(&config, Path::new("song.wav")));
        assert!(needs_conversion(&config, Path::new("song.mp3")));
        assert!(needs_conversion(&config, Path::new("song.opus")));

        // Doesn't need conversion
        assert!(!needs_conversion(&config, Path::new("song.ogg")));
        assert!(!needs_conversion(&config, Path::new("song.OGG")));
    }
}
