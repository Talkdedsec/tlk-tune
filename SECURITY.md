# Security

## Reporting

Report anything security relevant privately through
[GitHub security advisories](https://github.com/Talkdedsec/tlk-tune/security/advisories/new)
rather than a public issue. A reply should come within a week.

## What the player touches

Worth knowing before reading the code, and worth checking if you are auditing
a build:

- **Files**: reads audio files under the configured library folders, writes
  `config.txt`, `session.json` and `stats.json` under `%APPDATA%\tlk-tune`,
  and caches lyrics, artwork and downloaded streams under
  `%LOCALAPPDATA%\tlk-tune`.
- **Network**: only two destinations, both optional. `lrclib.net` for lyrics,
  and whatever `yt-dlp` resolves when online search is used. Nothing is sent
  anywhere else, there is no telemetry, and no account or key is involved.
- **Processes**: runs `yt-dlp` if it is on PATH, never a shell. Arguments are
  passed as arguments, so nothing is interpolated into a command line.
- **Registry**: `--install` writes under `HKCU\Environment` and
  `HKCU\Software\Classes` only, and `--uninstall` removes exactly what it
  wrote. Nothing runs with elevation.
- **Unsafe code**: three places, each with the invariant written above it -
  the lock free PCM buffer, the console mode calls, and the registry and
  hotkey bindings.
