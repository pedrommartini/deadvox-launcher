use crate::{
    engine::{PackCatalog, validate_catalog},
    install,
    progress::Reporter,
};
use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    repository: Option<String>,
    client_channel: String,
    voices_channel: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientRelease {
    pub schema_version: u32,
    pub version: String,
    pub bytes: u64,
    pub sha256: String,
    pub download_url: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceRelease {
    pub schema_version: u32,
    pub minimum_client_version: String,
    pub packs: Vec<crate::engine::Pack>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Available {
    pub repository: Option<String>,
    pub client: Option<ClientRelease>,
    pub voices: Option<VoiceRelease>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct Summary {
    pub version: String,
    pub bytes: u64,
}

pub fn validate_repository(value: &str) -> Result<String, String> {
    let parts: Vec<_> = value.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|s| {
            s.is_empty()
                || *s == "."
                || *s == ".."
                || s.len() > 100
                || !s
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
    {
        return Err("Use a GitHub repository in the format owner/repository.".into());
    }
    Ok(value.to_string())
}

fn source() -> Result<Source, String> {
    serde_json::from_str(include_str!("../resources/update-source.json")).map_err(|e| e.to_string())
}

pub fn repository(_data: &Path) -> Result<Option<String>, String> {
    // Only the developer-owned configuration embedded at compile time is trusted.
    let candidate = source()?.repository;
    candidate
        .map(|value| validate_repository(&value))
        .transpose()
}

pub fn validate_url(value: &str, repository: &str) -> Result<(), String> {
    validate_repository(repository)?;
    let url = reqwest::Url::parse(value).map_err(|_| "Invalid GitHub release URL.".to_string())?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url
            .path()
            .starts_with(&format!("/{repository}/releases/download/"))
        || url.path().contains("%")
    {
        return Err("Updates must come from releases in the configured GitHub repository.".into());
    }
    Ok(())
}

fn client(timeout: Duration) -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("DeadVox/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            if attempt.previous().len() > 5
                || url.scheme() != "https"
                || !matches!(
                    url.host_str(),
                    Some(
                        "github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                    )
                )
            {
                attempt.error("Untrusted release redirect")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| e.to_string())
}

fn read_json<T: serde::de::DeserializeOwned>(client: &Client, url: &str) -> Result<T, String> {
    let mut response = client
        .get(url)
        .header("Cache-Control", "no-cache")
        .send()
        .map_err(|_| {
            "Could not connect to GitHub. You can continue using installed voices.".to_string()
        })?
        .error_for_status()
        .map_err(|e| {
            format!(
                "Release channel is not available on GitHub ({})",
                e.status().map_or(0, |s| s.as_u16())
            )
        })?;
    let mut bytes = Vec::new();
    (&mut response)
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("The update manifest is too large.".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "The release manifest is invalid.".into())
}

fn validate_client(release: &ClientRelease, repository: &str) -> Result<(), String> {
    if release.schema_version != 1
        || release.bytes < 2
        || release.bytes > 250 * 1024 * 1024
        || release.sha256.len() != 64
        || !release.sha256.chars().all(|c| c.is_ascii_hexdigit())
        || !release.download_url.ends_with(".exe")
    {
        return Err("Invalid launcher update manifest.".into());
    }
    Version::parse(&release.version).map_err(|_| "Invalid launcher version.".to_string())?;
    validate_url(&release.download_url, repository)
}

fn validate_voices(release: &VoiceRelease, repository: &str) -> Result<(), String> {
    Version::parse(&release.minimum_client_version)
        .map_err(|_| "Invalid required client version.".to_string())?;
    if release.packs.len() > 1 && newer("0.1.0-alpha.5", &release.minimum_client_version) {
        return Err("A multi-character release requires launcher alpha.5 or newer.".into());
    }
    validate_catalog(&PackCatalog {
        schema_version: release.schema_version,
        packs: release.packs.clone(),
    })?;
    for pack in &release.packs {
        Version::parse(&pack.version).map_err(|_| "Invalid voice pack version.".to_string())?;
        validate_url(
            pack.download_url
                .as_deref()
                .ok_or("The voice release has no download URL.")?,
            repository,
        )?;
        let url =
            reqwest::Url::parse(pack.download_url.as_ref().unwrap()).map_err(|e| e.to_string())?;
        if url.path().rsplit('/').next()
            != Path::new(&pack.file).file_name().and_then(|p| p.to_str())
        {
            return Err("Remote and local voice-pack filenames differ.".into());
        }
    }
    Ok(())
}

pub fn cached(data: &Path) -> Available {
    let current = repository(data).ok().flatten();
    let mut value = fs::read(data.join("available-updates.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Available>(&b).ok())
        .unwrap_or_default();
    if value.repository == current {
        if let Some(repo) = current {
            if value
                .client
                .as_ref()
                .is_some_and(|v| validate_client(v, &repo).is_err())
            {
                value.client = None;
            }
            if value
                .voices
                .as_ref()
                .is_some_and(|v| validate_voices(v, &repo).is_err())
            {
                value.voices = None;
            }
        } else {
            value.client = None;
            value.voices = None;
        }
        value
    } else {
        Available::default()
    }
}

pub fn newer(candidate: &str, installed: &str) -> bool {
    match (Version::parse(candidate), Version::parse(installed)) {
        (Ok(a), Ok(b)) => a > b,
        _ => false,
    }
}

pub fn check(data: &Path, report: &Reporter<'_>) -> Result<(), String> {
    let Some(repository) = repository(data)? else {
        return Ok(());
    };
    let config = source()?;
    let client = client(Duration::from_secs(20))?;
    let mut available = cached(data);
    available.repository = Some(repository.clone());
    let mut errors = Vec::new();
    report("Checking launcher updates", Some(10.0));
    let url = format!(
        "https://github.com/{repository}/releases/download/{}/client-update.json",
        config.client_channel
    );
    match read_json::<ClientRelease>(&client, &url).and_then(|v| {
        validate_client(&v, &repository)?;
        Ok(v)
    }) {
        Ok(value) => available.client = Some(value),
        Err(e) => errors.push(e),
    }
    report("Checking voice updates", Some(55.0));
    let url = format!(
        "https://github.com/{repository}/releases/download/{}/voice-update.json",
        config.voices_channel
    );
    match read_json::<VoiceRelease>(&client, &url).and_then(|v| {
        validate_voices(&v, &repository)?;
        Ok(v)
    }) {
        Ok(value) => available.voices = Some(value),
        Err(e) => errors.push(e),
    }
    available.error = if errors.is_empty() {
        None
    } else {
        Some(errors.join(" · "))
    };
    install::write_atomic(
        &data.join("available-updates.json"),
        &serde_json::to_vec_pretty(&available).map_err(|e| e.to_string())?,
    )
}

/// Stream into a temporary sibling. Never publish truncated or corrupt bytes.
pub fn receive<R: Read>(
    mut response: R,
    target: &Path,
    bytes: u64,
    expected: &str,
    report: &Reporter<'_>,
) -> Result<(), String> {
    let parent = target.parent().ok_or("Invalid download destination.")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(".download-{}.part", install::stamp()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 256 * 1024];
        let mut done = 0u64;
        report("Downloading", Some(0.0));
        loop {
            let length = response.read(&mut buffer).map_err(|_| {
                "Download was interrupted. The previous version is still available.".to_string()
            })?;
            if length == 0 {
                break;
            }
            done = done
                .checked_add(length as u64)
                .ok_or("Download size overflow.")?;
            if done > bytes {
                return Err("The download is larger than its manifest.".into());
            }
            file.write_all(&buffer[..length])
                .map_err(|e| e.to_string())?;
            digest.update(&buffer[..length]);
            report("Downloading", Some(95.0 * done as f64 / bytes as f64));
        }
        report("Verifying download", Some(97.0));
        if done != bytes || format!("{:x}", digest.finalize()) != expected.to_ascii_lowercase() {
            return Err("Download integrity check failed. The previous version was kept.".into());
        }
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temporary, target).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn download(
    url: &str,
    target: &Path,
    bytes: u64,
    sha256: &str,
    repository: &str,
    report: &Reporter<'_>,
) -> Result<(), String> {
    validate_url(url, repository)?;
    let response = client(Duration::from_secs(30 * 60))?
        .get(url)
        .send()
        .map_err(|e| format!("Could not start the download: {e}"))?
        .error_for_status()
        .map_err(|e| format!("GitHub download failed: {e}"))?;
    if response
        .content_length()
        .is_some_and(|length| length != bytes)
    {
        return Err("The release file size does not match its manifest.".into());
    }
    receive(response, target, bytes, sha256, report)
}

pub fn voices(root: &Path, data: &Path, report: &Reporter<'_>) -> Result<(), String> {
    let repository =
        repository(data)?.ok_or("Online updates are not available in this build yet.")?;
    let available = cached(data);
    let release = available
        .voices
        .ok_or("Check for updates before downloading voices.")?;
    validate_voices(&release, &repository)?;
    if newer(&release.minimum_client_version, env!("CARGO_PKG_VERSION")) {
        return Err("Update the launcher before installing this voice pack.".into());
    }
    fetch_voices(root, data, release, report, |pack, target, progress| {
        download(
            pack.download_url.as_ref().unwrap(),
            target,
            pack.bytes,
            &pack.sha256,
            &repository,
            progress,
        )
    })
}

fn fetch_voices<F>(
    root: &Path,
    data: &Path,
    release: VoiceRelease,
    report: &Reporter<'_>,
    mut fetch: F,
) -> Result<(), String>
where
    F: FnMut(&crate::engine::Pack, &Path, &Reporter<'_>) -> Result<(), String>,
{
    let installed = crate::engine::installed_catalog(data)?;
    validate_transition(&release, &installed)?;
    let pending: Vec<_> = release
        .packs
        .iter()
        .filter(|p| !crate::packs::valid_file(&root.join(&p.file), p))
        .collect();
    let total: u64 = pending.iter().map(|p| p.bytes).sum();
    let mut done = 0u64;
    for pack in pending {
        let target = root.join(&pack.file);
        if target.exists() {
            return Err("An existing voice file differs from its manifest. Preserve it and use a new versioned filename.".into());
        }
        let progress = |phase: &str, percent: Option<f64>| {
            report(
                phase,
                percent
                    .map(|p| 95.0 * (done as f64 + pack.bytes as f64 * p / 100.0) / total as f64),
            );
        };
        fetch(pack, &target, &progress)?;
        if !crate::packs::valid_file(&target, pack) {
            return Err(
                "A downloaded character failed validation. The previous catalog was kept.".into(),
            );
        }
        done += pack.bytes;
    }
    report("Saving voice catalog", Some(98.0));
    let catalog = PackCatalog {
        schema_version: release.schema_version,
        packs: release.packs,
    };
    install::write_atomic(
        &data.join("voice-catalog.json"),
        &serde_json::to_vec_pretty(&catalog).map_err(|e| e.to_string())?,
    )
}

const CLIENT_HELPER: &str = r#"param([int]$LauncherPid,[string]$Source,[string]$Destination,[string]$ExpectedHash,[switch]$NoRestart)
$ErrorActionPreference = 'Stop'
$folder = [IO.Path]::GetDirectoryName($Source)
$log = [IO.Path]::Combine($folder, 'client-update-error.txt')
$backup = [IO.Path]::Combine($folder, 'previous-client.exe')
$temporary = $Destination + '.updating'
try {
    $launcher = $null
    try { $launcher = [Diagnostics.Process]::GetProcessById($LauncherPid) } catch [ArgumentException] {}
    if ($launcher -and !$launcher.WaitForExit(120000)) { throw 'Launcher did not exit. Update was not applied.' }
    $file = [IO.File]::OpenRead($Source)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try { $actualHash = [BitConverter]::ToString($hasher.ComputeHash($file)).Replace('-','').ToLowerInvariant() } finally { $file.Dispose(); $hasher.Dispose() }
    if ($actualHash -ne $ExpectedHash) { throw 'Client integrity check failed.' }
    [IO.File]::Copy($Source, $temporary, $true)
    [IO.File]::Replace($temporary, $Destination, $backup)
    if (!$NoRestart) {
        $start = [Diagnostics.ProcessStartInfo]::new($Destination)
        $start.UseShellExecute = $true
        $start.WorkingDirectory = [IO.Path]::GetDirectoryName($Destination)
        [Diagnostics.Process]::Start($start) | Out-Null
    }
} catch {
    [IO.File]::WriteAllText($log, $_.Exception.ToString())
    if ([IO.File]::Exists($backup) -and ![IO.File]::Exists($Destination)) { [IO.File]::Copy($backup, $Destination) }
    exit 1
}
"#;

pub fn stage_client(root: &Path, data: &Path, report: &Reporter<'_>) -> Result<(), String> {
    let repository =
        repository(data)?.ok_or("Online updates are not available in this build yet.")?;
    let release = cached(data)
        .client
        .ok_or("No launcher update is available.")?;
    validate_client(&release, &repository)?;
    if !newer(&release.version, env!("CARGO_PKG_VERSION")) {
        return Err("The launcher is already up to date.".into());
    }
    let directory = data.join("updates");
    let target = directory.join("next-client.exe");
    download(
        &release.download_url,
        &target,
        release.bytes,
        &release.sha256,
        &repository,
        report,
    )?;
    let mut magic = [0u8; 2];
    fs::File::open(&target)
        .and_then(|mut file| file.read_exact(&mut magic))
        .map_err(|e| e.to_string())?;
    if magic != *b"MZ" {
        return Err("The update is not a Windows executable.".into());
    }
    let helper = directory.join("apply-client-update.ps1");
    install::write_atomic(&helper, CLIENT_HELPER.as_bytes())?;
    report("Preparing restart", Some(99.0));
    use std::os::windows::process::CommandExt;
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&helper)
        .arg("-LauncherPid")
        .arg(std::process::id().to_string())
        .arg("-Source")
        .arg(&target)
        .arg("-Destination")
        .arg(std::env::current_exe().map_err(|e| e.to_string())?)
        .arg("-ExpectedHash")
        .arg(&release.sha256)
        .current_dir(root)
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| format!("Could not start the update helper: {e}"))?;
    Ok(())
}

pub fn summary_client(data: &Path) -> Option<Summary> {
    let value = cached(data).client?;
    newer(&value.version, env!("CARGO_PKG_VERSION")).then_some(Summary {
        version: value.version,
        bytes: value.bytes,
    })
}

pub fn summary_voices(data: &Path, catalog: &PackCatalog) -> Option<Summary> {
    let value = cached(data).voices?;
    voices_changed(&value, catalog).then(|| Summary {
        version: crate::engine::catalog_version(&PackCatalog {
            schema_version: value.schema_version,
            packs: value.packs.clone(),
        }),
        bytes: value
            .packs
            .iter()
            .filter(|p| {
                !catalog
                    .packs
                    .iter()
                    .any(|old| old.id == p.id && old.sha256 == p.sha256)
            })
            .map(|p| p.bytes)
            .sum(),
    })
}

pub fn validate_transition(release: &VoiceRelease, installed: &PackCatalog) -> Result<(), String> {
    for old in &installed.packs {
        let incoming = release.packs.iter().find(|p| p.id == old.id).ok_or(
            "The voice catalog removed an installed character. Keep the previous release.",
        )?;
        if newer(&old.version, &incoming.version)
            || (old.version == incoming.version
                && (old.sha256 != incoming.sha256 || old.bytes != incoming.bytes))
            || (newer(&incoming.version, &old.version)
                && old.file == incoming.file
                && old.sha256 != incoming.sha256)
        {
            return Err(
                "A voice release downgraded or reused a version. Keep the installed voices.".into(),
            );
        }
    }
    Ok(())
}

pub fn voices_changed(release: &VoiceRelease, installed: &PackCatalog) -> bool {
    validate_transition(release, installed).is_ok()
        && release.packs.iter().any(|pack| {
            !installed.packs.iter().any(|old| {
                old.id == pack.id && old.version == pack.version && old.sha256 == pack.sha256
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multi_character_download_keeps_catalog_on_failure_and_retries_only_missing_files() {
        let root =
            std::env::temp_dir().join(format!("deadvox-batch-download-{}", install::stamp()));
        let data = root.join("user-data");
        let mut installed = crate::engine::catalog().unwrap();
        installed.packs.truncate(1);
        let old = &mut installed.packs[0];
        old.bytes = 8;
        old.sha256 = format!("{:x}", Sha256::digest(b"old-pack"));
        install::write_atomic(&root.join(&old.file), b"old-pack").unwrap();
        install::write_atomic(
            &data.join("voice-catalog.json"),
            &serde_json::to_vec(&installed).unwrap(),
        )
        .unwrap();
        let previous = fs::read(data.join("voice-catalog.json")).unwrap();
        let mut release = VoiceRelease {
            schema_version: 1,
            minimum_client_version: "0.1.0-alpha.5".into(),
            packs: installed.packs.clone(),
        };
        for id in ["pt-br-abrams", "pt-br-bebop"] {
            let mut pack = installed.packs[0].clone();
            pack.id = id.into();
            pack.file = format!("voice-packs/pt-BR/{id}_dir.vpk");
            pack.heroes[0].asset_id = id.trim_start_matches("pt-br-").into();
            pack.sha256 = format!("{:x}", Sha256::digest(b"new-pack"));
            release.packs.push(pack);
        }
        let mut calls = Vec::new();
        let result = fetch_voices(
            &root,
            &data,
            release.clone(),
            &crate::progress::quiet,
            |pack, target, report| {
                calls.push(pack.id.clone());
                if pack.id == "pt-br-bebop" {
                    return Err("Simulated interrupted second file".into());
                }
                receive(&b"new-pack"[..], target, pack.bytes, &pack.sha256, report)
            },
        );
        assert!(result.is_err());
        assert_eq!(calls, ["pt-br-abrams", "pt-br-bebop"]);
        assert_eq!(fs::read(data.join("voice-catalog.json")).unwrap(), previous);
        calls.clear();
        fetch_voices(
            &root,
            &data,
            release,
            &crate::progress::quiet,
            |pack, target, report| {
                calls.push(pack.id.clone());
                receive(&b"new-pack"[..], target, pack.bytes, &pack.sha256, report)
            },
        )
        .unwrap();
        assert_eq!(calls, ["pt-br-bebop"]);
        assert_eq!(
            crate::engine::installed_catalog(&data).unwrap().packs.len(),
            3
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn full_roster_and_aggregate_over_two_gib_are_supported_with_bounded_individual_assets() {
        let mut catalog = crate::engine::catalog().unwrap();
        let template = catalog.packs[0].clone();
        catalog.packs.clear();
        for index in 0..100 {
            let mut pack = template.clone();
            pack.id = format!("pt-br-character-{index}");
            pack.file = format!("voice-packs/pt-BR/character-{index}_dir.vpk");
            pack.heroes[0].asset_id = format!("character_{index}");
            pack.bytes = 200 * 1024 * 1024;
            catalog.packs.push(pack);
        }
        validate_catalog(&catalog).unwrap();
        let mut release = VoiceRelease {
            schema_version: 1,
            minimum_client_version: "0.1.0-alpha.5".into(),
            packs: catalog.packs.clone(),
        };
        assert!(!voices_changed(&release, &catalog));
        release.packs[99].version = "0.2.0".into();
        assert!(voices_changed(&release, &catalog));
        release.packs.remove(0);
        assert!(validate_transition(&release, &catalog).is_err());
        catalog.packs[0].bytes = 2 * 1024 * 1024 * 1024;
        assert!(validate_catalog(&catalog).is_err());
    }
    #[test]
    fn local_settings_cannot_override_embedded_update_source() {
        let root = std::env::temp_dir().join(format!("deadlock-source-{}", install::stamp()));
        install::write_atomic(
            &root.join("state.json"),
            br#"{"version":1,"update_repository":"untrusted/override"}"#,
        )
        .unwrap();
        let expected = source().unwrap().repository;
        assert_eq!(repository(&root).unwrap(), expected);
        let state = install::load_state(&root).unwrap();
        install::save_state(&root, &state).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("state.json")).unwrap()).unwrap();
        assert!(saved.get("update_repository").is_none());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn versions_channels_and_origins_are_separate() {
        assert!(newer("0.1.0-alpha.10", "0.1.0-alpha.2"));
        assert!(!newer("0.1.0-alpha.1", "0.1.0-alpha.2"));
        assert!(validate_repository("owner/repo").is_ok());
        assert!(validate_repository("../repo").is_err());
        assert!(
            validate_url(
                "https://github.com/owner/repo/releases/download/client-v1/app.exe",
                "owner/repo"
            )
            .is_ok()
        );
        for bad in [
            "http://github.com/owner/repo/releases/download/tag/a.exe",
            "https://evil.example/owner/repo/releases/download/tag/a.exe",
            "https://github.com/other/repo/releases/download/tag/a.exe",
            "https://github.com/owner/repo/releases/download/../a.exe",
        ] {
            assert!(validate_url(bad, "owner/repo").is_err());
        }
    }
    #[test]
    fn interrupted_and_corrupt_downloads_keep_previous_version() {
        let root = std::env::temp_dir().join(format!("deadlock-download-{}", install::stamp()));
        let file = root.join("pack.vpk");
        install::write_atomic(&file, b"old-pack").unwrap();
        let digest = format!("{:x}", Sha256::digest(b"new-pack"));
        assert!(receive(&b"new"[..], &file, 8, &digest, &crate::progress::quiet).is_err());
        assert!(receive(&b"bad-pack"[..], &file, 8, &digest, &crate::progress::quiet).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"old-pack");
        assert!(fs::read_dir(&root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .path()
                .extension()
                .is_some_and(|e| e == "part")
        }));
        receive(&b"new-pack"[..], &file, 8, &digest, &crate::progress::quiet).unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"new-pack");
    }

    #[test]
    fn github_manifest_transport_validates_public_payload() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let release = ClientRelease {
            schema_version: 1,
            version: "0.2.0".into(),
            bytes: 42,
            sha256: "a".repeat(64),
            download_url: "https://github.com/owner/repo/releases/download/client-v0.2.0/app.exe"
                .into(),
        };
        let body = serde_json::to_vec(&release).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0u8; 4096];
            stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        // Local HTTP is confined to this test transport; production requires HTTPS.
        let transport = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let value: ClientRelease =
            read_json(&transport, &format!("http://{address}/client-update.json")).unwrap();
        validate_client(&value, "owner/repo").unwrap();
        assert_eq!(value.version, "0.2.0");
        assert!(serde_json::from_value::<ClientRelease>(serde_json::json!({"schema_version":1,"version":"0.2.0","bytes":42,"sha256":"a".repeat(64),"download_url":value.download_url,"private_field":"not allowed"})).is_err());
        server.join().unwrap();
    }

    #[test]
    fn replacement_helper_keeps_previous_executable_and_rejects_corruption() {
        use std::os::windows::process::CommandExt;
        let directory =
            std::env::temp_dir().join(format!("deadlock-client-helper-{}", install::stamp()));
        let destination = directory.join("client.exe");
        let source = directory.join("next-client.exe");
        let helper = directory.join("apply.ps1");
        install::write_atomic(&destination, b"MZprevious-client").unwrap();
        install::write_atomic(&source, b"MZnew-client").unwrap();
        install::write_atomic(&helper, CLIENT_HELPER.as_bytes()).unwrap();
        let run = |hash: &str| {
            std::process::Command::new("powershell.exe")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                ])
                .arg(&helper)
                .args(["-LauncherPid", "2147483647", "-Source"])
                .arg(&source)
                .arg("-Destination")
                .arg(&destination)
                .arg("-ExpectedHash")
                .arg(hash)
                .arg("-NoRestart")
                .creation_flags(0x08000000)
                .status()
                .unwrap()
        };
        assert!(run(&install::sha256(&source).unwrap()).success());
        assert_eq!(fs::read(&destination).unwrap(), b"MZnew-client");
        assert_eq!(
            fs::read(directory.join("previous-client.exe")).unwrap(),
            b"MZprevious-client"
        );
        install::write_atomic(&source, b"MZinvalid-new-client").unwrap();
        run(&"0".repeat(64));
        assert_eq!(fs::read(&destination).unwrap(), b"MZnew-client");
        assert!(directory.join("client-update-error.txt").exists());
    }
}
