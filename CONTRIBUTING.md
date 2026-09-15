# Contributing

Bug reports and patches are welcome. The project is small enough that there is
no ceremony beyond what is below.

## Building

```
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

All four have to pass; CI runs exactly those on Windows.

## Checking the interface without launching it

```
cargo run -- --preview 155
cargo run -- --preview 155 oguzhan
cargo run -- --config path\to\config.txt --preview 120
cargo run -- --preview 120 --rows 30 --screen settings
```

`--preview` renders one frame and exits. It is how a colour scheme or a
layout change gets checked; the width argument is what makes narrow terminals
easy to reason about, `--rows` does the same for short ones, and `--screen`
reaches the settings and the help overlay. `--config` points the whole profile
somewhere else, which is how the screenshots in the README were taken without
anybody's own library in them.

## House rules

A few of these exist because breaking them is silent rather than loud.

- **Every line of a frame is exactly the width it was asked for.** The only
  exception is the bottom border of the progress panel, which is open on the
  volume side by design. `--preview` output is the way to check.
- **Widths are measured, never counted.** `text::width` understands wide
  glyphs and combining marks; `str::len` does not, and ANSI escapes are not
  visible columns.
- **Hit targets live in `ui/layout.rs` and nowhere else.** Drawing and
  clicking derive from the same geometry, and
  `targets_land_on_what_was_drawn` scans a rendered frame to prove they still
  agree. Add a clickable thing, extend that test.
- **Every key the config parser understands must be written back by
  `config::render`.** The player rewrites the file when it exits, so a key
  that only parses is a setting that disappears. Add the field to
  `nothing_default()` in the same module and the round trip test will hold you
  to it.
- **Nothing allocates or locks in the audio callback.** The mixer reads decks
  through `arc-swap` and the PCM buffer never reallocates.
- **Tests do not touch the machine they run on.** Use `App::headless()` rather
  than `App::new()`: the latter probes audio devices and claims the global
  media keys, which crashes a runner with no sound card.
- **Comments say why, not what.** If the code already shows it, leave it out.

## Reporting something

Include the terminal you were using, the output of `tlk-tune --version`, and
what the file was if it involves one track in particular. A frame captured
with `--preview` helps for anything that looks wrong on screen.
