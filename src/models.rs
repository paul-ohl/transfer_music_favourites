use std::path::PathBuf;

use serde::Deserialize;

pub struct SyncConfig {
    pub navidrome_dir: PathBuf,
    pub local_dir: PathBuf,
    pub dest_dir: PathBuf,
    /// The audio format to convert the file to
    pub format: Option<Format>,
    pub on_conflict: ConflictStrategy,
    pub priority: ConversionPriority,
    pub whitelist: Option<Vec<String>>,
    pub blacklist: Option<Vec<String>>,
}

pub struct ApiConfig {
    pub url: String,
    pub user: String,
    pub password: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Mp3,
    Opus,
    Ogg,
}

impl AsRef<str> for Format {
    fn as_ref(&self) -> &str {
        match *self {
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Ogg => "ogg",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStrategy {
    Overwrite,
    Ignore,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversionPriority {
    Quality,
    Balance,
    Compression,
}

#[derive(Deserialize, Debug)]
pub struct SubsonicResponse {
    #[serde(rename = "subsonic-response")]
    pub subsonic_response: SubsonicResponseBody,
}

#[derive(Deserialize, Debug)]
pub struct SubsonicResponseBody {
    pub status: String,
    pub starred: Option<Starred>,
}

#[derive(Deserialize, Debug)]
pub struct Starred {
    pub song: Option<Vec<Song>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub path: String,
}

pub struct SongToConvert {
    pub title: String,
    pub navidrome_path: String,
    pub src_path: PathBuf,
    pub dst_path: PathBuf,
}
