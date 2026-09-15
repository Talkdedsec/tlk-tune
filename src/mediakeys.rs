use std::sync::mpsc::Sender;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MediaKey {
    PlayPause,
    Next,
    Previous,
    Stop,
}

/// Claims the keyboard's transport keys so the player responds while the
/// terminal is in the background. Silently does nothing if another
/// application already holds them.
#[cfg(windows)]
pub fn listen(tx: Sender<MediaKey>) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK,
        VK_MEDIA_STOP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    std::thread::spawn(move || {
        let bindings = [
            (1, VK_MEDIA_PLAY_PAUSE, MediaKey::PlayPause),
            (2, VK_MEDIA_NEXT_TRACK, MediaKey::Next),
            (3, VK_MEDIA_PREV_TRACK, MediaKey::Previous),
            (4, VK_MEDIA_STOP, MediaKey::Stop),
        ];
        let mut claimed = Vec::new();
        for (id, vk, action) in bindings {
            let ok = unsafe { RegisterHotKey(std::ptr::null_mut(), id, 0, vk as u32) };
            if ok != 0 {
                claimed.push((id, action));
            }
        }
        if claimed.is_empty() {
            return;
        }

        let mut message: MSG = unsafe { std::mem::zeroed() };
        loop {
            let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
            if result <= 0 {
                return;
            }
            if message.message != WM_HOTKEY {
                continue;
            }
            let id = message.wParam as i32;
            if let Some((_, action)) = claimed.iter().find(|(bound, _)| *bound == id) {
                if tx.send(*action).is_err() {
                    return;
                }
            }
        }
    });
}

#[cfg(not(windows))]
pub fn listen(_tx: Sender<MediaKey>) {}
