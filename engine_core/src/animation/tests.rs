use super::*;
use crate::prelude::*;
use bishop::prelude::*;
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;

struct PanicLoader;

impl TextureLoader for PanicLoader {
    fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
        panic!("test should use cached animation sprites")
    }

    fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
        panic!("test should use cached animation sprites")
    }

    fn empty_texture(&self) -> Texture2D {
        panic!("test should use cached animation sprites")
    }
}

#[test]
fn updates_explicit_entities_without_current_room() {
    let loader = PanicLoader;
    let mut ecs = Ecs::default();
    let mut asset_registry = AssetRegistry::default();
    let mut sprite_manager = SpriteManager::default();
    let entity = ecs.create_entity().with(Transform::default()).finish();
    let mut animation = Animation {
        clips: HashMap::from([(ClipId::Idle, ClipDef::default())]),
        current: Some(ClipId::Idle),
        sprite_cache: HashMap::from([(ClipId::Idle, SpriteId(7))]),
        ..Default::default()
    };
    animation.init_runtime();
    ecs.add_component_to_entity(entity, animation);

    update_entity_animations(
        &loader,
        &mut ecs,
        &mut asset_registry,
        &mut sprite_manager,
        0.3,
        &HashSet::from([entity]),
    );

    let frame = ecs
        .get::<CurrentFrame>(entity)
        .expect("explicit entity animation should produce a frame");
    assert_eq!(frame.clip_id, ClipId::Idle);
    assert_eq!(frame.col, 1);
    assert_eq!(frame.row, 0);
    assert_eq!(frame.sprite_id, SpriteId(7));
}

#[test]
fn clip_names_drive_asset_filenames_and_import_mapping() {
    use super::clip_id_helpers::{clip_id_from_name, json_filename, sprite_filename};

    let builtins =
        ClipId::iter().filter(|clip_id| !matches!(clip_id, ClipId::Custom(_) | ClipId::New));

    for clip_id in builtins.chain([ClipId::Custom("Fidget".to_string()), ClipId::New]) {
        let label = clip_id.canonical_name();
        assert_eq!(json_filename(&clip_id), format!("{label}.json"));

        if clip_id == ClipId::New {
            assert_eq!(sprite_filename(&clip_id), None);
        } else {
            assert_eq!(sprite_filename(&clip_id), Some(format!("{label}.png")));
            assert_eq!(clip_id_from_name(&label), clip_id);
        }
    }

    assert_eq!(
        clip_id_from_name(&ClipId::New.canonical_name()),
        ClipId::Custom(ClipId::New.canonical_name())
    );
}

#[test]
fn generated_animations_lua_uses_canonical_clip_names() {
    let idle = ClipId::Idle.canonical_name();
    let jump = ClipId::Jump.canonical_name();
    let lua = generate_animations_lua(&[
        idle.clone(),
        "My Clip".to_string(),
        jump,
        "My Clip".to_string(),
    ]);

    for clip_id in
        ClipId::iter().filter(|clip_id| !matches!(clip_id, ClipId::Custom(_) | ClipId::New))
    {
        let label = clip_id.canonical_name();
        let expected_line = format!("    {label} = \"{label}\",");
        assert_eq!(lua.matches(&expected_line).count(), 1);
    }

    assert_eq!(lua.matches("    MyClip = \"My Clip\",").count(), 1);
}
