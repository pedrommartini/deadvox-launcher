//! Immutable per-character mounts; commit the entire catalog in one transaction.
use crate::{
    engine::{Pack, PackCatalog, validate_catalog},
    game, install,
    progress::Reporter,
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

type DigestCache = HashMap<PathBuf, (u64, SystemTime, String)>;
static STATUS_DIGESTS: OnceLock<Mutex<DigestCache>> = OnceLock::new();

// Read-only snapshots may reuse a digest until the file changes. Downloads and
// installation always use the uncached integrity checks below.
pub fn status_hash(path: &Path) -> Result<String, String> {
    let before = fs::metadata(path).map_err(|e| e.to_string())?;
    let modified = before.modified().map_err(|e| e.to_string())?;
    let cache = STATUS_DIGESTS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(entries) = cache.lock() {
        if let Some((length, time, digest)) = entries.get(path) {
            if *length == before.len() && *time == modified {
                return Ok(digest.clone());
            }
        }
    }
    let digest = install::sha256(path)?;
    let after = fs::metadata(path).map_err(|e| e.to_string())?;
    if after.len() != before.len() || after.modified().map_err(|e| e.to_string())? != modified {
        return Err("Voice file changed during verification.".into());
    }
    if let Ok(mut entries) = cache.lock() {
        if entries.len() > 4096 {
            entries.clear();
        }
        entries.insert(path.into(), (before.len(), modified, digest.clone()));
    }
    Ok(digest)
}

pub fn status_file(path: &Path, pack: &Pack) -> bool {
    fs::metadata(path).is_ok_and(|m| m.len() == pack.bytes)
        && status_hash(path).is_ok_and(|hash| hash == pack.sha256)
}

fn mount(id: &str, hash: &str) -> String {
    format!("citadel/dubbing_ptbr/packs/{id}/{hash}")
}

pub fn archive(game_root: &Path, id: &str, hash: &str) -> PathBuf {
    game_root
        .join("game")
        .join(mount(id, hash))
        .join("pak01_dir.vpk")
}

pub fn valid_file(path: &Path, pack: &Pack) -> bool {
    fs::metadata(path).is_ok_and(|m| m.len() == pack.bytes)
        && install::sha256(path).is_ok_and(|hash| hash == pack.sha256)
}

pub fn mounted(game_root: &Path, state: &install::State, pack: &Pack) -> bool {
    state.mounted_packs.get(&pack.id).is_some_and(|hash| {
        status_hash(&archive(game_root, &pack.id, hash)).is_ok_and(|actual| actual == *hash)
    })
}

pub fn enable(
    root: &Path,
    game_root: &Path,
    data: &Path,
    catalog: &PackCatalog,
    report: &Reporter<'_>,
) -> Result<(), String> {
    validate_catalog(catalog)?;
    let _guard = game::LaunchGuard::acquire(game_root)?;
    let mut state = install::load_state(data)?;
    let info = game_root.join("game/citadel/gameinfo.gi");
    let original = fs::read_to_string(&info).map_err(|e| e.to_string())?;
    if install::has_block(&original)
        && state.mounted_packs.is_empty()
        && install::archive(game_root).exists()
    {
        let legacy = install::sha256(&install::archive(game_root))?;
        if state.package_sha256.as_deref() != Some(&legacy)
            && !catalog.packs.iter().any(|p| p.sha256 == legacy)
        {
            return Err("The existing mounted file belongs to an unrecognized mod. No settings were changed.".into());
        }
    }
    let mounts: Vec<_> = catalog
        .packs
        .iter()
        .map(|p| mount(&p.id, &p.sha256))
        .collect();
    let patched = install::patch_mounts(&original, &mounts)?;
    // Validate all sources before copying any content or changing game settings.
    for pack in &catalog.packs {
        if !valid_file(&root.join(&pack.file), pack) {
            return Err(
                "A voice pack is missing or corrupt. Download the complete release first.".into(),
            );
        }
    }
    for (index, pack) in catalog.packs.iter().enumerate() {
        report(
            "Applying character voices",
            Some(85.0 * index as f64 / catalog.packs.len() as f64),
        );
        let target = archive(game_root, &pack.id, &pack.sha256);
        if target.exists() {
            if !valid_file(&target, pack) {
                return Err(
                    "An unrecognized file occupies a voice-pack destination. Nothing was replaced."
                        .into(),
                );
            }
        } else {
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            let stage = target.with_file_name(format!(".{}.installing", install::stamp()));
            let copy = (|| {
                fs::copy(root.join(&pack.file), &stage).map_err(|e| e.to_string())?;
                if !valid_file(&stage, pack) {
                    return Err("Installed voice integrity check failed.".into());
                }
                fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&stage)
                    .and_then(|f| f.sync_all())
                    .map_err(|e| e.to_string())?;
                fs::rename(&stage, &target).map_err(|e| e.to_string())
            })();
            if copy.is_err() {
                let _ = fs::remove_file(&stage);
            }
            copy?;
        }
    }
    let backup = data
        .join("backups")
        .join(format!("{}-gameinfo.gi", install::stamp()));
    install::write_atomic(&backup, original.as_bytes())?;
    state.backups.push(backup);
    state.mounted_packs = catalog
        .packs
        .iter()
        .map(|p| (p.id.clone(), p.sha256.clone()))
        .collect();
    state.game_root = Some(game_root.into());
    state.version = 1;
    state.voices_enabled = Some(true);
    report("Saving voice language", Some(95.0));
    install::write_atomic(&info, patched.as_bytes())?;
    if let Err(error) = install::save_state(data, &state) {
        install::write_atomic(&info, original.as_bytes())
            .map_err(|e| format!("{error}; restoring game settings failed: {e}"))?;
        return Err(error);
    }
    Ok(())
}

pub fn disable(
    game_root: &Path,
    data: &Path,
    catalog: &PackCatalog,
    report: &Reporter<'_>,
) -> Result<(), String> {
    let mut state = install::load_state(data)?;
    if state.mounted_packs.is_empty() {
        return install::disable_with_progress(game_root, &catalog.packs[0].sha256, data, report);
    }
    let _guard = game::LaunchGuard::acquire(game_root)?;
    for (id, hash) in &state.mounted_packs {
        if install::sha256(&archive(game_root, id, hash))? != *hash {
            return Err("An active voice pack is not recognized. No files were changed.".into());
        }
    }
    let info = game_root.join("game/citadel/gameinfo.gi");
    let text = fs::read_to_string(&info).map_err(|e| e.to_string())?;
    report("Restoring original voices", Some(90.0));
    // Immutable packages are retained. Removing the owned mount block disables all of them.
    install::write_atomic(&info, install::restore(&text)?.as_bytes())?;
    state.voices_enabled = Some(false);
    if let Err(error) = install::save_state(data, &state) {
        install::write_atomic(&info, text.as_bytes())
            .map_err(|e| format!("{error}; restoring game settings failed: {e}"))?;
        return Err(error);
    }
    Ok(())
}

pub fn remove(root: &Path, game_root: &Path, data: &Path, catalog: &PackCatalog, report: &Reporter<'_>) -> Result<(), String> {
    validate_catalog(catalog)?;
    let state = install::load_state(data)?;
    for (id, hash) in &state.mounted_packs {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            || hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid stored voice package identity.".into());
        }
    }
    let game_info = fs::read_to_string(game_root.join("game/citadel/gameinfo.gi")).map_err(|e| e.to_string())?;
    if install::has_block(&game_info) {
        disable(game_root, data, catalog, report)?;
    }
    let _guard = game::LaunchGuard::acquire(game_root)?;
    for pack in &catalog.packs {
        let file = root.join(&pack.file);
        if file.exists() {
            if !valid_file(&file, pack) { return Err("A voice file is unrecognized and was preserved.".into()); }
            fs::remove_file(file).map_err(|e| e.to_string())?;
        }
    }
    for (id, hash) in &state.mounted_packs {
        let file = archive(game_root, id, hash);
        if file.exists() {
            if install::sha256(&file)? != *hash { return Err("A mounted voice file is unrecognized and was preserved.".into()); }
            fs::remove_file(file).map_err(|e| e.to_string())?;
        }
    }
    let mut updated = install::load_state(data)?;
    updated.mounted_packs.clear();
    updated.voices_enabled = Some(false);
    install::save_state(data, &updated)
}

/// Restore the selected PT-BR mounts after DMM/Steam edits, before launching.
/// Read-only status never repairs files; an explicit Original selection wins.
pub fn ensure_priority(
    root: &Path,
    game_root: &Path,
    data: &Path,
    catalog: &PackCatalog,
    report: &Reporter<'_>,
) -> Result<(), String> {
    let mut state = install::load_state(data)?;
    let info = fs::read_to_string(game_root.join("game/citadel/gameinfo.gi"))
        .map_err(|e| e.to_string())?;
    if !state
        .voices_enabled
        .unwrap_or_else(|| install::has_block(&info))
    {
        return Ok(());
    }
    let mounts: Vec<_> = catalog
        .packs
        .iter()
        .map(|p| mount(&p.id, &p.sha256))
        .collect();
    if install::patch_mounts(&info, &mounts)? != info {
        report("Restoring voice priority", None);
        enable(root, game_root, data, catalog, report)?;
    } else if state.voices_enabled.is_none() {
        // Adopt a currently active installation from earlier launcher versions.
        state.voices_enabled = Some(true);
        install::save_state(data, &state)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remove_restores_original_and_deletes_known_packages() {
        let (root, game_root, catalog) = setup();
        let data = root.join("user-data");
        let info = game_root.join("game/citadel/gameinfo.gi");
        let original = fs::read(&info).unwrap();
        enable(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        remove(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read(info).unwrap(), original);
        for pack in &catalog.packs {
            assert!(!root.join(&pack.file).exists());
            assert!(!archive(&game_root, &pack.id, &pack.sha256).exists());
        }
        assert!(install::load_state(&data).unwrap().mounted_packs.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn status_cache_invalidates_changed_files_and_operations_still_verify_uncached() {
        let (root, _, catalog) = setup();
        let pack = &catalog.packs[0];
        let file = root.join(&pack.file);
        assert!(status_file(&file, pack));
        assert!(status_file(&file, pack));
        install::write_atomic(&file, b"corrupt-character-content").unwrap();
        assert!(!status_file(&file, pack));
        assert!(!valid_file(&file, pack));
        fs::remove_dir_all(root).unwrap();
    }
    fn setup() -> (PathBuf, PathBuf, PackCatalog) {
        let root = std::env::temp_dir().join(format!("deadvox-packs-{}", install::stamp()));
        let game_root = root.join("deadlock");
        install::write_atomic(
            &game_root.join("game/citadel/gameinfo.gi"),
            b"GameInfo\n{ FileSystem { SearchPaths\n{\n Game citadel\n Game core\n}\n}}\n",
        )
        .unwrap();
        let mut catalog = crate::engine::catalog().unwrap();
        catalog.packs.truncate(1);
        let mut other = catalog.packs[0].clone();
        other.id = "pt-br-abrams".into();
        other.file = "voice-packs/pt-BR/abrams_dir.vpk".into();
        other.heroes[0].asset_id = "atlas".into();
        catalog.packs.push(other);
        for (i, pack) in catalog.packs.iter_mut().enumerate() {
            let contents = format!("test-character-{i}");
            install::write_atomic(&root.join(&pack.file), contents.as_bytes()).unwrap();
            pack.bytes = contents.len() as u64;
            pack.sha256 = install::sha256(&root.join(&pack.file)).unwrap();
        }
        (root, game_root, catalog)
    }
    #[test]
    fn two_characters_update_independently_and_restore_exact_original() {
        let (root, game_root, mut catalog) = setup();
        let data = root.join("user-data");
        let info = game_root.join("game/citadel/gameinfo.gi");
        let original = fs::read(&info).unwrap();
        enable(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        let old = catalog.packs[0].clone();
        let untouched = archive(&game_root, &catalog.packs[1].id, &catalog.packs[1].sha256);
        let contents = fs::read(&untouched).unwrap();
        let updated = &mut catalog.packs[0];
        install::write_atomic(&root.join(&updated.file), b"new-character").unwrap();
        updated.sha256 = install::sha256(&root.join(&updated.file)).unwrap();
        updated.bytes = 13;
        enable(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read(untouched).unwrap(), contents);
        assert!(archive(&game_root, &old.id, &old.sha256).exists());
        let patched = fs::read_to_string(&info).unwrap();
        assert_eq!(
            patched
                .matches("Game                citadel/dubbing_ptbr/packs/")
                .count(),
            2
        );
        disable(&game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read(info).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn selected_pt_br_recovers_after_dmm_removes_or_demotes_mounts() {
        let (root, game_root, catalog) = setup();
        let data = root.join("user-data");
        let info = game_root.join("game/citadel/gameinfo.gi");
        enable(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(
            install::load_state(&data).unwrap().voices_enabled,
            Some(true)
        );
        let original = install::restore(&fs::read_to_string(&info).unwrap()).unwrap();
        let dmm = original.replace(
            " Game citadel\n",
            " Game citadel/addons\n Mod citadel\n Write citadel\n Game citadel\n",
        );
        // Steam/DMM can remove the mount block entirely.
        install::write_atomic(&info, dmm.as_bytes()).unwrap();
        ensure_priority(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        let repaired = fs::read_to_string(&info).unwrap();
        assert!(
            repaired.find("citadel/dubbing_ptbr/packs/").unwrap()
                < repaired.find("citadel/addons").unwrap()
        );
        assert_eq!(install::restore(&repaired).unwrap(), dmm);
        // DMM can instead insert its addons ahead of an existing block.
        let demoted = repaired.replace(" Game citadel/addons\n", "").replace(
            &format!(" {}", install::BEGIN),
            &format!(" Game citadel/addons\n {}", install::BEGIN),
        );
        install::write_atomic(&info, demoted.as_bytes()).unwrap();
        ensure_priority(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read_to_string(&info).unwrap(), repaired);
        let state_before = fs::read(data.join("state.json")).unwrap();
        ensure_priority(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read(data.join("state.json")).unwrap(), state_before);
        // Choosing Original must persist even though installed packs are retained.
        disable(&game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(
            install::load_state(&data).unwrap().voices_enabled,
            Some(false)
        );
        ensure_priority(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read_to_string(&info).unwrap(), dmm);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn old_active_state_is_adopted_but_unselected_installations_stay_original() {
        let (root, game_root, catalog) = setup();
        let data = root.join("user-data");
        let info = game_root.join("game/citadel/gameinfo.gi");
        let original = fs::read(&info).unwrap();
        ensure_priority(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read(&info).unwrap(), original);
        assert!(!data.join("state.json").exists());
        enable(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        let mut state = install::load_state(&data).unwrap();
        state.voices_enabled = None;
        install::save_state(&data, &state).unwrap();
        let active = fs::read(&info).unwrap();
        ensure_priority(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read(&info).unwrap(), active);
        assert_eq!(
            install::load_state(&data).unwrap().voices_enabled,
            Some(true)
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_original_preference_save_restores_pt_br_configuration() {
        let (root, game_root, catalog) = setup();
        let data = root.join("user-data");
        enable(&root, &game_root, &data, &catalog, &crate::progress::quiet).unwrap();
        let info = game_root.join("game/citadel/gameinfo.gi");
        let active = fs::read(&info).unwrap();
        let file = data.join("state.json");
        let permissions = fs::metadata(&file).unwrap().permissions();
        let mut readonly = permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&file, readonly).unwrap();
        assert!(disable(&game_root, &data, &catalog, &crate::progress::quiet).is_err());
        assert_eq!(fs::read(&info).unwrap(), active);
        assert_eq!(
            install::load_state(&data).unwrap().voices_enabled,
            Some(true)
        );
        fs::set_permissions(&file, permissions).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_second_character_and_failed_state_commit_leave_previous_mounts() {
        let (root, game_root, catalog) = setup();
        let data = root.join("user-data");
        let info = game_root.join("game/citadel/gameinfo.gi");
        let original = fs::read(&info).unwrap();
        let file = root.join(&catalog.packs[1].file);
        let good = fs::read(&file).unwrap();
        install::write_atomic(&file, b"corrupt").unwrap();
        assert!(enable(&root, &game_root, &data, &catalog, &crate::progress::quiet).is_err());
        assert_eq!(fs::read(&info).unwrap(), original);
        install::write_atomic(&file, &good).unwrap();
        install::write_atomic(&data.join("state.json"), b"{}").unwrap();
        let state_file = data.join("state.json");
        let permissions = fs::metadata(&state_file).unwrap().permissions();
        let mut protected = permissions.clone();
        protected.set_readonly(true);
        fs::set_permissions(&state_file, protected).unwrap();
        assert!(enable(&root, &game_root, &data, &catalog, &crate::progress::quiet).is_err());
        assert_eq!(fs::read(&info).unwrap(), original);
        fs::set_permissions(state_file, permissions).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
