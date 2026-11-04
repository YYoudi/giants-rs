//! Desktop entry point. Configuration and logs never live in the game installation.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) fn user_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("GIANTS_USER_DATA_ROOT") {
        return Ok(path.into());
    }
    Ok(
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("User data directory unavailable")?)
            .join("Giants-Recomp/preview"),
    )
}

fn valid_installation(root: &Path, user: &Path) -> bool {
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    let Ok(user) = user.canonicalize() else {
        return false;
    };
    !root.starts_with(&user)
        && !user.starts_with(&root)
        && root.join("Bin/GTextEnglish.bin").is_file()
        && root.join("Bin/WorldList.bin").is_file()
        && std::fs::read_dir(root.join("Bin")).is_ok_and(|entries| {
            entries.filter_map(Result::ok).any(|e| {
                e.path()
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("gzp"))
            })
        })
}

fn adjacent_installation(executable: &Path, user: &Path) -> Option<PathBuf> {
    let directory = executable.parent()?;
    if valid_installation(directory, user) {
        return Some(directory.to_path_buf());
    }
    if directory
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("Bin"))
    {
        let parent = directory.parent()?;
        if valid_installation(parent, user) {
            return Some(parent.to_path_buf());
        }
    }
    None
}

fn select_installation(user: &Path) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    loop {
        let Some(root) = platform::folder()? else {
            return Ok(None);
        };
        if valid_installation(&root, user) {
            std::fs::write(
                user.join("installation.txt"),
                root.to_string_lossy().as_bytes(),
            )?;
            return Ok(Some(root));
        }
        error(
            "Choose the original Giants folder containing Bin and its game archives. User data must remain outside that folder.",
        );
    }
}

pub(crate) fn arguments() -> Result<Option<Vec<OsString>>, Box<dyn std::error::Error>> {
    let user = user_root()?;
    std::fs::create_dir_all(&user)?;
    platform::log(&user.join("Giants.log"))?;
    let configured = std::env::var_os("GIANTS_DATA_ROOT")
        .map(PathBuf::from)
        .filter(|path| valid_installation(path, &user))
        .or_else(|| adjacent_installation(&std::env::current_exe().ok()?, &user))
        .or_else(|| {
            std::fs::read_to_string(user.join("installation.txt"))
                .ok()
                .map(PathBuf::from)
        })
        .filter(|path| valid_installation(path, &user));
    let root = match configured {
        Some(root) => root,
        None => match select_installation(&user)? {
            Some(root) => root,
            None => return Ok(None),
        },
    };
    Ok(Some(vec![
        "--mecc-menu".into(),
        "--data-root".into(),
        root.into_os_string(),
        "--user-data-root".into(),
        user.into_os_string(),
    ]))
}

pub(crate) fn change_installation() -> Result<bool, Box<dyn std::error::Error>> {
    let user = user_root()?;
    std::fs::create_dir_all(&user)?;
    let Some(root) = select_installation(&user)? else {
        return Ok(false);
    };
    std::process::Command::new(std::env::current_exe()?)
        .env("GIANTS_DATA_ROOT", root)
        .spawn()?;
    Ok(true)
}

pub(crate) fn error(message: &str) {
    platform::error(message);
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{
        ffi::c_void,
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            io::IntoRawHandle,
        },
    };
    #[repr(C)]
    struct BrowseInfo {
        owner: *mut c_void,
        root: *const c_void,
        display: *mut u16,
        title: *const u16,
        flags: u32,
        callback: Option<unsafe extern "system" fn(*mut c_void, u32, isize, isize) -> i32>,
        parameter: isize,
        image: i32,
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHBrowseForFolderW(info: *const BrowseInfo) -> *mut c_void;
        fn SHGetPathFromIDListEx(id: *const c_void, path: *mut u16, size: u32, flags: u32) -> i32;
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(reserved: *mut c_void, mode: u32) -> i32;
        fn CoUninitialize();
        fn CoTaskMemFree(memory: *mut c_void);
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(owner: *mut c_void, text: *const u16, title: *const u16, flags: u32) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetStdHandle(kind: u32, handle: *mut c_void) -> i32;
    }
    fn wide(text: &str) -> Vec<u16> {
        std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(Some(0))
            .collect()
    }
    pub(super) fn folder() -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
        let title = wide("Giants — select your game installation (folder containing Bin)");
        let mut display = [0u16; 260];
        let mut path = vec![0u16; 32768];
        // All pointers refer to live buffers; the shell owns the returned PIDL.
        unsafe {
            let initialized = CoInitializeEx(std::ptr::null_mut(), 2);
            if initialized < 0 {
                return Err("Cannot initialize the folder picker".into());
            }
            let info = BrowseInfo {
                owner: std::ptr::null_mut(),
                root: std::ptr::null(),
                display: display.as_mut_ptr(),
                title: title.as_ptr(),
                flags: 0x51,
                callback: None,
                parameter: 0,
                image: 0,
            };
            let id = SHBrowseForFolderW(&info);
            let result = if id.is_null() {
                Ok(None)
            } else {
                let valid = SHGetPathFromIDListEx(id, path.as_mut_ptr(), path.len() as u32, 0);
                CoTaskMemFree(id);
                if valid == 0 {
                    Err("Cannot read the selected directory".into())
                } else {
                    let end = path.iter().position(|v| *v == 0).unwrap_or(path.len());
                    Ok(Some(PathBuf::from(OsString::from_wide(&path[..end]))))
                }
            };
            CoUninitialize();
            result
        }
    }
    pub(super) fn error(message: &str) {
        let message = wide(message);
        let title = wide("Giants");
        unsafe {
            MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), 0);
        }
    }
    pub(super) fn log(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let handle = std::fs::File::create(path)?.into_raw_handle();
        // Keep the file handle alive until process exit; stdout and stderr share it.
        unsafe {
            SetStdHandle((-11i32) as u32, handle);
            SetStdHandle((-12i32) as u32, handle);
        }
        Ok(())
    }
}
#[cfg(not(windows))]
mod platform {
    use super::*;
    pub(super) fn folder() -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
        Err("Desktop installation selection is currently Windows-only".into())
    }
    pub(super) fn error(message: &str) {
        eprintln!("Giants: {message}");
    }
    pub(super) fn log(_: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installation_checks_keep_user_state_outside_original_data() {
        let temp = std::env::temp_dir().join(format!("giants-desktop-{}", std::process::id()));
        let game = temp.join("game");
        let user = temp.join("user");
        std::fs::create_dir_all(game.join("Bin")).unwrap();
        std::fs::create_dir_all(&user).unwrap();
        for file in ["GTextEnglish.bin", "WorldList.bin", "synthetic.gzp"] {
            std::fs::write(game.join("Bin").join(file), b"synthetic").unwrap();
        }
        assert!(valid_installation(&game, &user));
        assert_eq!(
            adjacent_installation(&game.join("giants-rs.exe"), &user),
            Some(game.clone())
        );
        assert_eq!(
            adjacent_installation(&game.join("Bin/giants-rs.exe"), &user),
            Some(game.clone())
        );
        assert!(adjacent_installation(&user.join("giants-rs.exe"), &user).is_none());
        assert!(!valid_installation(&game, &game));
        assert!(!valid_installation(&game, &temp));
        std::fs::create_dir_all(game.join("user")).unwrap();
        assert!(!valid_installation(&game, &game.join("user")));
        std::fs::remove_file(game.join("Bin/WorldList.bin")).unwrap();
        assert!(!valid_installation(&game, &user));
        std::fs::remove_dir_all(temp).unwrap();
    }
}
