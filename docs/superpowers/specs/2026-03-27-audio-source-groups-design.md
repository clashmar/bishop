# Audio Source Groups Design

## Goal

Replace the current flat `AudioSource.sounds` list with named sound groups that scripts address explicitly via `entity:play_sound(sound.Footsteps)`, while supporting reusable editor-managed presets that can stay linked to entity-local groups or be detached for per-entity overrides.

## Current Behavior

`AudioSource` currently stores a single `Vec<String>` of sound IDs and one set of playback settings for the entire component. `entity:play_sound()` always forwards that whole list to the audio manager. One-shot playback picks a random entry from the list. Looping playback also picks one random entry from the same list and keeps looping it. Adding another sound to the inspector therefore only adds another candidate to the same random pool; it does not create another callable sound identity.

## Design Summary

The new design introduces three distinct concepts:

1. `AudioSource` stores a set of local sound groups on each entity.
2. The editor stores a project-wide library of sound presets with the same shape as a local group.
3. Scripts use generated constants from `_engine/sounds.lua` to reference group names, but playback still resolves only against the local groups present on the target entity.

This keeps runtime ownership local to the entity while allowing authoring-time reuse through linked presets.

## Data Model

### Runtime component

`AudioSource` will change from a flat list to a collection keyed by sound-group identifier.

Proposed shape:

```rust
pub struct AudioSource {
    pub groups: HashMap<SoundGroupId, AudioGroup>,
    #[serde(skip)]
    pub current: Option<SoundGroupId>,
}
```

`SoundGroupId` will follow the `ClipId` pattern used by animation:

```rust
pub enum SoundGroupId {
    Custom(String),
    New,
}
```

`New` is editor-only transient state for group creation flows. It is not valid serialized game data and must not be persisted inside `AudioSource.groups`.

`AudioGroup` owns the actual playable data:

```rust
pub struct AudioGroup {
    pub sounds: Vec<String>,
    pub volume: f32,
    pub pitch_variation: f32,
    pub volume_variation: f32,
    pub looping: bool,
    pub preset_link: Option<SoundPresetLink>,
}
```

`SoundPresetLink` stores editor-facing linkage metadata:

```rust
pub struct SoundPresetLink {
    pub preset_name: String,
}
```

The runtime game only needs enough data to know whether a group is still linked. The actual preset definitions live in editor-managed storage.

### Editor preset library

The editor will maintain a project-wide collection of presets:

```rust
pub struct SoundPresetLibrary {
    pub presets: HashMap<String, AudioGroupPreset>,
}
```

Each preset has the same editable fields as an entity-local group:

```rust
pub struct AudioGroupPreset {
    pub sounds: Vec<String>,
    pub volume: f32,
    pub pitch_variation: f32,
    pub volume_variation: f32,
    pub looping: bool,
}
```

The preset name is also the generated Lua identifier source. Presets define the project vocabulary. Entity groups using that vocabulary may stay linked or be detached.

## Runtime Behavior

### Script API

`entity:play_sound()` will change from no arguments to a required sound-group argument:

```lua
local sound = require("_engine.sounds")
entity:play_sound(sound.Footsteps)
```

Resolution rules:

1. The Lua argument is a string group name from `_engine/sounds.lua`.
2. The engine looks for a matching local group on the entity's `AudioSource`.
3. If found, playback uses that group's local sound list and local playback settings.
4. If missing, playback is a no-op and logs a warning that includes the missing group and entity.

This keeps group availability entity-scoped even though names come from a shared generated file.

### Randomization

Randomization remains group-local:

1. One-shot groups pick one random sound from the group's `sounds` list each time.
2. Looping groups pick one random sound when playback starts and continue looping that chosen file until stopped.
3. Pitch and volume variation are applied using the group's own settings, not component-global settings.

### Ref counting and cache warmup

All sound IDs across all local groups participate in ref counting.

Implications:

1. `post_create` increments refs for every sound referenced by every local group.
2. `post_remove` decrements refs for every sound referenced by every local group and still stops any active loop tied to the entity.
3. Game load warmup logic must collect all sounds from all groups, not a flat `sounds` field.
4. Inspector edits that add, remove, relink, sync, or detach groups must keep ref counts correct as the group's effective local sound list changes.

## Preset Linking Model

The linking model is intentionally simple:

1. Creating a group from a preset copies the preset data into the entity-local group and stores a link to the preset name.
2. While linked, preset edits may be pushed into matching linked entity groups.
3. Detaching removes the link and preserves the current local data as a normal standalone group.
4. Renaming or deleting a preset breaks or updates links explicitly through editor-managed operations; links should never silently resolve to a different preset.

To avoid hidden behavior, linked groups still display and serialize their local copy on the entity. The link adds synchronization behavior in the editor, not indirection at playback time.

## Editor UX

The UI does not need to be pretty, but it does need to be explicit and workable.

### Inspector layout

The audio inspector will gain a group selector area at the top, modeled after the animation inspector:

1. Left dropdown: select the current local group on the entity.
2. Right dropdown: add a new local group or assign a preset into the entity.
3. Rename field: appears when creating or renaming a custom local group.
4. Link controls: show whether the current local group is linked, detached, or missing its preset.

Below the selector, the inspector edits the currently selected local group's:

1. sound list
2. volume
3. pitch variation
4. volume variation
5. looping

The inspector also exposes minimal preset management controls:

1. Create preset from current local group
2. Re-sync current linked group from preset
3. Push current local group changes back into preset
4. Detach current group from preset

This is enough to support the linked workflow without introducing a full preset-management window in the first pass.

### Group creation flows

Supported flows:

1. Add empty local group, then rename it.
2. Create local group from an existing preset, linked by default.
3. Create new preset from the current local group.
4. Duplicate a local group into a detached copy.

### Visual state

The current group row should show basic state text:

1. `Linked: Talk`
2. `Detached`
3. `Missing Preset: Talk`

Warnings can use the existing toast pattern where needed.

## Generated Lua File

The editor will create `_engine/sounds.lua` similarly to `_engine/animations.lua`.

Generation rules:

1. Start from the preset-library names because presets define the shared vocabulary.
2. Include detached local group names found in entities so local-only groups still get IDE support.
3. Deduplicate and sort names.
4. Sanitize names into valid Lua identifiers using the same style as animation.

Example:

```lua
-- Auto-generated. Do not edit.
---@meta

---@enum SoundGroupId
local SoundGroupId = {
    Footsteps = "Footsteps",
    Talk = "Talk",
}

return SoundGroupId
```

## Storage

The preset library should be stored in editor-managed game data alongside other generated/editor-authored metadata, with save/load paths integrated through `editor_storage`.

Requirements:

1. Saving the game regenerates `_engine/sounds.lua`.
2. Creating a new game writes an initial empty `sounds.lua` into `_engine`.
3. Loading an existing game loads the preset library before inspector editing needs it.
4. Missing or invalid preset-library data should fail soft by falling back to an empty library.

The exact storage file can be a new editor-side manifest or a dedicated RON file; it does not need to be embedded into the runtime `Game` serialization if that complicates separation of concerns.

## Error Handling

Expected failure modes:

1. Missing group on entity during `play_sound(sound.X)`: warn and no-op.
2. Empty group sound list: no-op.
3. Missing audio file in a group: existing audio-load error path remains in place.
4. Missing preset for a linked group: keep the local copy, mark the inspector state as missing, and let the user re-link, push, or detach.
5. Duplicate group names on the same entity: disallow in the inspector.
6. Duplicate preset names in the library: disallow in the editor.

## Compatibility

This feature does not preserve the old flat `AudioSource` serialization shape.

Requirements:

1. `AudioSource` serializes and deserializes only the grouped format.
2. Existing scripts using `entity:play_sound()` with no argument will need updating; this is an intentional API break.

## Testing And Verification

Primary verification should be end-to-end, not unit-test only.

Required checks:

1. `cargo check --workspace`
2. Editor smoke test:
   - create local group
   - create preset from group
   - assign preset to a second entity
   - edit preset and verify linked group updates
   - detach one entity group and verify later preset edits no longer affect it
   - save and reload the project
3. Script smoke test:
   - require `_engine.sounds`
   - call `entity:play_sound(sound.Footsteps)`
   - verify the correct local group randomizes within its own list
   - verify missing group logs a warning and does not crash

## Files Likely To Change

Core runtime:

1. `engine_core/src/audio/audio_source.rs`
2. `engine_core/src/audio/mod.rs`
3. `game/src/scripting/modules/entity_module.rs`
4. `game/src/engine/game_instance.rs`

Editor inspector:

1. `editor/src/gui/inspector/audio_source_module.rs`
2. Any shared inspector helpers needed for dropdowns, renaming, or toasts

Editor storage and generated assets:

1. `editor/src/storage/editor_storage.rs`
2. `editor/src/editor_assets/assets.rs`
3. `editor/build.rs`
4. `editor/scripts/_engine/sounds.lua` or the embedded generation path for it

## Recommendation

Implement this in one feature slice, but keep the preset-management UI intentionally thin. The important part is the model:

1. explicit named groups
2. local playback ownership
3. shared generated names for scripts
4. linked preset workflow with detach support

That gives the engine a clear long-term audio authoring model without turning `AudioSource` into a global sound registry.
