use crate::animation::ClipDef;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, btree_map::Entry};

/// Bounds of one named Aseprite slice key within a sprite-sheet frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AsepriteSliceBounds {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Sparse named-slice bounds keyed by flat sprite-sheet frame index.
#[derive(Debug, Clone, PartialEq)]
pub struct AsepriteSliceImport {
    pub frames: BTreeMap<usize, AsepriteSliceBounds>,
    pub warnings: Vec<AsepriteSliceWarning>,
}

/// Non-fatal issue encountered while extracting named slice bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsepriteSliceWarning {
    DuplicateFrame { frame_index: usize },
    FrameOutOfRange { frame_index: usize },
    InvalidBounds { frame_index: usize },
}

/// Fatal named-slice import error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsepriteSliceImportError {
    ParseFailed(String),
}

#[derive(Deserialize)]
struct AsepriteSliceJson {
    meta: AsepriteSliceMetaJson,
}

#[derive(Deserialize)]
struct AsepriteSliceMetaJson {
    #[serde(default)]
    slices: Vec<AsepriteSliceJsonEntry>,
}

#[derive(Deserialize)]
struct AsepriteSliceJsonEntry {
    name: String,
    #[serde(default)]
    keys: Vec<AsepriteSliceKeyJson>,
}

#[derive(Deserialize)]
struct AsepriteSliceKeyJson {
    frame: usize,
    bounds: AsepriteSliceBoundsJson,
}

#[derive(Deserialize)]
struct AsepriteSliceBoundsJson {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Extracts sparse named slice bounds from Aseprite slice JSON.
pub fn extract_named_aseprite_slice_bounds(
    json: &str,
    clip: &ClipDef,
    slice_name: &str,
) -> Result<AsepriteSliceImport, AsepriteSliceImportError> {
    let parsed: AsepriteSliceJson = serde_json::from_str(json)
        .map_err(|err| AsepriteSliceImportError::ParseFailed(err.to_string()))?;
    let mut frames = BTreeMap::new();
    let mut warnings = Vec::new();
    let cols = clip.cols.max(1);
    let frame_count = cols.saturating_mul(clip.rows.max(1));

    for slice in parsed.meta.slices {
        if !slice.name.eq_ignore_ascii_case(slice_name) {
            continue;
        }

        for key in slice.keys {
            if key.frame >= frame_count {
                warnings.push(AsepriteSliceWarning::FrameOutOfRange {
                    frame_index: key.frame,
                });
                continue;
            }

            let Some(bounds) = validated_bounds(key.bounds, clip) else {
                warnings.push(AsepriteSliceWarning::InvalidBounds {
                    frame_index: key.frame,
                });
                continue;
            };

            if let Entry::Vacant(entry) = frames.entry(key.frame) {
                entry.insert(bounds);
            } else {
                warnings.push(AsepriteSliceWarning::DuplicateFrame {
                    frame_index: key.frame,
                });
            }
        }
    }

    Ok(AsepriteSliceImport { frames, warnings })
}

fn validated_bounds(bounds: AsepriteSliceBoundsJson, clip: &ClipDef) -> Option<AsepriteSliceBounds> {
    let frame_width = f64::from(clip.frame_size.x);
    let frame_height = f64::from(clip.frame_size.y);
    let right = bounds.x + bounds.w;
    let bottom = bounds.y + bounds.h;

    if !bounds.x.is_finite()
        || !bounds.y.is_finite()
        || !bounds.w.is_finite()
        || !bounds.h.is_finite()
        || !frame_width.is_finite()
        || !frame_height.is_finite()
        || frame_width <= 0.0
        || frame_height <= 0.0
        || bounds.x < 0.0
        || bounds.y < 0.0
        || bounds.w <= 0.0
        || bounds.h <= 0.0
        || !right.is_finite()
        || !bottom.is_finite()
        || right > frame_width
        || bottom > frame_height
    {
        return None;
    }

    let converted = AsepriteSliceBounds {
        x: bounds.x as f32,
        y: bounds.y as f32,
        w: bounds.w as f32,
        h: bounds.h as f32,
    };

    (converted.w > 0.0 && converted.h > 0.0)
    .then_some(converted)
}

#[cfg(test)]
mod tests {
    use super::{AsepriteSliceBounds, extract_named_aseprite_slice_bounds};
    use crate::animation::ClipDef;
    use bishop::prelude::vec2;

    fn clip() -> ClipDef {
        ClipDef {
            cols: 2,
            rows: 2,
            frame_size: vec2(100.0, 80.0),
            ..Default::default()
        }
    }

    fn slice_json(extra_slices: &str) -> String {
        format!(
            r##"{{
                "frames": {{}},
                "meta": {{
                    "image": "Run.png",
                    "size": {{ "w": 200, "h": 160 }},
                    "slices": [{}]
                }}
            }}"##,
            extra_slices,
        )
    }

    #[test]
    fn aseprite_slice_import_maps_named_bounds_to_flat_frame_index() {
        let json = slice_json(
            r##"{
                "name": "glow",
                "keys": [
                    { "frame": 1, "bounds": { "x": 10, "y": 20, "w": 30, "h": 40 } }
                ]
            }"##,
        );

        let imported = extract_named_aseprite_slice_bounds(&json, &clip(), "glow")
            .expect("valid slice metadata should parse");

        assert_eq!(
            imported.frames.get(&1),
            Some(&AsepriteSliceBounds {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 40.0,
            }),
        );
        assert!(imported.warnings.is_empty());
    }

    #[test]
    fn aseprite_slice_import_matches_name_case_insensitively_and_ignores_other_slices() {
        let json = slice_json(
            r##"{
                "name": "selection",
                "keys": [
                    { "frame": 0, "bounds": { "x": 0, "y": 0, "w": 10, "h": 10 } }
                ]
            },
            {
                "name": "Glow",
                "keys": [
                    { "frame": 2, "bounds": { "x": 1, "y": 2, "w": 3, "h": 4 } }
                ]
            }"##,
        );

        let imported = extract_named_aseprite_slice_bounds(&json, &clip(), "glow")
            .expect("valid slice metadata should parse");

        assert!(imported.frames.contains_key(&2));
        assert!(!imported.frames.contains_key(&0));
        assert!(imported.warnings.is_empty());
    }

    #[test]
    fn aseprite_slice_import_leaves_frames_absent_when_name_is_missing() {
        let json = slice_json(
            r##"{
                "name": "selection",
                "keys": [
                    { "frame": 0, "bounds": { "x": 0, "y": 0, "w": 10, "h": 10 } }
                ]
            }"##,
        );

        let imported = extract_named_aseprite_slice_bounds(&json, &clip(), "glow")
            .expect("valid metadata without a glow slice should parse");

        assert!(imported.frames.is_empty());
        assert!(imported.warnings.is_empty());
    }

    #[test]
    fn aseprite_slice_import_duplicate_frame_keeps_first_and_warns() {
        let json = slice_json(
            r##"{
                "name": "glow",
                "keys": [
                    { "frame": 0, "bounds": { "x": 1, "y": 2, "w": 3, "h": 4 } },
                    { "frame": 0, "bounds": { "x": 9, "y": 9, "w": 9, "h": 9 } }
                ]
            }"##,
        );

        let imported = extract_named_aseprite_slice_bounds(&json, &clip(), "glow")
            .expect("duplicate slice keys should parse with a warning");

        assert_eq!(
            imported.frames.get(&0),
            Some(&AsepriteSliceBounds {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            }),
        );
        assert_eq!(imported.warnings.len(), 1);
    }

    #[test]
    fn aseprite_slice_import_skips_out_of_range_and_invalid_bounds_with_warnings() {
        let json = slice_json(
            r##"{
                "name": "glow",
                "keys": [
                    { "frame": 8, "bounds": { "x": 1, "y": 2, "w": 3, "h": 4 } },
                    { "frame": 0, "bounds": { "x": 1, "y": 2, "w": 0, "h": 4 } },
                    { "frame": 1, "bounds": { "x": 1e300, "y": 2, "w": 3, "h": 4 } },
                    { "frame": 2, "bounds": { "x": 1, "y": 2, "w": 1e-300, "h": 4 } }
                ]
            }"##,
        );

        let imported = extract_named_aseprite_slice_bounds(&json, &clip(), "glow")
            .expect("invalid frame bounds should be skipped with warnings");

        assert!(imported.frames.is_empty());
        assert_eq!(imported.warnings.len(), 4);
    }
}
