use crate::constants::CONFIG_FILE_NAME;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Copy liked songs from Navidrome to a local directory"
)]
pub struct Args {
    /// Path to the TOML configuration file.
    /// If not provided, the application will search for the file
    /// in the current directory and then in the XDG config directory.
    #[arg(short = 'i', long, help = format!("Path to the TOML configuration file. If not provided, the application will search for '{}' in the current directory and then in the XDG config directory.", CONFIG_FILE_NAME))]
    pub config: Option<PathBuf>,
}
