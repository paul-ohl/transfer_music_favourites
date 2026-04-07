use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use md5::{Digest, Md5};
use rand::{RngExt, distr::Alphanumeric};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Copy liked songs from Navidrome to a local directory"
)]
struct Args {
    /// Navidrome server URL (e.g., http://localhost:4533)
    #[arg(short, long)]
    url: String,

    /// Navidrome username
    #[arg(long)]
    user: String,

    /// Navidrome password
    #[arg(long, env = "NAVIDROME_PASSWORD")]
    password: String,

    /// The root music directory as seen by Navidrome (e.g., /music)
    #[arg(long)]
    navidrome_dir: String,

    /// The root music directory on your local machine (e.g., /mnt/storage/music)
    #[arg(long)]
    local_dir: PathBuf,

    /// The destination directory to copy liked songs to
    #[arg(long)]
    dest_dir: PathBuf,
}

#[derive(Deserialize, Debug)]
struct SubsonicResponse {
    #[serde(rename = "subsonic-response")]
    subsonic_response: SubsonicResponseBody,
}

#[derive(Deserialize, Debug)]
struct SubsonicResponseBody {
    status: String,
    starred: Option<Starred>,
}

#[derive(Deserialize, Debug)]
struct Starred {
    song: Option<Vec<Song>>,
}

#[derive(Deserialize, Debug)]
struct Song {
    title: String,
    path: String,
}

fn generate_salt() -> String {
    let rng = rand::rng();
    let s: String = rng
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    s
}

fn generate_auth_token(password: &str, salt: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    hex::encode(hasher.finalize())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let salt = generate_salt();
    let token = generate_auth_token(&args.password, &salt);

    let client = reqwest::Client::new();
    let api_url = format!("{}/rest/getStarred", args.url.trim_end_matches('/'));

    println!("Fetching liked songs from Navidrome...");

    let res = client
        .get(&api_url)
        .query(&[
            ("u", args.user.as_str()),
            ("t", token.as_str()),
            ("s", salt.as_str()),
            ("v", "1.16.1"),
            ("c", "navidrome-sync"),
            ("f", "json"),
        ])
        .send()
        .await
        .context("Failed to connect to Navidrome")?
        .error_for_status()
        .context("Navidrome API returned an HTTP error")?;

    let response_text = res.text().await?;
    let response: SubsonicResponse = serde_json::from_str(&response_text).context(format!(
        "Failed to parse Subsonic API response: {}",
        response_text
    ))?;

    if response.subsonic_response.status != "ok" {
        anyhow::bail!(
            "Subsonic API returned an error: {}",
            response.subsonic_response.status
        );
    }

    let songs = response
        .subsonic_response
        .starred
        .and_then(|s| s.song)
        .unwrap_or_default();

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

    let navidrome_dir_path = Path::new(&args.navidrome_dir);

    for song in songs {
        // Handle paths whether they are absolute (from Navidrome's view) or already relative
        let song_path = Path::new(&song.path);
        let rel_path = song_path
            .strip_prefix(navidrome_dir_path)
            .unwrap_or(song_path);
        let rel_path = rel_path.strip_prefix("/").unwrap_or(rel_path);

        let source_path = args.local_dir.join(rel_path);
        let dest_path = args.dest_dir.join(rel_path);

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
