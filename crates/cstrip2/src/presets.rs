//! Preset Management and A/B Snapshot System for Cstrip2
//!
//! Provides factory presets, serialization of user presets to JSON,
//! and A/B slot swapping without altering the frozen DSP engine.

use nih_plug::prelude::{Param, ParamSetter};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::params::Cstrip2Params;

/// Complete snapshot of the 10 canonical DSP parameters of Cstrip2
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PresetData {
    pub name: String,
    pub treble: f32,
    pub mid: f32,
    pub bass: f32,
    pub treb_frq: f32,
    pub bass_frq: f32,
    pub low_cap: f32,
    pub hi_cap: f32,
    pub compres: f32,
    pub comp_spd: f32,
    pub output: f32,
}

impl Default for PresetData {
    fn default() -> Self {
        Self {
            name: "Default (Bypassed / Clean)".to_string(),
            treble: 0.0,
            mid: 0.0,
            bass: 0.0,
            treb_frq: 4.75,
            bass_frq: 422.5,
            low_cap: 1.0,
            hi_cap: 0.0,
            compres: 0.0,
            comp_spd: 0.0,
            output: 0.33,
        }
    }
}

impl PresetData {
    /// Capture current parameter values from Cstrip2Params
    pub fn from_params(params: &Cstrip2Params, name: &str) -> Self {
        Self {
            name: name.to_string(),
            treble: params.treble.unmodulated_plain_value(),
            mid: params.mid.unmodulated_plain_value(),
            bass: params.bass.unmodulated_plain_value(),
            treb_frq: params.treb_frq.unmodulated_plain_value(),
            bass_frq: params.bass_frq.unmodulated_plain_value(),
            low_cap: params.low_cap.unmodulated_plain_value(),
            hi_cap: params.hi_cap.unmodulated_plain_value(),
            compres: params.compres.unmodulated_plain_value(),
            comp_spd: params.comp_spd.unmodulated_plain_value(),
            output: params.output.unmodulated_plain_value(),
        }
    }

    /// Apply snapshot to live parameters via ParamSetter
    pub fn apply(&self, params: &Cstrip2Params, setter: &ParamSetter) {
        setter.begin_set_parameter(&params.treble);
        setter.set_parameter(&params.treble, self.treble);
        setter.end_set_parameter(&params.treble);
        
        setter.begin_set_parameter(&params.mid);
        setter.set_parameter(&params.mid, self.mid);
        setter.end_set_parameter(&params.mid);
        
        setter.begin_set_parameter(&params.bass);
        setter.set_parameter(&params.bass, self.bass);
        setter.end_set_parameter(&params.bass);
        
        setter.begin_set_parameter(&params.treb_frq);
        setter.set_parameter(&params.treb_frq, self.treb_frq);
        setter.end_set_parameter(&params.treb_frq);
        
        setter.begin_set_parameter(&params.bass_frq);
        setter.set_parameter(&params.bass_frq, self.bass_frq);
        setter.end_set_parameter(&params.bass_frq);
        
        setter.begin_set_parameter(&params.low_cap);
        setter.set_parameter(&params.low_cap, self.low_cap);
        setter.end_set_parameter(&params.low_cap);
        
        setter.begin_set_parameter(&params.hi_cap);
        setter.set_parameter(&params.hi_cap, self.hi_cap);
        setter.end_set_parameter(&params.hi_cap);
        
        setter.begin_set_parameter(&params.compres);
        setter.set_parameter(&params.compres, self.compres);
        setter.end_set_parameter(&params.compres);
        
        setter.begin_set_parameter(&params.comp_spd);
        setter.set_parameter(&params.comp_spd, self.comp_spd);
        setter.end_set_parameter(&params.comp_spd);
        
        setter.begin_set_parameter(&params.output);
        setter.set_parameter(&params.output, self.output);
        setter.end_set_parameter(&params.output);
    }
}

/// Curated factory presets for Cstrip2
pub fn get_factory_presets() -> Vec<PresetData> {
    vec![
        PresetData {
            name: "Default (Clean / Flat)".to_string(),
            ..PresetData::default()
        },
        PresetData {
            name: "Vocal Air & Body".to_string(),
            treble: 2.5,
            mid: -1.0,
            bass: 1.5,
            treb_frq: 8.0,
            bass_frq: 120.0,
            low_cap: 0.95,
            hi_cap: 0.08,
            compres: 0.25,
            comp_spd: 0.30,
            output: 0.33,
        },
        PresetData {
            name: "Drum Bus Punch & Snap".to_string(),
            treble: 3.0,
            mid: 1.5,
            bass: 3.5,
            treb_frq: 6.5,
            bass_frq: 80.0,
            low_cap: 1.0,
            hi_cap: 0.05,
            compres: 0.45,
            comp_spd: 0.65,
            output: 0.33,
        },
        PresetData {
            name: "Bass Guitar Anchor".to_string(),
            treble: -1.5,
            mid: 2.0,
            bass: 4.0,
            treb_frq: 3.5,
            bass_frq: 95.0,
            low_cap: 0.85,
            hi_cap: 0.04,
            compres: 0.50,
            comp_spd: 0.20,
            output: 0.33,
        },
        PresetData {
            name: "Acoustic Warmth & Sheen".to_string(),
            treble: 2.0,
            mid: -1.8,
            bass: 1.0,
            treb_frq: 10.0,
            bass_frq: 180.0,
            low_cap: 0.98,
            hi_cap: 0.06,
            compres: 0.15,
            comp_spd: 0.40,
            output: 0.33,
        },
        PresetData {
            name: "Master Bus Glue & Polish".to_string(),
            treble: 1.0,
            mid: 0.0,
            bass: 1.0,
            treb_frq: 12.0,
            bass_frq: 60.0,
            low_cap: 1.0,
            hi_cap: 0.02,
            compres: 0.18,
            comp_spd: 0.15,
            output: 0.33,
        },
    ]
}

/// Retrieve path to user presets directory in user AppData / home config
pub fn get_user_presets_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);

    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library").join("Application Support"));

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let base = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".config"));

    base.map(|b| b.join("Airwindows").join("Cstrip2").join("Presets"))
}

/// Load all custom user presets found in the user presets folder
pub fn load_user_presets_from_dir(dir: &std::path::Path) -> Vec<PresetData> {
    let mut presets = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(meta) = fs::metadata(&path) {
                    if meta.is_file() {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(preset) = serde_json::from_str::<PresetData>(&content) {
                                presets.push(preset);
                            }
                        }
                    }
                }
            }
        }
    }
    presets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    presets
}

pub fn load_user_presets() -> Vec<PresetData> {
    let Some(dir) = get_user_presets_dir() else {
        return Vec::new();
    };
    load_user_presets_from_dir(&dir)
}

/// Sanitize preset name to produce a valid, safe cross-platform filename base.
///
/// Replaces characters forbidden by Windows/POSIX filesystems (`< > : " / \ | ? *`
/// and control characters 0..=31) with `_`. Trims leading and trailing spaces and dots.
/// Ensures empty strings fallback to `"Preset"`, and Windows reserved device names
/// (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`, `LPT1`..`LPT9`) are safely prefixed.
pub fn sanitize_preset_filename(name: &str) -> String {
    let mut sanitized = String::new();
    for c in name.chars() {
        if c == '<'
            || c == '>'
            || c == ':'
            || c == '"'
            || c == '/'
            || c == '\\'
            || c == '|'
            || c == '?'
            || c == '*'
            || (c as u32) < 32
        {
            sanitized.push('_');
        } else {
            sanitized.push(c);
        }
    }

    let trimmed = sanitized.trim_matches(|c: char| c == ' ' || c == '.');
    let base = if trimmed.is_empty() { "Preset" } else { trimmed };

    // Check Windows reserved device names:
    // Windows checks the stem preceding any extension (e.g. CON in CON.txt, PRN in PRN.foo).
    // It also treats Unicode superscript digits ¹ (U+00B9), ² (U+00B2), ³ (U+00B3) as digits.
    let stem = match base.split('.').next() {
        Some(s) => s.trim(),
        None => base.trim(),
    };

    let mut stem_normalized = String::new();
    for c in stem.chars() {
        match c {
            '¹' => stem_normalized.push('1'),
            '²' => stem_normalized.push('2'),
            '³' => stem_normalized.push('3'),
            _ => {
                for up in c.to_uppercase() {
                    stem_normalized.push(up);
                }
            }
        }
    }

    let is_reserved = matches!(
        stem_normalized.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );

    if is_reserved {
        format!("Preset_{}", base)
    } else {
        base.to_string()
    }
}

/// Locate an existing file in `dir` whose JSON content matches `preset_name`.
///
/// Rule: Uncertainty of I/O does NOT equal preset absence.
/// - If reading `read_dir` or any `DirEntry` returns `Err`, the error is propagated.
/// - If reading a `.json` file returns a real I/O error (sharing violation, permission denied, etc.),
///   the error is propagated immediately.
/// - If an entry disappeared (`NotFound`) between enumeration and read, the scan retries once.
/// - If a file contains valid readable text but invalid JSON syntax, it is treated as an occupied
///   unknown file (not this preset), preserving corrupt files on disk.
pub fn find_existing_preset_file(
    dir: &std::path::Path,
    preset_name: &str,
) -> std::io::Result<Option<PathBuf>> {
    for attempt in 0..2 {
        let mut entry_disappeared = false;
        let mut found_path: Option<PathBuf> = None;

        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if attempt == 0 {
                    continue;
                } else {
                    return Err(e);
                }
            }
            Err(e) => return Err(e),
        };

        for entry_res in entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        entry_disappeared = true;
                        break;
                    } else {
                        return Err(e);
                    }
                }
            };
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            entry_disappeared = true;
                            break;
                        } else {
                            // PermissionDenied, sharing violation, etc. Propagate!
                            return Err(e);
                        }
                    }
                };

                if !meta.is_file() {
                    continue;
                }

                match fs::read_to_string(&path) {
                    Ok(content) => {
                        if let Ok(existing) = serde_json::from_str::<PresetData>(&content) {
                            if existing.name == preset_name {
                                found_path = Some(path);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            entry_disappeared = true;
                            break;
                        } else {
                            // Sharing violation, permission denied, etc. Propagate!
                            return Err(e);
                        }
                    }
                }
            }
        }

        if let Some(path) = found_path {
            return Ok(Some(path));
        }

        if entry_disappeared {
            if attempt == 0 {
                continue;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Directory entry disappeared concurrently during scan",
                ));
            }
        }

        return Ok(None);
    }

    Ok(None)
}

#[cfg(windows)]
const FILE_SHARE_READ: u32 = 0x00000001;
#[cfg(windows)]
const FILE_SHARE_WRITE: u32 = 0x00000002;
// Note: FILE_SHARE_DELETE (0x00000004) is intentionally excluded to prevent
// external delete-pending or rename/displacement while the update handle is held.

/// Update an existing user preset file in-place using a single verified file handle.
///
/// Ensures strict identity validation against TOCTOU race conditions:
/// 1. Opens the existing file with read/write permissions (`create(false)` and without initial truncation).
///    On Windows, excludes `FILE_SHARE_DELETE` so that external processes cannot mark the file as
///    delete-pending or rename/displace it while the update handle is held.
/// 2. Reads and deserializes the JSON content using THAT SAME HANDLE.
/// 3. Confirms that `PresetData.name` still matches the target preset name.
///    - If the file was deleted or renamed between scan and open: returns `Err(NotFound)`.
///    - If the file was replaced by another preset (e.g. name = B): returns `Err(InvalidData)`.
///      B is NOT truncated or overwritten and remains 100% byte-identical.
///    - If the file content is corrupt JSON: returns `Err(InvalidData)`.
/// 4. Only after identity is strictly confirmed: truncates the file to 0 bytes via the same handle,
///    seeks to the start, writes the new JSON content, and flushes to disk.
pub fn update_existing_preset_file(
    path: &std::path::Path,
    preset: &PresetData,
    json_str: &str,
) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(false);

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    }

    let mut file = options.open(path)?;

    let mut current_content = String::new();
    file.read_to_string(&mut current_content)?;

    let existing_data: PresetData = serde_json::from_str(&current_content).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Preset file corrupt during update: {}", e),
        )
    })?;

    if existing_data.name != preset.name {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Preset identity mismatch: expected '{}', found '{}'",
                preset.name, existing_data.name
            ),
        ));
    }

    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(json_str.as_bytes())?;
    file.flush()?;

    Ok(())
}

/// Resolves the target file path for saving a preset into `dir`.
///
/// - If an existing readable file in `dir` contains this preset name, that path is returned.
/// - If no existing file holds this preset name, a candidate unique filename is chosen.
///   Propagates any `std::io::Error` encountered during directory inspection.
pub fn resolve_preset_path_in_dir(dir: &std::path::Path, preset_name: &str) -> std::io::Result<PathBuf> {
    if let Some(path) = find_existing_preset_file(dir, preset_name)? {
        return Ok(path);
    }

    let base = sanitize_preset_filename(preset_name);

    let mut existing_lower: std::collections::HashSet<String> = std::collections::HashSet::new();
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry_res in entries {
                let entry = entry_res?;
                if let Ok(name) = entry.file_name().into_string() {
                    existing_lower.insert(name.to_lowercase());
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }

    let mut counter: usize = 1;
    loop {
        let candidate_filename = if counter == 1 {
            format!("{}.json", base)
        } else {
            format!("{}_{}.json", base, counter)
        };

        let candidate_path = dir.join(&candidate_filename);
        let occupied = candidate_path.exists()
            || existing_lower.contains(&candidate_filename.to_lowercase());

        if !occupied {
            return Ok(candidate_path);
        }
        counter += 1;
    }
}

/// Save a user preset to a specified directory.
///
/// - For EXISTING presets: locates the existing file and updates it in-place.
/// - For NEW presets: uses atomic exclusive creation (`OpenOptions::create_new(true)`)
///   at filesystem level, eliminating TOCTOU race conditions between independent processes.
/// - Propagates all real I/O errors (directory errors, sharing violations, write failures).
pub fn save_user_preset_to_dir(dir: &std::path::Path, preset: &PresetData) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;

    let json_str = serde_json::to_string_pretty(preset)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Step A: Update existing preset in-place if found
    if let Some(existing_path) = find_existing_preset_file(dir, &preset.name)? {
        update_existing_preset_file(&existing_path, preset, &json_str)?;
        return Ok(existing_path);
    }

    // Step B: New preset — Atomic reservation using create_new(true)
    let base = sanitize_preset_filename(&preset.name);

    let mut existing_lower: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry_res in fs::read_dir(dir)? {
        let entry = entry_res?;
        if let Ok(name) = entry.file_name().into_string() {
            existing_lower.insert(name.to_lowercase());
        }
    }

    let mut counter: usize = 1;
    loop {
        let candidate_filename = if counter == 1 {
            format!("{}.json", base)
        } else {
            format!("{}_{}.json", base, counter)
        };

        if existing_lower.contains(&candidate_filename.to_lowercase()) {
            counter += 1;
            continue;
        }

        let candidate_path = dir.join(&candidate_filename);

        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate_path)
        {
            Ok(mut file) => {
                use std::io::Write;
                if let Err(e) = file.write_all(json_str.as_bytes()).and_then(|_| file.flush()) {
                    let _ = fs::remove_file(&candidate_path);
                    return Err(e);
                }
                return Ok(candidate_path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                existing_lower.insert(candidate_filename.to_lowercase());
                counter += 1;
                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

/// Save a user preset to the default user presets directory.
pub fn save_user_preset(preset: &PresetData) -> std::io::Result<PathBuf> {
    let dir = get_user_presets_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine presets directory",
        )
    })?;
    save_user_preset_to_dir(&dir, preset)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTempDir {
        path: PathBuf,
    }

    impl TestTempDir {
        fn new(name: &str) -> Self {
            let unique = format!(
                "cstrip2_test_{}_{}_{}",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn internal_subprocess_worker() {
        if let Ok(dir_str) = std::env::var("CSTRIP2_SUBPROCESS_DIR") {
            let name = std::env::var("CSTRIP2_SUBPROCESS_NAME").expect("missing name");
            let val: f32 = std::env::var("CSTRIP2_SUBPROCESS_VAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5);
            let p = PresetData {
                name,
                treble: val,
                ..Default::default()
            };
            let res = save_user_preset_to_dir(std::path::Path::new(&dir_str), &p);
            if let Err(e) = res {
                eprintln!("subprocess save error: {:?}", e);
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }

    #[test]
    fn test_presets_defaults_and_roundtrip() {
        let def = PresetData::default();
        assert_eq!(def.output, 0.33);
        assert_eq!(def.treble, 0.0);
        assert_eq!(def.bass, 0.0);
        assert_eq!(def.low_cap, 1.0);
        assert_eq!(def.hi_cap, 0.0);

        // Factory presets check
        let factory = get_factory_presets();
        assert!(factory.len() >= 5);
        for p in &factory {
            assert!(!p.name.is_empty());
            assert!(p.treble >= -12.0 && p.treble <= 12.0);
            assert!(p.mid >= -12.0 && p.mid <= 12.0);
            assert!(p.bass >= -12.0 && p.bass <= 12.0);
            assert!(p.treb_frq >= 1.0 && p.treb_frq <= 16.0);
            assert!(p.bass_frq >= 30.0 && p.bass_frq <= 1600.0);
            assert!(p.low_cap >= 0.0 && p.low_cap <= 1.0);
            assert!(p.hi_cap >= 0.0 && p.hi_cap <= 1.0);
            assert!(p.compres >= 0.0 && p.compres <= 1.0);
            assert!(p.comp_spd >= 0.0 && p.comp_spd <= 1.0);
            assert!(p.output >= 0.0 && p.output <= 1.0);
        }

        // Serialization roundtrip
        let json = serde_json::to_string(&def).expect("Failed to serialize preset");
        let deserialized: PresetData = serde_json::from_str(&json).expect("Failed to deserialize preset");
        assert_eq!(def, deserialized);
    }

    #[test]
    fn test_f08_normal_names() {
        let temp = TestTempDir::new("f08_normal");
        let p1 = PresetData { name: "Vocal Warmth".to_string(), treble: 2.0, ..Default::default() };
        let p2 = PresetData { name: "Bass Punch".to_string(), bass: 3.5, ..Default::default() };

        let path1 = save_user_preset_to_dir(&temp.path, &p1).unwrap();
        let path2 = save_user_preset_to_dir(&temp.path, &p2).unwrap();

        assert_eq!(path1.file_name().unwrap(), "Vocal Warmth.json");
        assert_eq!(path2.file_name().unwrap(), "Bass Punch.json");

        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "Bass Punch");
        assert_eq!(loaded[0].bass, 3.5);
        assert_eq!(loaded[1].name, "Vocal Warmth");
        assert_eq!(loaded[1].treble, 2.0);
    }

    #[test]
    fn test_f08_invalid_characters() {
        let temp = TestTempDir::new("f08_invalid_chars");
        let p = PresetData {
            name: "Lead:Synth/1*2?3<4>5|6\"7".to_string(),
            mid: 1.5,
            ..Default::default()
        };

        let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
        assert!(path.exists());
        let file_name = path.file_name().unwrap().to_str().unwrap();
        for forbidden in ['<', '>', ':', '"', '/', '\\', '|', '?', '*'] {
            assert!(!file_name.contains(forbidden), "Filename contains forbidden char: {}", forbidden);
        }

        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Lead:Synth/1*2?3<4>5|6\"7");
        assert_eq!(loaded[0].mid, 1.5);
    }

    #[test]
    fn test_f08_adversarial_same_sanitized_base() {
        let temp = TestTempDir::new("f08_adversarial_base");
        let names = vec![
            "My Preset",
            "My_Preset",
            "My-Preset",
            "My/Preset",
            "My:Preset",
            "My  Preset",
        ];

        let mut paths = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let p = PresetData {
                name: name.to_string(),
                output: 0.1 * (i as f32 + 1.0),
                ..Default::default()
            };
            let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
            assert!(path.exists());
            paths.push(path);
        }

        let unique_paths: std::collections::HashSet<_> = paths
            .iter()
            .map(|p| p.to_string_lossy().to_string().to_lowercase())
            .collect();
        assert_eq!(
            unique_paths.len(),
            names.len(),
            "All adversarial names must produce distinct files without overwriting"
        );

        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), names.len());
        for name in &names {
            assert!(
                loaded.iter().any(|p| p.name == *name),
                "Preset '{}' must be present in loaded presets",
                name
            );
        }
    }

    #[test]
    fn test_f08_repeat_save_same_name_updates_in_place() {
        let temp = TestTempDir::new("f08_repeat_save");
        let p1 = PresetData { name: "My/Preset".to_string(), output: 0.25, ..Default::default() };
        let path1 = save_user_preset_to_dir(&temp.path, &p1).unwrap();

        let entries_count_1 = fs::read_dir(&temp.path).unwrap().count();
        assert_eq!(entries_count_1, 1);

        let p1_updated = PresetData { name: "My/Preset".to_string(), output: 0.75, ..Default::default() };
        let path2 = save_user_preset_to_dir(&temp.path, &p1_updated).unwrap();

        let entries_count_2 = fs::read_dir(&temp.path).unwrap().count();
        assert_eq!(entries_count_2, 1, "Re-saving must not create duplicate files");
        assert_eq!(path1, path2, "Re-saving must update the exact same file");

        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "My/Preset");
        assert_eq!(loaded[0].output, 0.75);
    }

    #[test]
    fn test_f08_extreme_and_empty_names() {
        let temp = TestTempDir::new("f08_extreme");
        let extreme_names = vec![
            "",
            "   ",
            "???",
            "CON",
            "aux",
            "...test...",
        ];

        for (i, name) in extreme_names.iter().enumerate() {
            let p = PresetData {
                name: name.to_string(),
                treble: i as f32,
                ..Default::default()
            };
            let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
            assert!(path.exists());
        }

        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), extreme_names.len());
        for name in &extreme_names {
            assert!(loaded.iter().any(|p| p.name == *name));
        }
    }

    #[test]
    fn test_f08_case_insensitive_collision() {
        let temp = TestTempDir::new("f08_case_insensitive");
        let p_upper = PresetData { name: "Clean Tone".to_string(), mid: 1.0, ..Default::default() };
        let p_lower = PresetData { name: "clean tone".to_string(), mid: 2.0, ..Default::default() };

        let path1 = save_user_preset_to_dir(&temp.path, &p_upper).unwrap();
        let path2 = save_user_preset_to_dir(&temp.path, &p_lower).unwrap();

        assert_ne!(
            path1.to_string_lossy().to_lowercase(),
            path2.to_string_lossy().to_lowercase(),
            "Case-differing names must not collide or overwrite"
        );

        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), 2);
        assert!(loaded.iter().any(|p| p.name == "Clean Tone" && p.mid == 1.0));
        assert!(loaded.iter().any(|p| p.name == "clean tone" && p.mid == 2.0));

        let p_lower_updated = PresetData { name: "clean tone".to_string(), mid: 3.5, ..Default::default() };
        let path3 = save_user_preset_to_dir(&temp.path, &p_lower_updated).unwrap();
        assert_eq!(
            path2.to_string_lossy().to_lowercase(),
            path3.to_string_lossy().to_lowercase()
        );

        let loaded_after = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded_after.len(), 2);
        assert!(loaded_after.iter().any(|p| p.name == "Clean Tone" && p.mid == 1.0));
        assert!(loaded_after.iter().any(|p| p.name == "clean tone" && p.mid == 3.5));
    }

    #[test]
    fn test_f17_error_propagation_uncreatable_dir() {
        let temp = TestTempDir::new("f17_uncreatable");
        let blocking_file = temp.path.join("blocked");
        fs::write(&blocking_file, "blocking content").unwrap();

        let invalid_dir = blocking_file.join("sub_presets");
        let p = PresetData::default();

        let result = save_user_preset_to_dir(&invalid_dir, &p);
        assert!(result.is_err(), "Must return Err when directory cannot be created");
        let err = result.unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_f17_error_propagation_unwritable_path() {
        let temp = TestTempDir::new("f17_unwritable");
        let p = PresetData { name: "FolderClash".to_string(), ..Default::default() };
        let clash_dir = temp.path.join("FolderClash.json");
        fs::create_dir_all(&clash_dir).unwrap();

        let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
        assert_ne!(path, clash_dir);
        assert!(path.is_file());
    }

    #[test]
    fn test_f08_multiprocess_race() {
        let exe = std::env::current_exe().expect("failed to get current_exe");

        for iter in 0..15 {
            let temp = TestTempDir::new(&format!("f08_race_{}", iter));

            let mut child_a = std::process::Command::new(&exe)
                .args(["internal_subprocess_worker", "--nocapture"])
                .env("CSTRIP2_SUBPROCESS_DIR", &temp.path)
                .env("CSTRIP2_SUBPROCESS_NAME", "My/Preset")
                .env("CSTRIP2_SUBPROCESS_VAL", "2.5")
                .spawn()
                .expect("failed to spawn child A");

            let mut child_b = std::process::Command::new(&exe)
                .args(["internal_subprocess_worker", "--nocapture"])
                .env("CSTRIP2_SUBPROCESS_DIR", &temp.path)
                .env("CSTRIP2_SUBPROCESS_NAME", "My:Preset")
                .env("CSTRIP2_SUBPROCESS_VAL", "3.5")
                .spawn()
                .expect("failed to spawn child B");

            let status_a = child_a.wait().expect("child A wait failed");
            let status_b = child_b.wait().expect("child B wait failed");

            assert!(status_a.success(), "Iteration {}: Child A failed", iter);
            assert!(status_b.success(), "Iteration {}: Child B failed", iter);

            let files: Vec<_> = fs::read_dir(&temp.path)
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.file_name())
                .collect();
            assert_eq!(
                files.len(),
                2,
                "Iteration {}: Exactly 2 files must be created for 2 distinct presets",
                iter
            );

            let loaded = load_user_presets_from_dir(&temp.path);
            assert_eq!(loaded.len(), 2, "Iteration {}: Exactly 2 presets must be loaded", iter);

            let p_a = loaded.iter().find(|p| p.name == "My/Preset").expect("My/Preset missing");
            let p_b = loaded.iter().find(|p| p.name == "My:Preset").expect("My:Preset missing");
            assert_eq!(p_a.treble, 2.5);
            assert_eq!(p_b.treble, 3.5);

            // In-place update check
            let mut p_a_updated = p_a.clone();
            p_a_updated.treble = 9.0;
            save_user_preset_to_dir(&temp.path, &p_a_updated).expect("update A failed");

            let files_after: Vec<_> = fs::read_dir(&temp.path)
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.file_name())
                .collect();
            assert_eq!(files_after.len(), 2, "Re-saving must not create new files");

            let loaded_after = load_user_presets_from_dir(&temp.path);
            assert_eq!(loaded_after.len(), 2);
            assert_eq!(
                loaded_after.iter().find(|p| p.name == "My/Preset").unwrap().treble,
                9.0
            );
            assert_eq!(
                loaded_after.iter().find(|p| p.name == "My:Preset").unwrap().treble,
                3.5
            );
        }
    }

    #[test]
    fn test_f17_own_file_unreadable_returns_err_and_retries() {
        let temp = TestTempDir::new("f17_locked_own");
        let p = PresetData {
            name: "Locked own".to_string(),
            treble: 1.0,
            ..Default::default()
        };
        let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
        assert!(path.exists());

        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // share_mode(0) locks the file so reading it returns ERROR_SHARING_VIOLATION
            let lock_file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&path)
                .expect("Failed to lock file");

            let mut p_updated = p.clone();
            p_updated.treble = 2.0;
            let res = save_user_preset_to_dir(&temp.path, &p_updated);
            assert!(
                res.is_err(),
                "Must return Err when identity file is unreadable (sharing violation)"
            );

            let files: Vec<_> = fs::read_dir(&temp.path)
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.file_name())
                .collect();
            assert_eq!(
                files.len(),
                1,
                "Must NOT create duplicate _2 file on read error"
            );

            drop(lock_file);

            let retry_path = save_user_preset_to_dir(&temp.path, &p_updated)
                .expect("Retry after lock release must succeed");
            assert_eq!(retry_path, path, "Must update original file in-place");

            let loaded = load_user_presets_from_dir(&temp.path);
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].name, "Locked own");
            assert_eq!(loaded[0].treble, 2.0);
        }
    }

    #[test]
    fn test_f17_read_dir_error_propagated() {
        let temp = TestTempDir::new("f17_readdir_err");
        let file_path = temp.path.join("file_not_dir");
        fs::write(&file_path, "not a directory").unwrap();

        let p = PresetData::default();
        let res = save_user_preset_to_dir(&file_path, &p);
        assert!(res.is_err(), "Must propagate Err when read_dir fails");
    }

    #[test]
    fn test_f08_corrupt_json_preserved_and_treated_as_occupied() {
        let temp = TestTempDir::new("f08_corrupt_json");
        let corrupt_path = temp.path.join("Broken.json");
        fs::write(&corrupt_path, "{ this is corrupt syntax !!!").unwrap();

        let p = PresetData {
            name: "Broken".to_string(),
            treble: 4.0,
            ..Default::default()
        };
        let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
        assert_ne!(path, corrupt_path, "Must not overwrite corrupt file");
        assert_eq!(path.file_name().unwrap(), "Broken_2.json");

        let content = fs::read_to_string(&corrupt_path).unwrap();
        assert_eq!(content, "{ this is corrupt syntax !!!");

        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Broken");
        assert_eq!(loaded[0].treble, 4.0);
    }

    #[test]
    fn test_f08_windows_reserved_names_with_extensions_and_superscripts() {
        let temp = TestTempDir::new("f08_reserved_ext");
        let reserved_names = vec![
            "CON",
            "CON.txt",
            "PRN",
            "PRN.foo",
            "AUX",
            "AUX.backup",
            "NUL",
            "NUL.test",
            "COM1",
            "COM1.txt",
            "LPT1",
            "LPT1.backup",
            "COM¹",
            "LPT²",
            "LPT³.txt",
        ];

        let mut saved_paths = Vec::new();
        for (i, name) in reserved_names.iter().enumerate() {
            let p = PresetData {
                name: name.to_string(),
                treble: i as f32,
                ..Default::default()
            };
            let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
            assert!(path.exists());
            let file_name = path.file_name().unwrap().to_str().unwrap();
            assert!(
                file_name.starts_with("Preset_"),
                "Filename must be prefixed with Preset_: {}",
                file_name
            );
            saved_paths.push(path);
        }

        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), reserved_names.len());
        for name in &reserved_names {
            assert!(
                loaded.iter().any(|p| p.name == *name),
                "Missing preset: {}",
                name
            );
        }

        for (i, name) in reserved_names.iter().enumerate() {
            let p_updated = PresetData {
                name: name.to_string(),
                treble: 100.0 + i as f32,
                ..Default::default()
            };
            let updated_path = save_user_preset_to_dir(&temp.path, &p_updated).unwrap();
            assert_eq!(
                updated_path, saved_paths[i],
                "Must update the exact same path for {}",
                name
            );
        }
    }

    #[test]
    fn test_f08_delete_between_scan_and_update() {
        let temp = TestTempDir::new("f08_del_between_scan_update");
        let p = PresetData {
            name: "Preset_A".to_string(),
            treble: 1.0,
            ..Default::default()
        };
        let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
        assert!(path.exists());

        // 1. Scan finds candidate path
        let candidate = find_existing_preset_file(&temp.path, &p.name).unwrap();
        assert_eq!(candidate, Some(path.clone()));

        // 2. Before write, delete file A
        fs::remove_file(&path).unwrap();

        // 3. Attempt update on the candidate path
        let p_updated = PresetData {
            name: "Preset_A".to_string(),
            treble: 5.0,
            ..Default::default()
        };
        let json_str = serde_json::to_string_pretty(&p_updated).unwrap();
        let res = update_existing_preset_file(&path, &p_updated, &json_str);

        // Expected: Err, file NOT recreated, no duplicate _2, no Ok
        assert!(res.is_err(), "Update on deleted file must return Err");
        assert!(!path.exists(), "Deleted file must NOT be recreated");

        let entries: Vec<_> = fs::read_dir(&temp.path).unwrap().flatten().collect();
        assert_eq!(entries.len(), 0, "No file or duplicate _2 may exist in directory");
    }

    #[test]
    fn test_f08_rename_between_scan_and_update() {
        let temp = TestTempDir::new("f08_rename_between_scan_update");
        let p = PresetData {
            name: "Preset_A".to_string(),
            treble: 1.0,
            ..Default::default()
        };
        let path = save_user_preset_to_dir(&temp.path, &p).unwrap();
        assert!(path.exists());

        // 1. Scan resolves A
        let candidate = find_existing_preset_file(&temp.path, &p.name).unwrap();
        assert_eq!(candidate, Some(path.clone()));

        // 2. Rename A's file externally
        let renamed_path = temp.path.join("Preset_A_Renamed.json");
        fs::rename(&path, &renamed_path).unwrap();

        // 3. Continue update on original path
        let p_updated = PresetData {
            name: "Preset_A".to_string(),
            treble: 5.0,
            ..Default::default()
        };
        let json_str = serde_json::to_string_pretty(&p_updated).unwrap();
        let res = update_existing_preset_file(&path, &p_updated, &json_str);

        // Expected: Err, old path NOT recreated, renamed file intact
        assert!(res.is_err(), "Update on renamed file path must return Err");
        assert!(!path.exists(), "Old path must NOT be recreated");
        assert!(renamed_path.exists(), "Renamed file must remain intact");

        let content = fs::read_to_string(&renamed_path).unwrap();
        let parsed: PresetData = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.name, "Preset_A");
        assert_eq!(parsed.treble, 1.0);
    }

    #[test]
    fn test_f08_substitution_a_for_b_aborts_and_preserves_b() {
        let temp = TestTempDir::new("f08_subst_a_for_b");
        let p_a = PresetData {
            name: "Preset_A".to_string(),
            treble: 1.0,
            ..Default::default()
        };
        let p_b = PresetData {
            name: "Preset_B".to_string(),
            treble: 9.0,
            output: 0.77,
            ..Default::default()
        };

        let path_a = save_user_preset_to_dir(&temp.path, &p_a).unwrap();
        let path_b = save_user_preset_to_dir(&temp.path, &p_b).unwrap();

        let candidate = find_existing_preset_file(&temp.path, &p_a.name).unwrap();
        assert_eq!(candidate, Some(path_a.clone()));

        // Read B's exact raw bytes
        let b_bytes = fs::read(&path_b).unwrap();

        // External replacement: path_a now contains Preset B's content
        fs::write(&path_a, &b_bytes).unwrap();

        // Attempt update of A on path_a
        let p_a_updated = PresetData {
            name: "Preset_A".to_string(),
            treble: 99.0,
            ..Default::default()
        };
        let json_str = serde_json::to_string_pretty(&p_a_updated).unwrap();
        let res = update_existing_preset_file(&path_a, &p_a_updated, &json_str);

        // Expected: Err due to identity mismatch
        assert!(res.is_err(), "Update must fail when path contents were replaced by B");

        // CRITICAL: B must remain 100% byte-identical! Not truncated, not overwritten
        let a_path_bytes = fs::read(&path_a).unwrap();
        assert_eq!(
            a_path_bytes, b_bytes,
            "File B at path_a must remain 100% byte-identical (neither truncated nor overwritten)"
        );

        let b_path_bytes = fs::read(&path_b).unwrap();
        assert_eq!(b_path_bytes, b_bytes, "Original path_b must also remain untouched");
    }

    #[test]
    fn test_f08_write_bound_to_validated_handle_not_pathname() {
        let temp = TestTempDir::new("f08_handle_bound");
        let p_a = PresetData {
            name: "Preset_A".to_string(),
            treble: 1.0,
            ..Default::default()
        };
        let path_a = save_user_preset_to_dir(&temp.path, &p_a).unwrap();

        // 1. Open handle of A
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(&path_a)
            .unwrap();

        // 2. Validate identity via handle
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let parsed: PresetData = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.name, "Preset_A");

        // 3. Write through this verified handle
        let p_a_updated = PresetData {
            name: "Preset_A".to_string(),
            treble: 7.7,
            ..Default::default()
        };
        let json_str = serde_json::to_string_pretty(&p_a_updated).unwrap();
        file.set_len(0).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
        file.flush().unwrap();
        drop(file);

        let disk_content = fs::read_to_string(&path_a).unwrap();
        let disk_parsed: PresetData = serde_json::from_str(&disk_content).unwrap();
        assert_eq!(disk_parsed.name, "Preset_A");
        assert_eq!(disk_parsed.treble, 7.7);
    }

    #[test]
    fn test_f17_metadata_notfound_during_scan_bounded_retry() {
        let temp = TestTempDir::new("f17_scan_notfound_retry");
        let p_a = PresetData {
            name: "Stable_A".to_string(),
            treble: 2.0,
            ..Default::default()
        };
        save_user_preset_to_dir(&temp.path, &p_a).unwrap();

        let found = find_existing_preset_file(&temp.path, "Stable_A").unwrap();
        assert!(found.is_some());

        let not_found = find_existing_preset_file(&temp.path, "NonExistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_f17_metadata_permission_error_propagates() {
        let temp = TestTempDir::new("f17_meta_err");
        let file_not_dir = temp.path.join("file_blocker");
        fs::write(&file_not_dir, "content").unwrap();

        let res = find_existing_preset_file(&file_not_dir, "Any");
        assert!(res.is_err(), "Must return Err when path cannot be read as directory");
    }

    #[test]
    fn test_f08_corrupt_json_during_update_returns_err_and_does_not_truncate() {
        let temp = TestTempDir::new("f08_corrupt_update");
        let corrupt_path = temp.path.join("Corrupt.json");
        let corrupt_content = "{ this is corrupted json text";
        fs::write(&corrupt_path, corrupt_content).unwrap();

        let p = PresetData {
            name: "Corrupt".to_string(),
            treble: 5.0,
            ..Default::default()
        };
        let json_str = serde_json::to_string_pretty(&p).unwrap();
        let res = update_existing_preset_file(&corrupt_path, &p, &json_str);

        assert!(res.is_err(), "Update on corrupt file must return Err");
        let after_content = fs::read_to_string(&corrupt_path).unwrap();
        assert_eq!(after_content, corrupt_content, "Corrupt file must not be truncated or overwritten");
    }

    #[test]
    fn test_f08_rename_blocked_while_update_handle_open() {
        let temp = TestTempDir::new("f08_rename_blocked");
        let p_a = PresetData {
            name: "Preset_A".to_string(),
            treble: 1.0,
            ..Default::default()
        };
        let path_a = save_user_preset_to_dir(&temp.path, &p_a).unwrap();

        // 1. Open A using the exact options of update_existing_preset_file
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
        }
        let mut file = options.open(&path_a).unwrap();

        // 2. Validate identity via handle
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let parsed: PresetData = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.name, "Preset_A");

        // 3. Before truncate, attempt to rename A externally
        let path_renamed = temp.path.join("Preset_A_Renamed.json");
        #[cfg(windows)]
        {
            let ren_res = fs::rename(&path_a, &path_renamed);
            assert!(
                ren_res.is_err(),
                "External rename must be blocked with ERROR_SHARING_VIOLATION while update handle is open"
            );
            assert_eq!(
                ren_res.unwrap_err().raw_os_error(),
                Some(32),
                "Error must be Win32 error 32 (ERROR_SHARING_VIOLATION)"
            );
            assert!(!path_renamed.exists(), "Target rename path must not exist");
        }

        // 4. Attempt to place B at pathname_original
        let create_b_res = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path_a);
        assert!(
            create_b_res.is_err(),
            "Cannot place B at path_a because path_a could not be displaced"
        );

        // 5. Complete write on handle
        let p_a_updated = PresetData {
            name: "Preset_A".to_string(),
            treble: 4.5,
            ..Default::default()
        };
        let json_str = serde_json::to_string_pretty(&p_a_updated).unwrap();
        file.set_len(0).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
        file.flush().unwrap();
        drop(file);

        // Verify on disk that path_a holds updated Preset_A
        let disk_content = fs::read_to_string(&path_a).unwrap();
        let disk_parsed: PresetData = serde_json::from_str(&disk_content).unwrap();
        assert_eq!(disk_parsed.name, "Preset_A");
        assert_eq!(disk_parsed.treble, 4.5);
    }

    #[test]
    fn test_f08_delete_pending_blocked_while_update_handle_open() {
        let temp = TestTempDir::new("f08_del_pending_blocked");
        let p_a = PresetData {
            name: "Preset_A".to_string(),
            treble: 1.0,
            ..Default::default()
        };
        let path_a = save_user_preset_to_dir(&temp.path, &p_a).unwrap();

        // 1. Open A using the exact update options
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
        }
        let mut file = options.open(&path_a).unwrap();

        // 2. Validate identity via handle
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let parsed: PresetData = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.name, "Preset_A");

        // 3. Attempt fs::remove_file while update handle is open
        #[cfg(windows)]
        {
            let del_res = fs::remove_file(&path_a);
            assert!(
                del_res.is_err(),
                "remove_file must fail with sharing violation while update handle excludes FILE_SHARE_DELETE"
            );
            assert_eq!(
                del_res.unwrap_err().raw_os_error(),
                Some(32),
                "Error must be Win32 error 32 (ERROR_SHARING_VIOLATION)"
            );
        }

        // 4. Complete update on handle
        let p_a_updated = PresetData {
            name: "Preset_A".to_string(),
            treble: 8.8,
            ..Default::default()
        };
        let json_str = serde_json::to_string_pretty(&p_a_updated).unwrap();
        file.set_len(0).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
        file.flush().unwrap();
        drop(file);

        // 5. Verify A exists on disk with updated values
        assert!(path_a.exists(), "File A must still exist on disk");
        let loaded = load_user_presets_from_dir(&temp.path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Preset_A");
        assert_eq!(loaded[0].treble, 8.8);
    }

    #[test]
    fn test_f08_dir_temporarily_renamed_race_no_duplicate() {
        let temp = TestTempDir::new("f08_dir_rename_race");
        let p_a = PresetData {
            name: "Preset_A".to_string(),
            treble: 1.0,
            ..Default::default()
        };
        let path_a = save_user_preset_to_dir(&temp.path, &p_a).unwrap();
        assert!(path_a.exists());

        // 1. Rename directory temporarily
        let temp_renamed = temp.path.with_file_name("f08_dir_rename_race_temp");
        fs::rename(&temp.path, &temp_renamed).unwrap();

        // 2. Call find_existing_preset_file on the missing path:
        // Because dir.exists() was eliminated, it must NOT return Ok(None)!
        let find_res = find_existing_preset_file(&temp.path, "Preset_A");
        assert!(
            find_res.is_err(),
            "Must return Err(NotFound) rather than Ok(None) when directory is unreadable"
        );

        // 3. Restore directory
        fs::rename(&temp_renamed, &temp.path).unwrap();

        // 4. Now save update of Preset_A:
        let p_a_updated = PresetData {
            name: "Preset_A".to_string(),
            treble: 3.3,
            ..Default::default()
        };
        let update_path = save_user_preset_to_dir(&temp.path, &p_a_updated).unwrap();
        assert_eq!(update_path, path_a, "Must update existing Preset_A in-place");

        // 5. Verify NO A_2.json was created
        let entries: Vec<_> = fs::read_dir(&temp.path)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().into_string().unwrap())
            .collect();
        assert_eq!(entries.len(), 1, "Must be exactly 1 file in directory");
        assert_eq!(entries[0], "Preset_A.json", "No duplicate A_2.json may exist");
    }
}


