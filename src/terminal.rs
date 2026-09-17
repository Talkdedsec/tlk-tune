use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::{execute, terminal};

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
    ShiftUp,
    ShiftDown,
    Char(char),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Input {
    None,
    Key(Key),
    Click { col: usize, row: usize },
    RightClick { col: usize, row: usize },
    Drag { col: usize, row: usize },
    Scroll { col: usize, row: usize, up: bool },
    Resize,
}

pub struct Console {
    raw: bool,
}

impl Console {
    /// True when stdout really is a console. Started with its output piped to
    /// a file the player would happily draw frames into it forever, so the
    /// caller refuses instead.
    pub fn available() -> bool {
        io::stdout().is_terminal()
    }

    pub fn open() -> io::Result<Console> {
        #[cfg(windows)]
        use_utf8_codepage();
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        execute!(out, EnableMouseCapture)?;
        out.write_all(b"\x1b[?25l")?;
        out.flush()?;
        Ok(Console { raw: true })
    }

    pub fn cols(&self) -> i32 {
        terminal::size().map(|(w, _)| w as i32).unwrap_or(155)
    }

    pub fn rows(&self) -> i32 {
        terminal::size().map(|(_, h)| h as i32).unwrap_or(40)
    }

    /// Returns one pending event, or `Input::None`. Never blocks.
    pub fn read(&self) -> Input {
        if !event::poll(Duration::from_millis(0)).unwrap_or(false) {
            return Input::None;
        }
        match event::read() {
            Ok(Event::Key(k)) => match translate(k) {
                Key::None => Input::None,
                key => Input::Key(key),
            },
            Ok(Event::Mouse(m)) => {
                let col = m.column as usize + 1;
                let row = m.row as usize + 1;
                match m.kind {
                    MouseEventKind::Down(MouseButton::Left) => Input::Click { col, row },
                    MouseEventKind::Down(MouseButton::Right) => Input::RightClick { col, row },
                    MouseEventKind::Drag(MouseButton::Left) => Input::Drag { col, row },
                    MouseEventKind::ScrollUp => Input::Scroll { col, row, up: true },
                    MouseEventKind::ScrollDown => Input::Scroll {
                        col,
                        row,
                        up: false,
                    },
                    _ => Input::None,
                }
            }
            Ok(Event::Resize(_, _)) => Input::Resize,
            _ => Input::None,
        }
    }

    /// Sets the window title through the terminal's own escape, which both
    /// Windows Terminal and conhost honour.
    pub fn set_title(&self, title: &str) {
        let clean: String = title.chars().filter(|c| !c.is_control()).collect();
        let mut out = io::stdout().lock();
        let _ = write!(out, "\x1b]0;{}\x07", clean);
        let _ = out.flush();
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
        let _ = execute!(out, DisableMouseCapture);
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
    let shift = k.modifiers.contains(KeyModifiers::SHIFT);
    match k.code {
        KeyCode::Up if shift => Key::ShiftUp,
        KeyCode::Down if shift => Key::ShiftDown,
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
