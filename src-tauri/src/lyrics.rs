use serde::{Deserialize, Serialize};

use crate::error::Result;

const API_BASE: &str = "https://lrclib.net/api";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsResult {
    pub id: Option<i64>,
    pub track_name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: Option<bool>,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

pub async fn get_lyrics(
    track: &str,
    artist: &str,
    album: &str,
    duration: f64,
) -> Result<Option<LyricsResult>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{API_BASE}/get"))
        .query(&[
            ("track_name", track),
            ("artist_name", artist),
            ("album_name", album),
        ])
        .send()
        .await?
        .error_for_status()?;
    let lyrics: LyricsResult = resp.json().await?;
    // LRCLIB returns a valid object even on not-found (with a 404 status usually),
    // so we treat an empty track_name as "not found".
    if lyrics.track_name.as_deref().unwrap_or("").is_empty() && lyrics.synced_lyrics.is_none() {
        return Ok(None);
    }
    let _ = duration;
    Ok(Some(lyrics))
}

pub async fn search_lyrics(query: &str) -> Result<Vec<LyricsResult>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{API_BASE}/search"))
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?;
    let results: Vec<LyricsResult> = resp.json().await?;
    Ok(results)
}
