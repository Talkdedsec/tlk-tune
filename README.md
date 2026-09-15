# tlk-tune

A terminal music player for Windows. One executable, no runtime dependencies,
keyboard only.

```
tlk-tune
```

[Türkçe](README.tr.md)

## What it does

- Plays local files (MP3, FLAC, WAV, OGG, Opus, M4A, AAC, AIFF) decoded
  natively in Rust — no ffmpeg, no codec pack.
- Live FFT spectrum with configurable fluidity, decay and viscosity.
- Braille waveform across the progress bar, built from the decoded PCM.
- Spinning record with a colour gradient.
- Synced lyrics with active-word highlighting, from a sidecar `.lrc` or LRCLIB.
- Queue, shuffle, repeat, folder filter, fuzzy search.
- Online search and streaming when `yt-dlp` is on PATH; everything else works
  without it.
- English and Turkish interface.
- An in-app settings screen: every colour, toggle, animation and hotkey.

## Install

Needs a Rust toolchain.

```
git clone https://github.com/Talkdedsec/tlk-tune.git
cd tlk-tune
cargo build --release
```

The binary lands at `target/release/tlk-tune.exe`. Copy it anywhere on PATH.

Optional: install [yt-dlp](https://github.com/yt-dlp/yt-dlp) for online search,
streaming and downloads.

## Keys

| Action | Key |
| :--- | :--- |
| Search local | `/` |
| Search online | `/` then `s: query` |
| Play / pause | `p` |
| Play selection | `Enter` |
| Next / previous | `n` / `b` |
| Seek | `←` / `→` |
| Volume | `2` / `1` |
| Shuffle / repeat | `m` / `r` |
| Queue add / remove | `a` / `d` |
| Switch panel | `Tab` |
| Filter by folder | `f` |
| Clear filter | `c` |
| Change sort | `o` |
| Download stream | `y` |
| Settings | `s` |
| Quit | `q` |

Every binding is remappable on the settings screen's REFERENCE tab.

## Configuration

Written to `%APPDATA%\tlk-tune\config.txt` on exit, and re-read on start.
Colours are ANSI 256 palette indices as bare numbers; an empty value means
"leave it to the terminal".

```ini
Language=en

ColorBorderTop=250
ColorBorderBottom=250
ColorDiskTop=240
ColorDiskBottom=255

ElimentDisk=true
ElimentQueue=true
LyricsPlaceholderBall=false

VisualizerFluidity=10
LyricsAnimation=word by word

LocalMusicPath=D:\Music
LocalMusicPath=E:\Albums
```

With no `LocalMusicPath` set, the player scans your Music and Downloads
folders.

## Lyrics

A `.lrc` file next to the track wins. Enhanced LRC with `<mm:ss.xx>` word tags
drives the karaoke highlight directly. Otherwise the player asks LRCLIB and
spreads each line's timing over its words by character count, so the highlight
still moves word by word. Results are cached under
`%LOCALAPPDATA%\tlk-tune\lyrics`.

## Other flags

```
tlk-tune --preview 155   render one frame at width 155 and exit
tlk-tune --version
```

`--preview` is how to check a colour scheme without launching the player.

## Terminal

Needs a terminal that draws braille and box-drawing glyphs. Windows Terminal
works out of the box; classic `conhost` needs a font like Cascadia Mono or
DejaVu Sans Mono. The player switches the console to UTF-8 itself.

## License

MIT. See [LICENSE](LICENSE).
