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
- **Mouse driven as well as keyboard**: click the transport buttons, click or
  drag the waveform to seek, click the volume bar, click a row to select and
  again to play, right-click to queue, scroll to move through the list.
- Live FFT spectrum with configurable fluidity, decay and viscosity.
- Braille waveform across the progress bar, built from the decoded PCM.
- Spinning record with a colour gradient.
- Synced lyrics with active-word highlighting, from a sidecar `.lrc` or LRCLIB.
- Queue, shuffle, repeat, folder filter, and a fuzzy search that ignores
  accents, so `oguzhan` finds `Oguzhan` and `dunya` finds `Dunya`.
- **Album art** drawn in colour where the record sits, from the cover embedded
  in the file or a `cover.jpg` beside it; the spinning vinyl is the fallback.
- **Ten-band equaliser** with presets, adjustable by arrow keys or by clicking
  the slider.
- **Gapless** track changes, and an optional crossfade up to twelve seconds.
- **Volume levelling** to EBU R 128: every track is measured once, then played
  at a consistent loudness, with a soft limiter instead of hard clipping.
- **Likes, play counts and views**: `l` likes a track, `v` cycles the list
  between all, liked, most played and recently played. Likes and counts import
  themselves from tlk-player on first run if it is installed.
- Titles come from the tags, so a folder of `001 - Artist - Title.mp3` reads as
  the titles rather than the filenames.
- Online search and streaming when `yt-dlp` is on PATH; everything else works
  without it.
- English and Turkish interface.
- An in-app settings screen: every colour, toggle, animation, music folder,
  output device and hotkey — no text editor needed.
- Reads m3u, m3u8 and pls playlists as if they were folders.
- Remembers the track, position, volume and queue between runs.
- Media keys work while the terminal is in the background.
- Survives an unplugged headset: it reopens the device and keeps going.

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
| Like / unlike | `l` |
| Change view | `v` |
| Filter by artist | `g` |
| Reorder the queue | `Shift+↑` / `Shift+↓` |
| Download stream | `y` |
| Settings | `s` |
| Quit | `q` |

### Mouse

| Action | Gesture |
| :--- | :--- |
| Play / pause | Click the record, or the middle button |
| Previous / next | Click `<<<` / `>>>` |
| Seek | Click or drag the waveform |
| Volume | Click or drag the volume bar |
| Select a track | Click its row |
| Play a track | Click the row again |
| Queue a track | Right-click its row |
| Remove from queue | Right-click it in the queue |
| Scroll | Wheel over the list or the queue |
| Search | Click the search line |
| Settings | Click the ✦ box |
| In settings | Click a tab, click a value to cycle it, click twice to type |

Every binding is remappable on the settings screen's REFERENCE tab.

## Configuration

Written to `%APPDATA%\tlk-tune\config.txt` on exit, and re-read on start.
Colours are ANSI 256 palette indices as bare numbers; an empty value means
"leave it to the terminal".

```ini
Language=en
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

LocalMusicPath=D:\Music
LocalMusicPath=%USERPROFILE%\Music
LocalMusicPath=~/Music
LocalMusicPath=D:\Lists\night.m3u
```

An entry may be a folder or a playlist file, and `~`, `%VAR%` and `$VAR` are
expanded. With no `LocalMusicPath` set, the player scans your Music and
Downloads folders. The PATHS tab in the settings screen edits the same list
and shows whether each entry still resolves.

Tag reads are cached in `%LOCALAPPDATA%\tlk-tune\library.json`, keyed by size
and modification time, so only new or edited files are opened again on a later
start.

## Lyrics

A `.lrc` file next to the track wins. Enhanced LRC with `<mm:ss.xx>` word tags
drives the karaoke highlight directly. Otherwise the player asks LRCLIB and
spreads each line's timing over its words by character count, so the highlight
still moves word by word. Results are cached under
`%LOCALAPPDATA%\tlk-tune\lyrics`.

## Online

With `yt-dlp` on PATH, `/` then `s: query` searches. Playback streams over HTTP
range requests and starts on the first packet; a full download into
`%LOCALAPPDATA%\tlk-tune\stream` is the fallback for anything that refuses
range requests. `y` saves the selected result as mp3 into your first music
folder.

## Other flags

```
tlk-tune --preview 155          render one frame at width 155 and exit
tlk-tune --preview 155 oguzhan  render that frame with a search applied
tlk-tune --version
```

`--preview` is how to check a colour scheme without launching the player.

## Terminal

Needs a terminal that draws braille and box-drawing glyphs. Windows Terminal
works out of the box; classic `conhost` needs a font like Cascadia Mono or
DejaVu Sans Mono. The player switches the console to UTF-8 itself.

## License

MIT. See [LICENSE](LICENSE).
