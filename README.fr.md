<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="120" />

# Halite

**Outil musical IA gratuit et open source.**

Séparez les chansons en pistes · Téléchargez de l'audio · Affichez des paroles synchronisées

[![License](https://img.shields.io/badge/licence-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/fait%20avec-Tauri-ffc131.svg)](https://tauri.app)
[![Platform](https://img.shields.io/badge/plateforme-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)]()
[![ONNX Runtime](https://img.shields.io/badge/onnxruntime-CoreML%2FCPU-8a2be2.svg)]()

*[English](README.md) · [Türkçe](README.tr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · Français · [Русский](README.ru.md) · [日本語](README.ja.md)*

</div>

---

## Qu'est-ce que Halite ?

Halite est une application de bureau **gratuite et open source** qui exécute des
outils musicaux IA **entièrement sur votre appareil** : aucun cloud, aucun compte,
aucun abonnement. Elle est construite avec [Tauri v2](https://tauri.app)
(Rust + interface web) et utilise [ONNX Runtime](https://onnxruntime.ai) pour
l'inférence locale.

Trois espaces de travail :

| 🎚️ Séparer | ⬇️ Télécharger | 🎤 Paroles |
|---|---|---|
| Séparez n'importe quelle chanson en **voix, batterie, basse et autres** grâce au réseau HT-Demucs. | Téléchargez de l'audio depuis YouTube, SoundCloud, Bandcamp et plus via `yt-dlp`. | Affichez des **paroles synchronisées** pour la chanson en cours (Apple Music / Spotify). |

---

## ✨ Fonctionnalités

- **Séparation des pistes par IA** — sortie 4 pistes (voix / batterie / basse /
  autres) et 2 pistes (voix / instrumental) avec les modèles HT-Demucs ONNX.
- **Accélération matérielle** — CoreML sur Apple Silicon, CPU partout ailleurs.
- **Aperçu des pistes** — écoutez chaque piste avant l'export.
- **Traitement par lots** — mettez plusieurs fichiers en file d'attente.
- **Plusieurs formats d'export** — WAV, FLAC, MP3, M4A.
- **Rognage audio** — traitez uniquement une section.
- **Détection du BPM et de la tonalité** — analyse musicale rapide.
- **Téléchargeur** — téléchargements audio via `yt-dlp` avec progression.
- **Paroles synchronisées** — via l'API gratuite [LRCLIB](https://lrclib.net).
- **7 langues** — English, Türkçe, Deutsch, Español, Français, Русский, 日本語.
- **Thème clair / sombre / système**.
- **100 % local et privé** — votre audio ne quitte jamais votre machine.

---

## 🖼️ Captures d'écran

> Captures à venir prochainement. Contributions bienvenues !

---

## 📦 Installation

Téléchargez la dernière version pour votre plateforme depuis la page
[Releases](https://github.com/taliyigit2-prog/Halite/releases).

| Plateforme | Paquet |
|---|---|
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi` (ou `.exe`) |
| Linux | `Halite_*.AppImage` (ou `.deb`) |

### Remarque macOS

L'application n'étant pas notariée avec un compte Apple Developer, Gatekeeper
peut afficher un avertissement au premier lancement. Pour l'ouvrir :

- Faites un clic droit sur l'application et choisissez **Ouvrir**, ou
- Exécutez `xattr -d com.apple.quarantine "/Applications/Halite.app"`.

### Premier lancement

Les modèles IA sont **inclus dans l'application** ; aucun téléchargement ni réglage
manuel n'est nécessaire. Seul l'onglet Télécharger récupère à sa première utilisation
un petit utilitaire vérifié (`yt-dlp`) dans les données de Halite. Les formats à
convertir utilisent FFmpeg du système ou l'y préparent une fois.

---

## 🧠 Comment fonctionne la séparation

Halite utilise le modèle open source
[HT-Demucs](https://github.com/facebookresearch/demucs) exporté en ONNX :

- `htdemucs.onnx` — haute qualité, 4 pistes (~316 Mo)
- `htdemucs_fp16weights.onnx` — plus petit, mêmes pistes (~166 Mo)

Les modèles sont fournis par
[StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx)
(MIT), inclus dans l'application et exécutés localement avec ONNX Runtime.

---

## 🛠️ Compiler depuis les sources

### Prérequis

- [Rust](https://rustup.rs/) 1.92+
- [Node.js](https://nodejs.org/) 18+
- [Dépendances système de Tauri v2](https://tauri.app/start/prerequisites/)

### Commandes

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm install
npm run tauri dev       # développement
npm run tauri build     # paquet de release
```

---

## 🗺️ Feuille de route

- [ ] Modèle 6 pistes (guitare + piano)
- [ ] Accès rapide depuis la barre de menus
- [ ] Récupération des pochettes d'album pour la vue des paroles

---

## 🙏 Remerciements

Halite repose sur ces formidables projets open source :

- [facebookresearch/demucs](https://github.com/facebookresearch/demucs) — modèle HT-Demucs (MIT)
- [StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx) — export ONNX (MIT)
- [pykeio/ort](https://github.com/pykeio/ort) — ONNX Runtime pour Rust
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — téléchargeur multimédia
- [LRCLIB](https://lrclib.net) — base de paroles ouverte
- [Tauri](https://tauri.app) — le framework de l'application
- [AnaghSharma/Carol](https://github.com/AnaghSharma/Carol) — inspiration pour la vue des paroles

---

## 📄 Licence

Halite est publié sous la [Licence MIT](LICENSE). Les modèles IA sont sous licence
MIT de leurs auteurs respectifs.

> **Avertissement :** Halite est fourni pour un usage personnel, éducatif et
> légitime. Respectez les droits d'auteur et les conditions d'utilisation du
> contenu que vous traitez ou téléchargez.
