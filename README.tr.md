# tlk-tune

Windows için terminal müzik çalar. Tek exe, harici bağımlılık yok, klavye ve
fare ile.

```
tlk-tune
```

[English](README.md)

## Ne yapar

- Yerel dosyaları çalar (MP3, FLAC, WAV, OGG, Opus, M4A, AAC, AIFF); çözme
  Rust içinde yapılır — ffmpeg yok, codec pack yok.
- **Fareyle de kullanılır**: düğmelere tıkla, dalga formunu sürükleyip sar, ses
  çubuğuna tıkla, satıra tıklayıp seç ve tekrar tıklayıp çal, sağ tıkla sıraya
  ekle, tekerlekle kaydır.
- Canlı FFT spektrumu; akıcılık, sönme hızı ve kıvam ayarlanabilir.
- İlerleme çubuğu boyunca braille dalga formu, çözülmüş PCM'den üretilir.
- Renk geçişli dönen plak.
- Senkron sözler, aktif kelime vurgusuyla; yanındaki `.lrc` dosyasından ya da
  LRCLIB'den.
- Sıra, karıştır, tekrarla, klasör filtresi ve aksan tanımayan bulanık arama:
  `oguzhan` yazınca `Oğuzhan`, `dunya` yazınca `Dünya` geliyor.
- **Albüm kapağı** plağın yerinde, renkli olarak basılır; gömülü kapaktan ya da
  yanındaki `cover.jpg` dosyasından gelir, kapak yoksa dönen plak kalır.
- **10 bantlı ekolayzer**, hazır ayarlarla; ok tuşuyla ya da kaydırağa
  tıklayarak.
- **Boşluksuz** parça geçişi, istenirse 12 saniyeye kadar çapraz geçiş.
- Pencereye uyar: kısa terminalde liste küçülür, yer kalmazsa plak paneli
  çekilir — kare yukarı kaçmaz.
- Uyku zamanlayıcısı ve çalanı gösteren pencere başlığı.
- **Ses eşitleme**, EBU R 128'e göre: her parça bir kez ölçülüp aynı yükseklikte
  çalınıyor, sert kırpma yerine yumuşak sınırlayıcı var.
- **Beğeni, çalma sayısı ve görünümler**: `l` beğenir, `v` listeyi tümü /
  beğeniler / en çok çalan / son çalanlar arasında çevirir. tlk-player
  kuruluysa beğeniler ve sayaçlar ilk açılışta kendiliğinden aktarılır.
- Başlıklar etiketlerden gelir; `001 - Sanatçı - Başlık.mp3` dolu bir klasör
  dosya adı yerine gerçek başlıklarla okunur.
- `yt-dlp` PATH'teyse çevrimiçi arama ve akış; olmasa da geri kalanı çalışır.
- Türkçe ve İngilizce arayüz.
- Uygulama içi ayar ekranı: her renk, anahtar, animasyon, müzik klasörü, ses
  çıkışı ve kısayol — metin düzenleyiciye gerek yok.
- m3u, m3u8 ve pls çalma listelerini klasör gibi okur.
- Parçayı, konumu, sesi ve sırayı çalıştırmalar arasında hatırlar.
- Terminal arka plandayken medya tuşları çalışır.
- Kulaklık çıkarsa cihazı yeniden açıp devam eder.

## Kurulum

Rust araç zinciri gerekir.

```
git clone https://github.com/Talkdedsec/tlk-tune.git
cd tlk-tune
cargo build --release
```

Çıktı: `target/release/tlk-tune.exe`. Sonra:

```
target\release\tlk-tune.exe --install
```

Exe'yi `%LOCALAPPDATA%\Programs\tlk-tune` altına kopyalar, o klasörü kullanıcı
PATH'ine ekler, yanına `tune.cmd` koyar (iki adla da açılır) ve ses
dosyalarının sağ tık menüsüne "Play with tlk-tune" girdisi ekler. Yönetici
gerekmez, `--uninstall` hepsini geri alır. Yeni bir terminal aç, `tlk-tune`
yazman yeterli.

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
| Beğen / beğeniyi kaldır | `l` |
| Görünümü değiştir | `v` |
| Sanatçıya göre filtrele | `g` |
| Sırayı yeniden dizle | `Shift+↑` / `Shift+↓` |
| Sözleri kaydır | `[` / `]` |
| Uyku zamanlayıcısı | `t` |
| Tüm tuşlar, ekranda | `?` |
| Akışı indir | `y` |
| Ayarlar | `s` |
| Çıkış | `q` |

### Fare

| İşlem | Hareket |
| :--- | :--- |
| Çal / duraklat | Plağa ya da ortadaki düğmeye tıkla |
| Önceki / sonraki | `<<<` / `>>>` tıkla |
| Sarma | Dalga formuna tıkla ya da sürükle |
| Ses | Ses çubuğuna tıkla ya da sürükle |
| Parça seç | Satırına tıkla |
| Parçayı çal | Aynı satıra tekrar tıkla |
| Sıraya ekle | Satıra sağ tıkla |
| Sıradan çıkar | Sıradaki satıra sağ tıkla |
| Kaydır | Liste ya da sıra üzerinde tekerlek |
| Arama | Arama satırına tıkla |
| Ayarlar | ✦ kutusuna tıkla |
| Ayarlarda | Sekmeye tıkla, değere tıklayınca değişir, iki kez tıklayınca yazılır |

Tüm tuşlar ayar ekranının KISAYOLLAR sekmesinden değiştirilebilir.

## Yapılandırma

Çıkışta `%APPDATA%\tlk-tune\config.txt` dosyasına yazılır, açılışta okunur.
Renkler ANSI 256 palet indeksidir, düz sayı olarak yazılır; boş değer
"terminale bırak" demektir.

```ini
Language=tr
OutputDevice=

ColorBorderTop=250
ColorBorderBottom=250
ColorDiskTop=240
ColorDiskBottom=255

ElimentDisk=true
ElimentQueue=true
LyricsPlaceholderBall=false

VisualizerFluidity=10
LyricsAnimation=word by word
CrossfadeMs=0
Normalize=true
NormalizeTarget=-18.0
Equalizer=0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0

LocalMusicPath=D:\Muzik
LocalMusicPath=%USERPROFILE%\Music
LocalMusicPath=~/Music
LocalMusicPath=D:\Listeler\gece.m3u
```

Bir satır klasör de olabilir çalma listesi de; `~`, `%VAR%` ve `$VAR`
genişletilir. `LocalMusicPath` yazılmazsa Müzik ve İndirilenler klasörleri
taranır. Ayar ekranının KLASORLER sekmesi aynı listeyi düzenler ve her satırın
hâlâ var olup olmadığını gösterir.

Etiket okumaları `%LOCALAPPDATA%\tlk-tune\library.json` içinde, boyut ve
değişiklik zamanına göre saklanır; sonraki açılışta sadece yeni ya da
değişmiş dosyalar tekrar açılır.

## Sözler

Parçanın yanındaki `.lrc` dosyası önceliklidir. `<mm:ss.xx>` kelime etiketli
gelişmiş LRC karaoke vurgusunu doğrudan besler. Yoksa LRCLIB'e sorulur ve her
satırın süresi harf sayısına göre kelimelere dağıtılır, böylece vurgu yine
kelime kelime ilerler. Sonuçlar `%LOCALAPPDATA%\tlk-tune\lyrics` altında
saklanır.

Yayınlanan zamanlamalar aynı şarkının rip'iyle genelde tutmaz; `[` ve `]`
çeyrek saniyelik adımlarla kaydırır ve kaydırma parça başına hatırlanır.

## Çevrimiçi

`yt-dlp` PATH'teyken `/` sonra `s: sorgu` arar. Çalma HTTP range istekleriyle
akar ve ilk pakette başlar; range kabul etmeyen kaynaklar için
`%LOCALAPPDATA%\tlk-tune\stream` altına tam indirme yedek yoldur. `y` seçili
sonucu mp3 olarak ilk müzik klasörüne kaydeder.

## Diğer bayraklar

```
tlk-tune --install              PATH'e kurar, iki adla da çalışır
tlk-tune --uninstall            geri alır
tlk-tune <dosya>                o dosyayı çalar, klasörünü kitaplığa ekler
tlk-tune <kelimeler>            kitaplık aranmış hâlde açılır
tlk-tune --preview 155          155 sütun genişliğinde tek kare basıp çıkar
tlk-tune --preview 155 oguzhan  aynı kareyi arama uygulanmış hâlde basar
tlk-tune --version
```

`--preview`, çaları açmadan renk şemasını kontrol etmek için.

## Terminal

Braille ve kutu çizimi karakterlerini basabilen bir terminal gerekir. Windows
Terminal kutudan çıktığı gibi çalışır; klasik `conhost` için Cascadia Mono ya
da DejaVu Sans Mono gibi bir font gerekir. Konsol kod sayfasını program kendisi
UTF-8'e alır. Fare için terminalin fare raporlamasını desteklemesi yeterli;
Windows Terminal ve conhost ikisi de destekler.

## Lisans

MIT. Bkz. [LICENSE](LICENSE).
