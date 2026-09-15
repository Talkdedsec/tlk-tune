use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    None,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Tab,
    Esc,
    Backspace,
    Char(char),
}

pub struct Console {
    raw: bool,
}

impl Console {
    pub fn open() -> io::Result<Console> {
        #[cfg(windows)]
        use_utf8_codepage();
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        out.write_all(b"\x1b[?25l")?;
        out.flush()?;
        Ok(Console { raw: true })
    }

    pub fn cols(&self) -> i32 {
        terminal::size().map(|(w, _)| w as i32).unwrap_or(155)
    }

    /// Returns a pending key, or `Key::None`. Never blocks.
    pub fn read_key(&self) -> Key {
        if !event::poll(Duration::from_millis(0)).unwrap_or(false) {
            return Key::None;
        }
        match event::read() {
            Ok(Event::Key(k)) => translate(k),
            _ => Key::None,
        }
    }

    pub fn write(&self, frame: &str) {
        let mut out = io::stdout().lock();
        let _ = out.write_all(frame.as_bytes());
        let _ = out.flush();
    }

    pub fn close(&mut self) {
        if !self.raw {
            return;
        }
        self.raw = false;
        let mut out = io::stdout();
        let _ = out.write_all(b"\x1b[0m\x1b[2J\x1b[H\x1b[?25h");
        let _ = out.flush();
        let _ = terminal::disable_raw_mode();
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        self.close();
    }
}

fn translate(k: KeyEvent) -> Key {
    // The Windows console reports both press and release; drop the release.
    if k.kind == KeyEventKind::Release {
        return Key::None;
    }
    if k.modifiers.contains(KeyModifiers::CONTROL) {
        return match k.code {
            KeyCode::Char('c') => Key::Char('\u{3}'),
            _ => Key::None,
        };
    }
    match k.code {
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Esc => Key::Esc,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Char(c) => Key::Char(c),
        _ => Key::None,
    }
}

/// Resolves a config binding name ("ARROW_KEY_UP", "ENTER", "p") to a key.
pub fn binding(name: &str) -> Key {
    match name {
        "ARROW_KEY_UP" => Key::Up,
        "ARROW_KEY_DOWN" => Key::Down,
        "ARROW_KEY_LEFT" => Key::Left,
        "ARROW_KEY_RIGHT" => Key::Right,
        "ENTER" => Key::Enter,
        "TAB" => Key::Tab,
        "ESC" => Key::Esc,
        _ => match name.chars().next() {
            Some(c) => Key::Char(c),
            None => Key::None,
        },
    }
}

#[cfg(windows)]
fn use_utf8_codepage() {
    // Without codepage 65001 the braille and box-drawing glyphs come out as
    // question marks in cmd.exe.
    extern "system" {
        fn SetConsoleOutputCP(code: u32) -> i32;
        fn SetConsoleCP(code: u32) -> i32;
    }
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }
}
