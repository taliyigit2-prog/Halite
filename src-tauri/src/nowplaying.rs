use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlaying {
    pub app: String,
    pub track: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub position: f64,
    pub playing: bool,
    pub artwork: Option<String>,
}

/// Try to detect the currently playing track via AppleScript (Apple Music / Spotify).
/// Best-effort: returns `None` when no source is available.
#[cfg(target_os = "macos")]
pub fn now_playing() -> Option<NowPlaying> {
    // Query both apps: a paused Music session must not hide a track actively
    // playing in Spotify (or vice versa).
    let mut music = query_app("Music").map(|mut np| {
        np.app = "Apple Music".to_string();
        np
    });
    let mut spotify = query_app("Spotify").map(|mut np| {
        np.app = "Spotify".to_string();
        np
    });
    if music.as_ref().is_some_and(|np| np.playing) {
        return music.take();
    }
    if spotify.as_ref().is_some_and(|np| np.playing) {
        return spotify.take();
    }
    music.or(spotify)
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
            set p to player position
            set s to player state as string
            if s is "stopped" then return ""
            set sep to character id 30
            return t & sep & a & sep & al & sep & d & sep & p & sep & s
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
    let parts: Vec<&str> = text.split('\u{1e}').collect();
    if parts.len() < 6 {
        return None;
    }
    let track = parts[0].trim().to_string();
    let artist = parts
        .get(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let album = parts
        .get(2)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let duration = parts
        .get(3)
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0);
    let position = parts
        .get(4)
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0);
    let playing = parts
        .get(5)
        .map(|s| s.trim().eq_ignore_ascii_case("playing"))
        .unwrap_or(false);

    // Spotify exposes track duration in milliseconds while Apple Music uses
    // seconds. Player position is seconds in both applications.
    let duration = if app == "Spotify" {
        duration / 1000.0
    } else {
        duration
    };

    if track.is_empty() {
        return None;
    }

    Some(NowPlaying {
        app: app.to_string(),
        track,
        artist,
        album,
        duration,
        position,
        playing,
        artwork: None,
    })
}
