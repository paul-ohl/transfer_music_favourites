use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Format {
    Mp3,
    Opus,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum ConflictStrategy {
    Overwrite,
    Ignore,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum ConversionPriority {
    Quality,
    Balance,
    Compression,
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Copy liked songs from Navidrome to a local directory"
)]
pub struct Args {
    /// Navidrome server URL (e.g., http://localhost:4533)
    #[arg(short, long)]
    pub url: String,

    /// Navidrome username
    #[arg(long)]
    pub user: String,

    /// Navidrome password
    #[arg(long, env = "NAVIDROME_PASSWORD")]
    pub password: String,

    /// The root music directory as seen by Navidrome (e.g., /music)
    #[arg(long)]
    pub navidrome_dir: String,

    /// The root music directory on your local machine (e.g., /mnt/storage/music)
    #[arg(long)]
    pub local_dir: PathBuf,

    /// The destination directory to copy liked songs to
    #[arg(long)]
    pub dest_dir: PathBuf,

    /// Format to convert songs to (mp3, opus). Copied as-is if omitted.
    #[arg(long)]
    pub format: Option<Format>,

    /// Strategy to use if a file with the same name but different extension exists.
    #[arg(long, default_value = "overwrite")]
    pub on_conflict: ConflictStrategy,

    /// Priority for conversion (quality, balance, compression)
    #[arg(long, default_value = "balance")]
    pub priority: ConversionPriority,
}
