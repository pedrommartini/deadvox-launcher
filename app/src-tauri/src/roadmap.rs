//! Read a single voice preview from installed VPKs, without copying production data.
use crate::{engine, packs};
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::{Path, PathBuf}};

// Each path was verified against a saved human choice and the matching build report.
// A pack update requires a fresh verification before its preview can play.
fn approved_preview(character: &str) -> Result<(&'static str, &'static str), String> {
    match character {
        "ghost" => Ok(("sounds/vo/ghost/ghost_kill_anyhero_01.vsnd_c", "7cbc3a2587d2683b9bdea3382d08b597654a1a3bc44304a746355a64c2465e3e")),
        "announcer" => Ok(("sounds/vo/announcer/female_patron/patron_female_ally_atlas_start_01.vsnd_c", "90d5d82626053d78ba74b17dccc1e8d576fbb216e90a9074955231008eafdbf4")),
        "hidden_king" => Ok(("sounds/vo/announcer/male_patron/patron_male_ally_atlas_start_01.vsnd_c", "9c2c87e716aed3297e82b33ee5924aed86496d4d388deb6fde253d1e425af258")),
        "spirit_jar" => Ok(("sounds/vo/spirit_jar/spirit_jar_holder_stalls_bebop_05.vsnd_c", "e54fa732e12c0d87b7526b8f9e7e1cbfa2471c74015c6d9c9cb64e0ecdddeac7")),
        "shopkeeper" => Ok(("sounds/vo/shopkeeper/shopkeeper_hotdog_buy_armor_01.vsnd_c", "230888b8a48eb542de0720bcd2f79b3fbdad4245ef5168fb0a3b27a025be9673")),
        _ => Err("No human-selected voice preview is available for this character.".into()),
    }
}

fn u16_at(b: &[u8], i: usize) -> Result<u16, String> { Ok(u16::from_le_bytes(b.get(i..i+2).ok_or("Truncated VPK")?.try_into().unwrap())) }
fn u32_at(b: &[u8], i: usize) -> Result<u32, String> { Ok(u32::from_le_bytes(b.get(i..i+4).ok_or("Truncated VPK")?.try_into().unwrap())) }
fn string(b: &[u8], pos: &mut usize) -> Result<String, String> {
    let n = b.get(*pos..).ok_or("Truncated VPK")?.iter().position(|x| *x == 0).ok_or("Invalid VPK tree")?;
    let value = std::str::from_utf8(&b[*pos..*pos+n]).map_err(|_| "Invalid VPK text")?.to_owned();
    *pos += n + 1; Ok(value)
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for byte in bytes { c ^= u32::from(*byte); for _ in 0..8 { c = (c >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(c & 1)); } }
    !c
}
fn read_clip(path: &Path, expected_path: &str) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|_| "Voice archive unavailable")?;
    let mut header = [0u8; 12]; file.read_exact(&mut header).map_err(|_| "Invalid VPK header")?;
    if u32_at(&header,0)? != 0x55aa1234 { return Err("Invalid VPK signature".into()); }
    let version = u32_at(&header,4)?; let tree_len = u32_at(&header,8)? as usize;
    if ![1,2].contains(&version) || tree_len == 0 || tree_len > 128*1024*1024 { return Err("Invalid VPK tree".into()); }
    let base = if version == 2 { 28 } else { 12 };
    file.seek(SeekFrom::Start(base)).map_err(|_| "Invalid VPK header")?;
    let mut tree = vec![0u8;tree_len]; file.read_exact(&mut tree).map_err(|_| "Truncated VPK tree")?;
    let mut pos = 0; let mut selected: Option<(String,u32,u16,u32,u32,Vec<u8>)> = None;
    loop { let ext = string(&tree,&mut pos)?; if ext.is_empty(){break}
        loop { let dir = string(&tree,&mut pos)?; if dir.is_empty(){break}
            loop { let name = string(&tree,&mut pos)?; if name.is_empty(){break}
                let crc = u32_at(&tree,pos)?; let preload = u16_at(&tree,pos+4)? as usize;
                let archive = u16_at(&tree,pos+6)?; let offset = u32_at(&tree,pos+8)?; let size = u32_at(&tree,pos+12)?;
                if u16_at(&tree,pos+16)? != 0xffff { return Err("Invalid VPK entry".into()); }
                pos += 18; let pre = tree.get(pos..pos+preload).ok_or("Invalid VPK preload")?.to_vec(); pos += preload;
                let full = format!("{dir}/{name}.{ext}");
                if ext == "vsnd_c" && full == expected_path && size > 0 && size <= 4*1024*1024 {
                    selected = Some((full,crc,archive,offset,size,pre));
                }
            }
        }
    }
    let (_,crc,archive,offset,size,mut bytes) = selected.ok_or("No matching voice sample")?;
    let (mut source, start) = if archive == 0x7fff { (file, base + tree_len as u64 + u64::from(offset)) } else {
        let stem = path.file_stem().and_then(|s| s.to_str()).ok_or("Invalid VPK filename")?.strip_suffix("_dir").ok_or("Invalid VPK archive")?;
        let part = path.with_file_name(format!("{stem}_{archive:03}.vpk"));
        (File::open(part).map_err(|_| "VPK archive unavailable")?, u64::from(offset))
    };
    source.seek(SeekFrom::Start(start)).map_err(|_| "Invalid VPK offset")?;
    let old = bytes.len(); bytes.resize(old + size as usize, 0); source.read_exact(&mut bytes[old..]).map_err(|_| "Truncated voice sample")?;
    if crc32(&bytes) != crc { return Err("Voice sample integrity failed".into()); }
    let resource_size = u32_at(&bytes,0)? as usize;
    let stream = bytes.get(resource_size..).ok_or("Invalid voice resource")?;
    if stream.len() < 4 || !(stream.starts_with(b"ID3") || (stream[0] == 0xff && stream[1] & 0xe0 == 0xe0)) { return Err("Unsupported voice format".into()); }
    Ok(stream.to_vec())
}

pub fn sample(character: &str, dubbed: bool) -> Result<Vec<u8>, String> {
    if !dubbed { return Err("No human-selected voice preview is available for this character.".into()); }
    let (asset, approved_hash) = approved_preview(character)?;
    let root = engine::executable_root()?;
    let data = root.join("user-data");
    let catalog = engine::installed_catalog(&data)?;
    let pack = catalog.packs.iter().find(|p| p.heroes.iter().any(|h| h.asset_id == character && h.available > 0)).ok_or("Dubbed voice pack is not installed")?;
    if pack.sha256 != approved_hash { return Err("The current voice pack has no verified selected preview.".into()); }
    let archive: PathBuf = root.join(&pack.file);
    if !packs::status_file(&archive, pack) { return Err("Dubbed voice pack is missing or damaged".into()); }
    read_clip(&archive, asset)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn previews_are_explicit_and_crc_is_standard() {
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
        assert!(approved_preview("../other").is_err());
        assert!(approved_preview("priest").is_err());
        assert!(approved_preview("drifter").is_err());
        assert_eq!(approved_preview("spirit_jar").unwrap().0, "sounds/vo/spirit_jar/spirit_jar_holder_stalls_bebop_05.vsnd_c");
    }
    #[test] fn selected_previews_are_pinned_to_the_shipped_packs() {
        let catalog: serde_json::Value = serde_json::from_str(include_str!("../resources/voice-packs.json")).unwrap();
        let packs = catalog["packs"].as_array().unwrap();
        for character in ["ghost", "announcer", "hidden_king", "spirit_jar", "shopkeeper"] {
            let (_, hash) = approved_preview(character).unwrap();
            let pack = packs.iter().find(|pack| pack["heroes"].as_array().unwrap().iter().any(|hero| hero["asset_id"] == character)).unwrap();
            assert_eq!(pack["sha256"].as_str(), Some(hash), "{character}");
        }
    }
    #[test] fn invalid_vpk_is_rejected() {
        let path = std::env::temp_dir().join(format!("deadvox-invalid-{}", crate::install::stamp()));
        std::fs::write(&path, b"not a valid VPK").unwrap();
        assert!(read_clip(&path, approved_preview("ghost").unwrap().0).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test] fn selected_archives_supply_selected_mp3_when_provided() {
        if let Ok(app) = std::env::var("DEADVOX_TEST_APP") {
            let root = Path::new(&app);
            for (id, file) in [("ghost", "lady-geist-0.1.0_dir.vpk"), ("announcer", "pt-br-archmother-0.1.0-alpha.1_dir.vpk"), ("hidden_king", "pt-br-hidden-king-0.1.0-alpha.1_dir.vpk"), ("spirit_jar", "pt-br-the-urn-0.1.0-alpha.2_dir.vpk"), ("shopkeeper", "pt-br-the-shopkeeper-0.1.0-alpha.1_dir.vpk")] {
                let path = root.join("voice-packs/pt-BR").join(file);
                let audio = read_clip(&path, approved_preview(id).unwrap().0).unwrap_or_else(|e| panic!("{id}: {e}"));
                assert!(audio.len() > 1000);
                assert!(read_clip(&path, "sounds/vo/does-not-exist.vsnd_c").is_err());
            }
        }
    }
}
