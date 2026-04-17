use super::*;
use crate::prelude::*;
use bishop::prelude::*;
use std::collections::{HashMap, HashSet};

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
