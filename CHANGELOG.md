# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-09-16

First release.

### Playback

- Native decoding through symphonia for MP3, FLAC, WAV, OGG, Opus, M4A, AAC
  and AIFF. No ffmpeg, no codec pack, one executable.
- WASAPI output through cpal. The device opens on the first track and stays
  open, so track changes are gapless.
- Optional crossfade of up to twelve seconds, started before the outgoing
  track ends so the two actually overlap.
- Ten band equaliser, 31 Hz to 16 kHz at ±12 dB, with seven presets. Flat is
  bypassed entirely.
- Volume levelling to EBU R 128. Each track is measured once and corrected
  from the first sample on every later play, with a soft limiter in place of
  hard clipping.
- Playback starts while the file is still decoding; seeking is clamped to what
  has been decoded so it never lands in silence.

### Library

- Folders and `m3u`, `m3u8` and `pls` playlists are both valid library roots.
  `~`, `%VAR%` and `$VAR` are expanded.
- Tag reads are cached by size and modification time, so only new or edited
  files are opened again on a later start.
- Search ignores accents: `oguzhan` finds `Oğuzhan`, `dunya` finds `Dünya`.
- Views for all, liked, most played and recently played, plus artist and
  folder filters and four sort orders.
- Play counts are recorded once a track has really been listened to.

### Interface

- Mouse throughout: the transport buttons, the waveform as a seek bar, the
  volume bar, rows, the queue, the wheel, and every control in the settings.
- Album art drawn in colour from the embedded cover or a `cover.jpg` beside
  the track; a procedural spinning record when there is none.
- Live FFT spectrum, braille waveform, and an audio reactive sphere.
- Synced lyrics with word level highlighting from a sidecar `.lrc` or LRCLIB,
  shiftable per track when the published timings do not match the recording.
- Settings screen with seven tabs covering colours, elements, animation, the
  equaliser, library folders, hotkeys and about.
- English and Turkish, switchable without restarting.
- Fits the window: the list shrinks on a short terminal and the record panel
  steps aside rather than letting the frame scroll away.

### Elsewhere

- Online search, streaming over HTTP range requests and downloads through
  `yt-dlp` when it is installed.
- Media keys work while the terminal is in the background.
- The track, position, volume and queue survive between runs.
- `--config` points the whole profile — settings, session and statistics — at
  another folder, so the player can run from a stick.
- `--install` puts the player on the user PATH under both `tlk-tune` and
  `tune`, and adds a right-click entry for audio files. `--uninstall` reverses
  all of it. Neither needs administrator rights.

[Unreleased]: https://github.com/Talkdedsec/tlk-tune/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Talkdedsec/tlk-tune/releases/tag/v0.1.0
