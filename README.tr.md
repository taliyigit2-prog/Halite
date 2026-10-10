<div align="center">

<img src="src-tauri/icons/icon.png" alt="Halite" width="112" />

# Halite

### Müziğiniz. Cihazınız. Kontrol sizde.

**Müziği ayırmak, düzenlemek, indirmek ve anlamak için yerel yapay zekâ kullanan ücretsiz, açık kaynaklı masaüstü stüdyosu.**

[![MIT Lisansı](https://img.shields.io/badge/lisans-MIT-6d5dfc?style=flat-square)](LICENSE)
[![Tauri 2](https://img.shields.io/badge/masaüstü-Tauri%202-24c8db?style=flat-square)](https://tauri.app)
[![Yerel odaklı](https://img.shields.io/badge/gizlilik-yerel--odaklı-38c98b?style=flat-square)](#gizlilik-tasarımın-bir-parçası)

[Sürümü indir](https://github.com/taliyigit2-prog/Halite/releases) · [Sorun bildir](https://github.com/taliyigit2-prog/Halite/issues) · [Katkı yap](#geliştirme)

[English](README.md) · Türkçe · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Русский](README.ru.md) · [日本語](README.ja.md)

</div>

---

## Sakin, yetenekli bir müzik çalışma alanı

Halite; Rust ve Tauri ile oluşturulmuş yerel bir masaüstü uygulamasıdır. Ayrıştırma, etiket düzenleme ve ses üretimi için hesabınız, aboneliğiniz veya ses dosyanızı bir buluta yüklemeniz gerekmez.

| Üret | Düzenle | Keşfet |
| --- | --- | --- |
| HT-Demucs ile şarkıları vokal, davul, bas ve diğer katmanlara ayırın. | Etiketleri inceleme adımı, otomatik yerel yedek ve kapak görseli desteğiyle düzenleyin. | İlerleme göstergesiyle ses indirin ya da çalan parçanın senkronize sözlerini bulun. |
| Chatterbox Multilingual V3 ile cihazınızda konuşma üretin. | Çıktıları önizleyin ve iş akışınıza uygun formatta dışa aktarın. | BPM/ton analizi, toplu işleme ve yedi arayüz dilinden yararlanın. |

## Özellikler

### 🎚️ Stem ayrıştırma

- 4 katman (vokal, davul, bas, diğer) ve 2 katman (vokal, enstrümantal) ayrıştırma.
- Apple Silicon’da desteklendiğinde CoreML, aksi hâlde güvenli CPU geri dönüşüyle tamamen yerel HT-Demucs ONNX çıkarımı.
- Toplu kuyruk, kırpma aralığı, BPM/ton analizi, stem önizleme ve WAV/FLAC/MP3/M4A dışa aktarma.

### 🏷️ Müzik arşivinizi düzenleyin

- Başlık, sanatçı, albüm, albüm sanatçısı, tür, tarih, parça/disk numarası, besteci, telif, yorum ve gömülü söz alanlarını yönetin.
- Kapak ekleyin, kaldırın veya dışa aktarın; isteğe bağlı MusicBrainz araması ses dosyanızı göndermez.
- Her değişiklik önce incelenir, geçici dosyada doğrulanır ve özgün dosyanın yanına `*.halite-backup` yedeği alınır. Yedeği silmeden önce tek tıklamayla geri yükleyebilirsiniz.

### 🎙️ Yerel ses üretimi

- İsteğe bağlı **Chatterbox Multilingual V3** ortamı, ilk kullanımda boyut bilgisini göstererek otomatik kurulur; elle Python kurulumu gerekmez.
- Metinden konuşma ve açık rıza gerektiren referans ses modu. Referans ses cihazınızda kalır; yalnızca 3–30 saniyelik yerel kayıt kabul edilir.
- Üretilen konuşma PerTh filigranı ve kaynak bilgisiyle işaretlenir.
- İndirmeler sabit sürüm ve SHA-256 ile doğrulanır; kurulumdan sonra üretim çalışanı ağ erişimini engeller.

### ⬇️ İndirin, dinleyin, anlayın

- `yt-dlp`'nin desteklediği kaynaklardan görünür ilerleme bilgisiyle ses indirin.
- Uygun olduğunda Apple Music veya Spotify’daki geçerli parça için LRCLIB’den senkronize söz gösterin.
- English, Türkçe, Deutsch, Español, Français, Русский ve 日本語; açık, koyu ve sistem temaları.

## Gizlilik tasarımın bir parçası

- Ses ayrıştırma, metadata düzenleme ve ses üretimi cihazınızda gerçekleşir.
- Ses çalışma zamanı yalnızca **Otomatik hazırla** seçildiğinde doğrulanmış bileşenlerini indirir. Kurulum sırasında yaklaşık 11 GB boş alan gerekir; model uygulama paketinin dışında tutulur.
- Çevrim içi işlevler sınırlı ve görünürdür: indirme kaynak URL’sini kullanır, MusicBrainz yalnızca arama yaptığınızda çağrılır, sözler LRCLIB’den istenir.
- Halite, yerel araçları kullanmak için müzik arşivinizi veya referans sesinizi yüklemenizi istemez.

Lütfen yalnızca kullanma hakkınız olan içeriği işleyin ve indirdiğiniz kaynakların şartlarına uyun.

## Kurulum

Güncel paketi [GitHub Releases](https://github.com/taliyigit2-prog/Halite/releases) sayfasından indirin.

| Platform | Paket |
| --- | --- |
| macOS | `Halite_*.dmg` |
| Windows | `Halite_*.msi` veya `.exe` |
| Linux | `Halite_*.AppImage` veya `.deb` |

Apple tarafından noter onaylı bir sürüm yayınlanana kadar macOS Gatekeeper ilk açılışta uyarı gösterebilir. Bu durumda uygulamaya sağ tıklayıp **Aç** seçeneğini kullanın.

## Geliştirme

```bash
git clone https://github.com/taliyigit2-prog/Halite.git
cd Halite
npm ci
npm run tauri dev
```

```bash
npm run check-locales
npm run check-js
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

`npm run tauri build` platform paketini oluşturur. Büyük Chatterbox modeli pakete gömülü değildir; isteğe bağlı, hash ile doğrulanan ilk kullanım indirmesidir.

## Güvenlik ve lisanslar

- GitHub Actions en az yetki ilkesi ve commit’e sabitlenmiş eylemler kullanır.
- CI; JavaScript, yerelleştirme, Rust format/test/lint, bağımlılık duyuruları ve sır sızıntısı kontrollerini çalıştırır.
- Dependabot npm, Cargo ve iş akışı bağımlılıklarını haftalık izler.
- Çalışma zamanı indirmeleri sabit manifest, beklenen boyut ve SHA-256 denetimi kullanır.

Halite MIT lisanslıdır. Ayrıntılı üçüncü taraf bildirimleri için [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) dosyasına bakın.
