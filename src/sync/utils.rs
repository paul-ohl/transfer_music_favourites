use std::{path::Path, sync::Arc};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::process::Command;

use crate::models::SyncConfig;

pub(super) async fn check_ffmpeg_installed() -> Result<()> {
    let output = Command::new("ffmpeg").arg("-version").output().await;
    match output {
        Ok(out) if out.status.success() => Ok(()),
        _ => anyhow::bail!(
            "ffmpeg is required for format conversion but was not found or failed to execute."
        ),
    }
}

pub(super) fn needs_conversion(config: &SyncConfig, source_path: &Path) -> bool {
    let target_format_str = match &config.format {
        Some(f) => f.as_ref(),
        None => return false,
    };

    let source_ext = match source_path.extension().and_then(|s| s.to_str()) {
        Some(ext) => ext,
        None => return true, // No extension, assume it needs conversion if a target is set
    };

    if source_ext.eq_ignore_ascii_case(target_format_str) {
        return false;
    }

    if let Some(whitelist) = &config.whitelist {
        if !whitelist
            .iter()
            .any(|ext| ext.eq_ignore_ascii_case(source_ext))
        {
            return false;
        }
    } else if let Some(blacklist) = &config.blacklist
        && blacklist
            .iter()
            .any(|ext| ext.eq_ignore_ascii_case(source_ext))
    {
        return false;
    }

    true
}

pub(super) fn setup_progress_bar(size: u64) -> Result<Arc<ProgressBar>> {
    let progress_bar = ProgressBar::new(size);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")?
            .progress_chars("##-"),
    );

    Ok(Arc::new(progress_bar))
}
