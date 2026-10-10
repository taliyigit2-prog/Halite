<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="112" />

# Halite

### Your music. Your device. Your control.

**A free, open-source desktop studio for separating, organising, downloading and understanding music — with local AI.**

[![MIT License](https://img.shields.io/badge/license-MIT-6d5dfc?style=flat-square)](LICENSE)
[![Tauri 2](https://img.shields.io/badge/desktop-Tauri%202-24c8db?style=flat-square)](https://tauri.app)
[![Local first](https://img.shields.io/badge/privacy-local--first-38c98b?style=flat-square)](#privacy-by-design)
[![CI](https://github.com/taliyigit2-prog/Halite/actions/workflows/ci.yml/badge.svg)](https://github.com/taliyigit2-prog/Halite/actions/workflows/ci.yml)

[Download a release](https://github.com/taliyigit2-prog/Halite/releases) · [Report an issue](https://github.com/taliyigit2-prog/Halite/issues) · [Contribute](#development)

English · [Türkçe](README.tr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Русский](README.ru.md) · [日本語](README.ja.md)

</div>

---

## A calm, capable music workspace

Halite is a native desktop application built with Rust and Tauri. It keeps the core creative work on your computer: no account, no subscription, and no upload of your audio for separation, tagging or speech generation.

| Make | Organise | Explore |
| --- | --- | --- |
| **Separate** a song into vocals, drums, bass and other stems with HT-Demucs. | **Edit tags** with a review step, automatic local backup and cover-art support. | **Download** audio with clear progress, or find synchronised lyrics for the track playing now. |
| **Create speech** locally with the managed Chatterbox Multilingual V3 runtime. | **Preview and export** outputs in formats that suit your workflow. | **Analyse** BPM and key, batch process files, and work in seven interface languages. |

## What you can do

### 🎚️ Separate stems

- 4-stem (vocals, drums, bass, other) and 2-stem (vocals, instrumental) separation.
- Runs HT-Demucs ONNX models locally, with CoreML on supported Apple Silicon hardware and a safe CPU fallback.
- Batch queue, trim range, BPM/key analysis, previewable stems and WAV, FLAC, MP3 or M4A export.

### 🏷️ Keep your library tidy

- Read and edit title, artist, album, album artist, genre, date, disc/track numbers, composer, copyright, comments and embedded lyrics.
- Add, remove or export artwork; optionally search MusicBrainz without uploading your audio file.
- Every write is staged, validated, reviewed in the UI and backed up beside the original as `*.halite-backup`. A restore action is available before you discard that backup.
- Supports common WAV, FLAC, MP3, M4A/MP4, OGG/Opus and AIFF workflows when their tag format supports the field.

### 🎙️ Produce local speech

- Optional, managed **Chatterbox Multilingual V3** voice runtime, installed on first use after a clear size notice — no manual Python setup.
- Text-to-speech and consent-gated reference-voice mode. Reference audio stays on the device; only a 3–30 second local recording is accepted.
- Generated audio is marked with PerTh watermarking and accompanied by provenance data.
- Downloads are pinned to known revisions and SHA-256 verified; after setup, the worker blocks network access while generating.

### ⬇️ Download and understand

- Download audio from sites supported by `yt-dlp`, with verified helper updates and visible status.
- Show synchronised lyrics from LRCLIB for the current Apple Music or Spotify track when available.
- English, Türkçe, Deutsch, Español, Français, Русский and 日本語 — with light, dark and system themes.

## Privacy by design

Halite is local-first, not cloud-first.

- Audio separation, metadata editing and speech generation happen on your machine.
- The voice runtime only downloads its verified components when you choose **Set up automatically**. It needs roughly 11 GB free during setup and stores its model separately from the app bundle.
- Optional online features are narrow and visible: downloading media uses its source URL, MusicBrainz is queried only when you search, and lyrics are requested from LRCLIB.
- Halite never asks you to upload your music library or reference voice to use the local tools.

Please process only material you have the right to use, and respect the terms of the sources you download from.

## Install

Get the current package from [GitHub Releases](https://github.com/taliyigit2-prog/Halite/releases).

| Platform | Package |
| --- | --- |
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi` or `.exe` |
| Linux | `Halite_*.AppImage` or `.deb` |

### macOS note

Release builds should be signed and notarised before public distribution. Until a notarised release is available, Gatekeeper may require you to right-click the app and choose **Open** on first launch.

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 22 or newer
- [Rust](https://rustup.rs/) stable
- [Tauri system prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm ci
npm run tauri dev
```

Useful checks:

```bash
npm run check-locales
npm run check-js
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

`npm run tauri build` creates the platform bundle. The Chatterbox model is deliberately **not** embedded in that bundle: it is a large, optional, hash-verified first-use download.

## Security and supply chain

- GitHub Actions use least-privilege workflow permissions and commit-pinned actions.
- CI checks JavaScript, locale completeness, Rust format/tests/lints, dependency advisories and secret exposure.
- Dependabot monitors npm, Cargo and workflow dependencies weekly.
- Runtime downloads use pinned manifests, expected sizes and SHA-256 verification. The source archive extraction rejects path traversal and symlinks.

Please report security concerns privately to the repository maintainers rather than opening a public issue.

## Credits and licenses

Halite is MIT licensed. Its model and runtime ecosystem remains the work of many open-source projects, including [Demucs](https://github.com/facebookresearch/demucs), [StemSplitio’s ONNX export](https://huggingface.co/StemSplitio/htdemucs-onnx), [Chatterbox](https://github.com/resemble-ai/chatterbox), [Lofty](https://github.com/Serial-ATA/lofty-rs), [yt-dlp](https://github.com/yt-dlp/yt-dlp), [LRCLIB](https://lrclib.net) and [Tauri](https://tauri.app).

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for component-specific notices and licences.
