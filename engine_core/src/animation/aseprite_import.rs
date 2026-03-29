// engine_core/src/animation/aseprite_import.rs

use crate::animation::animation_clip::*;
use crate::storage::path_utils::assets_folder;
use bishop::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Result of attempting to import Aseprite JSON metadata.
pub enum JsonImportResult {
    /// Successfully parsed the JSON and created a ClipDef.
    Success(ClipDef),
    /// The JSON file was not found at the expected path.
    NotFound,
    /// An error occurred while parsing or processing the JSON.
    Error(String),
}

/// Aseprite frame rectangle for sprite source size.
#[derive(Deserialize)]
struct AseSpriteSourceSize {
    x: i32,
    y: i32,
}

/// Aseprite size.
#[derive(Deserialize)]
struct AseSize {
    w: i32,
    h: i32,
}

/// Aseprite frame data.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AseFrame {
    trimmed: bool,
    sprite_source_size: AseSpriteSourceSize,
    source_size: AseSize,
    duration: i32,
}

/// Aseprite meta section.
#[derive(Deserialize)]
struct AseMeta {
    #[allow(dead_code)]
    image: String,
    size: AseSize,
}

/// Aseprite JSON root (json-hash format).
#[derive(Deserialize)]
struct AsepriteJson {
    frames: HashMap<String, AseFrame>,
    meta: AseMeta,
}

/// Parses an Aseprite JSON file and returns a ClipDef with the extracted metadata.
pub fn import_aseprite_metadata(json_path: &Path) -> JsonImportResult {
    if !json_path.exists() {
        return JsonImportResult::NotFound;
    }

    let content = match fs::read_to_string(json_path) {
        Ok(c) => c,
        Err(e) => return JsonImportResult::Error(format!("Failed to read file: {}", e)),
    };

    let ase_json: AsepriteJson = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(e) => return JsonImportResult::Error(format!("Failed to parse JSON: {}", e)),
    };

    if ase_json.frames.is_empty() {
        return JsonImportResult::Error("No frames found in JSON".to_string());
    }

    let mut sorted_frames: Vec<(&String, &AseFrame)> = ase_json.frames.iter().collect();
    sorted_frames.sort_by(|a, b| a.0.cmp(b.0));

    let first_frame = sorted_frames[0].1;
    let frame_w = first_frame.source_size.w as f32;
    let frame_h = first_frame.source_size.h as f32;

    let sheet_w = ase_json.meta.size.w as f32;
    let sheet_h = ase_json.meta.size.h as f32;

    let cols = (sheet_w / frame_w).round() as usize;
    let rows = (sheet_h / frame_h).round() as usize;

    let raw_durations: Vec<i32> = sorted_frames.iter().map(|(_, f)| f.duration).collect();

    // Check if all frames have the same duration
    let all_same = raw_durations.windows(2).all(|w| w[0] == w[1]);

    // Only use per-frame durations if timing varies; otherwise use FPS
    let (frame_durations, fps) = if all_same && !raw_durations.is_empty() {
        let duration_ms = raw_durations[0] as f32;
        let fps = (1000.0 / duration_ms).round();
        (Vec::new(), fps)
    } else {
        let durations: Vec<f32> = raw_durations.iter().map(|&d| d as f32 / 1000.0).collect();
        let total: f32 = durations.iter().sum();
        let avg = total / durations.len() as f32;
        let fps = (1.0 / avg).round();
        (durations, fps)
    };

    let offset = if first_frame.trimmed {
        Vec2::new(
            first_frame.sprite_source_size.x as f32,
            first_frame.sprite_source_size.y as f32,
        )
    } else {
        Vec2::ZERO
    };

    let mirrored = rows == 1;

    let clip_def = ClipDef {
        frame_size: Vec2::new(frame_w, frame_h),
        cols,
        rows,
        fps,
        frame_durations,
        looping: true,
        offset,
        mirrored,
    };

    JsonImportResult::Success(clip_def)
}

/// Resolves the path to the Aseprite JSON file for a given clip.
pub fn resolve_json_path(variant_folder: &VariantFolder, clip_id: &ClipId) -> PathBuf {
    let filename = match clip_id {
        ClipId::Idle => "Idle.json",
        ClipId::Walk => "Walk.json",
        ClipId::Run => "Run.json",
        ClipId::Attack => "Attack.json",
        ClipId::Jump => "Jump.json",
        ClipId::Fall => "Fall.json",
        ClipId::Custom(name) => &format!("{}.json", name),
        ClipId::New => "New.json",
    };
    assets_folder().join(&variant_folder.0).join(filename)
}

/// Result of exporting Aseprite files in a folder.
pub enum AseExportResult {
    /// All files exported successfully.
    Success,
    /// Aseprite CLI was not found in PATH.
    AsepriteNotFound,
    /// Export failed for a specific file.
    ExportFailed { file: String, error: String },
}

/// Export all .ase/.aseprite files in the folder to PNG + JSON using Aseprite CLI.
pub fn export_aseprite_folder(folder: &Path) -> AseExportResult {
    let entries = match fs::read_dir(folder) {
        Ok(e) => e,
        Err(e) => {
            return AseExportResult::ExportFailed {
                file: folder.display().to_string(),
                error: format!("Failed to read directory: {}", e),
            };
        }
    };

    let aseprite_path = find_aseprite_executable();

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_aseprite_file(&path) {
            continue;
        }

        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let png_name = format!("{}.png", stem);
        let json_name = format!("{}.json", stem);

        let output = Command::new(&aseprite_path)
            .args([
                "-b",
                path.to_string_lossy().as_ref(),
                "--sheet",
                &png_name,
                "--format",
                "json-hash",
                "--data",
                &json_name,
            ])
            .current_dir(folder)
            .output();

        match output {
            Ok(result) => {
                if !result.status.success() {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    return AseExportResult::ExportFailed {
                        file: stem,
                        error: stderr.to_string(),
                    };
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return AseExportResult::AsepriteNotFound;
                }
                return AseExportResult::ExportFailed {
                    file: stem,
                    error: e.to_string(),
                };
            }
        }
    }

    AseExportResult::Success
}

/// Find the Aseprite executable, checking common installation paths.
fn find_aseprite_executable() -> String {
    #[cfg(target_os = "macos")]
    {
        let macos_path = "/Applications/Aseprite.app/Contents/MacOS/aseprite";
        if Path::new(macos_path).exists() {
            return macos_path.to_string();
        }
    }

    #[cfg(target_os = "windows")]
    {
        let common_paths = [
            r"C:\Program Files\Aseprite\Aseprite.exe",
            r"C:\Program Files (x86)\Aseprite\Aseprite.exe",
            r"C:\Program Files\Steam\steamapps\common\Aseprite\Aseprite.exe",
        ];
        for path in common_paths {
            if Path::new(path).exists() {
                return path.to_string();
            }
        }
    }

    "aseprite".to_string()
}

/// Returns true if the path has an Aseprite file extension.
fn is_aseprite_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("ase") | Some("aseprite")
    )
}

/// Result of importing a variant folder.
pub struct FolderImportResult {
    /// Successfully imported clips mapped by their ClipId.
    pub clips: HashMap<ClipId, ClipDef>,
    /// Files that failed to parse (non-fatal errors).
    pub skipped: Vec<String>,
}

/// Import all JSON files in a folder and create clips.
/// Missing or malformed JSON files are skipped.
pub fn import_variant_folder(folder: &Path) -> Result<FolderImportResult, String> {
    let entries = fs::read_dir(folder).map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut clips = HashMap::new();
    let mut skipped = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("json") {
            continue;
        }

        let clip_id = filename_to_clip_id(&path);

        match import_aseprite_metadata(&path) {
            JsonImportResult::Success(clip_def) => {
                clips.insert(clip_id, clip_def);
            }
            JsonImportResult::Error(e) => {
                skipped.push(format!("{}: {}", path.display(), e));
            }
            JsonImportResult::NotFound => {}
        }
    }

    Ok(FolderImportResult { clips, skipped })
}

/// Map a JSON filename to the appropriate ClipId.
fn filename_to_clip_id(path: &Path) -> ClipId {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    match stem {
        "Idle" => ClipId::Idle,
        "Walk" => ClipId::Walk,
        "Run" => ClipId::Run,
        "Attack" => ClipId::Attack,
        "Jump" => ClipId::Jump,
        "Fall" => ClipId::Fall,
        other => ClipId::Custom(other.to_string()),
    }
}
