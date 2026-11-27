use crate::animation::{
    AsepriteSliceImportError, AsepriteSliceWarning, ClipDef, extract_named_aseprite_slice_bounds,
};
use crate::ecs::{ColliderData, ColliderFrameKey, Pivot};
use crate::physics::collider_data_from_frame_bounds;
use bishop::prelude::Rect;
use std::collections::BTreeMap;

/// Aseprite slice name used as the primary gameplay collider.
pub const PRIMARY_COLLIDER_SLICE_NAME: &str = "collider";

/// Sparse collider data imported from Aseprite slice metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct AsepriteColliderImport {
    pub frames: BTreeMap<ColliderFrameKey, ColliderData>,
    pub warnings: Vec<AsepriteColliderWarning>,
}

/// Non-fatal collider slice import warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsepriteColliderWarning {
    DuplicatePrimarySlice { frame: ColliderFrameKey },
    SliceFrameOutOfRange { frame_index: usize },
    InvalidSliceBounds { frame_index: usize },
}

/// Fatal collider slice import error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsepriteColliderImportError {
    ParseFailed(String),
}

/// Extracts sparse AABB collider data from Aseprite slice JSON.
pub fn extract_aseprite_collider_slices(
    json: &str,
    clip: &ClipDef,
    pivot: Pivot,
) -> Result<AsepriteColliderImport, AsepriteColliderImportError> {
    let imported = extract_named_aseprite_slice_bounds(json, clip, PRIMARY_COLLIDER_SLICE_NAME)
        .map_err(|error| match error {
            AsepriteSliceImportError::ParseFailed(message) => {
                AsepriteColliderImportError::ParseFailed(message)
            }
        })?;
    let cols = clip.cols.max(1);
    let frames = imported
        .frames
        .into_iter()
        .map(|(frame_index, bounds)| {
            let frame = ColliderFrameKey {
                row: frame_index / cols,
                col: frame_index % cols,
            };
            let data = collider_data_from_frame_bounds(
                clip.frame_size,
                Rect::new(bounds.x, bounds.y, bounds.w, bounds.h),
                pivot,
            );
            (frame, data)
        })
        .collect();
    let warnings = imported
        .warnings
        .into_iter()
        .map(|warning| match warning {
            AsepriteSliceWarning::DuplicateFrame { frame_index } => {
                AsepriteColliderWarning::DuplicatePrimarySlice {
                    frame: ColliderFrameKey {
                        row: frame_index / cols,
                        col: frame_index % cols,
                    },
                }
            }
            AsepriteSliceWarning::FrameOutOfRange { frame_index } => {
                AsepriteColliderWarning::SliceFrameOutOfRange { frame_index }
            }
            AsepriteSliceWarning::InvalidBounds { frame_index } => {
                AsepriteColliderWarning::InvalidSliceBounds { frame_index }
            }
        })
        .collect();

    Ok(AsepriteColliderImport { frames, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::ColliderShape;
    use bishop::prelude::{Vec2, vec2};

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

    fn assert_aabb(data: ColliderData, width: f32, height: f32, offset: Vec2) {
        assert_eq!(data.shape, ColliderShape::Aabb { width, height });
        assert_eq!(data.offset, offset);
    }

    #[test]
    fn aseprite_collider_slice_import_maps_slice_to_frame_data() {
        let json = slice_json(
            r##"{
                "name": "collider",
                "keys": [
                    { "frame": 1, "bounds": { "x": 10, "y": 20, "w": 30, "h": 40 } }
                ]
            }"##,
        );

        let imported = extract_aseprite_collider_slices(&json, &clip(), Pivot::BottomCenter)
            .expect("collider slices should parse");

        assert_eq!(imported.warnings, Vec::new());
        assert_aabb(
            imported.frames[&ColliderFrameKey { row: 0, col: 1 }],
            30.0,
            40.0,
            vec2(-25.0, -20.0),
        );
    }

    #[test]
    fn aseprite_collider_slice_import_matches_name_case_insensitively() {
        let json = slice_json(
            r##"{
                "name": "Collider",
                "keys": [
                    { "frame": 1, "bounds": { "x": 10, "y": 20, "w": 30, "h": 40 } }
                ]
            }"##,
        );

        let imported = extract_aseprite_collider_slices(&json, &clip(), Pivot::BottomCenter)
            .expect("collider slices should parse");

        assert!(imported.frames.contains_key(&ColliderFrameKey { row: 0, col: 1 }));
    }

    #[test]
    fn aseprite_collider_slice_import_leaves_unsliced_frames_absent() {
        let json = slice_json(
            r##"{
                "name": "selection",
                "keys": [
                    { "frame": 0, "bounds": { "x": 0, "y": 0, "w": 10, "h": 10 } }
                ]
            }"##,
        );

        let imported = extract_aseprite_collider_slices(&json, &clip(), Pivot::TopLeft)
            .expect("non-collider slices should parse");

        assert!(imported.frames.is_empty());
        assert!(imported.warnings.is_empty());
    }

    #[test]
    fn aseprite_collider_slice_import_duplicate_primary_warns_and_keeps_first() {
        let json = slice_json(
            r##"{
                "name": "collider",
                "keys": [
                    { "frame": 0, "bounds": { "x": 1, "y": 2, "w": 3, "h": 4 } }
                ]
            },
            {
                "name": "collider",
                "keys": [
                    { "frame": 0, "bounds": { "x": 9, "y": 9, "w": 9, "h": 9 } }
                ]
            }"##,
        );

        let imported = extract_aseprite_collider_slices(&json, &clip(), Pivot::TopLeft)
            .expect("duplicate collider slices should parse with warning");

        assert_eq!(
            imported.warnings,
            vec![AsepriteColliderWarning::DuplicatePrimarySlice {
                frame: ColliderFrameKey { row: 0, col: 0 },
            }],
        );
        assert_aabb(
            imported.frames[&ColliderFrameKey { row: 0, col: 0 }],
            3.0,
            4.0,
            vec2(1.0, 2.0),
        );
    }

    #[test]
    fn aseprite_collider_slice_import_out_of_range_frame_warns_and_skips() {
        let json = slice_json(
            r##"{
                "name": "collider",
                "keys": [
                    { "frame": 9, "bounds": { "x": 1, "y": 2, "w": 3, "h": 4 } }
                ]
            }"##,
        );

        let imported = extract_aseprite_collider_slices(&json, &clip(), Pivot::TopLeft)
            .expect("out of range keys should parse with warning");

        assert!(imported.frames.is_empty());
        assert_eq!(
            imported.warnings,
            vec![AsepriteColliderWarning::SliceFrameOutOfRange { frame_index: 9 }],
        );
    }
}
