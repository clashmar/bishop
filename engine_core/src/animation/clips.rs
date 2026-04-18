use crate::animation::clip_id_helpers::{builtin_clip_ids, sprite_filename};
use crate::assets::sprite_manager::SpriteManager;
use crate::constants::DEFAULT_GRID_SIZE;
use crate::ecs::SpriteId;
use crate::scripting::lua_constants::LUA_OWNER_GAME_GENERATED;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, FromInto};
use std::fmt;
use std::{collections::HashMap, path::PathBuf};
use strum_macros::EnumIter;

/// Logical name of a clip.
#[derive(
    EnumIter, Debug, Default, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub enum ClipId {
    #[default]
    Idle,
    Walk,
    Run,
    Attack,
    Jump,
    Fall,
    Custom(String),
    New,
}

impl ClipId {
    /// Returns the canonical name for this clip.
    pub fn canonical_name(&self) -> String {
        match self {
            ClipId::New => "New".to_string(),
            ClipId::Custom(name) => name.clone(),
            _ => format!("{self:?}"),
        }
    }
}

impl fmt::Display for ClipId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.canonical_name())
    }
}

/// Definition for an animation set.
#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipDef {
    /// Width and height of a single cell.
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub frame_size: Vec2,
    /// Frames per row.
    pub cols: usize,
    /// Number of rows that belong to this clip.
    pub rows: usize,
    /// Playback speed in frames per second (used when frame_durations is empty).
    pub fps: f32,
    /// Per-frame durations in seconds. If empty, uniform timing from fps is used.
    pub frame_durations: Vec<f32>,
    /// Whether the clip loops.
    pub looping: bool,
    /// Optional offset for drawing.
    #[serde_as(as = "FromInto<[f32; 2]>")]
    pub offset: Vec2,
    /// Whether to auto-flip based on FacingDirection component.
    pub mirrored: bool,
}

impl Default for ClipDef {
    fn default() -> ClipDef {
        ClipDef {
            frame_size: Vec2::new(DEFAULT_GRID_SIZE, DEFAULT_GRID_SIZE),
            cols: 5,
            rows: 1,
            fps: 4.0,
            frame_durations: Vec::new(),
            looping: true,
            offset: Vec2::ZERO,
            mirrored: false,
        }
    }
}

/// A full set of clip definitions that can be reused.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AnimationDef {
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    pub clips: HashMap<ClipId, ClipDef>,
}

/// A variant is a folder that contains the spritesheets for an entity variant.
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VariantFolder(pub PathBuf);

/// Runtime state for a single clip.
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipState {
    /// Accumulated time since the last frame change.
    pub timer: f32,
    /// Current column index (0-based).
    pub col: usize,
    /// Current row index (0-based, relative to the clip's `rows`).
    pub row: usize,
    /// Whether the clip has finished playing yet.
    pub finished: bool,
}

/// Returns the `SpriteId` for the current variant clip.
pub fn resolve_sprite_id(
    loader: &impl TextureLoader,
    sprite_manager: &mut SpriteManager,
    variant_folder: &VariantFolder,
    clip_id: &ClipId,
) -> SpriteId {
    let Some(path) = sprite_path(variant_folder, clip_id) else {
        return SpriteId(0);
    };

    match sprite_manager.init_texture(loader, &path) {
        Ok(id) => id,
        Err(_) => SpriteId(0),
    }
}

pub fn sprite_path(variant_folder: &VariantFolder, clip_id: &ClipId) -> Option<PathBuf> {
    sprite_filename(clip_id).map(|filename| variant_folder.0.join(filename))
}

/// Generates the content for animations.lua with built-in and optional custom clips.
pub fn generate_animations_lua(custom_clips: &[String]) -> String {
    use std::collections::HashSet;

    let mut lua = format!(
        "-- Auto-generated. Do not edit.\n\
        {LUA_OWNER_GAME_GENERATED}\n\
        ---@meta\n\n\
        ---@enum ClipId\n\
        local ClipId = {{\n"
    );

    let mut builtin_names = HashSet::new();
    for clip_id in builtin_clip_ids() {
        let name = clip_id.canonical_name();
        builtin_names.insert(name.clone());
        lua.push_str(&format!("    {} = \"{}\",\n", name, name));
    }

    let mut custom_sorted: Vec<&String> = custom_clips
        .iter()
        .filter(|c| !builtin_names.contains(*c))
        .collect();
    custom_sorted.sort();
    custom_sorted.dedup();

    for clip in custom_sorted {
        let key = sanitize_lua_identifier(clip);
        lua.push_str(&format!("    {} = \"{}\",\n", key, clip));
    }

    lua.push_str("}\n\nreturn ClipId\n");
    lua
}

/// Converts a clip name to a valid Lua identifier.
fn sanitize_lua_identifier(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize {
                out.push(ch.to_ascii_uppercase());
                capitalize = false;
            } else {
                out.push(ch);
            }
        } else {
            capitalize = true;
        }
    }
    if out.is_empty()
        || out
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
    {
        format!(
            "Clip_{}",
            s.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
        )
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_animations_lua_marks_file_as_game_generated() {
        let lua = generate_animations_lua(&[]);

        assert!(lua.contains(LUA_OWNER_GAME_GENERATED));
    }
}
