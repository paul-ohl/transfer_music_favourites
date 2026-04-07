use clap::Parser;
use std::path::PathBuf;

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
}
