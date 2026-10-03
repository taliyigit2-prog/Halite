<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="120" />

# Halite

**Free &amp; open-source AI music toolkit.**

Split any song into stems · Download audio from the web · See synchronized lyrics

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/made%20with-Tauri-ffc131.svg)](https://tauri.app)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)]()
[![ONNX Runtime](https://img.shields.io/badge/onnxruntime-CoreML%2FCPU-8a2be2.svg)]()

*English · [Türkçe](README.tr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Русский](README.ru.md) · [日本語](README.ja.md)*

</div>

---

## What is Halite?

Halite is a **free, open-source desktop application** that runs AI-powered music
tools **entirely on your device** — no cloud, no account, no subscription. It is
built with [Tauri v2](https://tauri.app) (Rust + web frontend) and uses
[ONNX Runtime](https://onnxruntime.ai) for on-device inference.

It offers three workspaces:

| 🎚️ Separate | ⬇️ Download | 🎤 Lyrics |
|---|---|---|
| Split any song into **vocals, drums, bass and other** using the HT-Demucs neural network. | Grab audio from YouTube, SoundCloud, Bandcamp and more via `yt-dlp`. | See **synchronized lyrics** for the song currently playing (Apple Music / Spotify). |

---

## ✨ Features

- **AI stem separation** — 4-stem (vocals / drums / bass / other) and 2-stem
  (vocals / instrumental) output, powered by HT-Demucs ONNX models.
- **Hardware acceleration** — CoreML on Apple Silicon, CPU everywhere else.
- **Stem preview** — click any stem to listen before exporting.
- **Batch processing** — queue multiple files at once.
- **Multiple export formats** — WAV, FLAC, MP3, M4A.
- **Audio trimming** — process only a section of a track.
- **BPM & key detection** — quick musical analysis.
- **Downloader** — `yt-dlp` powered audio downloads with progress.
- **Synchronized lyrics** — via the free [LRCLIB](https://lrclib.net) API.
- **7 languages** — English, Türkçe, Deutsch, Español, Français, Русский, 日本語.
- **Light / dark / system theme**.
- **100% local & private** — your audio never leaves your machine.

---

## 🖼️ Screenshots

> Screenshots coming soon. Contributions welcome!

---

## 📦 Installation

Download the latest release for your platform from the
[Releases](https://github.com/taliyigit2-prog/Halite/releases) page.

| Platform | Package |
|---|---|
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi` (or `.exe`) |
| Linux | `Halite_*.AppImage` (or `.deb`) |

### macOS note

Because the app is not notarized with an Apple Developer account, macOS
Gatekeeper may warn on first launch. To open it:

- Right-click the app and choose **Open**, or
- Run `xattr -d com.apple.quarantine "/Applications/Halite.app"`.

### First run

The AI models are **bundled inside the app** — no download and no manual setup
needed. Only the Download tab downloads a small helper (`yt-dlp`) the first time
you use it; that helper is verified and cached in Halite's application-data folder.
Formats that require conversion also use system FFmpeg or prepare it there once.

---

## 🧠 How separation works

Halite uses the open-source
[HT-Demucs](https://github.com/facebookresearch/demucs) model exported to ONNX:

- `htdemucs.onnx` — high quality, 4 stems (~316 MB)
- `htdemucs_fp16weights.onnx` — lighter, same stems (~166 MB)

Both models are **bundled in the app** and run locally with
[ONNX Runtime](https://onnxruntime.ai). They are provided by
[StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx)
(MIT).

---

## 🛠️ Building from source

### Prerequisites

- [Rust](https://rustup.rs/) 1.92+
- [Node.js](https://nodejs.org/) 18+
- [Tauri v2 system dependencies](https://tauri.app/start/prerequisites/)

### Commands

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm install
npm run tauri dev       # development
npm run tauri build     # release bundle
```

---

## 🗺️ Roadmap

- [ ] 6-stem model (guitar + piano)
- [ ] Menu-bar quick access
- [ ] Album artwork fetching for the lyrics view

---

## 🙏 Acknowledgements

Halite stands on the shoulders of these great open-source projects:

- [facebookresearch/demucs](https://github.com/facebookresearch/demucs) — HT-Demucs model (MIT)
- [StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx) — ONNX export (MIT)
- [pykeio/ort](https://github.com/pykeio/ort) — ONNX Runtime for Rust
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — media downloader
- [LRCLIB](https://lrclib.net) — open lyrics database
- [Tauri](https://tauri.app) — the app framework
- [AnaghSharma/Carol](https://github.com/AnaghSharma/Carol) — inspiration for the lyrics view

---

## 📄 License

Halite is released under the [MIT License](LICENSE). The AI models are MIT
licensed by their respective authors. See [Third-party notices](THIRD_PARTY_NOTICES.md)
for bundled and runtime-downloaded components.

> **Disclaimer:** Halite is provided for personal, educational and fair use.
> Please respect the copyright and terms of service of the content you process
> or download.
