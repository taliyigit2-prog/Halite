<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="120" />

# Halite

**Бесплатный музыкальный ИИ-инструмент с открытым исходным кодом.**

Разделяйте песни на дорожки · Скачивайте аудио · Смотрите синхронизированные тексты

[![License](https://img.shields.io/badge/лицензия-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/сделано%20на-Tauri-ffc131.svg)](https://tauri.app)
[![Platform](https://img.shields.io/badge/платформа-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)]()
[![ONNX Runtime](https://img.shields.io/badge/onnxruntime-CoreML%2FCPU-8a2be2.svg)]()

*[English](README.md) · [Türkçe](README.tr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · Русский · [日本語](README.ja.md)*

</div>

---

## Что такое Halite?

Halite — это **бесплатное приложение с открытым исходным кодом**, которое
выполняет ИИ-инструменты для музыки **полностью на вашем устройстве** — без
облака, без аккаунта, без подписки. Оно построено на [Tauri v2](https://tauri.app)
(Rust + веб-интерфейс) и использует [ONNX Runtime](https://onnxruntime.ai)
для локального вывода.

Три рабочих области:

| 🎚️ Разделить | ⬇️ Скачать | 🎤 Текст |
|---|---|---|
| Разделите любую песню на **вокал, ударные, бас и прочее** с помощью нейросети HT-Demucs. | Скачивайте аудио с YouTube, SoundCloud, Bandcamp и других сайтов через `yt-dlp`. | Смотрите **синхронизированный текст** играющей песни (Apple Music / Spotify). |

---

## ✨ Возможности

- **Разделение дорожек с помощью ИИ** — выход из 4 дорожек (вокал / ударные /
  бас / прочее) и 2 дорожек (вокал / инструментал) на моделях HT-Demucs ONNX.
- **Аппаратное ускорение** — CoreML на Apple Silicon, CPU везде.
- **Предпросмотр дорожек** — прослушайте каждую дорожку перед экспортом.
- **Пакетная обработка** — ставьте несколько файлов в очередь.
- **Несколько форматов экспорта** — WAV, FLAC, MP3, M4A.
- **Обрезка аудио** — обрабатывайте только нужный фрагмент.
- **Определение BPM и тональности** — быстрый музыкальный анализ.
- **Загрузчик** — скачивание аудио через `yt-dlp` с прогрессом.
- **Синхронизированные тексты** — через бесплатный API [LRCLIB](https://lrclib.net).
- **7 языков** — English, Türkçe, Deutsch, Español, Français, Русский, 日本語.
- **Светлая / тёмная / системная тема**.
- **100 % локально и конфиденциально** — ваш звук никогда не покидает устройство.

---

## 🖼️ Скриншоты

> Скриншоты скоро появятся. Приветствуются вклады!

---

## 📦 Установка

Скачайте последнюю версию для вашей платформы со страницы
[Releases](https://github.com/taliyigit2-prog/Halite/releases).

| Платформа | Пакет |
|---|---|
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi` (или `.exe`) |
| Linux | `Halite_*.AppImage` (или `.deb`) |

### Примечание для macOS

Приложение не нотаризовано через аккаунт Apple Developer, поэтому Gatekeeper
может предупредить при первом запуске. Чтобы открыть:

- Щёлкните правой кнопкой мыши по приложению и выберите **Открыть**, или
- Выполните `xattr -d com.apple.quarantine "/Applications/Halite.app"`.

### Первый запуск

ИИ-модели **включены в приложение**; скачивание и ручная настройка не нужны.
Только раздел загрузки при первом использовании получает небольшой проверенный
помощник (`yt-dlp`) и сохраняет его в папке данных Halite. Для конвертации
используется системный FFmpeg либо он один раз подготавливается в той же папке.

---

## 🧠 Как работает разделение

Halite использует модель с открытым исходным кодом
[HT-Demucs](https://github.com/facebookresearch/demucs), экспортированную в ONNX:

- `htdemucs.onnx` — высокое качество, 4 дорожки (~316 МБ)
- `htdemucs_fp16weights.onnx` — меньше, те же дорожки (~166 МБ)

Модели предоставлены
[StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx)
(MIT), включены в приложение и полностью локально работают через ONNX Runtime.

---

## 🛠️ Сборка из исходников

### Требования

- [Rust](https://rustup.rs/) 1.92+
- [Node.js](https://nodejs.org/) 18+
- [Системные зависимости Tauri v2](https://tauri.app/start/prerequisites/)

### Команды

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm install
npm run tauri dev       # разработка
npm run tauri build     # релизный пакет
```

---

## 🗺️ Дорожная карта

- [ ] Модель на 6 дорожек (гитара + пианино)
- [ ] Быстрый доступ из строки меню
- [ ] Получение обложек альбомов для просмотра текста

---

## 🙏 Благодарности

Halite опирается на эти замечательные open-source проекты:

- [facebookresearch/demucs](https://github.com/facebookresearch/demucs) — модель HT-Demucs (MIT)
- [StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx) — экспорт ONNX (MIT)
- [pykeio/ort](https://github.com/pykeio/ort) — ONNX Runtime для Rust
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — загрузчик медиа
- [LRCLIB](https://lrclib.net) — открытая база текстов
- [Tauri](https://tauri.app) — фреймворк приложения
- [AnaghSharma/Carol](https://github.com/AnaghSharma/Carol) — вдохновение для просмотра текста

---

## 📄 Лицензия

Halite выпущен под [лицензией MIT](LICENSE). ИИ-модели лицензированы по MIT их
соответствующими авторами.

> **Дисклеймер:** Halite предназначен для личного, образовательного и
> добросовестного использования. Уважайте авторские права и условия
> использования обрабатываемого или загружаемого контента.
