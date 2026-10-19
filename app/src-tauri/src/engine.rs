use crate::{game, install, packs, progress::Reporter, updates};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackCatalog {
    pub schema_version: u32,
    pub packs: Vec<Pack>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub id: String,
    pub language: String,
    pub version: String,
    pub file: String,
    pub sha256: String,
    pub bytes: u64,
    pub entry_count: usize,
    pub heroes: Vec<Coverage>,
    pub download_url: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub asset_id: String,
    pub total: usize,
    pub available: usize,
}
#[derive(Deserialize)]
struct Roster {
    heroes: Vec<RosterHero>,
    source: String,
}
#[derive(Deserialize)]
struct RosterHero {
    id: u32,
    name: String,
    asset_id: String,
    portrait: String,
}
#[derive(Serialize)]
pub struct Hero {
    id: u32,
    name: String,
    asset_id: String,
    portrait: String,
    total: usize,
    available: usize,
    content_available: usize,
    state: String,
    coverage: Option<f64>,
}
#[derive(Serialize)]
pub struct Snapshot {
    pub language: String,
    pub game_running: bool,
    pub game_root: Option<String>,
    pub game_error: Option<String>,
    pub pack_available: bool,
    pub pack_version: String,
    pub pack_download_available: bool,
    pub voices_pending_application: bool,
    pub update_repository: Option<String>,
    pub update_error: Option<String>,
    pub client_update: Option<updates::Summary>,
    pub voice_update: Option<updates::Summary>,
    pub heroes: Vec<Hero>,
    updated_at: String,
    version: String,
    portraits_source: String,
}

pub fn catalog() -> Result<PackCatalog, String> {
    let catalog: PackCatalog = serde_json::from_str(include_str!("../resources/voice-packs.json"))
        .map_err(|e| e.to_string())?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

pub fn validate_catalog(catalog: &PackCatalog) -> Result<(), String> {
    if catalog.schema_version != 1 || catalog.packs.is_empty() || catalog.packs.len() > 512 {
        return Err("Unsupported voice-pack catalog.".into());
    }
    let mut ids = std::collections::HashSet::new();
    let mut files = std::collections::HashSet::new();
    let mut heroes = std::collections::HashSet::new();
    for pack in &catalog.packs {
        let relative = Path::new(&pack.file);
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            || !pack.file.starts_with("voice-packs/pt-BR/")
            || !pack.file.ends_with(".vpk")
            || pack.sha256.len() != 64
            || !pack
                .sha256
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
            || pack.bytes == 0
            || pack.bytes >= 2 * 1024 * 1024 * 1024
            || pack.language != "pt-BR"
            || pack.id.is_empty()
            || pack.id.len() > 100
            || !pack
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            || !ids.insert(&pack.id)
            || !files.insert(pack.file.to_ascii_lowercase())
            || pack.file.contains('\\')
            || pack.file.contains(':')
            || pack
                .file
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
            || pack.version.is_empty()
            || pack.entry_count == 0
            || pack.heroes.is_empty()
        {
            return Err("Invalid voice-pack catalog.".into());
        }
        if semver::Version::parse(&pack.version).is_err()
            || pack.heroes.iter().any(|hero| {
                hero.available > hero.total
                    || hero.asset_id.is_empty()
                    || !hero
                        .asset_id
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                    || !heroes.insert(&hero.asset_id)
            })
            || pack
                .heroes
                .iter()
                .try_fold(0usize, |sum, h| sum.checked_add(h.available))
                .is_none_or(|sum| sum > pack.entry_count)
        {
            return Err("Invalid voice-pack coverage.".into());
        }
        if pack
            .download_url
            .as_deref()
            .is_some_and(|url| !url.starts_with("https://"))
        {
            return Err("Voice-pack download URLs must use HTTPS.".into());
        }
    }
    Ok(())
}

pub fn catalog_version(catalog: &PackCatalog) -> String {
    catalog
        .packs
        .iter()
        .filter_map(|p| semver::Version::parse(&p.version).ok())
        .max()
        .map_or_else(|| "Unknown".into(), |v| v.to_string())
}

pub fn installed_catalog(data: &Path) -> Result<PackCatalog, String> {
    let path = data.join("voice-catalog.json");
    if !path.exists() {
        return catalog();
    }
    let value: PackCatalog = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|_| "The installed voice catalog is invalid.".to_string())?;
    validate_catalog(&value)?;
    Ok(value)
}

pub fn executable_root() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .map(Path::to_path_buf)
        .ok_or("Could not locate the launcher folder.".into())
}

pub fn action_with_progress(
    action: &str,
    root: &Path,
    data: &Path,
    game_path: Option<&str>,
    report: &Reporter<'_>,
) -> Result<Snapshot, String> {
    if !matches!(
        action,
        "status"
            | "enable"
            | "disable"
            | "play"
            | "set_game_folder"
            | "auto_game_folder"
            | "check_updates"
            | "download_voices"
            | "update_client"
    ) {
        return Err("Unsupported launcher action.".into());
    }
    report("Locating Deadlock", Some(0.0));
    if matches!(action, "set_game_folder" | "auto_game_folder") {
        let mut state = install::load_state(data)?;
        match action {
            "set_game_folder" => {
                let selected = if let Some(value) = game_path {
                    Some(PathBuf::from(value))
                } else {
                    game::pick_folder()?
                };
                if let Some(path) = selected {
                    state.game_root = Some(game::normalize_root(&path)?);
                }
            }
            "auto_game_folder" => state.game_root = None,
            _ => {}
        }
        install::save_state(data, &state)?;
    }
    if action == "check_updates" {
        updates::check(data, report)?;
    }
    if action == "download_voices" {
        updates::voices(root, data, report)?;
    }
    if action == "update_client" {
        updates::stage_client(root, data, report)?;
    }
    let catalog = installed_catalog(data)?;
    let pack = &catalog.packs[0];
    let state = install::load_state(data)?;
    let detected = game::discover(state.game_root.as_deref());
    let was_mounted = detected.as_ref().is_ok_and(|game_root| {
        fs::read_to_string(game_root.join("game/citadel/gameinfo.gi"))
            .is_ok_and(|text| install::has_block(&text))
    });
    if matches!(action, "enable" | "disable" | "play")
        || (action == "download_voices" && was_mounted && !game::running()?)
    {
        let game_root = detected.as_ref().map_err(Clone::clone)?;
        match action {
            "enable" | "download_voices" => packs::enable(root, game_root, data, &catalog, report)?,
            "disable" => packs::disable(game_root, data, &catalog, report)?,
            "play" => {
                report("Starting Deadlock", None);
                game::play()?;
            }
            _ => unreachable!(),
        }
    }
    let mut language = "en";
    let mut mounted = false;
    let mut voices_pending_application = false;
    let state = install::load_state(data)?;
    report("Reading voice settings", Some(97.0));
    if let Ok(game_root) = &detected {
        let info = fs::read_to_string(game_root.join("game/citadel/gameinfo.gi"))
            .map_err(|e| e.to_string())?;
        if state.mounted_packs.is_empty() {
            let installed_hash = packs::status_hash(&install::archive(game_root)).ok();
            mounted = install::has_block(&info)
                && installed_hash.as_ref().is_some_and(|hash| {
                    *hash == pack.sha256 || state.package_sha256.as_ref() == Some(hash)
                });
            voices_pending_application = mounted
                && (catalog.packs.len() > 1
                    || installed_hash.as_deref() != Some(pack.sha256.as_str()));
        } else {
            mounted = install::has_block(&info)
                && state.mounted_packs.iter().all(|(id, hash)| {
                    info.contains(&format!("citadel/dubbing_ptbr/packs/{id}/{hash}"))
                        && packs::status_hash(&packs::archive(game_root, id, hash))
                            .is_ok_and(|actual| actual == *hash)
                });
            voices_pending_application = mounted
                && (state.mounted_packs.len() != catalog.packs.len()
                    || catalog
                        .packs
                        .iter()
                        .any(|p| state.mounted_packs.get(&p.id) != Some(&p.sha256)));
        }
        if mounted {
            language = "pt-BR";
        }
    }
    let local_packs: std::collections::HashSet<_> = catalog
        .packs
        .iter()
        .filter(|p| packs::status_file(&root.join(&p.file), p))
        .map(|p| p.id.as_str())
        .collect();
    let pack_available = local_packs.len() == catalog.packs.len();
    let remote = updates::cached(data).voices;
    let visible_packs = remote
        .as_ref()
        .filter(|v| updates::voices_changed(v, &catalog))
        .map_or(catalog.packs.as_slice(), |v| v.packs.as_slice());
    let roster: Roster = serde_json::from_str(include_str!("../../public/assets/heroes.json"))
        .map_err(|e| e.to_string())?;
    let mut heroes: Vec<Hero> = roster
        .heroes
        .into_iter()
        .map(|hero| {
            let content = visible_packs
                .iter()
                .flat_map(|p| &p.heroes)
                .find(|entry| entry.asset_id == hero.asset_id);
            let total = content.map_or(0, |c| c.total);
            let local = catalog
                .packs
                .iter()
                .find(|p| p.heroes.iter().any(|h| h.asset_id == hero.asset_id));
            let available = local
                .filter(|p| {
                    local_packs.contains(p.id.as_str())
                        || (mounted
                            && (state.mounted_packs.is_empty()
                                || detected
                                    .as_ref()
                                    .is_ok_and(|g| packs::mounted(g, &state, p))))
                })
                .and_then(|p| p.heroes.iter().find(|h| h.asset_id == hero.asset_id))
                .map_or(0, |c| c.available);
            let status = if available > 0 && available == total {
                "complete"
            } else if available > 0 {
                "partial"
            } else {
                "not-started"
            };
            Hero {
                id: hero.id,
                name: hero.name,
                asset_id: hero.asset_id,
                portrait: hero.portrait,
                total,
                available,
                content_available: content.map_or(0, |c| c.available),
                state: status.into(),
                coverage: if total > 0 {
                    Some((available as f64 / total as f64 * 1000.0).round() / 10.0)
                } else {
                    None
                },
            }
        })
        .collect();
    heroes.sort_by(|a, b| {
        (a.content_available == 0)
            .cmp(&(b.content_available == 0))
            .then(a.name.cmp(&b.name))
    });
    Ok(Snapshot {
        language: language.into(),
        game_running: game::running()?,
        game_root: detected.as_ref().ok().map(|p| {
            p.to_string_lossy()
                .trim_start_matches("\\\\?\\")
                .to_string()
        }),
        game_error: detected.err(),
        pack_available,
        pack_version: catalog_version(&catalog),
        pack_download_available: updates::cached(data).voices.is_some(),
        voices_pending_application,
        update_repository: updates::repository(data)?,
        update_error: updates::cached(data).error,
        client_update: updates::summary_client(data),
        voice_update: updates::summary_voices(data, &catalog),
        heroes,
        updated_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string(),
        version: env!("CARGO_PKG_VERSION").into(),
        portraits_source: roster.source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_catalog_rejects_unrelated_fields() {
        let catalog = catalog().unwrap();
        assert_eq!(catalog.packs[0].heroes[0].asset_id, "ghost");
        assert!(
            serde_json::from_str::<PackCatalog>(
                r#"{"schema_version":1,"packs":[],"private_field":"secret"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn update_source_cannot_be_changed_by_command() {
        let result = action_with_progress(
            "set_repository",
            Path::new("unused"),
            Path::new("unused"),
            None,
            &crate::progress::quiet,
        );
        assert!(matches!(result, Err(error) if error == "Unsupported launcher action."));
    }

    #[test]
    fn released_content_is_available_even_without_a_local_voice_pack() {
        let root = std::env::temp_dir().join(format!("deadvox-content-{}", install::stamp()));
        let game_root = root.join("fixture-game");
        let data = root.join("user-data");
        install::write_atomic(&game_root.join("game/citadel/gameinfo.gi"), b"GameInfo {}").unwrap();
        install::write_atomic(&game_root.join("game/citadel/pak01_dir.vpk"), b"fixture").unwrap();
        install::save_state(
            &data,
            &install::State {
                game_root: Some(game_root),
                ..Default::default()
            },
        )
        .unwrap();
        let snapshot =
            action_with_progress("status", &root, &data, None, &crate::progress::quiet).unwrap();
        assert!(!snapshot.pack_available);
        assert_eq!(snapshot.language, "en");
        let geist = snapshot
            .heroes
            .iter()
            .find(|hero| hero.asset_id == "ghost")
            .unwrap();
        assert_eq!(geist.content_available, 1835);
        assert_eq!(geist.available, 0);
        assert_eq!(geist.state, "not-started");
        assert!(
            snapshot
                .heroes
                .iter()
                .filter(|hero| hero.asset_id != "ghost")
                .all(|hero| hero.content_available == 0)
        );
        fs::remove_dir_all(root).unwrap();
    }
}
