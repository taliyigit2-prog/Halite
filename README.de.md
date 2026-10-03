<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="120" />

# Halite

**Kostenloses Open-Source-KI-Musikwerkzeug.**

Songs in Stems zerlegen · Audio aus dem Web laden · Synchronisierte Songtexte ansehen

[![License](https://img.shields.io/badge/Lizenz-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/erstellt%20mit-Tauri-ffc131.svg)](https://tauri.app)
[![Platform](https://img.shields.io/badge/Plattform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)]()
[![ONNX Runtime](https://img.shields.io/badge/onnxruntime-CoreML%2FCPU-8a2be2.svg)]()

*[English](README.md) · [Türkçe](README.tr.md) · Deutsch · [Español](README.es.md) · [Français](README.fr.md) · [Русский](README.ru.md) · [日本語](README.ja.md)*

</div>

---

## Was ist Halite?

Halite ist eine **kostenlose Open-Source-Desktop-App**, die KI-gestützte
Musikwerkzeuge **vollständig auf deinem Gerät** ausführt — keine Cloud, kein
Konto, kein Abo. Sie basiert auf [Tauri v2](https://tauri.app) (Rust + Web-UI)
und nutzt [ONNX Runtime](https://onnxruntime.ai) für die lokale Inferenz.

Drei Arbeitsbereiche:

| 🎚️ Trennen | ⬇️ Download | 🎤 Songtext |
|---|---|---|
| Zerlege jeden Song in **Gesang, Drums, Bass und Sonstiges** mit dem HT-Demucs-Netzwerk. | Lade Audio von YouTube, SoundCloud, Bandcamp und mehr über `yt-dlp`. | Sieh **synchronisierte Songtexte** für den gerade laufenden Titel (Apple Music / Spotify). |

---

## ✨ Funktionen

- **KI-Stem-Trennung** — 4 Stems (Gesang / Drums / Bass / Sonstiges) und 2 Stems
  (Gesang / Instrumental) mit HT-Demucs-ONNX-Modellen.
- **Hardware-Beschleunigung** — CoreML auf Apple Silicon, sonst CPU.
- **Stem-Vorschau** — jeden Stem vor dem Export anhören.
- **Stapelverarbeitung** — mehrere Dateien gleichzeitig in die Warteschlange.
- **Mehrere Exportformate** — WAV, FLAC, MP3, M4A.
- **Audio zuschneiden** — nur einen Abschnitt verarbeiten.
- **BPM- & Tonart-Erkennung** — schnelle Musikanalyse.
- **Downloader** — `yt-dlp`-gestützte Audio-Downloads mit Fortschritt.
- **Synchronisierte Songtexte** — über die kostenlose [LRCLIB](https://lrclib.net)-API.
- **7 Sprachen** — English, Türkçe, Deutsch, Español, Français, Русский, 日本語.
- **Helles / dunkles / System-Design**.
- **100 % lokal & privat** — dein Audio verlässt nie dein Gerät.

---

## 🖼️ Screenshots

> Screenshots folgen in Kürze. Beiträge willkommen!

---

## 📦 Installation

Lade die neueste Version für deine Plattform von der
[Releases-Seite](https://github.com/taliyigit2-prog/Halite/releases) herunter.

| Plattform | Paket |
|---|---|
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi` (oder `.exe`) |
| Linux | `Halite_*.AppImage` (oder `.deb`) |

### Hinweis für macOS

Da die App nicht mit einem Apple-Developer-Konto notarisiert ist, kann Gatekeeper
beim ersten Start warnen. Zum Öffnen:

- Rechtsklick auf die App und **Öffnen** wählen, oder
- `xattr -d com.apple.quarantine "/Applications/Halite.app"` ausführen.

### Erster Start

Die KI-Modelle sind **in der App enthalten**; ein Download oder eine manuelle
Einrichtung ist nicht nötig. Nur der Download-Bereich lädt bei der ersten Nutzung
einen kleinen, geprüften Helfer (`yt-dlp`) in Halites App-Datenordner. Formate mit
Konvertierung nutzen System-FFmpeg oder richten es dort einmalig ein.

---

## 🧠 So funktioniert die Trennung

Halite nutzt das Open-Source-Modell
[HT-Demucs](https://github.com/facebookresearch/demucs) als ONNX-Export:

- `htdemucs.onnx` — hohe Qualität, 4 Stems (~316 MB)
- `htdemucs_fp16weights.onnx` — kleiner, dieselben Stems (~166 MB)

Die Modelle stammen von
[StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx)
(MIT), sind in der App enthalten und laufen mit ONNX Runtime vollständig lokal.

---

## 🛠️ Aus dem Quellcode bauen

### Voraussetzungen

- [Rust](https://rustup.rs/) 1.92+
- [Node.js](https://nodejs.org/) 18+
- [Tauri-v2-Systemabhängigkeiten](https://tauri.app/start/prerequisites/)

### Befehle

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm install
npm run tauri dev       # Entwicklung
npm run tauri build     # Release-Bundle
```

---

## 🗺️ Roadmap

- [ ] 6-Stem-Modell (Gitarre + Klavier)
- [ ] Schnellzugriff über die Menüleiste
- [ ] Albumcover für die Songtext-Ansicht

---

## 🙏 Danksagung

Halite steht auf den Schultern dieser großartigen Open-Source-Projekte:

- [facebookresearch/demucs](https://github.com/facebookresearch/demucs) — HT-Demucs-Modell (MIT)
- [StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx) — ONNX-Export (MIT)
- [pykeio/ort](https://github.com/pykeio/ort) — ONNX Runtime für Rust
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — Medien-Downloader
- [LRCLIB](https://lrclib.net) — offene Songtext-Datenbank
- [Tauri](https://tauri.app) — das App-Framework
- [AnaghSharma/Carol](https://github.com/AnaghSharma/Carol) — Inspiration für die Songtext-Ansicht

---

## 📄 Lizenz

Halite wird unter der [MIT-Lizenz](LICENSE) veröffentlicht. Die KI-Modelle sind
von ihren jeweiligen Autoren unter MIT lizenziert.

> **Haftungsausschluss:** Halite ist für private, lehrreiche und faire Nutzung
> gedacht. Bitte respektiere das Urheberrecht und die Nutzungsbedingungen der
> Inhalte, die du verarbeitest oder herunterlädst.
