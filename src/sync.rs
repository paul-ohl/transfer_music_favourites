use crate::cli::{ConflictStrategy, ConversionPriority, Format};
use crate::models::Song;
use anyhow::Result;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use tokio::process::Command;

pub struct SyncConfig {
    pub navidrome_dir: String,
    pub local_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub format: Option<Format>,
    pub on_conflict: ConflictStrategy,
    pub priority: ConversionPriority,
    pub delete_unliked: bool,
}

async fn check_ffmpeg_installed() -> Result<()> {
    let output = Command::new("ffmpeg").arg("-version").output().await;
    match output {
        Ok(out) if out.status.success() => Ok(()),
        _ => anyhow::bail!(
            "ffmpeg is required for format conversion but was not found or failed to execute."
        ),
    }
}

pub async fn sync_songs(config: &SyncConfig, songs: Vec<Song>) -> Result<()> {
    if songs.is_empty() {
        println!("No liked songs found.");
        return Ok(());
    }

    if config.format.is_some() {
        check_ffmpeg_installed().await?;
    }

    if config.delete_unliked {
        delete_unliked_songs(config, &songs).await?;
    }

    println!("Found {} liked songs. Starting copy...", songs.len());

    let progress_bar = ProgressBar::new(songs.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")?
            .progress_chars("##-"),
    );

    let progress_bar = Arc::new(progress_bar);
    let navidrome_dir_path = PathBuf::from(&config.navidrome_dir);

    // Determine optimal concurrency based on available CPU cores.
    // This allows tokio and ffmpeg to utilize the system's full parallel capabilities
    // without overloading the OS with too many simultaneous file/process handles.
    let workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    // Create a stream from the songs vector and process them concurrently
    // Note: We use futures::stream rather than rayon because we're running async tasks
    // like tokio::fs and tokio::process.
    stream::iter(songs)
        .for_each_concurrent(workers, |song| {
            let config = &config;
            let progress_bar = Arc::clone(&progress_bar);
            let navidrome_dir_path = navidrome_dir_path.clone();

            async move {
                progress_bar.set_message(song.title.clone());

                if let Err(e) = process_song(config, &song, &navidrome_dir_path).await {
                    // Only print errors that aren't just "skip this file" messages
                    progress_bar.println(format!(
                        "Warning: Could not process '{}': {}",
                        song.title, e
                    ));
                }

                progress_bar.inc(1);
            }
        })
        .await;

    progress_bar.finish_with_message("Done!");

    Ok(())
}

async fn delete_unliked_songs(config: &SyncConfig, songs: &[Song]) -> Result<()> {
    if !config.dest_dir.exists() {
        return Ok(());
    }

    println!("Checking for unliked songs to delete...");

    let navidrome_dir_path = PathBuf::from(&config.navidrome_dir);
    let mut expected_paths = HashSet::new();

    // Build expected paths without extension
    for song in songs {
        let song_path = Path::new(&song.path);
        let rel_path = song_path
            .strip_prefix(&navidrome_dir_path)
            .unwrap_or(song_path);
        let rel_path = rel_path.strip_prefix("/").unwrap_or(rel_path);
        expected_paths.insert(rel_path.with_extension(""));
    }

    let mut dirs = vec![config.dest_dir.clone()];
    let mut deleted_count = 0;

    let audio_extensions = [
        "mp3", "flac", "opus", "wav", "m4a", "ogg", "aac", "alac", "wma",
    ];

    while let Some(dir) = dirs.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                // Only consider deleting files with known audio extensions
                // or temp files created by this tool
                let is_audio = audio_extensions.contains(&ext.as_str());
                let is_temp = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(".tmp."));

                if (is_audio || is_temp)
                    && let Ok(rel_path) = path.strip_prefix(&config.dest_dir)
                {
                    let rel_no_ext = if is_temp {
                        // Strip `.tmp.` prefix and actual extension
                        let file_name = rel_path.file_name().unwrap().to_string_lossy();
                        let clean_name = file_name.strip_prefix(".tmp.").unwrap_or(&file_name);
                        rel_path.with_file_name(clean_name).with_extension("")
                    } else {
                        rel_path.with_extension("")
                    };

                    if !expected_paths.contains(&rel_no_ext) {
                        if let Err(e) = tokio::fs::remove_file(&path).await {
                            eprintln!("Failed to delete unliked song {:?}: {}", path, e);
                        } else {
                            deleted_count += 1;
                        }
                    }
                }
            }
        }
    }

    if deleted_count > 0 {
        println!("Deleted {} unliked song(s).", deleted_count);
    } else {
        println!("No unliked songs found to delete.");
    }

    Ok(())
}

async fn process_song(config: &SyncConfig, song: &Song, navidrome_dir_path: &Path) -> Result<()> {
    // 1. Path resolution
    let song_path = Path::new(&song.path);
    let rel_path = song_path
        .strip_prefix(navidrome_dir_path)
        .unwrap_or(song_path);
    let rel_path = rel_path.strip_prefix("/").unwrap_or(rel_path);

    let source_path = config.local_dir.join(rel_path);
    let mut dest_path = config.dest_dir.join(rel_path);

    if let Some(format) = &config.format {
        let ext = match format {
            Format::Mp3 => "mp3",
            Format::Opus => "opus",
        };
        dest_path.set_extension(ext);
    }

    if dest_path.exists() {
        return Ok(());
    }

    if !source_path.exists() {
        anyhow::bail!("Source file not found: {:?}", source_path);
    }

    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent).await?;

        // 2. Conflict resolution
        if handle_conflicts(config, &dest_path).await? {
            return Ok(());
        }
    }

    // 3. Copy or convert
    if !needs_conversion(config, &source_path) {
        tokio::fs::copy(&source_path, &dest_path).await?;
    } else {
        convert_song(config, &source_path, &dest_path).await?;
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

fn needs_conversion(config: &SyncConfig, source_path: &Path) -> bool {
    match &config.format {
        Some(Format::Mp3) => !source_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mp3")),
        Some(Format::Opus) => !source_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("opus")),
        None => false,
    }
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
            navidrome_dir: "/music".to_string(),
            local_dir: PathBuf::from("/local"),
            dest_dir: PathBuf::from("/dest"),
            format: Some(Format::Mp3),
            on_conflict: ConflictStrategy::Ignore,
            priority: ConversionPriority::Balance,
            delete_unliked: false,
        };

        // Needs conversion
        assert!(needs_conversion(&config, Path::new("song.flac")));
        assert!(needs_conversion(&config, Path::new("song.wav")));
        assert!(needs_conversion(&config, Path::new("song.opus")));

        // Doesn't need conversion
        assert!(!needs_conversion(&config, Path::new("song.mp3")));
        assert!(!needs_conversion(&config, Path::new("song.MP3")));
    }

    #[test]
    fn test_needs_conversion_opus() {
        let config = SyncConfig {
            navidrome_dir: "/music".to_string(),
            local_dir: PathBuf::from("/local"),
            dest_dir: PathBuf::from("/dest"),
            format: Some(Format::Opus),
            on_conflict: ConflictStrategy::Ignore,
            priority: ConversionPriority::Balance,
            delete_unliked: false,
        };

        // Needs conversion
        assert!(needs_conversion(&config, Path::new("song.flac")));
        assert!(needs_conversion(&config, Path::new("song.wav")));
        assert!(needs_conversion(&config, Path::new("song.mp3")));

        // Doesn't need conversion
        assert!(!needs_conversion(&config, Path::new("song.opus")));
        assert!(!needs_conversion(&config, Path::new("song.OPUS")));
    }

    #[test]
    fn test_needs_conversion_none() {
        let config = SyncConfig {
            navidrome_dir: "/music".to_string(),
            local_dir: PathBuf::from("/local"),
            dest_dir: PathBuf::from("/dest"),
            format: None,
            on_conflict: ConflictStrategy::Ignore,
            priority: ConversionPriority::Balance,
            delete_unliked: false,
        };

        // Never needs conversion if no format is specified
        assert!(!needs_conversion(&config, Path::new("song.flac")));
        assert!(!needs_conversion(&config, Path::new("song.wav")));
        assert!(!needs_conversion(&config, Path::new("song.mp3")));
        assert!(!needs_conversion(&config, Path::new("song.opus")));
    }
}
