<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="120" />

# Halite

**無料・オープンソースのAIミュージックツール。**

曲をステムに分離 · Webから音声をダウンロード · 同期歌詞を表示

[![License](https://img.shields.io/badge/ライセンス-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri%E8%A3%BD-ffc131.svg)](https://tauri.app)
[![Platform](https://img.shields.io/badge/プラットフォーム-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)]()
[![ONNX Runtime](https://img.shields.io/badge/onnxruntime-CoreML%2FCPU-8a2be2.svg)]()

*[English](README.md) · [Türkçe](README.tr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Русский](README.ru.md) · 日本語*

</div>

---

## Haliteとは？

Haliteは、AIを活用した音楽ツールを**デバイス上で完全に**実行する、
**無料・オープンソース**のデスクトップアプリです。クラウド不要、アカウント不要、
サブスクリプション不要。[Tauri v2](https://tauri.app)（Rust + Web UI）で構築され、
オンデバイス推論に[ONNX Runtime](https://onnxruntime.ai)を使用しています。

3つのワークスペースを提供します：

| 🎚️ 分離 | ⬇️ ダウンロード | 🎤 歌詞 |
|---|---|---|
| HT-Demucsニューラルネットワークで、どんな曲も**ボーカル・ドラム・ベース・その他**に分離。 | `yt-dlp`でYouTube、SoundCloud、Bandcampなどから音声をダウンロード。 | 再生中の曲（Apple Music / Spotify）の**同期歌詞**を表示。 |

---

## ✨ 機能

- **AIステム分離** — HT-Demucs ONNXモデルによる4ステム（ボーカル / ドラム /
  ベース / その他）と2ステム（ボーカル / インストゥルメンタル）出力。
- **ハードウェアアクセラレーション** — Apple SiliconではCoreML、それ以外はCPU。
- **ステムプレビュー** — 書き出し前に各ステムを試聴。
- **バッチ処理** — 複数ファイルを一度にキューへ。
- **複数の書き出し形式** — WAV、FLAC、MP3、M4A。
- **音声トリミング** — 一部だけを処理。
- **BPM・キー検出** — すばやい音楽解析。
- **ダウンローダー** — `yt-dlp`による進捗付き音声ダウンロード。
- **同期歌詞** — 無料の[LRCLIB](https://lrclib.net) API経由。
- **7言語対応** — English, Türkçe, Deutsch, Español, Français, Русский, 日本語。
- **ライト / ダーク / システムテーマ**。
- **100%ローカル・プライベート** — 音声がデバイスから出ることはありません。

---

## 🖼️ スクリーンショット

> スクリーンショットは近日公開予定。コントリビューション歓迎！

---

## 📦 インストール

プラットフォームに合った最新版を
[Releases](https://github.com/taliyigit2-prog/Halite/releases)からダウンロードしてください。

| プラットフォーム | パッケージ |
|---|---|
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi`（または `.exe`） |
| Linux | `Halite_*.AppImage`（または `.deb`） |

### macOSについて

Apple Developerアカウントで公証されていないため、初回起動時にGatekeeperの
警告が出ることがあります。開くには：

- アプリを右クリックして**開く**を選択するか、
- `xattr -d com.apple.quarantine "/Applications/Halite.app"` を実行してください。

### 初回起動

AIモデルは**アプリに同梱**されているため、ダウンロードや手動設定は不要です。
ダウンロード画面のみ、初回使用時に検証済みの小さなヘルパー（`yt-dlp`）を
Haliteのアプリデータフォルダーへ取得します。変換が必要な形式ではシステムの
FFmpegを使用するか、同じ場所に一度だけ準備します。

---

## 🧠 分離の仕組み

Haliteはオープンソースの[HT-Demucs](https://github.com/facebookresearch/demucs)
モデルをONNXに書き出したものを使用します：

- `htdemucs.onnx` — 高品質・4ステム（約316 MB）
- `htdemucs_fp16weights.onnx` — 小容量・同じステム（約166 MB）

モデルは
[StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx)
（MIT）が提供し、アプリに同梱され、ONNX Runtimeで完全にローカル実行されます。

---

## 🛠️ ソースからビルド

### 前提条件

- [Rust](https://rustup.rs/) 1.92以上
- [Node.js](https://nodejs.org/) 18以上
- [Tauri v2のシステム依存関係](https://tauri.app/start/prerequisites/)

### コマンド

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm install
npm run tauri dev       # 開発
npm run tauri build     # リリースパッケージ
```

---

## 🗺️ ロードマップ

- [ ] 6ステムモデル（ギター＋ピアノ）
- [ ] メニューバーからのクイックアクセス
- [ ] 歌詞ビューへのアルバムアート取得

---

## 🙏 謝辞

Haliteは、以下の素晴らしいオープンソースプロジェクトの上に成り立っています：

- [facebookresearch/demucs](https://github.com/facebookresearch/demucs) — HT-Demucsモデル（MIT）
- [StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx) — ONNXエクスポート（MIT）
- [pykeio/ort](https://github.com/pykeio/ort) — Rust用ONNX Runtime
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — メディアダウンローダー
- [LRCLIB](https://lrclib.net) — オープンな歌詞データベース
- [Tauri](https://tauri.app) — アプリフレームワーク
- [AnaghSharma/Carol](https://github.com/AnaghSharma/Carol) — 歌詞ビューの着想

---

## 📄 ライセンス

Haliteは[MITライセンス](LICENSE)で公開されています。AIモデルは、それぞれの
作者によりMITでライセンスされています。

> **免責事項：** Haliteは個人的・教育的・公正な利用のために提供されます。
> 処理・ダウンロードするコンテンツの著作権と利用規約を尊重してください。
