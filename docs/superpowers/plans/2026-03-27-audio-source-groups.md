# Audio Source Groups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `AudioSource` with named local groups plus linked editor presets, require `entity:play_sound(sound.GroupName)`, and generate `_engine/sounds.lua` for IDE support.

**Architecture:** The runtime owns only entity-local audio groups and resolves playback against the target entity. The editor owns a project-wide preset library and generated Lua identifiers, synchronizing linked local copies into components without introducing global runtime playback.

**Tech Stack:** Rust workspace, Serde/RON serialization, Bishop editor GUI widgets, Lua API generation, cargo test/check.

---

## File Structure

Runtime audio model and Lua generation:

- Modify: `engine_core/src/audio/audio_source.rs`
- Modify: `engine_core/src/audio/mod.rs`
- Modify: `engine_core/src/audio/command_queue.rs`
- Modify: `engine_core/src/audio/mod.rs`
- Modify: `game/src/scripting/modules/entity_module.rs`
- Modify: `game/src/engine/game_instance.rs`

Editor storage and generated assets:

- Modify: `editor/src/storage/editor_storage.rs`
- Modify: `editor/src/editor_assets/assets.rs`
- Modify: `editor/src/editor_assets/mod.rs`
- Modify: `editor/build.rs`
- Create: `editor/scripts/_engine/sounds.lua`

Editor inspector:

- Modify: `editor/src/gui/inspector/audio_source_module.rs`

Tests:

- Add unit tests in `engine_core/src/audio/audio_source.rs`
- Add unit tests in `editor/src/editor_assets/assets.rs`

### Task 1: Core Audio Group Model

**Files:**
- Modify: `engine_core/src/audio/audio_source.rs`
- Test: `engine_core/src/audio/audio_source.rs`

- [ ] **Step 1: Write the failing tests for audio group IDs, sound flattening, and grouped deserialization**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[test]
    fn sound_group_id_ui_label_uses_custom_name() {
        assert_eq!(
            SoundGroupId::Custom("Footsteps".to_string()).ui_label(),
            "Footsteps"
        );
    }

    #[test]
    fn all_sound_ids_collects_every_group_sound() {
        let mut source = AudioSource::default();
        source.groups.insert(
            SoundGroupId::Custom("Footsteps".to_string()),
            AudioGroup {
                sounds: vec!["footstep_a".to_string(), "footstep_b".to_string()],
                ..Default::default()
            },
        );
        source.groups.insert(
            SoundGroupId::Custom("Talk".to_string()),
            AudioGroup {
                sounds: vec!["talk_a".to_string()],
                ..Default::default()
            },
        );

        assert_eq!(
            source.all_sound_ids(),
            vec![
                "footstep_a".to_string(),
                "footstep_b".to_string(),
                "talk_a".to_string(),
            ]
        );
    }

    #[test]
    fn deserializing_grouped_audio_source_preserves_groups() {
        #[derive(Deserialize)]
        struct Wrapper {
            source: AudioSource,
        }

        let ron = r#"
            (
                source: (
                    groups: {
                        Custom("Talk"): (
                            sounds: ["talk_1", "talk_2"],
                            volume: 0.8,
                            pitch_variation: 0.1,
                            volume_variation: 0.2,
                            looping: false,
                        ),
                    },
                ),
            )
        "#;

        let wrapper: Wrapper = ron::from_str(ron).unwrap();
        let group = wrapper
            .source
            .groups
            .get(&SoundGroupId::Custom("Talk".to_string()))
            .unwrap();

        assert_eq!(group.sounds, vec!["talk_1".to_string(), "talk_2".to_string()]);
        assert_eq!(group.volume, 0.8);
        assert_eq!(group.pitch_variation, 0.1);
        assert_eq!(group.volume_variation, 0.2);
        assert!(!group.looping);
        assert!(group.preset_link.is_none());
        assert!(wrapper.source.current.is_none());
    }
}
```

- [ ] **Step 2: Run the targeted test command and verify it fails for missing group types/helpers**

Run: `cargo test -p engine_core audio_source::tests -- --nocapture`
Expected: FAIL with missing `SoundGroupId`, `AudioGroup`, or `all_sound_ids`.

- [ ] **Step 3: Implement the new `AudioSource` data model**

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoundGroupId {
    #[default]
    New,
    Custom(String),
}

// `New` is an editor-only creation sentinel and must not persist
// inside serialized `AudioSource.groups` data.

impl SoundGroupId {
    pub fn ui_label(&self) -> String {
        match self {
            Self::Custom(name) => name.clone(),
            Self::New => "New".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundPresetLink {
    pub preset_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioGroup {
    pub sounds: Vec<String>,
    pub volume: f32,
    pub pitch_variation: f32,
    pub volume_variation: f32,
    pub looping: bool,
    pub preset_link: Option<SoundPresetLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "AudioSourceSerde")]
pub struct AudioSource {
    pub groups: HashMap<SoundGroupId, AudioGroup>,
    #[serde(skip)]
    pub current: Option<SoundGroupId>,
}

impl AudioSource {
    pub fn all_sound_ids(&self) -> Vec<String> {
        self.groups
            .values()
            .flat_map(|group| group.sounds.iter().cloned())
            .collect()
    }
}
```

- [ ] **Step 4: Update lifecycle hooks to use all group sounds**

```rust
fn post_create(source: &mut AudioSource, _entity: &Entity, _ctx: &mut GameCtxMut) {
    push_audio_command(AudioCommand::IncrementRefs(source.all_sound_ids()));
}

fn post_remove(source: &mut AudioSource, entity: &Entity, _ctx: &mut GameCtxMut) {
    push_audio_command(AudioCommand::StopLoop(**entity as u64));
    push_audio_command(AudioCommand::DecrementRefs(source.all_sound_ids()));
}
```

- [ ] **Step 5: Re-run the targeted tests and verify they pass**

Run: `cargo test -p engine_core audio_source::tests -- --nocapture`
Expected: PASS for the three new tests.

- [ ] **Step 6: Commit Task 1**

```bash
git add engine_core/src/audio/audio_source.rs
git commit -m "feat(audio): add grouped audio source model"
```

### Task 2: Runtime Playback API And Sound Enum Generation

**Files:**
- Modify: `engine_core/src/audio/mod.rs`
- Modify: `game/src/scripting/modules/entity_module.rs`
- Modify: `game/src/engine/game_instance.rs`
- Modify: `editor/src/editor_assets/assets.rs`
- Modify: `editor/src/editor_assets/mod.rs`
- Modify: `editor/build.rs`
- Create: `editor/scripts/_engine/sounds.lua`
- Test: `editor/src/editor_assets/assets.rs`

- [ ] **Step 1: Write the failing tests for `sounds.lua` generation**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sounds_lua_sorts_and_sanitizes_names() {
        let lua = generate_sounds_lua(&[
            "Talk".to_string(),
            "footsteps".to_string(),
            "1 Boss Attack".to_string(),
        ]);

        assert!(lua.contains("Footsteps = \"footsteps\""));
        assert!(lua.contains("Talk = \"Talk\""));
        assert!(lua.contains("Sound_1_Boss_Attack = \"1 Boss Attack\""));
    }
}
```

- [ ] **Step 2: Run the targeted test command and verify it fails for missing generation helpers**

Run: `cargo test -p editor generate_sounds_lua -- --nocapture`
Expected: FAIL with missing `generate_sounds_lua`.

- [ ] **Step 3: Add the `sounds.lua` generator and editor asset writer**

```rust
pub fn generate_sounds_lua(group_names: &[String]) -> String {
    let mut lua = String::from(
        "-- Auto-generated. Do not edit.\n\
        ---@meta\n\n\
        ---@enum SoundGroupId\n\
        local SoundGroupId = {\n",
    );

    let mut names = group_names.to_vec();
    names.sort();
    names.dedup();

    for name in names {
        let key = sanitize_lua_identifier_with_prefix(&name, "Sound");
        lua.push_str(&format!("    {} = \"{}\",\n", key, name));
    }

    lua.push_str("}\n\nreturn SoundGroupId\n");
    lua
}
```

- [ ] **Step 4: Update runtime playback to require a group name argument**

```rust
methods.add_method(ENTITY_PLAY_SOUND, |lua, this, group_name: String| {
    let ctx = LuaGameCtx::borrow_ctx(lua)?;
    let game_instance = ctx.game_instance.borrow();
    let ecs = &game_instance.game.ecs;
    let Some(source) = ecs.get::<AudioSource>(this.entity) else {
        return Ok(());
    };

    let group_id = SoundGroupId::Custom(group_name.clone());
    let Some(group) = source.groups.get(&group_id) else {
        log::warn!(
            "Entity {:?} tried to play missing sound group '{}'",
            this.entity,
            group_name
        );
        return Ok(());
    };

    if group.looping {
        push_audio_command(AudioCommand::PlayLoop {
            handle: *this.entity as u64,
            sounds: group.sounds.clone(),
            volume: group.volume,
            pitch_variation: group.pitch_variation,
            volume_variation: group.volume_variation,
        });
    } else {
        push_audio_command(AudioCommand::PlayVariedSfx {
            sounds: group.sounds.clone(),
            volume: group.volume,
            pitch_variation: group.pitch_variation,
            volume_variation: group.volume_variation,
        });
    }
    Ok(())
});
```

- [ ] **Step 5: Update game-load cache warmup to flatten all local groups**

```rust
for source in AudioSource::store(&game.ecs).data.values() {
    push_audio_command(AudioCommand::IncrementRefs(source.all_sound_ids()));
}
```

- [ ] **Step 6: Re-run targeted tests, then compile the affected crates**

Run: `cargo test -p editor generate_sounds_lua -- --nocapture`
Expected: PASS

Run: `cargo check -p engine_core -p game -p editor`
Expected: PASS

- [ ] **Step 7: Commit Task 2**

```bash
git add engine_core/src/audio/mod.rs game/src/scripting/modules/entity_module.rs game/src/engine/game_instance.rs editor/src/editor_assets/assets.rs editor/src/editor_assets/mod.rs editor/build.rs editor/scripts/_engine/sounds.lua
git commit -m "feat(audio): require named sound groups in lua"
```

### Task 3: Editor Preset Library Storage And Save/Load Hooks

**Files:**
- Modify: `editor/src/storage/editor_storage.rs`
- Modify: `editor/src/editor_assets/assets.rs`
- Test: `editor/src/editor_assets/assets.rs`

- [ ] **Step 1: Write the failing tests for collecting sound names**

```rust
#[test]
fn collect_sound_group_names_merges_presets_and_local_groups() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().id();
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("Talk".to_string()),
        AudioGroup::default(),
    );
    source.groups.insert(
        SoundGroupId::Custom("Footsteps".to_string()),
        AudioGroup::default(),
    );
    ecs.add_component(entity, source);

    let library = SoundPresetLibrary {
        presets: HashMap::from([(
            "Ambient".to_string(),
            AudioGroupPreset::default(),
        )]),
    };

    let names = collect_sound_group_names(&ecs, &library);
    assert_eq!(names, vec![
        "Ambient".to_string(),
        "Footsteps".to_string(),
        "Talk".to_string(),
    ]);
}
```

- [ ] **Step 2: Run the targeted test command and verify it fails**

Run: `cargo test -p editor collect_sound_group_names -- --nocapture`
Expected: FAIL with missing `SoundPresetLibrary` or `collect_sound_group_names`.

- [ ] **Step 3: Add preset-library types and persistence helpers in editor storage**

```rust
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioGroupPreset {
    pub sounds: Vec<String>,
    pub volume: f32,
    pub pitch_variation: f32,
    pub volume_variation: f32,
    pub looping: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundPresetLibrary {
    pub presets: HashMap<String, AudioGroupPreset>,
}

pub fn load_sound_preset_library(game_name: &str) -> SoundPresetLibrary {
    let path = game_folder(game_name).join("sound_presets.ron");
    fs::read_to_string(path)
        .ok()
        .and_then(|ron| ron::from_str(&ron).ok())
        .unwrap_or_default()
}

pub fn save_sound_preset_library(
    game_name: &str,
    library: &SoundPresetLibrary,
) -> io::Result<()> {
    let path = game_folder(game_name).join("sound_presets.ron");
    let ron = ron::ser::to_string_pretty(library, ron::ser::PrettyConfig::new())
        .map_err(Error::other)?;
    fs::write(path, ron)
}
```

- [ ] **Step 4: Regenerate `_engine/sounds.lua` during save and new-game creation**

```rust
let sound_library = load_sound_preset_library(&game.name);
let sound_names = collect_sound_group_names(&game.ecs, &sound_library);
if let Err(e) = write_sounds_lua(&scripts_folder(), &sound_names) {
    onscreen_error!("Could not write sounds.lua: {e}");
}
```

- [ ] **Step 5: Re-run the targeted tests and a focused editor build**

Run: `cargo test -p editor collect_sound_group_names -- --nocapture`
Expected: PASS

Run: `cargo check -p editor`
Expected: PASS

- [ ] **Step 6: Commit Task 3**

```bash
git add editor/src/storage/editor_storage.rs editor/src/editor_assets/assets.rs
git commit -m "feat(editor): persist sound preset library"
```

### Task 4: Audio Inspector Groups, Linking UI, And Sync Actions

**Files:**
- Modify: `editor/src/gui/inspector/audio_source_module.rs`
- Modify: `editor/src/storage/editor_storage.rs`
- Modify: `engine_core/src/audio/audio_source.rs`

- [ ] **Step 1: Add a failing regression test for linked-group sync logic**

```rust
#[test]
fn apply_preset_to_linked_group_overwrites_local_fields() {
    let preset = AudioGroupPreset {
        sounds: vec!["talk_a".to_string()],
        volume: 0.5,
        pitch_variation: 0.1,
        volume_variation: 0.2,
        looping: false,
    };

    let mut group = AudioGroup {
        sounds: vec!["old".to_string()],
        volume: 1.0,
        pitch_variation: 0.0,
        volume_variation: 0.0,
        looping: true,
        preset_link: Some(SoundPresetLink {
            preset_name: "Talk".to_string(),
        }),
    };

    group.apply_preset("Talk", &preset);

    assert_eq!(group.sounds, vec!["talk_a".to_string()]);
    assert_eq!(group.volume, 0.5);
    assert!(!group.looping);
}
```

- [ ] **Step 2: Run the targeted test command and verify it fails**

Run: `cargo test -p engine_core apply_preset_to_linked_group_overwrites_local_fields -- --nocapture`
Expected: FAIL with missing `apply_preset`.

- [ ] **Step 3: Implement reusable sync helpers before touching inspector UI**

```rust
impl AudioGroup {
    pub fn apply_preset(&mut self, preset_name: &str, preset: &AudioGroupPreset) {
        self.sounds = preset.sounds.clone();
        self.volume = preset.volume;
        self.pitch_variation = preset.pitch_variation;
        self.volume_variation = preset.volume_variation;
        self.looping = preset.looping;
        self.preset_link = Some(SoundPresetLink {
            preset_name: preset_name.to_string(),
        });
    }
}
```

- [ ] **Step 4: Rework the inspector around a selected local group plus preset controls**

```rust
pub struct AudioSourceModule {
    select_dropdown_id: WidgetId,
    assign_dropdown_id: WidgetId,
    rename_field_id: WidgetId,
    preset_action_dropdown_id: WidgetId,
    volume_id: WidgetId,
    pitch_id: WidgetId,
    volume_var_id: WidgetId,
    warning: Option<Toast>,
    pending_rename: bool,
    rename_initial_value: String,
    has_groups: bool,
}
```

The implementation for this step must include:

```rust
fn draw_group_dropdowns(
    &mut self,
    ctx: &mut WgpuContext,
    blocked: bool,
    rect: Rect,
    source: &mut AudioSource,
    library: &mut SoundPresetLibrary,
) {
    // 1. select current group
    // 2. add empty group or create from preset
    // 3. show rename field for custom names
    // 4. expose link actions: sync from preset, push to preset, detach
}
```

- [ ] **Step 5: Update inspector editing and ref-count bookkeeping when group contents change**

Run the implementation so that these operations adjust refs correctly:

```rust
let before = current_group.sounds.clone();
current_group.sounds.push(id);
push_audio_command(AudioCommand::IncrementRefs(
    diff_added_sounds(&before, &current_group.sounds),
));
```

And on removal:

```rust
push_audio_command(AudioCommand::DecrementRefs(vec![removed_id]));
```

- [ ] **Step 6: Re-run focused tests and compile the full workspace**

Run: `cargo test -p engine_core apply_preset_to_linked_group_overwrites_local_fields -- --nocapture`
Expected: PASS

Run: `cargo check --workspace`
Expected: PASS

- [ ] **Step 7: Manual editor/game smoke test**

Run these checks manually:

1. Create a local `Talk` group on one entity and save.
2. Create a preset from `Talk`.
3. Assign the preset to a second entity and confirm the group is linked.
4. Edit the preset and confirm both linked groups update.
5. Detach one entity’s group and confirm later preset edits do not change it.
6. Save, reload, and confirm linkage state survives.
7. In Lua, require `_engine.sounds` and call `entity:play_sound(sound.Talk)`.
8. Verify missing groups warn and do not crash.

- [ ] **Step 8: Commit Task 4**

```bash
git add engine_core/src/audio/audio_source.rs editor/src/gui/inspector/audio_source_module.rs editor/src/storage/editor_storage.rs
git commit -m "feat(editor): add linked audio source groups"
```

## Self-Review

Spec coverage check:

- Named local groups: covered by Task 1.
- Required `play_sound(sound.X)` API: covered by Task 2.
- Generated `_engine/sounds.lua`: covered by Tasks 2 and 3.
- Linked preset library and detach flow: covered by Tasks 3 and 4.
- Inspector UI for group/preset management: covered by Task 4.
- Grouped serialization shape only: covered by Task 1.

Placeholder scan:

- No `TODO`, `TBD`, or deferred implementation markers remain in the task steps.

Type consistency:

- Uses `SoundGroupId`, `AudioGroup`, `SoundPresetLink`, `AudioGroupPreset`, and `SoundPresetLibrary` consistently across tasks.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-03-27-audio-source-groups.md`. Execution approach already chosen: subagent-driven implementation in this session.
