<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="120" />

# Halite

**Herramienta musical de IA gratuita y de código abierto.**

Separa canciones en pistas · Descarga audio de la web · Mira letras sincronizadas

[![License](https://img.shields.io/badge/licencia-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/hecho%20con-Tauri-ffc131.svg)](https://tauri.app)
[![Platform](https://img.shields.io/badge/plataforma-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)]()
[![ONNX Runtime](https://img.shields.io/badge/onnxruntime-CoreML%2FCPU-8a2be2.svg)]()

*[English](README.md) · [Türkçe](README.tr.md) · [Deutsch](README.de.md) · Español · [Français](README.fr.md) · [Русский](README.ru.md) · [日本語](README.ja.md)*

</div>

---

## ¿Qué es Halite?

Halite es una aplicación de escritorio **gratuita y de código abierto** que ejecuta
herramientas musicales con IA **completamente en tu dispositivo**: sin nube, sin
cuenta, sin suscripción. Está construida con [Tauri v2](https://tauri.app)
(Rust + interfaz web) y usa [ONNX Runtime](https://onnxruntime.ai) para la
inferencia local.

Ofrece tres espacios de trabajo:

| 🎚️ Separar | ⬇️ Descargar | 🎤 Letras |
|---|---|---|
| Divide cualquier canción en **voz, batería, bajo y otros** con la red HT-Demucs. | Descarga audio de YouTube, SoundCloud, Bandcamp y más mediante `yt-dlp`. | Mira **letras sincronizadas** de la canción en reproducción (Apple Music / Spotify). |

---

## ✨ Características

- **Separación de pistas con IA** — salida de 4 pistas (voz / batería / bajo / otros)
  y 2 pistas (voz / instrumental) con modelos HT-Demucs ONNX.
- **Aceleración por hardware** — CoreML en Apple Silicon, CPU en el resto.
- **Vista previa de pistas** — escucha cada pista antes de exportar.
- **Procesamiento por lotes** — encola varios archivos a la vez.
- **Varios formatos de exportación** — WAV, FLAC, MP3, M4A.
- **Recorte de audio** — procesa solo una sección.
- **Detección de BPM y tonalidad** — análisis musical rápido.
- **Descargador** — descargas de audio con `yt-dlp` y barra de progreso.
- **Letras sincronizadas** — mediante la API gratuita [LRCLIB](https://lrclib.net).
- **7 idiomas** — English, Türkçe, Deutsch, Español, Français, Русский, 日本語.
- **Tema claro / oscuro / sistema**.
- **100 % local y privado** — tu audio nunca sale de tu equipo.

---

## 🖼️ Capturas de pantalla

> ¡Capturas en breve! ¡Contribuciones bienvenidas!

---

## 📦 Instalación

Descarga la última versión para tu plataforma desde la página de
[Releases](https://github.com/taliyigit2-prog/Halite/releases).

| Plataforma | Paquete |
|---|---|
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi` (o `.exe`) |
| Linux | `Halite_*.AppImage` (o `.deb`) |

### Nota sobre macOS

Como la app no está notarizada con una cuenta de Apple Developer, Gatekeeper
puede avisar en el primer inicio. Para abrirla:

- Haz clic derecho sobre la app y elige **Abrir**, o
- Ejecuta `xattr -d com.apple.quarantine "/Applications/Halite.app"`.

### Primer inicio

En el primer arranque, Halite descarga un pequeño asistente (`yt-dlp`) y, al
iniciar una separación, el modelo de IA elegido (~170–330 MB). Ambos se guardan
en caché local y se descargan una sola vez.

---

## 🧠 Cómo funciona la separación

Halite usa el modelo de código abierto
[HT-Demucs](https://github.com/facebookresearch/demucs) exportado a ONNX:

- `htdemucs.onnx` — alta calidad, 4 pistas (~331 MB)
- `htdemucs_fp16weights.onnx` — descarga más ligera (~174 MB)

Los modelos los proporciona
[StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx)
(MIT) y se descargan de Hugging Face en el primer uso.

---

## 🛠️ Compilar desde el código

### Requisitos

- [Rust](https://rustup.rs/) 1.92+
- [Node.js](https://nodejs.org/) 18+
- [Dependencias de sistema de Tauri v2](https://tauri.app/start/prerequisites/)

### Comandos

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm install
npm run tauri dev       # desarrollo
npm run tauri build     # paquete de lanzamiento
```

---

## 🗺️ Hoja de ruta

- [ ] Modelo de 6 pistas (guitarra + piano)
- [ ] Acceso rápido desde la barra de menú
- [ ] Obtención de carátulas para la vista de letras

---

## 🙏 Agradecimientos

Halite se apoya en estos fantásticos proyectos de código abierto:

- [facebookresearch/demucs](https://github.com/facebookresearch/demucs) — modelo HT-Demucs (MIT)
- [StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx) — exportación ONNX (MIT)
- [pykeio/ort](https://github.com/pykeio/ort) — ONNX Runtime para Rust
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — descargador multimedia
- [LRCLIB](https://lrclib.net) — base de datos abierta de letras
- [Tauri](https://tauri.app) — el framework de la app
- [AnaghSharma/Carol](https://github.com/AnaghSharma/Carol) — inspiración para la vista de letras

---

## 📄 Licencia

Halite se publica bajo la [Licencia MIT](LICENSE). Los modelos de IA tienen
licencia MIT de sus respectivos autores.

> **Aviso:** Halite se ofrece para uso personal, educativo y legítimo. Respeta
> los derechos de autor y los términos de servicio del contenido que proceses o
> descargues.
