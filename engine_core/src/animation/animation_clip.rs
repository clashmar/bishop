// engine_core/src/animation/animation_clip.rs
use crate::assets::sprite_manager::SpriteManager;
use crate::assets::sprite::SpriteId;
use crate::animation::clip_id_helpers::{builtin_clip_ids, sprite_filename};
use crate::constants::DEFAULT_GRID_SIZE;
use crate::ecs::entity::Entity;
use crate::game::*;
use crate::scripting::lua_constants::LUA_OWNER_GAME_GENERATED;
use bishop::prelude::*;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use serde_with::{FromInto, serde_as};
use std::fmt;
use std::{
    collections::HashMap,
    path::PathBuf,
};
use strum_macros::EnumIter;

/// The animation component for an entity.
#[ecs_component(post_create = post_create, post_remove = post_remove)]
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Animation {
    /// Defines the animations that belong to the entity.
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    pub clips: HashMap<ClipId, ClipDef>,
    /// Which animation variant to show.
    pub variant: VariantFolder,
    /// Which clip is currently active.
    #[serde(skip)]
    pub current: Option<ClipId>,
    /// Per-clip runtime data.
    #[serde(skip)]
    pub states: HashMap<ClipId, ClipState>,
    /// Cached SpriteId for each clip in the current variant.
    #[serde(skip)]
    pub sprite_cache: HashMap<ClipId, SpriteId>,
    /// Whether to flip the sprite horizontally (runtime state).
    #[serde(skip)]
    pub flip_x: bool,
    /// Playback speed multiplier (runtime state, defaults to 1.0).
    #[serde(skip)]
    pub speed_multiplier: f32,
}

impl Animation {
    /// Call after deserialization or after a clip has been added/removed.
    pub fn init_runtime(&mut self) {
        self.states.clear();
        for id in self.clips.keys() {
            self.states.insert(id.clone(), ClipState::default());
        }

        // If there is at least one clip but `current` is None, prefer Idle
        if self.current.is_none() && !self.clips.is_empty() {
            self.current = if self.clips.contains_key(&ClipId::Idle) {
                Some(ClipId::Idle)
            } else {
                Some(self.clips.keys().next().unwrap().clone())
            };
        }

        // Initialize speed multiplier to 1.0 if not set
        if self.speed_multiplier == 0.0 {
            self.speed_multiplier = 1.0;
        }
    }

    /// Switch to another clip safely. Only resets if switching to a different clip.
    pub fn set_clip(&mut self, id: &ClipId) {
        if !self.clips.contains_key(id) {
            return;
        }
        if self.current.as_ref() == Some(id) {
            return;
        }

        self.current = Some(id.clone());
        if let Some(state) = self.states.get_mut(id) {
            *state = ClipState::default();
        }
    }

    /// Clear the active clip.
    pub fn clear_clip(&mut self) {
        self.current = None;
    }

    /// Populate `sprite_cache` for the current variant without modifying ref counts.
    /// Use during game initialization when ref counts are already tracked by serialized state.
    pub fn init_sprite_cache(
        &mut self,
        loader: &impl TextureLoader,
        sprite_manager: &mut SpriteManager,
    ) {
        self.sprite_cache.clear();
        for clip_id in self.clips.keys() {
            let sprite_id = resolve_sprite_id(loader, sprite_manager, &self.variant, clip_id);
            self.sprite_cache.insert(clip_id.clone(), sprite_id);
        }
    }

    /// Populate `sprite_cache` from existing sprite path mappings without loading textures.
    pub fn init_sprite_cache_runtime(&mut self, sprite_manager: &SpriteManager) {
        self.sprite_cache.clear();
        restore_sprite_cache_from_known_paths(self, sprite_manager);
    }

    /// Decrements refs for all cached sprites and clears the cache.
    pub fn clear_sprite_cache(&mut self, sprite_manager: &mut SpriteManager) {
        for &sprite_id in self.sprite_cache.values() {
            sprite_manager.decrement_ref(sprite_id);
        }
        self.sprite_cache.clear();
    }

    /// Populate `sprite_cache` for the current variant.
    /// Called when the variant folder changes or a new clip is added.
    pub fn refresh_sprite_cache(
        &mut self,
        loader: &impl TextureLoader,
        sprite_manager: &mut SpriteManager,
    ) {
        self.clear_sprite_cache(sprite_manager);

        // Resolve and cache new sprite ids, incrementing refs
        for clip_id in self.clips.keys() {
            let sprite_id = resolve_sprite_id(loader, sprite_manager, &self.variant, clip_id);
            if sprite_id.0 != 0 {
                sprite_manager.increment_ref(sprite_id);
            }
            self.sprite_cache.insert(clip_id.clone(), sprite_id);
        }
    }

    /// Updates cache for a clip with a new SpriteId, handling ref counting.
    pub fn update_cache_entry(
        &mut self,
        current_id: &ClipId,
        sprite_id: SpriteId,
        sprite_manager: &mut SpriteManager,
    ) {
        // Decrement ref for old sprite if present
        if let Some(&old_id) = self.sprite_cache.get(current_id) {
            sprite_manager.decrement_ref(old_id);
        }

        if sprite_id.0 != 0 {
            sprite_manager.increment_ref(sprite_id);
            self.sprite_cache.insert(current_id.clone(), sprite_id);
        } else {
            self.sprite_cache.remove(current_id);
        }
    }
}

fn restore_sprite_cache_from_known_paths(animation: &mut Animation, sprite_manager: &SpriteManager) {
    let mut restored = HashMap::with_capacity(animation.clips.len());

    for clip_id in animation.clips.keys() {
        if let Some(&sprite_id) = animation.sprite_cache.get(clip_id)
            && sprite_id.0 != 0
        {
            restored.insert(clip_id.clone(), sprite_id);
            continue;
        }

        let Some(path) = sprite_path(&animation.variant, clip_id) else {
            continue;
        };

        if let Some(sprite_id) = sprite_manager.get_or_none(path) {
            restored.insert(clip_id.clone(), sprite_id);
        }
    }

    animation.sprite_cache = restored;
}

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
            // Empty
            ClipId::New => "New".to_string(),
            // Any non-empty custom name
            ClipId::Custom(name) => name.clone(),
            // Built-in variants
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
        Err(_) => SpriteId(0), // Sentinel
    }
}

fn sprite_path(variant_folder: &VariantFolder, clip_id: &ClipId) -> Option<PathBuf> {
    sprite_filename(clip_id).map(|filename| variant_folder.0.join(filename))
}

/// Initializes the component when an entity is instantiated into the world.
pub fn post_create(anim: &mut Animation, _entity: &Entity, ctx: &mut dyn EngineCtxMut) {
    anim.init_runtime();
    restore_sprite_cache_from_known_paths(anim, ctx.sprite_manager());

    for &sprite_id in anim.sprite_cache.values() {
        ctx.sprite_manager().increment_ref(sprite_id);
    }
}

/// Cleans up when the component is removed from an entity.
pub fn post_remove(anim: &mut Animation, _entity: &Entity, ctx: &mut dyn EngineCtxMut) {
    anim.clear_sprite_cache(ctx.sprite_manager());
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

    // Built-in clips from ClipId enum
    let mut builtin_names = HashSet::new();
    for clip_id in builtin_clip_ids() {
        let name = clip_id.canonical_name();
        builtin_names.insert(name.clone());
        lua.push_str(&format!("    {} = \"{}\",\n", name, name));
    }

    // Custom clips (deduplicated, sorted, excluding built-in names)
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
    use crate::game::Game;
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn generate_animations_lua_marks_file_as_game_generated() {
        let lua = generate_animations_lua(&[]);

        assert!(lua.contains(LUA_OWNER_GAME_GENERATED));
    }

    #[test]
    fn post_create_restores_cached_sprite_ids_for_all_clips() {
        let mut animation = Animation {
            clips: HashMap::from([
                (ClipId::Idle, ClipDef::default()),
                (ClipId::Run, ClipDef::default()),
            ]),
            variant: VariantFolder(Path::new("animations/player/male").to_path_buf()),
            ..Default::default()
        };
        let idle = SpriteId(11);
        let run = SpriteId(12);

        let mut game = Game::default();
        game.worlds.push(Default::default());
        game.sprite_manager
            .sprite_id_to_path
            .insert(idle, Path::new(&animation.variant.0).join("Idle.png"));
        game.sprite_manager
            .path_to_sprite_id
            .insert(Path::new(&animation.variant.0).join("Idle.png"), idle);
        game.sprite_manager
            .sprite_id_to_path
            .insert(run, Path::new(&animation.variant.0).join("Run.png"));
        game.sprite_manager
            .path_to_sprite_id
            .insert(Path::new(&animation.variant.0).join("Run.png"), run);

        let mut ctx = game.ctx_mut();
        post_create(&mut animation, &Entity(7), &mut ctx);

        assert_eq!(animation.sprite_cache.get(&ClipId::Idle), Some(&idle));
        assert_eq!(animation.sprite_cache.get(&ClipId::Run), Some(&run));
        assert_eq!(ctx.sprite_manager.get_ref_count(idle), 1);
        assert_eq!(ctx.sprite_manager.get_ref_count(run), 1);
    }

    #[test]
    fn post_create_prunes_stale_cached_clip_entries() {
        let idle = SpriteId(21);
        let stale_run = SpriteId(22);
        let mut animation = Animation {
            clips: HashMap::from([(ClipId::Idle, ClipDef::default())]),
            variant: VariantFolder(Path::new("animations/player/male").to_path_buf()),
            sprite_cache: HashMap::from([(ClipId::Idle, idle), (ClipId::Run, stale_run)]),
            ..Default::default()
        };

        let mut game = Game::default();
        game.worlds.push(Default::default());
        game.sprite_manager
            .sprite_id_to_path
            .insert(idle, Path::new(&animation.variant.0).join("Idle.png"));
        game.sprite_manager
            .path_to_sprite_id
            .insert(Path::new(&animation.variant.0).join("Idle.png"), idle);
        game.sprite_manager
            .sprite_id_to_path
            .insert(stale_run, Path::new(&animation.variant.0).join("Run.png"));
        game.sprite_manager
            .path_to_sprite_id
            .insert(Path::new(&animation.variant.0).join("Run.png"), stale_run);

        let mut ctx = game.ctx_mut();
        post_create(&mut animation, &Entity(9), &mut ctx);

        assert_eq!(animation.sprite_cache.len(), 1);
        assert_eq!(animation.sprite_cache.get(&ClipId::Idle), Some(&idle));
        assert!(!animation.sprite_cache.contains_key(&ClipId::Run));
        assert_eq!(ctx.sprite_manager.get_ref_count(idle), 1);
        assert_eq!(ctx.sprite_manager.get_ref_count(stale_run), 0);
    }

    #[test]
    fn init_sprite_cache_runtime_restores_cached_sprite_ids_without_loading() {
        let mut animation = Animation {
            clips: HashMap::from([
                (ClipId::Idle, ClipDef::default()),
                (ClipId::Run, ClipDef::default()),
            ]),
            variant: VariantFolder(Path::new("animations/player/male").to_path_buf()),
            ..Default::default()
        };
        let idle = SpriteId(31);
        let run = SpriteId(32);

        let mut sprite_manager = crate::assets::sprite_manager::SpriteManager::default();
        sprite_manager
            .sprite_id_to_path
            .insert(idle, Path::new(&animation.variant.0).join("Idle.png"));
        sprite_manager
            .path_to_sprite_id
            .insert(Path::new(&animation.variant.0).join("Idle.png"), idle);
        sprite_manager
            .sprite_id_to_path
            .insert(run, Path::new(&animation.variant.0).join("Run.png"));
        sprite_manager
            .path_to_sprite_id
            .insert(Path::new(&animation.variant.0).join("Run.png"), run);

        animation.init_sprite_cache_runtime(&sprite_manager);

        assert_eq!(animation.sprite_cache.get(&ClipId::Idle), Some(&idle));
        assert_eq!(animation.sprite_cache.get(&ClipId::Run), Some(&run));
        assert_eq!(sprite_manager.get_ref_count(idle), 0);
        assert_eq!(sprite_manager.get_ref_count(run), 0);
    }
}
