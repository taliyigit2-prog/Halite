use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlaying {
    pub app: String,
    pub track: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub artwork: Option<String>,
}

/// Try to detect the currently playing track via AppleScript (Apple Music / Spotify).
/// Best-effort: returns `Ok(None)` when no source is available.
#[cfg(target_os = "macos")]
pub fn now_playing() -> Option<NowPlaying> {
    // Apple Music (the app is named "Music" on modern macOS).
    if let Some(mut np) = query_app("Music") {
        np.app = "Apple Music".to_string();
        return Some(np);
    }
    // Spotify.
    if let Some(mut np) = query_app("Spotify") {
        np.app = "Spotify".to_string();
        return Some(np);
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn now_playing() -> Option<NowPlaying> {
    None
}

#[cfg(target_os = "macos")]
fn query_app(app: &str) -> Option<NowPlaying> {
    let script = format!(
        r#"tell application "{app}"
    if it is running then
        try
            set t to name of current track
            set a to artist of current track
            set al to album of current track
            set d to duration of current track
            return t & linefeed & a & linefeed & al & linefeed & d
        end try
    end if
end tell"#
    );

    let output = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }
    let parts: Vec<&str> = text.lines().collect();
    if parts.len() < 2 {
        return None;
    }
    let track = parts[0].trim().to_string();
    let artist = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
    let album = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
    let duration = parts
        .get(3)
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0);

    if track.is_empty() {
        return None;
    }

    Some(NowPlaying {
        app: app.to_string(),
        track,
        artist,
        album,
        duration,
        artwork: None,
    })
}
