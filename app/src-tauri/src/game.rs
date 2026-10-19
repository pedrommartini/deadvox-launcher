use crate::syntax::keyvalues;
use std::{
    fs::{self, File, OpenOptions},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    ptr,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW},
    },
    UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
};

pub fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn registry_string(hive: HKEY, key: &str, name: &str) -> Option<String> {
    let mut buffer = [0u16; 2048];
    let mut size = (buffer.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            hive,
            wide(key).as_ptr(),
            wide(name).as_ptr(),
            RRF_RT_REG_SZ,
            ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if result != 0 {
        return None;
    }
    let length = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..length]))
}

pub fn running() -> Result<bool, String> {
    let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if handle == INVALID_HANDLE_VALUE {
        return Err("Could not check whether Deadlock is running.".into());
    }
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut next = unsafe { Process32FirstW(handle, &mut entry) };
    let mut found = false;
    while next != 0 {
        let length = entry
            .szExeFile
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(entry.szExeFile.len());
        if String::from_utf16_lossy(&entry.szExeFile[..length]).eq_ignore_ascii_case("deadlock.exe")
        {
            found = true;
            break;
        }
        next = unsafe { Process32NextW(handle, &mut entry) };
    }
    unsafe {
        CloseHandle(handle);
    }
    Ok(found)
}

pub fn require_closed() -> Result<(), String> {
    if running()? {
        return Err("Close Deadlock before changing the voice language.".into());
    }
    Ok(())
}

pub fn valid_root(path: &Path) -> bool {
    path.join("game/citadel/gameinfo.gi").is_file()
        && path.join("game/citadel/pak01_dir.vpk").is_file()
}

pub fn normalize_root(path: &Path) -> Result<PathBuf, String> {
    for candidate in [
        path.to_path_buf(),
        path.join("Deadlock"),
        path.join("common/Deadlock"),
    ] {
        if valid_root(&candidate) {
            return fs::canonicalize(candidate).map_err(|e| e.to_string());
        }
    }
    if path.file_name().is_some_and(|s| s == "citadel") {
        if let Some(root) = path.parent().and_then(Path::parent) {
            if valid_root(root) {
                return fs::canonicalize(root).map_err(|e| e.to_string());
            }
        }
    }
    if path.file_name().is_some_and(|s| s == "game") {
        if let Some(root) = path.parent() {
            if valid_root(root) {
                return fs::canonicalize(root).map_err(|e| e.to_string());
            }
        }
    }
    Err("Select the Deadlock installation folder containing game/citadel.".into())
}

/// Windows denies execution of this image while the exclusive file handle lives.
/// This also covers Steam and desktop shortcuts outside our own Play button.
pub struct LaunchGuard {
    _image: Option<File>,
}
impl LaunchGuard {
    pub fn acquire(root: &Path) -> Result<Self, String> {
        require_closed()?;
        let image = root.join("game/bin/win64/deadlock.exe");
        let handle = if image.is_file() {
            Some(
                OpenOptions::new()
                    .read(true)
                    .share_mode(0)
                    .open(image)
                    .map_err(|_| {
                        "Deadlock is starting or its files are in use. Close it and try again."
                            .to_string()
                    })?,
            )
        } else {
            #[cfg(not(test))]
            return Err(
                "The Deadlock executable is missing. Check the selected game folder.".into(),
            );
            #[cfg(test)]
            None
        };
        require_closed()?;
        Ok(Self { _image: handle })
    }
}

pub fn pick_folder() -> Result<Option<PathBuf>, String> {
    use windows_sys::Win32::{
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize},
        UI::Shell::{
            BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, SHBrowseForFolderW,
            SHGetPathFromIDListW,
        },
    };
    unsafe {
        let initialized = CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED as u32);
        if initialized < 0 {
            return Err("The folder chooser could not start.".into());
        }
        let title = wide("Select your Deadlock installation folder");
        let info = BROWSEINFOW {
            lpszTitle: title.as_ptr(),
            ulFlags: BIF_NEWDIALOGSTYLE | BIF_RETURNONLYFSDIRS,
            ..Default::default()
        };
        let item = SHBrowseForFolderW(&info);
        let result = if item.is_null() {
            Ok(None)
        } else {
            let mut buffer = [0u16; 260];
            let ok = SHGetPathFromIDListW(item, buffer.as_mut_ptr());
            CoTaskMemFree(item.cast());
            if ok == 0 {
                Err("The selected folder could not be read.".into())
            } else {
                let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
                Ok(Some(PathBuf::from(String::from_utf16_lossy(
                    &buffer[..end],
                ))))
            }
        };
        CoUninitialize();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_and_normalizes_selected_game_folder() {
        let root =
            std::env::temp_dir().join(format!("deadlock-folder-{}", crate::install::stamp()));
        crate::install::write_atomic(&root.join("game/citadel/gameinfo.gi"), b"fixture").unwrap();
        crate::install::write_atomic(&root.join("game/citadel/pak01_dir.vpk"), b"fixture").unwrap();
        assert_eq!(
            normalize_root(&root.join("game/citadel")).unwrap(),
            fs::canonicalize(&root).unwrap()
        );
        assert!(normalize_root(&root.join("wrong")).is_err());
    }
    #[test]
    fn exclusive_image_handle_blocks_external_execution_until_released() {
        let root =
            std::env::temp_dir().join(format!("deadlock-launch-guard-{}", crate::install::stamp()));
        let image = root.join("game/bin/win64/deadlock.exe");
        fs::create_dir_all(image.parent().unwrap()).unwrap();
        fs::copy(std::env::current_exe().unwrap(), &image).unwrap();
        let guard = LaunchGuard::acquire(&root).unwrap();
        let blocked = std::process::Command::new(&image)
            .arg("--list")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        assert!(
            blocked.is_err(),
            "The fixture executable must not start while applying a mod"
        );
        drop(guard);
        let unlocked = root.join("unlocked-fixture.exe");
        fs::rename(&image, &unlocked).unwrap();
        assert!(
            std::process::Command::new(&unlocked)
                .arg("--list")
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap()
                .success()
        );
    }
}

pub fn discover(override_root: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(root) = override_root {
        return if valid_root(root) {
            fs::canonicalize(root).map_err(|e| e.to_string())
        } else {
            Err("The selected Deadlock folder does not contain a valid installation.".into())
        };
    }
    let mut libraries = Vec::new();
    for root in [
        registry_string(HKEY_CURRENT_USER, "Software\\Valve\\Steam", "SteamPath"),
        registry_string(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\WOW6432Node\\Valve\\Steam",
            "InstallPath",
        ),
    ]
    .into_iter()
    .flatten()
    {
        let root = PathBuf::from(root);
        if !libraries.contains(&root) {
            libraries.push(root.clone());
        }
        if let Ok(text) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            if let Ok(value) = keyvalues(&text) {
                if let Some(folders) = value["libraryfolders"].as_object() {
                    for (number, item) in folders {
                        if !number.chars().all(|c| c.is_ascii_digit()) {
                            continue;
                        }
                        if let Some(path) =
                            item.get("path").and_then(|v| v.as_str()).or(item.as_str())
                        {
                            let path = PathBuf::from(path);
                            if !libraries.contains(&path) {
                                libraries.push(path);
                            }
                        }
                    }
                }
            }
        }
    }
    for library in libraries {
        let manifest = library.join("steamapps/appmanifest_1422450.acf");
        if let Ok(text) = fs::read_to_string(manifest) {
            if let Ok(app) = keyvalues(&text) {
                if let Some(directory) = app["AppState"]["installdir"].as_str() {
                    let relative = Path::new(directory);
                    if relative
                        .components()
                        .any(|c| !matches!(c, std::path::Component::Normal(_)))
                    {
                        continue;
                    }
                    let root = library.join("steamapps/common").join(relative);
                    if valid_root(&root) {
                        return fs::canonicalize(root).map_err(|e| e.to_string());
                    }
                }
            }
        }
    }
    Err("Deadlock was not found in your Steam libraries.".into())
}

pub fn play() -> Result<(), String> {
    if running()? {
        return Err("Deadlock is already running.".into());
    }
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            wide("open").as_ptr(),
            wide("steam://rungameid/1422450").as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        Err("Steam could not open Deadlock.".into())
    } else {
        Ok(())
    }
}
