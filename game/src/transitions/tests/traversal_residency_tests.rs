use crate::engine::game_instance::GameInstance;
use crate::transitions::traversal_residency;
use bishop::audio::AudioBackend;
use engine_core::audio::AudioManager;
use engine_core::constants::paths::SFX_FOLDER;
use engine_core::ecs::{
    Active, AudioGroup, AudioSource, AudioStopBehavior, AudioTrigger, Entity, Script, ScriptId, SoundGroupId, SoundId
};
use engine_core::game::Game;
use engine_core::hydration::{HydrationScope, Hydratable, ResourceClass};
use engine_core::engine_global::set_game_name;
use engine_core::storage::{save_game_to_folder, test_utils::{game_fs_test_lock, TestGameFolder}};
use engine_core::task::BackgroundService;
use engine_core::worlds::{Room, RoomId, World, WorldId};
use mlua::Lua;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn room_bound_entity(
    game: &mut Game,
    room_id: RoomId,
    pinned: bool,
) -> Entity {
    let entity = game
        .ecs
        .create_entity()
        .with(Active::new(false))
        .with(Script {
            script_id: ScriptId(1),
            ..Default::default()
        })
        .with_current_room(room_id)
        .finish();
    if pinned {
        game.ecs.get_mut::<Active>(entity).unwrap().pin();
    }
    entity
}

struct TestBackend;

impl AudioBackend for TestBackend {
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(_render_fn: F) -> Self {
        Self
    }
}

fn two_room_game() -> Game {
    let mut world = World::default();
    world.id = WorldId(1);
    world.current_room_id = Some(RoomId(1));
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.add_room(Room {
        id: RoomId(2),
        ..Default::default()
    });

    let mut game = Game::default();
    game.add_world(world);
    game
}

fn register_sound(game: &mut Game, sound_id: SoundId, name: &str) {
    game.asset_registry
        .register_asset_relative_path(
            sound_id,
            PathBuf::from(SFX_FOLDER).join(format!("{name}.wav")),
        )
        .unwrap();
}

fn insert_auto_loop_source(
    game: &mut Game,
    entity: Entity,
    group_name: &str,
    sound_id: SoundId,
    stop_behavior: AudioStopBehavior,
) {
    game.ecs.insert_component(
        entity,
        AudioSource {
            groups: HashMap::from([(
                SoundGroupId::Custom(group_name.to_string()),
                AudioGroup {
                    sounds: vec![sound_id],
                    looping: true,
                    trigger: AudioTrigger::OnOwnerActivate,
                    stop_behavior,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        },
    );
}

fn attach_room_singleton_auto_loop(
    game: &mut Game,
    room_id: RoomId,
    group_name: &str,
    sound_name: &str,
    sound_id: SoundId,
    stop_behavior: AudioStopBehavior,
) {
    let entity = game.ecs.create_entity().finish();
    insert_auto_loop_source(game, entity, group_name, sound_id, stop_behavior);
    register_sound(game, sound_id, sound_name);
    game.current_world_mut()
        .unwrap()
        .get_room_mut(room_id)
        .unwrap()
        .singleton = entity;
}

fn write_silent_wav(path: &Path) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&44_100u32.to_le_bytes());
    bytes.extend_from_slice(&176_400u32.to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0]);

    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn traversal_refresh_fades_room_owned_loop_when_leaving_room() {
    let mut game = two_room_game();
    attach_room_singleton_auto_loop(
        &mut game,
        RoomId(1),
        "Ambience",
        "room_fade",
        SoundId(4),
        AudioStopBehavior::FadeOut { duration: 0.5 },
    );

    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("traversal_refresh_room_audio_fade");
    game.name = test_game.name().to_string();
    set_game_name(test_game.name());
    let resources_dir = test_game.path().join("Resources");
    save_game_to_folder(&game, &resources_dir).unwrap();
    write_silent_wav(
        &resources_dir
            .join("audio")
            .join("sfx")
            .join("room_fade.wav"),
    );

    let lua = Lua::new();
    let mut audio = AudioManager::new::<TestBackend>();
    audio.seed_silent_sound_for_test("sfx/room_fade");
    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };

    traversal_residency::refresh_after_traversal_runtime(&lua, &mut audio, &mut instance);
    instance.game.current_world_mut().unwrap().current_room_id = Some(RoomId(2));
    traversal_residency::refresh_after_traversal_runtime(&lua, &mut audio, &mut instance);

    assert_eq!(audio.ref_count(&"sfx/room_fade".to_string()), 1);
    audio.poll(0.5);
    assert_eq!(audio.ref_count(&"sfx/room_fade".to_string()), 0);
}

#[test]
fn room_transition_activates_current_room_and_deactivates_unpinned_previous_room_entities() {
    let mut game = two_room_game();
    let old_room_entity = room_bound_entity(&mut game, RoomId(1), false);
    let new_room_entity = room_bound_entity(&mut game, RoomId(2), false);

    game.current_world_mut().unwrap().current_room_id = Some(RoomId(2));

    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };
    traversal_residency::refresh_after_traversal(&mut instance);

    assert!(instance.game.ecs.get::<Active>(new_room_entity).unwrap().value);
    assert!(!instance.game.ecs.get::<Active>(old_room_entity).unwrap().value);
}

#[test]
fn pinned_entity_is_not_deactivated_or_unclaimed_when_player_leaves_its_room() {
    let mut game = two_room_game();
    let hunter = room_bound_entity(&mut game, RoomId(2), true);

    game.current_world_mut().unwrap().current_room_id = Some(RoomId(1));

    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };
    traversal_residency::refresh_after_traversal(&mut instance);

    assert!(instance.game.ecs.get::<Active>(hunter).unwrap().is_enabled());
    assert!(
        instance
            .game
            .hydration_coordinator
            .claim_count(&HydrationScope::Entity(hunter), ResourceClass::Script)
            > 0
    );
}

#[test]
fn pinned_room_payload_stays_claimed_outside_the_frontier() {
    let mut game = two_room_game();
    let hunter = room_bound_entity(&mut game, RoomId(2), true);
    game.current_world_mut().unwrap().current_room_id = Some(RoomId(1));

    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };
    traversal_residency::refresh_after_traversal(&mut instance);

    assert!(
        instance
            .game
            .hydration_coordinator
            .claim_count(&HydrationScope::Entity(hunter), ResourceClass::RoomPayload)
            > 0
    );
}

#[test]
fn unpin_releases_room_payload_on_the_next_refresh_when_no_other_claim_remains() {
    let mut game = two_room_game();
    let hunter = room_bound_entity(&mut game, RoomId(2), true);
    game.current_world_mut().unwrap().current_room_id = Some(RoomId(1));

    let mut instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };
    traversal_residency::refresh_after_traversal(&mut instance);
    instance.game.ecs.get_mut::<Active>(hunter).unwrap().unpin();
    traversal_residency::refresh_after_traversal(&mut instance);

    assert_eq!(
        instance
            .game
            .hydration_coordinator
            .claim_count(&HydrationScope::Entity(hunter), ResourceClass::RoomPayload),
        0
    );
}
