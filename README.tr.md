# tlk-tune

Windows için terminal müzik çalar. Tek exe, harici bağımlılık yok, tamamen
klavyeyle.

```
tlk-tune
```

[English](README.md)

## Ne yapar

- Yerel dosyaları çalar (MP3, FLAC, WAV, OGG, Opus, M4A, AAC, AIFF); çözme
  Rust içinde yapılır — ffmpeg yok, codec pack yok.
- Canlı FFT spektrumu; akıcılık, sönme hızı ve kıvam ayarlanabilir.
- İlerleme çubuğu boyunca braille dalga formu, çözülmüş PCM'den üretilir.
- Renk geçişli dönen plak.
- Senkron sözler, aktif kelime vurgusuyla; yanındaki `.lrc` dosyasından ya da
  LRCLIB'den.
- Sıra, karıştır, tekrarla, klasör filtresi, bulanık arama.
- `yt-dlp` PATH'teyse çevrimiçi arama ve akış; olmasa da geri kalan her şey
  çalışır.
- Türkçe ve İngilizce arayüz.
- Uygulama içi ayar ekranı: her renk, her anahtar, her animasyon, her kısayol.

## Kurulum

Rust araç zinciri gerekir.

```
git clone https://github.com/Talkdedsec/tlk-tune.git
cd tlk-tune
cargo build --release
```

Çıktı: `target/release/tlk-tune.exe`. PATH'te bir yere kopyala.

İsteğe bağlı: çevrimiçi arama, akış ve indirme için
[yt-dlp](https://github.com/yt-dlp/yt-dlp).

## Kısayollar

| İşlem | Tuş |
| :--- | :--- |
| Yerel arama | `/` |
| Çevrimiçi arama | `/` sonra `s: sorgu` |
| Çal / duraklat | `p` |
| Seçileni çal | `Enter` |
| Sonraki / önceki | `n` / `b` |
| İleri / geri sar | `←` / `→` |
| Ses | `2` / `1` |
| Karıştır / tekrarla | `m` / `r` |
| Sıraya ekle / çıkar | `a` / `d` |
| Panel değiştir | `Tab` |
| Klasöre göre filtrele | `f` |
| Filtreyi temizle | `c` |
| Sıralamayı değiştir | `o` |
| Akışı indir | `y` |
| Ayarlar | `s` |
| Çıkış | `q` |

Tüm tuşlar ayar ekranının KISAYOLLAR sekmesinden değiştirilebilir.

## Yapılandırma

Çıkışta `%APPDATA%\tlk-tune\config.txt` dosyasına yazılır, açılışta okunur.
Renkler ANSI 256 palet indeksidir, düz sayı olarak yazılır; boş değer
"terminale bırak" demektir.

```ini
Language=tr

ColorBorderTop=250
ColorBorderBottom=250
ColorDiskTop=240
ColorDiskBottom=255

ElimentDisk=true
ElimentQueue=true
LyricsPlaceholderBall=false

VisualizerFluidity=10
LyricsAnimation=word by word

LocalMusicPath=D:\Muzik
LocalMusicPath=E:\Albumler
```

`LocalMusicPath` yazılmazsa Müzik ve İndirilenler klasörleri taranır.

## Sözler

Parçanın yanındaki `.lrc` dosyası önceliklidir. `<mm:ss.xx>` kelime etiketli
gelişmiş LRC karaoke vurgusunu doğrudan besler. Yoksa LRCLIB'e sorulur ve her
satırın süresi harf sayısına göre kelimelere dağıtılır, böylece vurgu yine
kelime kelime ilerler. Sonuçlar `%LOCALAPPDATA%\tlk-tune\lyrics` altında
saklanır.

## Diğer bayraklar

```
tlk-tune --preview 155   155 sütun genişliğinde tek kare basıp çıkar
tlk-tune --version
```

`--preview`, çaları açmadan renk şemasını kontrol etmek için.

## Terminal

Braille ve kutu çizimi karakterlerini basabilen bir terminal gerekir. Windows
Terminal kutudan çıktığı gibi çalışır; klasik `conhost` için Cascadia Mono ya
da DejaVu Sans Mono gibi bir font gerekir. Konsol kod sayfasını program kendisi
UTF-8'e alır.

## Lisans

MIT. Bkz. [LICENSE](LICENSE).
