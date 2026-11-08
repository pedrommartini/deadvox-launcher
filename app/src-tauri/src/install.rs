use crate::{game, syntax::tokens};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const BEGIN: &str = "// Deadlock Dubbing launcher: begin";
pub const END: &str = "// Deadlock Dubbing launcher: end";
const OLD_BEGIN: &str = "// Deadlock AI Dubbing launcher: begin";
const OLD_END: &str = "// Deadlock AI Dubbing launcher: end";

pub fn stamp() -> String {
    format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id()
    )
}

pub fn sha256(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 1024 * 128];
    loop {
        let length = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if length == 0 {
            break;
        }
        digest.update(&buffer[..length]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid output folder.")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        stamp()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn has_block(text: &str) -> bool {
    text.lines()
        .any(|line| matches!(line.trim(), BEGIN | OLD_BEGIN))
}

pub fn restore(text: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut expected_end = None;
    let mut blocks = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if matches!(trimmed, BEGIN | OLD_BEGIN) {
            if expected_end.is_some() {
                return Err("The voice-language configuration has nested markers.".into());
            }
            blocks += 1;
            expected_end = Some(if trimmed == BEGIN { END } else { OLD_END });
        } else if matches!(trimmed, END | OLD_END) {
            if expected_end != Some(trimmed) {
                return Err("The voice-language configuration has incomplete markers.".into());
            }
            expected_end = None;
        } else if expected_end.is_none() {
            result.push_str(line);
        }
    }
    if expected_end.is_some() || blocks > 1 {
        return Err("The voice-language configuration has incomplete or duplicate markers.".into());
    }
    Ok(result)
}

pub fn patch(text: &str) -> Result<String, String> {
    let text = restore(text)?;
    let items = tokens(&text)?;
    let sections: Vec<_> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.value == "SearchPaths" && !item.brace)
        .collect();
    if sections.len() != 1 {
        return Err("The current Deadlock SearchPaths section could not be found.".into());
    }
    let index = sections[0].0 + 1;
    if items.get(index).is_none_or(|t| !t.brace || t.value != "{") {
        return Err("Deadlock SearchPaths is incomplete.".into());
    }
    let start = items[index].end;
    let mut depth = 1;
    let mut finish = None;
    for token in &items[index + 1..] {
        if token.brace && token.value == "{" {
            depth += 1;
        }
        if token.brace && token.value == "}" {
            depth -= 1;
        }
        if depth == 0 {
            finish = Some(token.start);
            break;
        }
    }
    let end = finish.ok_or("Deadlock SearchPaths is incomplete.")?;
    let mut position = start;
    let mut insertion = None;
    let mut core = false;
    let mut citadel = false;
    for line in text[start..end].split_inclusive('\n') {
        let mut fields = line.split_whitespace();
        if fields.next() == Some("Game") {
            // DMM can mount addons before citadel, including old voice packs.
            // Our block must precede all ordinary Game paths without reordering them.
            if insertion.is_none() {
                insertion = Some((position, &line[..line.len() - line.trim_start().len()]));
            }
            match fields.next().map(|s| s.trim_matches('"')) {
                Some("core") => core = true,
                Some("citadel") => citadel = true,
                _ => {}
            }
        }
        position += line.len();
    }
    if !core || !citadel || insertion.is_none() {
        return Err("Deadlock configuration is outdated. Open the vanilla game or verify it in Steam first.".into());
    }
    let (position, indent) = insertion.unwrap();
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines = [
        BEGIN,
        "Mod                 citadel",
        "Write               citadel",
        "Game                citadel/dubbing_ptbr",
        "Game                citadel/custom",
        END,
    ];
    let block: String = lines
        .iter()
        .map(|line| format!("{indent}{line}{newline}"))
        .collect();
    Ok(format!(
        "{}{}{}",
        &text[..position],
        block,
        &text[position..]
    ))
}

pub fn patch_mounts(text: &str, mounts: &[String]) -> Result<String, String> {
    if mounts.is_empty()
        || mounts.iter().any(|mount| {
            !mount.starts_with("citadel/dubbing_ptbr/packs/")
                || mount
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
                || !mount
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_'))
        })
    {
        return Err("Invalid voice-pack mount paths.".into());
    }
    let patched = patch(text)?;
    let newline = if patched.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut result = String::new();
    for line in patched.split_inclusive('\n') {
        if line.trim() == "Game                citadel/dubbing_ptbr" {
            let indent = &line[..line.len() - line.trim_start().len()];
            for mount in mounts {
                result.push_str(&format!("{indent}Game                {mount}{newline}"));
            }
        } else {
            result.push_str(line);
        }
    }
    Ok(result)
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct State {
    pub version: u32,
    pub package_sha256: Option<String>,
    pub game_root: Option<PathBuf>,
    pub backups: Vec<PathBuf>,
    pub mounted_packs: std::collections::BTreeMap<String, String>,
    pub voices_enabled: Option<bool>,
    // Accept obsolete local settings without ever using or writing an override.
    #[serde(rename = "update_repository", skip_serializing)]
    pub _legacy_update_repository: Option<serde_json::Value>,
}

pub fn load_state(data: &Path) -> Result<State, String> {
    let path = data.join("state.json");
    if !path.exists() {
        return Ok(State::default());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|_| "The saved launcher state is invalid.".into())
}

pub fn save_state(data: &Path, state: &State) -> Result<(), String> {
    write_atomic(
        &data.join("state.json"),
        &serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?,
    )
}

pub fn archive(game_root: &Path) -> PathBuf {
    game_root.join("game/citadel/dubbing_ptbr/pak01_dir.vpk")
}
fn inactive(game_root: &Path) -> PathBuf {
    game_root.join("game/citadel/dubbing_ptbr/deadlock_ai_dubbing_lady_geist_ptbr.disabled.vpk")
}

fn verify_owned(path: &Path, expected: &str, state: &State) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let hash = sha256(path)?;
    if hash != expected && state.package_sha256.as_deref() != Some(&hash) {
        return Err(format!(
            "{} is not a recognized voice pack. No files were replaced.",
            path.display()
        ));
    }
    Ok(Some(hash))
}

fn backup(path: &Path, data: &Path, state: &mut State) -> Result<PathBuf, String> {
    let directory = data.join("backups");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let target = directory.join(format!(
        "{}-{}",
        stamp(),
        path.file_name().unwrap().to_string_lossy()
    ));
    fs::copy(path, &target).map_err(|e| e.to_string())?;
    if sha256(path)? != sha256(&target)? {
        return Err("Backup integrity check failed.".into());
    }
    state.backups.push(target.clone());
    Ok(target)
}

#[cfg(test)]
pub fn enable(
    game_root: &Path,
    source: &Path,
    expected: &str,
    bytes: u64,
    data: &Path,
) -> Result<(), String> {
    enable_with_progress(
        game_root,
        source,
        expected,
        bytes,
        data,
        &crate::progress::quiet,
    )
}

#[cfg(test)]
pub fn enable_with_progress(
    game_root: &Path,
    source: &Path,
    expected: &str,
    bytes: u64,
    data: &Path,
    report: &crate::progress::Reporter,
) -> Result<(), String> {
    let _launch_guard = game::LaunchGuard::acquire(game_root)?;
    report("Checking voice pack", Some(0.0));
    if fs::metadata(source)
        .map_err(|_| "The PT-BR voice pack is not installed beside the launcher.")?
        .len()
        != bytes
        || sha256(source)? != expected
    {
        return Err(
            "The voice pack failed its integrity check. Install the complete package again.".into(),
        );
    }
    report("Backing up game settings", Some(10.0));
    let mut state = load_state(data)?;
    let active = archive(game_root);
    let disabled = inactive(game_root);
    let old_active = verify_owned(&active, expected, &state)?;
    let old_disabled = verify_owned(&disabled, expected, &state)?;
    let info = game_root.join("game/citadel/gameinfo.gi");
    let text = fs::read_to_string(&info).map_err(|e| e.to_string())?;
    let patched = patch(&text)?;
    backup(&info, data, &mut state)?;
    let old_backup = if old_active.is_some() && old_active.as_deref() != Some(expected) {
        Some(backup(&active, data, &mut state)?)
    } else {
        None
    };
    let disabled_backup = if old_disabled.is_some() && old_disabled.as_deref() != Some(expected) {
        Some(backup(&disabled, data, &mut state)?)
    } else {
        None
    };
    let reused_disabled = old_active.is_none() && old_disabled.as_deref() == Some(expected);
    if reused_disabled {
        fs::rename(&disabled, &active).map_err(|e| e.to_string())?;
    } else if old_active.as_deref() != Some(expected) {
        fs::create_dir_all(active.parent().unwrap()).map_err(|e| e.to_string())?;
        let staging = active.with_file_name(format!(".pack-{}.installing", stamp()));
        let staged = (|| {
            let mut input = File::open(source).map_err(|e| e.to_string())?;
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&staging)
                .map_err(|e| e.to_string())?;
            let mut done = 0u64;
            let mut buffer = [0u8; 256 * 1024];
            loop {
                let length = input.read(&mut buffer).map_err(|e| e.to_string())?;
                if length == 0 {
                    break;
                }
                output
                    .write_all(&buffer[..length])
                    .map_err(|e| e.to_string())?;
                done += length as u64;
                report(
                    "Applying PT-BR",
                    Some(10.0 + 75.0 * done as f64 / bytes as f64),
                );
            }
            output.sync_all().map_err(|e| e.to_string())?;
            drop(output);
            report("Verifying installed voices", Some(86.0));
            if sha256(&staging)? != expected {
                return Err("The installed copy failed its integrity check.".into());
            }
            game::require_closed()?;
            fs::rename(&staging, &active).map_err(|e| e.to_string())
        })();
        if staged.is_err() {
            let _ = fs::remove_file(&staging);
        }
        staged?;
    }
    report("Saving voice language", Some(93.0));
    if let Err(error) = write_atomic(&info, patched.as_bytes()) {
        if let Some(previous) = &old_backup {
            fs::copy(previous, &active)
                .map_err(|e| format!("{error}; restoring the previous package failed: {e}"))?;
        } else if reused_disabled {
            fs::rename(&active, &disabled)
                .map_err(|e| format!("{error}; reverting package activation failed: {e}"))?;
        } else if old_active.is_none() {
            fs::remove_file(&active)
                .map_err(|e| format!("{error}; reverting package activation failed: {e}"))?;
        }
        return Err(error);
    }
    if disabled.exists() {
        fs::remove_file(&disabled).map_err(|e| e.to_string())?;
    }
    state.version = 1;
    state.package_sha256 = Some(expected.into());
    state.voices_enabled = Some(true);
    state.game_root = Some(game_root.into());
    if let Err(error) = save_state(data, &state) {
        write_atomic(&info, text.as_bytes())
            .map_err(|e| format!("{error}; restoring game settings failed: {e}"))?;
        if let Some(previous) = old_backup {
            fs::copy(previous, &active)
                .map_err(|e| format!("{error}; restoring the previous package failed: {e}"))?;
        } else if reused_disabled {
            fs::rename(&active, &disabled)
                .map_err(|e| format!("{error}; reverting package activation failed: {e}"))?;
        } else if old_active.is_none() {
            fs::remove_file(&active)
                .map_err(|e| format!("{error}; reverting package installation failed: {e}"))?;
        }
        if let Some(previous) = disabled_backup {
            fs::copy(previous, &disabled)
                .map_err(|e| format!("{error}; restoring inactive voices failed: {e}"))?;
        } else if old_disabled.as_deref() == Some(expected) && !disabled.exists() {
            fs::copy(source, &disabled)
                .map_err(|e| format!("{error}; restoring inactive voices failed: {e}"))?;
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
pub fn disable(game_root: &Path, expected: &str, data: &Path) -> Result<(), String> {
    disable_with_progress(game_root, expected, data, &crate::progress::quiet)
}

pub fn disable_with_progress(
    game_root: &Path,
    expected: &str,
    data: &Path,
    report: &crate::progress::Reporter,
) -> Result<(), String> {
    let _launch_guard = game::LaunchGuard::acquire(game_root)?;
    report("Checking original voices", Some(5.0));
    let mut state = load_state(data)?;
    let active = archive(game_root);
    let disabled = inactive(game_root);
    let active_hash = verify_owned(&active, expected, &state)?;
    let disabled_hash = verify_owned(&disabled, expected, &state)?;
    if active_hash.is_some() && disabled_hash.is_some() && active_hash != disabled_hash {
        return Err(
            "A different disabled voice pack already exists. No files were changed.".into(),
        );
    }
    let info = game_root.join("game/citadel/gameinfo.gi");
    let text = fs::read_to_string(&info).map_err(|e| e.to_string())?;
    let restored = restore(&text)?;
    if has_block(&text) {
        backup(&info, data, &mut state)?;
    }
    report("Restoring original voices", Some(60.0));
    if active_hash.is_some() {
        fs::rename(&active, &disabled).map_err(|e| e.to_string())?;
    }
    if let Err(error) = write_atomic(&info, restored.as_bytes()) {
        if active_hash.is_some() {
            fs::rename(&disabled, &active)
                .map_err(|e| format!("{error}; reverting voice selection failed: {e}"))?;
        }
        return Err(error);
    }
    state.version = 1;
    state.package_sha256 = active_hash
        .clone()
        .or(disabled_hash.clone())
        .or(state.package_sha256);
    state.voices_enabled = Some(false);
    state.game_root = Some(game_root.into());
    report("Saving voice language", Some(95.0));
    if let Err(error) = save_state(data, &state) {
        write_atomic(&info, text.as_bytes())
            .map_err(|e| format!("{error}; restoring game settings failed: {e}"))?;
        if active_hash.is_some() {
            fs::rename(&disabled, &active)
                .map_err(|e| format!("{error}; restoring active voices failed: {e}"))?;
            if disabled_hash.is_some() {
                fs::copy(&active, &disabled).map_err(|e| e.to_string())?;
            }
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vanilla() -> &'static str {
        "GameInfo\n{\n FileSystem {\n SearchPaths\n {\n  Game_UILanguage citadel_*LANGUAGE*\n  Game citadel\n  Game core\n }\n }\n}\n"
    }
    #[test]
    fn patch_is_reversible_and_keeps_valve_paths() {
        for text in [vanilla().to_owned(), vanilla().replace('\n', "\r\n")] {
            let patched = patch(&text).unwrap();
            assert_eq!(patch(&patched).unwrap(), patched);
            assert_eq!(restore(&patched).unwrap(), text);
            assert!(patched.contains("Game core"));
            assert!(
                patched.find("citadel/dubbing_ptbr").unwrap()
                    < patched.find("citadel/custom").unwrap()
            );
        }
    }
    #[test]
    fn dmm_addons_follow_voice_mounts_without_changing_other_paths() {
        let dmm = vanilla().replace(
            "  Game citadel\n",
            "  // Deadlock Mod Manager - Start\n  Game \"citadel/addons\"\n  Mod citadel\n  Write citadel\n  Game citadel\n",
        );
        for original in [dmm.clone(), dmm.replace('\n', "\r\n")] {
            let mounts = vec!["citadel/dubbing_ptbr/packs/test/abcd".into()];
            for patched in [
                patch(&original).unwrap(),
                patch_mounts(&original, &mounts).unwrap(),
            ] {
                assert!(
                    patched.find("citadel/dubbing_ptbr").unwrap()
                        < patched.find("citadel/addons").unwrap()
                );
                assert_eq!(restore(&patched).unwrap(), original);
            }
            let patched = patch_mounts(&original, &mounts).unwrap();
            assert_eq!(patch_mounts(&patched, &mounts).unwrap(), patched);
        }
        assert!(patch(&dmm.replace("  Game citadel\n", "")).is_err());
    }
    #[test]
    fn legacy_block_is_adopted_without_duplicating_mounts() {
        let legacy = patch(vanilla())
            .unwrap()
            .replace(BEGIN, OLD_BEGIN)
            .replace(END, OLD_END);
        assert_eq!(restore(&legacy).unwrap(), vanilla());
        assert_eq!(patch(&legacy).unwrap(), patch(vanilla()).unwrap());
    }
    #[test]
    fn malformed_config_is_rejected() {
        assert!(patch(&vanilla().replace("Game core", "")).is_err());
        assert!(restore(&format!("{BEGIN}\nno end")).is_err());
        assert!(restore(&format!("{BEGIN}\n{END}\n{BEGIN}\n{END}\n")).is_err());
    }
    #[test]
    fn install_toggle_and_foreign_collision_protection() {
        let base = std::env::temp_dir().join(format!("deadlock-launcher-test-{}", stamp()));
        let root = base.join("game");
        let data = base.join("user-data");
        let info = root.join("game/citadel/gameinfo.gi");
        write_atomic(&info, vanilla().as_bytes()).unwrap();
        let source = base.join("source.vpk");
        write_atomic(&source, b"pack-content").unwrap();
        let hash = sha256(&source).unwrap();
        enable(&root, &source, &hash, 12, &data).unwrap();
        assert_eq!(sha256(&archive(&root)).unwrap(), hash);
        disable(&root, &hash, &data).unwrap();
        assert_eq!(fs::read_to_string(&info).unwrap(), vanilla());
        assert!(!archive(&root).exists());
        enable(&root, &source, &hash, 12, &data).unwrap();
        assert!(
            load_state(&data)
                .unwrap()
                .backups
                .iter()
                .all(|p| p.extension().is_some_and(|e| e == "gi"))
        );
        write_atomic(&inactive(&root), b"foreign-mod").unwrap();
        let before = fs::read(&info).unwrap();
        assert!(disable(&root, &hash, &data).is_err());
        assert_eq!(fs::read(&info).unwrap(), before);
        assert!(enable(&root, &source, &hash, 12, &data).is_err());
    }

    #[test]
    fn failed_configuration_write_restores_disabled_package() {
        let base = std::env::temp_dir().join(format!("deadlock-launcher-rollback-{}", stamp()));
        let root = base.join("game");
        let data = base.join("user-data");
        let info = root.join("game/citadel/gameinfo.gi");
        write_atomic(&info, vanilla().as_bytes()).unwrap();
        let source = base.join("source.vpk");
        write_atomic(&source, b"pack-content").unwrap();
        let hash = sha256(&source).unwrap();
        enable(&root, &source, &hash, 12, &data).unwrap();
        disable(&root, &hash, &data).unwrap();
        let original_permissions = fs::metadata(&info).unwrap().permissions();
        let mut permissions = original_permissions.clone();
        permissions.set_readonly(true);
        fs::set_permissions(&info, permissions).unwrap();
        assert!(enable(&root, &source, &hash, 12, &data).is_err());
        assert!(!archive(&root).exists());
        assert_eq!(sha256(&inactive(&root)).unwrap(), hash);
        assert_eq!(fs::read_to_string(&info).unwrap(), vanilla());
        fs::set_permissions(&info, original_permissions).unwrap();
    }

    #[test]
    fn failed_state_write_rolls_back_language_and_pack_updates() {
        let root = std::env::temp_dir().join(format!("deadlock-state-rollback-{}", stamp()));
        let data = root.join("user-data");
        let info = root.join("game/citadel/gameinfo.gi");
        write_atomic(&info, vanilla().as_bytes()).unwrap();
        let source = root.join("source.vpk");
        write_atomic(&source, b"old-pack").unwrap();
        let old_hash = sha256(&source).unwrap();
        enable(&root, &source, &old_hash, 8, &data).unwrap();
        let before = fs::read(&info).unwrap();
        let state_file = data.join("state.json");
        let original_permissions = fs::metadata(&state_file).unwrap().permissions();
        let mut protected = original_permissions.clone();
        protected.set_readonly(true);
        fs::set_permissions(&state_file, protected).unwrap();
        assert!(disable(&root, &old_hash, &data).is_err());
        assert_eq!(fs::read(&info).unwrap(), before);
        assert_eq!(sha256(&archive(&root)).unwrap(), old_hash);
        write_atomic(&source, b"new-pack").unwrap();
        let new_hash = sha256(&source).unwrap();
        assert!(enable(&root, &source, &new_hash, 8, &data).is_err());
        assert_eq!(sha256(&archive(&root)).unwrap(), old_hash);
        assert_eq!(fs::read(&info).unwrap(), before);
        fs::set_permissions(&state_file, original_permissions).unwrap();
    }
}
