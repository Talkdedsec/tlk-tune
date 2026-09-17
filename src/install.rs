//! Putting the player on PATH so `tlk-tune` works from any prompt.
//!
//! Everything here is per-user and needs no elevation, so `--uninstall` can
//! take all of it back out. Windows makes a folder under LOCALAPPDATA, adds it
//! to the user PATH and writes the right-click entry to HKCU. Unix uses
//! ~/.local/bin, which is already a convention, and on Linux writes a desktop
//! entry so a file manager knows what to offer.

use std::path::{Path, PathBuf};

#[cfg(windows)]
const FOLDER: &str = "tlk-tune";
#[cfg(windows)]
const EXE: &str = "tlk-tune.exe";
#[cfg(windows)]
const SHIM: &str = "tune.cmd";
#[cfg(windows)]
const VERB: &str = "tlk-tune";
#[cfg(windows)]
const AUDIO: [&str; 9] = [
    ".mp3", ".flac", ".wav", ".ogg", ".opus", ".m4a", ".aac", ".aiff", ".wma",
];

#[cfg(unix)]
const EXE: &str = "tlk-tune";
#[cfg(unix)]
const SHIM: &str = "tune";

/// Where the installed copy lives. Windows has nowhere on PATH by default, so
/// the installer makes a folder and adds it. Unix already has a convention.
#[cfg(windows)]
pub fn target_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("Programs").join(FOLDER))
}

#[cfg(unix)]
pub fn target_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".local").join("bin"))
}

#[cfg(unix)]
pub fn install() -> Result<String, String> {
    use std::os::unix::fs::PermissionsExt;

    let here = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = target_dir().ok_or("no home folder")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let exe = dir.join(EXE);
    if here != exe {
        // Copying onto a binary that is currently executing corrupts the
        // running image. Unlinking first leaves the old inode alive for
        // whoever still has it open, which is the usual unix answer.
        let _ = std::fs::remove_file(&exe);
        std::fs::copy(&here, &exe).map_err(|e| format!("copy: {e}"))?;
        let mut mode = std::fs::metadata(&exe)
            .map_err(|e| e.to_string())?
            .permissions();
        mode.set_mode(0o755);
        std::fs::set_permissions(&exe, mode).map_err(|e| e.to_string())?;
    }

    let shim = dir.join(SHIM);
    let _ = std::fs::remove_file(&shim);
    std::os::unix::fs::symlink(&exe, &shim).map_err(|e| format!("link: {e}"))?;

    let mut told = format!("installed to {}", dir.display());
    if desktop_entry(&exe).is_some() {
        told.push_str(", and registered for audio files");
    }
    if !on_path(&dir) {
        told.push_str(&format!(
            "\n{} is not on your PATH. Add this to your shell profile:\n  export PATH=\"$HOME/.local/bin:$PATH\"",
            dir.display()
        ));
    } else {
        told.push_str("\nOpen a new shell and `tlk-tune` or `tune` will be there.");
    }
    Ok(told)
}

#[cfg(unix)]
pub fn uninstall() -> Result<String, String> {
    let dir = target_dir().ok_or("no home folder")?;
    let mut removed = 0;
    for name in [EXE, SHIM] {
        if std::fs::remove_file(dir.join(name)).is_ok() {
            removed += 1;
        }
    }
    if let Some(entry) = desktop_path() {
        if std::fs::remove_file(entry).is_ok() {
            removed += 1;
        }
    }
    if removed == 0 {
        return Ok("nothing was installed".into());
    }
    Ok(format!("removed {removed} files from {}", dir.display()))
}

#[cfg(unix)]
fn on_path(dir: &Path) -> bool {
    std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).any(|p| p == dir))
        .unwrap_or(false)
}

/// The freedesktop equivalent of the right-click entry: a launcher that says
/// which audio types it opens, so a file manager offers it.
#[cfg(target_os = "linux")]
fn desktop_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("applications").join("tlk-tune.desktop"))
}

#[cfg(all(unix, not(target_os = "linux")))]
fn desktop_path() -> Option<PathBuf> {
    None
}

#[cfg(target_os = "linux")]
fn desktop_entry(exe: &Path) -> Option<PathBuf> {
    const MIME: &str = "audio/mpeg;audio/flac;audio/wav;audio/ogg;audio/opus;\
                        audio/mp4;audio/aac;audio/x-aiff;";
    let file = desktop_path()?;
    std::fs::create_dir_all(file.parent()?).ok()?;
    let body = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=tlk-tune\n\
         Comment=Terminal music player\n\
         Exec={} %f\n\
         Terminal=true\n\
         Categories=AudioVideo;Audio;Player;\n\
         MimeType={MIME}\n",
        exe.display()
    );
    std::fs::write(&file, body).ok()?;
    // Best effort: without this a file manager may not notice until it is
    // restarted, and its absence is not a reason to fail the install.
    let _ = std::process::Command::new("update-desktop-database")
        .arg(file.parent()?)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    Some(file)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn desktop_entry(_exe: &Path) -> Option<PathBuf> {
    None
}

#[cfg(windows)]
pub fn install() -> Result<String, String> {
    let here = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = target_dir().ok_or("no local app data folder")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let exe = dir.join(EXE);
    // A leftover from a previous update, now that whatever held it has gone.
    let stale = dir.join(format!("{EXE}.old"));
    let _ = std::fs::remove_file(&stale);

    if here != exe {
        if let Err(first) = std::fs::copy(&here, &exe) {
            // Windows refuses to overwrite a running exe but is happy to
            // rename it out of the way, which is how a program updates itself
            // while a copy of it is still open somewhere.
            std::fs::rename(&exe, &stale)
                .map_err(|_| format!("copy: {first}"))
                .and_then(|_| {
                    std::fs::copy(&here, &exe)
                        .map(|_| ())
                        .map_err(|e| format!("copy: {e}"))
                })?;
        }
    }

    // A one line shim so `tune` works as well as `tlk-tune`.
    std::fs::write(
        dir.join(SHIM),
        format!("@echo off\r\n\"%~dp0{EXE}\" %*\r\n"),
    )
    .map_err(|e| format!("shim: {e}"))?;

    let on_path = add_to_path(&dir)?;
    let associations = add_file_menu(&exe)?;
    broadcast_environment_change();

    Ok(format!(
        "installed to {}\n{}\nright-click entry added for {associations} audio types\nopen a new terminal, then: tlk-tune",
        dir.display(),
        if on_path {
            "added to your PATH"
        } else {
            "already on your PATH"
        }
    ))
}

#[cfg(windows)]
pub fn uninstall() -> Result<String, String> {
    let dir = target_dir().ok_or("no local app data folder")?;
    let removed_path = remove_from_path(&dir)?;
    let associations = remove_file_menu()?;

    let _ = std::fs::remove_file(dir.join(SHIM));
    let _ = std::fs::remove_file(dir.join(format!("{EXE}.old")));
    let exe_gone = std::fs::remove_file(dir.join(EXE)).is_ok();
    let _ = std::fs::remove_dir(&dir);
    broadcast_environment_change();

    Ok(format!(
        "{}\n{}\nright-click entry removed from {associations} audio types",
        if exe_gone {
            format!("removed {}", dir.display())
        } else {
            format!("left {} in place - it was running", dir.display())
        },
        if removed_path {
            "taken off your PATH"
        } else {
            "was not on your PATH"
        }
    ))
}

#[cfg(windows)]
mod registry {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    pub const HKEY_CURRENT_USER: isize = -2147483647;
    const REG_SZ: u32 = 1;
    pub const REG_EXPAND_SZ: u32 = 2;
    const KEY_READ: u32 = 0x2_0019;
    const KEY_WRITE: u32 = 0x2_0006;
    const ERROR_SUCCESS: i32 = 0;

    #[allow(non_snake_case)]
    extern "system" {
        fn RegOpenKeyExW(key: isize, sub: *const u16, opts: u32, sam: u32, out: *mut isize) -> i32;
        fn RegCreateKeyExW(
            key: isize,
            sub: *const u16,
            reserved: u32,
            class: *const u16,
            options: u32,
            sam: u32,
            security: *const core::ffi::c_void,
            out: *mut isize,
            disposition: *mut u32,
        ) -> i32;
        fn RegQueryValueExW(
            key: isize,
            name: *const u16,
            reserved: *const u32,
            kind: *mut u32,
            data: *mut u8,
            len: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            key: isize,
            name: *const u16,
            reserved: u32,
            kind: u32,
            data: *const u8,
            len: u32,
        ) -> i32;
        fn RegCloseKey(key: isize) -> i32;
        fn RegDeleteTreeW(key: isize, sub: *const u16) -> i32;
    }

    pub fn wide(text: &str) -> Vec<u16> {
        std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn from_wide(buffer: &[u16]) -> String {
        let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        std::ffi::OsString::from_wide(&buffer[..end])
            .to_string_lossy()
            .to_string()
    }

    /// Returns the value and the type it was stored as, so writing it back
    /// does not turn an expandable PATH into a literal one.
    pub fn read(sub: &str, name: &str) -> Option<(String, u32)> {
        let mut key = 0isize;
        unsafe {
            if RegOpenKeyExW(HKEY_CURRENT_USER, wide(sub).as_ptr(), 0, KEY_READ, &mut key)
                != ERROR_SUCCESS
            {
                return None;
            }
            let name = wide(name);
            let mut kind = 0u32;
            let mut len = 0u32;
            let ok = RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut len,
            );
            if ok != ERROR_SUCCESS {
                RegCloseKey(key);
                return None;
            }
            let mut bytes = vec![0u8; len as usize];
            let ok = RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                bytes.as_mut_ptr(),
                &mut len,
            );
            RegCloseKey(key);
            if ok != ERROR_SUCCESS {
                return None;
            }
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            Some((from_wide(&units), kind))
        }
    }

    pub fn write(sub: &str, name: &str, value: &str, kind: u32) -> Result<(), String> {
        let mut key = 0isize;
        unsafe {
            let ok = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(sub).as_ptr(),
                0,
                std::ptr::null(),
                0,
                KEY_WRITE,
                std::ptr::null(),
                &mut key,
                std::ptr::null_mut(),
            );
            if ok != ERROR_SUCCESS {
                return Err(format!("cannot open {sub} ({ok})"));
            }
            let data = wide(value);
            let bytes: Vec<u8> = data.iter().flat_map(|u| u.to_le_bytes()).collect();
            // An empty name is how the default value of a key is written, so
            // both cases go through the same buffer and it outlives the call.
            let name = wide(name);
            let ok = RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                if kind == 0 { REG_SZ } else { kind },
                bytes.as_ptr(),
                bytes.len() as u32,
            );
            RegCloseKey(key);
            if ok != ERROR_SUCCESS {
                return Err(format!("cannot write {sub} ({ok})"));
            }
        }
        Ok(())
    }

    pub fn delete_tree(sub: &str) -> bool {
        unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide(sub).as_ptr()) == ERROR_SUCCESS }
    }
}

/// Splits a PATH value, dropping the empties a trailing `;` leaves behind.
#[cfg(windows)]
fn path_entries(value: &str) -> Vec<&str> {
    value.split(';').filter(|p| !p.trim().is_empty()).collect()
}

#[cfg(windows)]
fn same_folder(a: &str, b: &Path) -> bool {
    let a = a.trim().trim_end_matches(['\\', '/']).to_lowercase();
    let b = b
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_lowercase();
    a == b
}

#[cfg(windows)]
fn add_to_path(dir: &Path) -> Result<bool, String> {
    let (current, kind) =
        registry::read("Environment", "Path").unwrap_or((String::new(), registry::REG_EXPAND_SZ));
    if path_entries(&current).iter().any(|p| same_folder(p, dir)) {
        return Ok(false);
    }
    let mut entries: Vec<String> = path_entries(&current)
        .iter()
        .map(|s| s.to_string())
        .collect();
    entries.push(dir.to_string_lossy().to_string());
    registry::write("Environment", "Path", &entries.join(";"), kind)?;
    Ok(true)
}

#[cfg(windows)]
fn remove_from_path(dir: &Path) -> Result<bool, String> {
    let Some((current, kind)) = registry::read("Environment", "Path") else {
        return Ok(false);
    };
    let kept: Vec<&str> = path_entries(&current)
        .into_iter()
        .filter(|p| !same_folder(p, dir))
        .collect();
    if kept.len() == path_entries(&current).len() {
        return Ok(false);
    }
    registry::write("Environment", "Path", &kept.join(";"), kind)?;
    Ok(true)
}

#[cfg(windows)]
fn add_file_menu(exe: &Path) -> Result<usize, String> {
    let command = format!("\"{}\" \"%1\"", exe.display());
    let mut added = 0;
    for ext in AUDIO {
        let base = format!("Software\\Classes\\SystemFileAssociations\\{ext}\\shell\\{VERB}");
        registry::write(&base, "", "Play with tlk-tune", 0)?;
        registry::write(&base, "Icon", &exe.display().to_string(), 0)?;
        registry::write(&format!("{base}\\command"), "", &command, 0)?;
        added += 1;
    }
    Ok(added)
}

#[cfg(windows)]
fn remove_file_menu() -> Result<usize, String> {
    let mut removed = 0;
    for ext in AUDIO {
        let base = format!("Software\\Classes\\SystemFileAssociations\\{ext}\\shell\\{VERB}");
        if registry::delete_tree(&base) {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Tells already-running shells and Explorer that the environment moved.
#[cfg(windows)]
fn broadcast_environment_change() {
    const HWND_BROADCAST: isize = 0xffff;
    const WM_SETTINGCHANGE: u32 = 0x001A;
    const SMTO_ABORTIFHUNG: u32 = 0x0002;
    extern "system" {
        fn SendMessageTimeoutW(
            hwnd: isize,
            msg: u32,
            wparam: usize,
            lparam: *const u16,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> isize;
    }
    let target = registry::wide("Environment");
    let mut result = 0usize;
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            target.as_ptr(),
            SMTO_ABORTIFHUNG,
            3000,
            &mut result,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn path_entries_drop_the_empties() {
        assert_eq!(path_entries("a;b;;c;"), vec!["a", "b", "c"]);
        assert!(path_entries("").is_empty());
        assert!(path_entries(";;").is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn folders_compare_without_case_or_trailing_slash() {
        let dir = PathBuf::from(r"C:\Users\x\AppData\Local\Programs\tlk-tune");
        assert!(same_folder(
            r"c:\users\x\appdata\local\programs\TLK-TUNE",
            &dir
        ));
        assert!(same_folder(
            r#"C:\Users\x\AppData\Local\Programs\tlk-tune\"#,
            &dir
        ));
        assert!(!same_folder(r"C:\Users\x\AppData\Local\Programs", &dir));
    }

    #[test]
    fn the_install_folder_is_somewhere_a_user_owns() {
        let dir = target_dir().expect("no home folder");
        if cfg!(windows) {
            assert!(dir.ends_with("tlk-tune"));
        } else {
            assert!(dir.ends_with(".local/bin"));
        }
    }
}
