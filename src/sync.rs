use crate::cli::{ConflictStrategy, ConversionPriority, Format};
use crate::models::Song;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct SyncConfig {
    pub navidrome_dir: String,
    pub local_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub format: Option<Format>,
    pub on_conflict: ConflictStrategy,
    pub priority: ConversionPriority,
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

        let mut dest_path = config.dest_dir.join(rel_path);

        if let Some(format) = &config.format {
            let ext = match format {
                Format::Mp3 => "mp3",
                Format::Opus => "opus",
            };
            dest_path.set_extension(ext);
        }

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

        if let Some(parent) = dest_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                pb.println(format!("Error creating directory {:?}: {}", parent, e));
                pb.inc(1);
                continue;
            }

            // Conflict resolution
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
                pb.inc(1);
                continue;
            }
        }

        let needs_conversion = match &config.format {
            Some(Format::Mp3) => !source_path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("mp3")),
            Some(Format::Opus) => !source_path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("opus")),
            None => false,
        };

        if !needs_conversion {
            if let Err(e) = tokio::fs::copy(&source_path, &dest_path).await {
                pb.println(format!(
                    "Error copying {:?} to {:?}: {}",
                    source_path, dest_path, e
                ));
            }
        } else {
            let tmp_dest = dest_path.with_extension(format!(
                "{}.tmp",
                dest_path.extension().unwrap_or_default().to_string_lossy()
            ));

            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-i").arg(&source_path);

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

            let output = cmd.output().await;
            match output {
                Ok(out) if out.status.success() => {
                    if let Err(e) = tokio::fs::rename(&tmp_dest, &dest_path).await {
                        pb.println(format!("Error renaming temp file {:?}: {}", tmp_dest, e));
                    }
                }
                Ok(out) => {
                    let _ = tokio::fs::remove_file(&tmp_dest).await;
                    pb.println(format!(
                        "FFmpeg error for {:?}: {}",
                        source_path,
                        String::from_utf8_lossy(&out.stderr)
                    ));
                }
                Err(e) => {
                    let _ = tokio::fs::remove_file(&tmp_dest).await;
                    pb.println(format!(
                        "Failed to execute FFmpeg for {:?}: {}",
                        source_path, e
                    ));
                }
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Done!");

    Ok(())
}
