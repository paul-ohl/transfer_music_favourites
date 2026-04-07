use crate::models::{Song, SubsonicResponse};
use anyhow::{Context, Result};
use md5::{Digest, Md5};
use rand::{RngExt, distr::Alphanumeric};

pub struct ApiConfig {
    pub url: String,
    pub user: String,
    pub password: String,
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

pub async fn fetch_starred_songs(config: &ApiConfig) -> Result<Vec<Song>> {
    let salt = generate_salt();
    let token = generate_auth_token(&config.password, &salt);

    let client = reqwest::Client::new();
    let api_url = format!("{}/rest/getStarred", config.url.trim_end_matches('/'));

    let res = client
        .get(&api_url)
        .query(&[
            ("u", config.user.as_str()),
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

    Ok(response
        .subsonic_response
        .starred
        .and_then(|s| s.song)
        .unwrap_or_default())
}
