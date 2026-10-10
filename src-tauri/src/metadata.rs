//! Local metadata editing. Never modifies the source until a validated copy is ready.
use crate::{
    error::{Error, Result},
    state::AppState,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{
    config::WriteOptions,
    file::{AudioFile, TaggedFileExt},
    picture::{Picture, PictureInformation, PictureType},
    prelude::ItemKey,
    probe::Probe,
    tag::Tag,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

fn failure(error: impl std::fmt::Display) -> Error {
    Error::Message(format!("HALITE_TAG_ERROR|{error}"))
}
fn tagged_file(path: &Path) -> Result<lofty::file::TaggedFile> {
    let mut probe = Probe::open(path).map_err(failure)?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("halite-backup") {
        if let Some(kind) = path
            .file_stem()
            .and_then(|name| lofty::file::FileType::from_path(Path::new(name)))
        {
            probe = probe.set_file_type(kind);
        }
    }
    if probe.file_type().is_none() || path.extension().and_then(|ext| ext.to_str()) == Some("ogg") {
        probe = probe.guess_file_type().map_err(failure)?;
    }
    probe.read().map_err(failure)
}

const FIELDS: &[(&str, ItemKey)] = &[
    ("title", ItemKey::TrackTitle),
    ("artist", ItemKey::TrackArtist),
    ("album", ItemKey::AlbumTitle),
    ("album_artist", ItemKey::AlbumArtist),
    ("genre", ItemKey::Genre),
    ("date", ItemKey::RecordingDate),
    ("track", ItemKey::TrackNumber),
    ("track_total", ItemKey::TrackTotal),
    ("disc", ItemKey::DiscNumber),
    ("disc_total", ItemKey::DiscTotal),
    ("comment", ItemKey::Comment),
    ("composer", ItemKey::Composer),
    ("copyright", ItemKey::CopyrightMessage),
    ("lyrics", ItemKey::Lyrics),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileStamp {
    pub bytes: u64,
    pub modified: u128,
}
fn stamp(path: &Path) -> Result<FileStamp> {
    let meta = std::fs::metadata(path)?;
    if !meta.is_file() || meta.len() > 8_000_000_000 {
        return Err(failure("invalid file or file over 8 GB"));
    }
    Ok(FileStamp {
        bytes: meta.len(),
        modified: meta
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    })
}

#[derive(Debug, Serialize)]
pub struct Metadata {
    pub path: String,
    pub name: String,
    pub stamp: FileStamp,
    pub fields: BTreeMap<String, String>,
    pub duration: f64,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bitrate: Option<u32>,
    pub cover: Option<String>,
    pub cover_count: usize,
    pub supported_fields: Vec<String>,
    pub supports_cover: bool,
    pub has_backup: bool,
}

pub fn read(path: &Path) -> Result<Metadata> {
    let before = stamp(path)?;
    let file = tagged_file(path)?;
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let fields = FIELDS
        .iter()
        .map(|(name, key)| {
            let value = tag
                .map(|tag| tag.get_strings(*key).collect::<Vec<_>>().join("; "))
                .unwrap_or_default();
            ((*name).to_owned(), value)
        })
        .collect();
    let cover = tag.and_then(|tag| {
        tag.get_picture_type(PictureType::CoverFront)
            .or_else(|| tag.pictures().first())
    });
    let properties = file.properties();
    Ok(Metadata {
        path: path.to_string_lossy().into(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        stamp: before,
        fields,
        duration: properties.duration().as_secs_f64(),
        sample_rate: properties.sample_rate(),
        channels: properties.channels(),
        bitrate: properties.audio_bitrate(),
        cover: cover
            .filter(|pic| pic.data().len() <= 10_000_000)
            .map(|pic| {
                format!(
                    "data:{};base64,{}",
                    pic.mime_type()
                        .map(|mime| mime.as_str())
                        .unwrap_or("image/jpeg"),
                    STANDARD.encode(pic.data())
                )
            }),
        cover_count: tag.map(|tag| tag.pictures().len()).unwrap_or(0),
        supported_fields: FIELDS
            .iter()
            .filter(|(_, key)| key.map_key(file.primary_tag_type()).is_some())
            .map(|(name, _)| (*name).into())
            .collect(),
        supports_cover: matches!(
            file.primary_tag_type(),
            lofty::tag::TagType::Id3v2
                | lofty::tag::TagType::VorbisComments
                | lofty::tag::TagType::Mp4Ilst
                | lofty::tag::TagType::Ape
        ),
        has_backup: backup_path(path).is_file(),
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct Edit {
    pub path: String,
    pub expected: FileStamp,
    pub fields: BTreeMap<String, String>,
    pub cover_path: Option<String>,
    #[serde(default)]
    pub remove_cover: bool,
}

pub fn write(edit: &Edit) -> Result<Metadata> {
    let path = Path::new(&edit.path);
    if stamp(path)? != edit.expected {
        return Err(Error::Message("HALITE_TAG_CHANGED".into()));
    }
    if std::fs::metadata(path)?.permissions().readonly() {
        return Err(failure("read-only file"));
    }
    for (name, value) in &edit.fields {
        if !FIELDS.iter().any(|(field, _)| name == field)
            || value.len() > 100_000
            || value.contains('\0')
        {
            return Err(failure("invalid field"));
        }
        if ["track", "track_total", "disc", "disc_total"].contains(&name.as_str())
            && !value.is_empty()
            && value.parse::<u32>().is_err()
        {
            return Err(failure("invalid number"));
        }
    }
    let parent = path.parent().ok_or_else(|| failure("missing parent"))?;
    let suffix = format!(
        ".{}",
        path.extension().unwrap_or_default().to_string_lossy()
    );
    let mut temporary = tempfile::Builder::new()
        .prefix(".halite-edit-")
        .suffix(&suffix)
        .tempfile_in(parent)?;
    std::io::copy(&mut std::fs::File::open(path)?, temporary.as_file_mut())?;
    let mut file = tagged_file(temporary.path())?;
    let tag_type = file.primary_tag_type();
    if (edit.cover_path.is_some() || edit.remove_cover)
        && !matches!(
            tag_type,
            lofty::tag::TagType::Id3v2
                | lofty::tag::TagType::VorbisComments
                | lofty::tag::TagType::Mp4Ilst
                | lofty::tag::TagType::Ape
        )
    {
        return Err(failure("cover unsupported by format"));
    }
    if file.primary_tag().is_none() {
        file.insert_tag(Tag::new(tag_type));
    }
    let tag = file
        .primary_tag_mut()
        .ok_or_else(|| failure("unsupported tags"))?;
    for (name, value) in &edit.fields {
        let key = FIELDS.iter().find(|(field, _)| name == field).unwrap().1;
        tag.remove_key(key);
        if !value.is_empty() && !tag.insert_text(key, value.clone()) {
            return Err(failure(format!("field {name} unsupported by format")));
        }
    }
    if edit.remove_cover || edit.cover_path.is_some() {
        if tag_type == lofty::tag::TagType::Mp4Ilst {
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
        } else {
            tag.remove_picture_type(PictureType::CoverFront);
        }
    }
    let mut expected_cover = None;
    if let Some(cover) = &edit.cover_path {
        if std::fs::metadata(cover)?.len() > 10_000_000 {
            return Err(failure("cover exceeds 10 MB"));
        }
        let mut picture =
            Picture::from_reader(&mut std::fs::File::open(cover)?).map_err(failure)?;
        let info = PictureInformation::from_picture(&picture).map_err(failure)?;
        if info.width > 8000 || info.height > 8000 {
            return Err(failure("cover exceeds 8000 pixels"));
        }
        picture.set_pic_type(PictureType::CoverFront);
        expected_cover = Some(picture.data().to_vec());
        tag.push_picture(picture);
    }
    file.save_to_path(temporary.path(), WriteOptions::default())
        .map_err(failure)?;
    let verified = read(temporary.path())?;
    if expected_cover.is_some() || edit.remove_cover {
        let checked = tagged_file(temporary.path())?;
        let front = checked.primary_tag().and_then(|tag| {
            tag.get_picture_type(PictureType::CoverFront).or_else(|| {
                if tag_type == lofty::tag::TagType::Mp4Ilst {
                    tag.pictures().first()
                } else {
                    None
                }
            })
        });
        if front.map(|picture| picture.data()) != expected_cover.as_deref() {
            return Err(failure("cover could not be verified"));
        }
    }
    for (name, expected) in &edit.fields {
        if verified
            .fields
            .get(name)
            .map(String::as_str)
            .unwrap_or_default()
            != expected
        {
            return Err(failure(format!("field {name} could not be verified")));
        }
    }
    let original = read(path)?;
    if (verified.duration - original.duration).abs() > 0.1
        || verified.sample_rate != original.sample_rate
        || verified.channels != original.channels
    {
        return Err(failure("audio properties changed"));
    }
    if stamp(path)? != edit.expected {
        return Err(Error::Message("HALITE_TAG_CHANGED".into()));
    }
    // Keep one recoverable backup per file; the original remains intact on any failure.
    let backup = backup_path(path);
    let mut backup_temp = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut std::fs::File::open(path)?, backup_temp.as_file_mut())?;
    backup_temp.as_file().sync_all()?;
    backup_temp.persist(&backup).map_err(failure)?;
    std::fs::set_permissions(temporary.path(), std::fs::metadata(path)?.permissions())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(failure)?;
    read(path)
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".halite-backup");
    path.with_file_name(name)
}

pub fn require_selected(state: &AppState, path: &str) -> Result<PathBuf> {
    let path = std::fs::canonicalize(path)?;
    if !state
        .selected_files
        .lock()
        .map_err(failure)?
        .contains(&path)
    {
        return Err(failure("file was not selected"));
    }
    Ok(path)
}

#[tauri::command]
pub async fn pick_tag_files(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Metadata>> {
    let paths = app
        .dialog()
        .file()
        .add_filter(
            "Audio",
            &[
                "mp3", "flac", "m4a", "mp4", "ogg", "opus", "wav", "aiff", "ape", "wv", "mpc",
            ],
        )
        .blocking_pick_files()
        .unwrap_or_default();
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .filter_map(|path| path.into_path().ok())
            .map(|path| {
                let path = std::fs::canonicalize(path)?;
                let metadata = read(&path)?;
                state.selected_files.lock().map_err(failure)?.insert(path);
                Ok(metadata)
            })
            .collect()
    })
    .await
    .map_err(failure)?
}

#[tauri::command]
pub async fn pick_cover(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<Option<String>> {
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("Cover", &["png", "jpg", "jpeg"])
        .blocking_pick_file()
        .and_then(|file| file.into_path().ok())
    else {
        return Ok(None);
    };
    let path = std::fs::canonicalize(path)?;
    state
        .selected_files
        .lock()
        .map_err(failure)?
        .insert(path.clone());
    Ok(Some(path.to_string_lossy().into()))
}

#[derive(Serialize)]
pub struct EditResult {
    pub path: String,
    pub metadata: Option<Metadata>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn save_tags(
    state: State<'_, Arc<AppState>>,
    edits: Vec<Edit>,
) -> Result<Vec<EditResult>> {
    if edits.is_empty() || edits.len() > 100 {
        return Err(failure("select 1–100 files"));
    }
    for edit in &edits {
        require_selected(&state, &edit.path)?;
        if let Some(path) = &edit.cover_path {
            require_selected(&state, path)?;
        }
    }
    tauri::async_runtime::spawn_blocking(move || {
        Ok(edits
            .into_iter()
            .map(|edit| match write(&edit) {
                Ok(metadata) => EditResult {
                    path: edit.path,
                    metadata: Some(metadata),
                    error: None,
                },
                Err(error) => EditResult {
                    path: edit.path,
                    metadata: None,
                    error: Some(error.to_string()),
                },
            })
            .collect())
    })
    .await
    .map_err(failure)?
}

#[tauri::command]
pub async fn restore_tags(state: State<'_, Arc<AppState>>, path: String) -> Result<Metadata> {
    let path = require_selected(&state, &path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let backup = backup_path(&path);
        read(&backup)?;
        let mut copy = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        std::io::copy(&mut std::fs::File::open(backup)?, copy.as_file_mut())?;
        std::fs::set_permissions(copy.path(), std::fs::metadata(&path)?.permissions())?;
        copy.as_file().sync_all()?;
        copy.persist(&path).map_err(failure)?;
        read(&path)
    })
    .await
    .map_err(failure)?
}

#[tauri::command]
pub async fn export_cover(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<()> {
    let source = require_selected(&state, &path)?;
    let file = tagged_file(&source)?;
    let picture = file
        .primary_tag()
        .or_else(|| file.first_tag())
        .and_then(|tag| {
            tag.get_picture_type(PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
        })
        .ok_or_else(|| failure("no cover"))?;
    let extension = picture
        .mime_type()
        .and_then(|mime| mime.ext())
        .unwrap_or("jpg");
    if let Some(path) = app
        .dialog()
        .file()
        .set_file_name(format!("cover.{extension}"))
        .blocking_save_file()
        .and_then(|file| file.into_path().ok())
    {
        std::fs::write(path, picture.data())?;
    }
    Ok(())
}

static MUSICBRAINZ_LAST_REQUEST: Mutex<Option<std::time::Instant>> = Mutex::new(None);
#[tauri::command]
pub async fn search_metadata(query: String) -> Result<serde_json::Value> {
    if query.trim().is_empty() || query.len() > 300 {
        return Err(failure("invalid query"));
    }
    // Serialize calls so concurrent callers cannot exceed the public API limit.
    tauri::async_runtime::spawn_blocking(move || {
        let mut last = MUSICBRAINZ_LAST_REQUEST.lock().map_err(failure)?;
        if let Some(time) = *last {
            std::thread::sleep(Duration::from_millis(1100).saturating_sub(time.elapsed()));
        }
        *last = Some(std::time::Instant::now());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let response = reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("Halite/0.2 (https://github.com/taliyigit2-prog/Halite)")
                .build()?
                .get("https://musicbrainz.org/ws/2/recording/")
                .query(&[("query", query.as_str()), ("fmt", "json"), ("limit", "8")])
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            Ok(response)
        })
    })
    .await
    .map_err(failure)?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wav_edit_preserves_audio_and_creates_restorable_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Türkçe.wav");
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 24000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for i in 0..24000 {
            writer
                .write_sample((f64::sin(i as f64 * 0.1) * 10000.0) as i16)
                .unwrap();
        }
        writer.finalize().unwrap();
        let before = read(&path).unwrap();
        let fields = BTreeMap::from([
            ("title".into(), "Şarkı — İstanbul".into()),
            ("artist".into(), "Yiğit".into()),
        ]);
        let edit = Edit {
            path: path.to_string_lossy().into(),
            expected: before.stamp,
            fields: fields.clone(),
            cover_path: None,
            remove_cover: false,
        };
        let after = write(&edit).unwrap();
        assert_eq!(after.fields["title"], fields["title"]);
        assert_eq!(after.duration, before.duration);
        assert!(backup_path(&path).is_file());
        assert!(write(&edit)
            .unwrap_err()
            .to_string()
            .contains("HALITE_TAG_CHANGED"));
    }
    #[test]
    fn invalid_file_is_never_modified() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.mp3");
        std::fs::write(&path, b"not audio").unwrap();
        let edit = Edit {
            path: path.to_string_lossy().into(),
            expected: stamp(&path).unwrap(),
            fields: BTreeMap::new(),
            cover_path: None,
            remove_cover: false,
        };
        assert!(write(&edit).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"not audio");
    }

    #[test]
    #[ignore = "requires FFmpeg; manual release metadata format regression"]
    fn common_formats_round_trip_artwork_and_unicode() {
        let directory = tempfile::tempdir().unwrap();
        let artwork = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/128x128.png");
        for extension in ["mp3", "flac", "m4a", "ogg", "opus", "wav", "aiff"] {
            eprintln!("metadata format: {extension}");
            let path = directory.path().join(format!("fixture.{extension}"));
            let mut command = std::process::Command::new("ffmpeg");
            command.args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=44100:duration=2",
                "-ac",
                "2",
            ]);
            if extension == "ogg" {
                command.args(["-c:a", "vorbis", "-strict", "-2"]);
            }
            let status = command.arg(&path).status().unwrap();
            assert!(status.success(), "{extension}");
            let original = read(&path).unwrap();
            let fields = FIELDS
                .iter()
                .filter(|(name, _)| original.supported_fields.contains(&(*name).to_owned()))
                .map(|(name, _)| {
                    let value = match *name {
                        "track" | "disc" => "1",
                        "track_total" | "disc_total" => "2",
                        "date" => "2026",
                        _ => "Şarkı — İstanbul",
                    };
                    ((*name).to_owned(), value.to_owned())
                })
                .collect::<BTreeMap<_, _>>();
            let edit = Edit {
                path: path.to_string_lossy().into(),
                expected: original.stamp,
                fields: fields.clone(),
                cover_path: original
                    .supports_cover
                    .then(|| artwork.to_string_lossy().into()),
                remove_cover: false,
            };
            let result = write(&edit).unwrap_or_else(|error| panic!("{extension}: {error}"));
            for (key, expected) in fields {
                assert_eq!(result.fields[&key], expected, "{extension}: {key}");
            }
            if original.supports_cover {
                assert!(result.cover.is_some(), "{extension}");
                let clear = Edit {
                    path: edit.path.clone(),
                    expected: result.stamp,
                    fields: BTreeMap::new(),
                    cover_path: None,
                    remove_cover: true,
                };
                assert!(write(&clear).unwrap().cover.is_none(), "{extension}");
            }
            assert!((original.duration - result.duration).abs() < 0.1);
        }
    }
}
