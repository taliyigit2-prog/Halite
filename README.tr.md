<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="120" />

# Halite

**Ücretsiz ve açık kaynaklı yapay zekâ müzik aracı.**

Her şarkıyı katmanlarına ayır · Web'den ses indir · Senkronize şarkı sözlerini gör

[![License](https://img.shields.io/badge/lisans-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri%20ile%20yap%C4%B1ld%C4%B1-ffc131.svg)](https://tauri.app)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)]()
[![ONNX Runtime](https://img.shields.io/badge/onnxruntime-CoreML%2FCPU-8a2be2.svg)]()

*[English](README.md) · Türkçe · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Русский](README.ru.md) · [日本語](README.ja.md)*

</div>

---

## Halite nedir?

Halite, yapay zekâ destekli müzik araçlarını **tamamen cihazında** çalıştıran
**ücretsiz ve açık kaynaklı** bir masaüstü uygulamasıdır — bulut yok, hesap yok,
abonelik yok. [Tauri v2](https://tauri.app) (Rust + web arayüzü) ile geliştirilmiştir
ve cihaz içi çıkarım için [ONNX Runtime](https://onnxruntime.ai) kullanır.

Üç çalışma alanı sunar:

| 🎚️ Ayrıştır | ⬇️ İndir | 🎤 Şarkı Sözü |
|---|---|---|
| HT-Demucs sinir ağı ile her şarkıyı **vokal, davul, bas ve diğer** katmanlarına ayır. | `yt-dlp` ile YouTube, SoundCloud, Bandcamp ve daha fazlasından ses indir. | Şu an çalan şarkının (Apple Music / Spotify) **senkronize sözlerini** gör. |

---

## ✨ Özellikler

- **Yapay zekâ ile stem ayrıştırma** — HT-Demucs ONNX modelleriyle 4 katman
  (vokal / davul / bas / diğer) ve 2 katman (vokal / enstrümantal) çıktı.
- **Donanım hızlandırma** — Apple Silicon'da CoreML, diğer her yerde CPU.
- **Stem önizleme** — dışa aktarmadan önce her katmanı dinle.
- **Toplu işleme** — birden fazla dosyayı aynı anda kuyruğa al.
- **Çoklu dışa aktarma formatı** — WAV, FLAC, MP3, M4A.
- **Ses kırpma** — şarkının yalnızca bir bölümünü işle.
- **BPM ve ton tespiti** — hızlı müzik analizi.
- **İndirici** — `yt-dlp` destekli, ilerleme göstergeli ses indirme.
- **Senkronize şarkı sözü** — ücretsiz [LRCLIB](https://lrclib.net) API'si ile.
- **7 dil** — English, Türkçe, Deutsch, Español, Français, Русский, 日本語.
- **Açık / koyu / sistem teması**.
- **%100 yerel ve gizli** — sesin asla cihazından çıkmaz.

---

## 🖼️ Ekran görüntüleri

> Ekran görüntüleri yakında eklenecek. Katkılara açığız!

---

## 📦 Kurulum

Platformunuza uygun son sürümü
[Releases](https://github.com/taliyigit2-prog/Halite/releases) sayfasından indirin.

| Platform | Paket |
|---|---|
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi` (veya `.exe`) |
| Linux | `Halite_*.AppImage` (veya `.deb`) |

### macOS notu

Uygulama Apple Developer hesabıyla noterize edilmediği için macOS Gatekeeper
ilk açılışta uyarı verebilir. Açmak için:

- Uygulamaya sağ tıklayıp **Aç**'ı seçin veya
- `xattr -d com.apple.quarantine "/Applications/Halite.app"` komutunu çalıştırın.

### İlk çalıştırma

İlk açılışta Halite küçük bir yardımcı (`yt-dlp`) ve bir ayrıştırma başlattığınızda
seçtiğiniz yapay zekâ modelini (~170–330 MB) indirir. İkisi de yerel olarak
önbelleğe alınır ve yalnızca bir kez indirilir.

---

## 🧠 Ayrıştırma nasıl çalışır?

Halite, açık kaynaklı [HT-Demucs](https://github.com/facebookresearch/demucs)
modelinin ONNX'e aktarılmış halini kullanır:

- `htdemucs.onnx` — yüksek kalite, 4 katman (~331 MB)
- `htdemucs_fp16weights.onnx` — daha hafif indirme (~174 MB)

Modeller [StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx)
(MIT) tarafından sağlanır ve ilk kullanımda Hugging Face'ten indirilir.

---

## 🛠️ Kaynaktan derleme

### Gereksinimler

- [Rust](https://rustup.rs/) 1.92+
- [Node.js](https://nodejs.org/) 18+
- [Tauri v2 sistem bağımlılıkları](https://tauri.app/start/prerequisites/)

### Komutlar

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm install
npm run tauri dev       # geliştirme
npm run tauri build     # sürüm paketi
```

---

## 🗺️ Yol haritası

- [ ] 6 katmanlı model (gitar + piyano)
- [ ] Menü çubuğundan hızlı erişim
- [ ] Şarkı sözü görünümü için albüm kapağı çekme

---

## 🙏 Teşekkürler

Halite şu harika açık kaynak projelerin üzerine inşa edilmiştir:

- [facebookresearch/demucs](https://github.com/facebookresearch/demucs) — HT-Demucs modeli (MIT)
- [StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx) — ONNX aktarımı (MIT)
- [pykeio/ort](https://github.com/pykeio/ort) — Rust için ONNX Runtime
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — medya indirici
- [LRCLIB](https://lrclib.net) — açık şarkı sözü veritabanı
- [Tauri](https://tauri.app) — uygulama çatısı
- [AnaghSharma/Carol](https://github.com/AnaghSharma/Carol) — şarkı sözü görünümü için ilham

---

## 📄 Lisans

Halite [MIT Lisansı](LICENSE) ile yayınlanmıştır. Yapay zekâ modelleri ilgili
yazarları tarafından MIT ile lisanslanmıştır.

> **Sorumluluk reddi:** Halite kişisel, eğitimsel ve adil kullanım için sunulur.
> İşlediğiniz veya indirdiğiniz içeriğin telif haklarına ve kullanım koşullarına
> saygı gösterin.
