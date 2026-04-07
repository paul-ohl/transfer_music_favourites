use serde::Deserialize;

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

#[derive(Deserialize, Debug)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub path: String,
}
