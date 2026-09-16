# How it is put together

About 11,000 lines of Rust in four groups of modules, one main loop, one audio
callback, and a handful of short-lived worker threads that talk back over a
channel. Nothing here is a framework; if a file looks like it does one job, it
does.

## The map

```
src/
  main.rs         argument handling, and the exit
  app.rs          all mutable state, the frame loop, every key and click
  config.rs       the settings file: parse and render, kept symmetrical
  session.rs      what was playing, so the next start continues it
  lang.rs         every string, twice, English and Turkish
  install.rs      PATH, the shim, the right-click entry, all per user
  terminal.rs     raw mode, UTF-8, the alternate screen, the window title
  text.rs         width and truncation that understand escapes and wide glyphs
  mediakeys.rs    the media keys, claimed globally

  audio/
    player.rs     the mixer, the decks, the crossfade, the output device
    decoder.rs    symphonia: probe, tags, embedded art, decode to the sink
    buffer.rs     PcmStream, the one buffer a track is decoded into
    eq.rs         ten biquads and their presets
    loudness.rs   EBU R 128 and the limiter

  source/
    local.rs      the folder walk, accent-insensitive matching, playlists
    library.rs    the tag cache, keyed by size and modification time
    stats.rs      likes, play counts, last played, measured loudness
    lyrics.rs     sidecar .lrc, the cache, then LRCLIB
    online.rs     yt-dlp search and resolve
    http.rs       range requests for streaming
    paths.rs      ~, %VAR% and $VAR

  ui/
    layout.rs     every rectangle on screen, and therefore every hit target
    panels.rs     the player: record, metadata, lyrics, progress, list, queue
    settings_screen.rs  seven tabs, editable with the mouse
    help.rs       the key and gesture sheet

  visual/
    disk.rs       the procedural record, drawn in braille
    artwork.rs    cover art into half-block cells
    spectrum.rs   the FFT and its bars
    waveform.rs   the whole track reduced to one strip
    sphere.rs     the reactive sphere
```

## Where the work happens

**The main thread** owns everything in `App` and runs a 40 ms frame: read
input, drain the channel, redraw. It never blocks on IO and never decodes.

**The audio callback**, on cpal's WASAPI thread, is the only place that must
be fast. It allocates nothing and locks nothing:

```
deck (PcmStream + cursor + replay gain)
  → crossfade ramp against the outgoing deck
  → ten band peaking EQ
  → master volume
  → soft clip limiter
  → the device
```

The current deck lives in an `ArcSwapOption`, so a track change is a pointer
swap rather than a lock. The device opens on the first track and stays open,
which is the whole reason track changes are gapless.

**Short-lived threads**, one set per track, report back through an `mpsc`
channel as `Message` values that the main loop drains:

| Thread | Sends |
| :--- | :--- |
| decode | fills `PcmStream`; playback starts before it finishes |
| waveform | `Waveform`, then `Loudness` once it has the whole track |
| artwork | `Artwork`, from the tag or a cover beside the file |
| lyrics | `Lyrics`, from the sidecar, the cache, or LRCLIB |
| library scan | `Library`, then `RowMeta` per row as tags are read |
| online | `SearchResults`, `Resolved` |
| media keys | its own `RegisterHotKey` message pump |

So every slow thing — reading a thousand tags, measuring loudness over a whole
track, waiting on a network — happens off the loop, and the interface stays at
its frame rate while it does.

## Rules that are silent when broken

These are the ones that cost real time to discover, which is why they are
written down and, where possible, tested.

- **A frame's every line is exactly the width that was asked for.** The one
  exception is the bottom border of the progress panel, open on the volume
  side by design. `--preview N` is how to check.
- **Widths are measured, never counted.** `text::width` knows about wide
  glyphs, combining marks and escape sequences. `str::len` knows none of it.
- **Hit targets are computed in `ui/layout.rs` and nowhere else.** Drawing and
  clicking come from the same geometry, and
  `targets_land_on_what_was_drawn` strips the escapes out of a rendered frame
  to prove the two still agree.
- **Every key the config parser accepts must also be written by
  `config::render`.** The file is rewritten on exit, so a key that only parses
  is a setting that quietly disappears. `saving_and_loading_keeps_every_setting`
  holds the line.
- **Nothing allocates or locks in the audio callback.** `PcmStream` reserves
  its capacity from the container's duration and never grows.
- **Tests touch nothing on the machine.** `App::headless()` exists because
  `App::new()` probes audio devices and claims the media keys, which crashes a
  runner with no sound card.

## Decoding and memory

A track is decoded once into a `PcmStream` sized from the duration in the
container, converted to the device's rate and channel count on the way in.
Playback reads it while it is still filling, and seeking is clamped to what has
arrived so it can never land in silence. The waveform is built by streaming
over that same buffer rather than materialising a second copy — an hour of
audio is about 690 MB either way, and once is enough.

## The settings file

`config::parse` and `config::render` are deliberately mirror images, and the
round-trip test compares a config where every field differs from the default.
Colours are plain ANSI 256 indices; an empty value means "leave it to the
terminal", which is not the same as `0`.
