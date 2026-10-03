use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Halite/0.1 (https://github.com/taliyigit2-prog/Halite)")
        .build()?;
    if track.trim().is_empty() || artist.trim().is_empty() {
        return Ok(None);
    }
    let mut query = vec![
        ("track_name", track.trim().to_string()),
        ("artist_name", artist.trim().to_string()),
    ];
    if !album.trim().is_empty() {
        query.push(("album_name", album.trim().to_string()));
    }
    if duration.is_finite() && duration > 0.0 {
        query.push(("duration", format!("{duration:.2}")));
    }
    let request = client.get(format!("{API_BASE}/get")).query(&query);
    let resp = request.send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let resp = resp.error_for_status()?;
    let lyrics: LyricsResult = resp.json().await?;
    // LRCLIB returns a valid object even on not-found (with a 404 status usually),
    // so we treat an empty track_name as "not found".
    if lyrics.track_name.as_deref().unwrap_or("").is_empty() && lyrics.synced_lyrics.is_none() {
        return Ok(None);
    }
    Ok(Some(lyrics))
}

pub async fn search_lyrics(query: &str) -> Result<Vec<LyricsResult>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let query: String = query.chars().take(200).collect();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Halite/0.1 (https://github.com/taliyigit2-prog/Halite)")
        .build()?;
    let resp = client
        .get(format!("{API_BASE}/search"))
        .query(&[("q", &query)])
        .send()
        .await?
        .error_for_status()?;
    let results: Vec<LyricsResult> = resp
        .json()
        .await
        .map_err(|e| Error::Message(format!("lyrics response error: {e}")))?;
    Ok(results)
}
