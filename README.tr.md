<div align="center">

# tlk-tune

### Masaüstü müzik çalarının yaptığı her şey, seksen sütunda.

Renkli albüm kapağı. Söylenen kelimeyi takip eden sözler. 10 bantlı ekolayzer.
Yayın standardında ses eşitleme.
**Tek bir 5.5 MB exe** — ffmpeg yok, codec pack yok, çalışma zamanı yok.

[![ci](https://github.com/Talkdedsec/tlk-tune/actions/workflows/ci.yml/badge.svg)](https://github.com/Talkdedsec/tlk-tune/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/Talkdedsec/tlk-tune)](https://github.com/Talkdedsec/tlk-tune/releases/latest)
[![license](https://img.shields.io/github/license/Talkdedsec/tlk-tune)](LICENSE)
[![platform](https://img.shields.io/badge/windows%20%C2%B7%20linux%20%C2%B7%20macos-0a7bbb)](https://github.com/Talkdedsec/tlk-tune/releases/latest)

**[İndir](https://github.com/Talkdedsec/tlk-tune/releases/latest)** ·
[English](README.md) ·
[Nasıl kurulu](docs/architecture.md)

<img src="docs/player.png" width="900" alt="Çalan parça: albüm kapağı, künye, senkron sözler, dalga formu ve kitaplık listesi">

</div>

## Kimsenin beklemediği kısım

Fareyle kullanılan bir metin programı.

**Plağa tıkla**, durur. **Dalga formunu sürükle**, sarar. **Satıra bir kez
tıkla** seçer, **bir daha tıkla** çalar, **sağ tıkla** sıraya alır.
**Tekerleği çevir**, liste kayar. Ayar ekranındaki her renk, her anahtar, her
kaydırak da tıklanabilir — istemedikçe hiçbir yapılandırma dosyası açman
gerekmiyor.

Klavye de hepsini yapar; `?` ikisini birden ekrana getirir.

<details>
<summary><b>Dört görüntü daha</b></summary>

<br>

Aksanı yok sayan arama — Türkçe karakteri olmayan bir klavyede yazılan
`dunya`, `Dünya`'yı buluyor:

<img src="docs/search.png" width="900" alt="Türkçe karakter kullanılmadan yazılmış aramayla süzülmüş kitaplık">

Her renk canlı önizleme üstünde düzenleniyor, metin düzenleyici yok:

<img src="docs/settings.png" width="900" alt="Ayar ekranının renkler sekmesi">

`?` ile bütün tuşlar ve fare hareketleri:

<img src="docs/help.png" width="900" alt="Tuşları ve fare hareketlerini listeleyen yardım ekranı">

Pencere daraldığında kendini topluyor:

<img src="docs/narrow.png" width="620" alt="Aynı çalar 78 sütunda, liste sığacak şekilde kısaltılmış">

</details>

## Üç dakika

Windows'ta [scoop](https://scoop.sh) hepsini tek seferde yapar:

```
scoop bucket add tlk https://github.com/Talkdedsec/scoop-tlk
scoop install tlk/tlk-tune
```

Değilse:

1. Sistemine uygun sürümü
   [son sürümden](https://github.com/Talkdedsec/tlk-tune/releases/latest) indir.
   En çok Windows'ta yaşadı; Linux ve macOS derlemeleri daha yeni ve daha az
   yol katetti.
2. Çalıştır. Hiçbir ayar yapılmamışken Müzik ve İndirilenler klasörlerini
   tarar, yani genelde hemen çalacak bir şey bulunur.
3. `?` bütün tuşları ve hareketleri gösterir. `s` ayarları açar; müziğinin
   gerçekten durduğu klasörü KLASORLER sekmesinden gösterirsin.

`%APPDATA%\tlk-tune` ve `%LOCALAPPDATA%\tlk-tune` dışına sen istemedikçe
hiçbir şey yazılmaz; `--install` ayrı ve geri alınabilir bir adımdır.

## Ne yapar

### Ses

- MP3, FLAC, WAV, OGG, Opus, M4A, AAC ve AIFF çalar; çözme Rust içinde yapılır.
  ffmpeg yok, codec pack yok, yanına kurulacak hiçbir şey yok.
- **Boşluksuz** parça geçişi — ses cihazı bir kez açılıp açık kalır — ve
  istenirse 12 saniyeye kadar çapraz geçiş.
- **10 bantlı ekolayzer**, 31 Hz ile 16 kHz arası ±12 dB, yedi hazır ayarla.
  Ok tuşuyla ya da kaydırağı sürükleyerek. Düz eğri hesaplanmaz, atlanır.
- **Ses eşitleme**, EBU R 128'e göre: her parça bir kez ölçülür ve sonrasında
  hep aynı yükseklikte çalar, sert kırpma yerine yumuşak sınırlayıcıyla.
- Kulaklık çıkarsa cihazı yeniden açıp devam eder.

### Ekranda

- **Albüm kapağı** plağın yerinde, renkli olarak; dosyaya gömülü kapaktan ya da
  yanındaki `cover.jpg`'den gelir. Kitty grafik protokolünü ya da sixel'i
  konuşan bir terminalde — Windows Terminal, kitty, WezTerm, Ghostty, foot,
  xterm — renkli bloklar değil, gerçek çözünürlükte gerçek bir resim basar.
  Hiç kapak yoksa yerine boş kutu değil, dönen prosedürel plak geçer.
- **Senkron sözler**, o an söylenen kelime vurgulanmış hâlde; yanındaki `.lrc`
  dosyasından ya da LRCLIB'den. Yayınlanan zamanlama senin kopyanla tutmazsa
  parça başına kaydırılabilir.
- Canlı FFT spektrumu (akıcılık, sönme ve kıvam ayarlanır), ilerleme çubuğu
  boyunca parçanın tamamını gösteren braille dalga formu, sese tepki veren küre.
- Pencereye uyar. Kısa terminalde liste küçülür, yer kalmazsa plak paneli
  çekilir — kare yukarı kaçmaz.
- **Yedi sekmeli ayar ekranı** — renkler, öğeler, animasyon, ekolayzer, müzik
  klasörleri, tuşlar, hakkında — hepsi canlı önizleme üstünde fareyle
  düzenlenir. Metin düzenleyiciye gerek yok.
- Türkçe ve İngilizce, yeniden başlatmadan değişir.

### Kitaplığın

- Klasörler de `m3u`, `m3u8`, `pls` çalma listeleri de eşit derecede geçerli
  kitaplık kökü sayılır.
- Başlıklar etiketlerden gelir; `001 - Sanatçı - Başlık.mp3` dolu bir klasör
  dosya adlarıyla değil gerçek başlıklarla okunur. Etiket okumaları boyut ve
  değişiklik zamanına göre saklanır, yani büyük kitaplık bedelini bir kez öder.
- **Arama aksan tanımaz**: `oguzhan` yazınca `Oğuzhan`, `dunya` yazınca
  `Dünya` geliyor. Türkçe karakteri olmayan bir klavyede de çalışır.
- Sıra, karıştır, tekrarla, klasör ve sanatçı filtresi ve albüme göre sıralama
  dahil beş sıralama düzeni — etiketi olmayan dosyalar klasöre düşer, çünkü
  rip'lenmiş bir albüm genelde bir klasördür.
- **Kurduğunu sakla**: `w`, sırayı — ya da arama/filtre sonrası listede ne
  duruyorsa onu — müziğinin yanına `m3u8` olarak yazar. Yollar göreli
  yazılır, yani klasör taşınınca liste bozulmaz.
- **Beğeni, çalma sayısı ve görünümler**: `l` beğenir, `v` listeyi tümü /
  beğeniler / en çok çalan / son çalanlar arasında çevirir. İlk açılışta
  tlk-player kuruluysa bunlar kendiliğinden aktarılır.
- Parçayı, konumu, sesi ve sırayı çalıştırmalar arasında hatırlar.

### Pencerenin dışında

- Her yerde fare: düğmeler, sarma çubuğu olarak dalga formu, ses çubuğu,
  satırlar, sıra, tekerlek, ayarlardaki her denetim.
- Terminal arka plandayken medya tuşları çalışır.
- Uyku zamanlayıcısı ve çalanı gösteren pencere başlığı.
- `yt-dlp` PATH'teyse çevrimiçi arama, akış ve indirme. Geri kalan her şey
  onsuz çalışır.
- `--install` iki adla PATH'e koyar ve ses dosyalarının sağ tık menüsüne girdi
  ekler, yönetici hakkı istemeden.

## PATH'e koymak

Exe bulunduğu yerden çalışır, yani bu adım isteğe bağlı. `tlk-tune`'un her
terminalde çalışmasını istediğinde:

```
tlk-tune.exe --install
```

Windows'ta kendini `%LOCALAPPDATA%\Programs\tlk-tune` altına kopyalar, o
klasörü kullanıcı PATH'ine ekler, yanına `tune.cmd` koyar (kısa ad da çalışsın
diye) ve ses dosyalarının sağ tık menüsüne "Play with tlk-tune" girdisi ekler.
Linux ve macOS'ta `~/.local/bin` altına girer, `tune` sembolik bağ olarak
yanında durur; Linux'ta ayrıca masaüstü girdisi yazılır, dosya yöneticisi ses
dosyalarında onu önersin diye. Hiçbirinde yönetici hakkı gerekmez,
`--uninstall` hepsini tek tek geri alır. Sonrasında yeni bir terminal aç, iki
ad da orada.

İsteğe bağlı: PATH'teki [yt-dlp](https://github.com/yt-dlp/yt-dlp) çevrimiçi
aramayı, akışı ve indirmeyi açar.

### Kendin derlemek

```
git clone https://github.com/Talkdedsec/tlk-tune.git
cd tlk-tune
cargo build --release
```

Rust 1.88 ya da üstü; Linux'ta ayrıca ALSA başlıkları (`libasound2-dev`).
Çıktı `target/release` altına düşer. Her sürüm etiketten CI tarafından,
platform başına bir ikili olarak derlenir ve yanında indirileni karşılaştırmak
için `SHA256SUMS` gelir:

```
sha256sum -c SHA256SUMS                  # linux, macos
Get-FileHash <dosya> -Algorithm SHA256   # windows
```

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
| Listeyi çalma listesi olarak kaydet | `w` |
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
tlk-tune --config <dosya> ...   profili varsayılan yerine o dosyada tutar
tlk-tune --version
```

`--preview`, çaları açmadan renk şemasını kontrol etmek için. Pencere yüksekliği
için `--rows N`, diğer iki ekran için `--screen settings|help` alır.

`--config` profilin tamamını taşır — ayarlar, oturum ve istatistikler tek
klasörde durur — yani çalar bir bellekten, takıldığı makineye hiçbir şey
yazmadan çalışabilir. Satırın başında olmalı.

## Terminal

Braille ve kutu çizimi karakterlerini basabilen bir terminal gerekir. Windows
Terminal kutudan çıktığı gibi çalışır; klasik `conhost` için Cascadia Mono ya
da DejaVu Sans Mono gibi bir font gerekir. Konsol kod sayfasını program kendisi
UTF-8'e alır. Fare için terminalin fare raporlamasını desteklemesi yeterli;
Windows Terminal ve conhost ikisi de destekler.

## Bir şey ters gittiğinde

| Gördüğün | Sebebi |
| :--- | :--- |
| Plak yerine kutu ya da soru işareti | Fontta braille yok. Cascadia Mono ve DejaVu Sans Mono'da var. |
| `--install` sonrası `tlk-tune` bulunamıyor | PATH değişikliği sadece yeni terminallere ulaşır. Yeni bir tane aç. |
| Ses yok, gerisi çalışıyor | Cihazı başka bir program tekelinde tutuyor ya da varsayılan değişti. Ayar ekranının ANIMATION sekmesindeki Ses Cikisi satırı gerçekte ne varsa aralarında gezdirir. |
| Medya tuşları çalışmıyor | Başka bir çalar önce kaptı. İlk kaydeden, kapanana kadar elinde tutar. |
| Sözler hiç gelmiyor | Parçanın yanında `.lrc` yok ve LRCLIB'de de kaydı yok. Gerçek başlık ve sanatçıyı yazan etiketler işi kolaylaştırır. |
| Çevrimiçi arama boş dönüyor | `yt-dlp` PATH'te değil. Yerel tarafın tamamı onsuz çalışır. |
| Kare kayıyor ya da yırtılıyor | Pencere düzenden kısa. Daraldıkça panel düşürür ama yaklaşık on iki satıra ihtiyacı var. |

## Koda bakmak

[docs/architecture.md](docs/architecture.md) haritadır: hangi modül ne yapar,
hangi iş parçacığında çalışır ve bozulduğunda ses çıkarmayan kurallar neler.

## Lisans

MIT. Bkz. [LICENSE](LICENSE).
