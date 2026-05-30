use std::path::Path;

use anyhow::Result;
use tokio::process::Command;

use crate::models::{ConversionPriority, Format, SyncConfig};

pub(crate) async fn convert_song(
    config: &SyncConfig,
    source_path: &Path,
    dest_path: &Path,
) -> Result<()> {
    let file_name = dest_path.file_name().unwrap_or_default().to_string_lossy();
    let tmp_dest = dest_path.with_file_name(format!(".tmp.{}", file_name));

    let mut cmd = Command::new("ffmpeg");
     cmd.arg("-i").arg(source_path);
    cmd.arg("-map_metadata").arg("0");
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
