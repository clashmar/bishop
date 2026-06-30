use engine_core::audio::{
    push_audio_command, AudioCommand, AudioLoopKey, AudioManager, AudioPlaybackOwner,
};
use engine_core::ecs::{
    Active, AudioSource, AudioStopBehavior, AudioTrigger, Entity, SoundGroupId,
    sound_command_ids,
};
use engine_core::game::Game;
use engine_core::task::BackgroundService;

/// Describes an authored scoped loop that should be playing now.
#[derive(Clone, Debug, PartialEq)]
pub struct DesiredLoopSpec {
    /// Unique owner/group identity for the loop.
    pub key: AudioLoopKey,
    /// Resolved audio command ids for the group's authored sounds.
    pub sounds: Vec<String>,
    /// Final loop volume after authored/runtime multiplication.
    pub volume: f32,
    /// Authored pitch variation to apply on start.
    pub pitch_variation: f32,
    /// Authored volume variation to apply on start.
    pub volume_variation: f32,
    /// Authored teardown behavior for stale loop reconciliation.
    pub stop_behavior: AudioStopBehavior,
}

/// Returns the authored gameplay owner for an entity-based loop source.
pub fn classify_audio_owner(game: &Game, entity: Entity) -> AudioPlaybackOwner {
    let current_world = game.current_world();
    if current_world.singleton == Some(entity) {
        return AudioPlaybackOwner::World(current_world.id);
    }
    if let Some(room) = current_world
        .rooms()
        .iter()
        .find(|room| room.singleton == Some(entity))
    {
        return AudioPlaybackOwner::Room(room.id);
    }

    for world in game.worlds() {
        if world.id == current_world.id {
            continue;
        }
        if world.singleton == Some(entity) {
            return AudioPlaybackOwner::World(world.id);
        }
        if let Some(room) = world.rooms().iter().find(|room| room.singleton == Some(entity)) {
            return AudioPlaybackOwner::Room(room.id);
        }
    }

    AudioPlaybackOwner::Entity(entity)
}

/// Returns the authored stop behavior for an entity loop source.
pub fn authored_stop_behavior_for_entity(game: &Game, entity: Entity) -> AudioStopBehavior {
    game.ecs
        .get::<AudioSource>(entity)
        .and_then(|source| {
            source
                .groups
                .values()
                .find(|group| group.looping)
                .map(|group| group.stop_behavior.clone())
        })
        .unwrap_or(AudioStopBehavior::Immediate)
}

/// Returns the authored stop behavior for an owner/group loop key.
pub fn authored_stop_behavior_for_key(game: &Game, key: &AudioLoopKey) -> AudioStopBehavior {
    let Some(entity) = entity_for_owner(game, &key.owner) else {
        return AudioStopBehavior::Immediate;
    };
    game.ecs
        .get::<AudioSource>(entity)
        .and_then(|source| source.groups.get(&key.group))
        .map(|group| group.stop_behavior.clone())
        .unwrap_or(AudioStopBehavior::Immediate)
}

/// Returns every auto-start looping sound that should currently be active.
pub fn desired_auto_loop_specs(game: &Game) -> Vec<DesiredLoopSpec> {
    let mut specs = Vec::new();
    collect_current_world_singleton_loops(game, &mut specs);
    collect_current_room_singleton_loops(game, &mut specs);
    collect_active_entity_loops(game, &mut specs);
    specs.sort_by(|left, right| {
        left.key
            .owner
            .cmp(&right.key.owner)
            .then(left.key.group.cmp(&right.key.group))
    });
    specs
}

/// Reconciles active scoped loops against the current game state.
pub fn reconcile_scoped_audio(game: &Game, audio_manager: &mut AudioManager) {
    let desired = desired_auto_loop_specs(game);
    let current = audio_manager.active_loop_keys();

    for spec in &desired {
        if current.contains(&spec.key) {
            continue;
        }
        push_audio_command(AudioCommand::PlayLoop {
            key: spec.key.clone(),
            sounds: spec.sounds.clone(),
            volume: spec.volume,
            pitch_variation: spec.pitch_variation,
            volume_variation: spec.volume_variation,
        });
    }

    for stale in current
        .into_iter()
        .filter(|key| !desired.iter().any(|spec| spec.key == *key))
    {
        push_audio_command(AudioCommand::StopLoops {
            owner: stale.owner.clone(),
            fade_out: authored_stop_behavior_for_key(game, &stale).fade_duration(),
        });
    }

    audio_manager.poll(0.0);
}

fn collect_current_world_singleton_loops(game: &Game, specs: &mut Vec<DesiredLoopSpec>) {
    let world = game.current_world();
    let Some(entity) = world.singleton else {
        return;
    };
    collect_entity_loops(game, entity, AudioPlaybackOwner::World(world.id), specs);
}

fn collect_current_room_singleton_loops(game: &Game, specs: &mut Vec<DesiredLoopSpec>) {
    let Some(room) = game.current_world().current_room() else {
        return;
    };
    let Some(entity) = room.singleton else {
        return;
    };
    collect_entity_loops(game, entity, AudioPlaybackOwner::Room(room.id), specs);
}

fn collect_active_entity_loops(game: &Game, specs: &mut Vec<DesiredLoopSpec>) {
    let entities = game
        .ecs
        .get_store::<AudioSource>()
        .data
        .keys()
        .copied()
        .collect::<Vec<_>>();

    for entity in entities {
        if !game
            .ecs
            .get::<Active>(entity)
            .is_some_and(Active::is_enabled)
        {
            continue;
        }
        let owner = classify_audio_owner(game, entity);
        if !matches!(owner, AudioPlaybackOwner::Entity(_)) {
            continue;
        }
        collect_entity_loops(game, entity, owner, specs);
    }
}

fn collect_entity_loops(
    game: &Game,
    entity: Entity,
    owner: AudioPlaybackOwner,
    specs: &mut Vec<DesiredLoopSpec>,
) {
    let Some(source) = game.ecs.get::<AudioSource>(entity) else {
        return;
    };

    for (group_id, group) in &source.groups {
        if matches!(group_id, SoundGroupId::New)
            || !group.looping
            || !matches!(group.trigger, AudioTrigger::OnOwnerActivate)
        {
            continue;
        }

        let sounds = sound_command_ids(&game.asset_registry, group.sounds.iter().copied());
        if sounds.is_empty() {
            continue;
        }

        specs.push(DesiredLoopSpec {
            key: AudioLoopKey::new(owner.clone(), group_id.clone()),
            sounds,
            volume: (group.volume * source.runtime_volume).clamp(0.0, 1.0),
            pitch_variation: group.pitch_variation,
            volume_variation: group.volume_variation,
            stop_behavior: group.stop_behavior.clone(),
        });
    }
}

fn entity_for_owner(game: &Game, owner: &AudioPlaybackOwner) -> Option<Entity> {
    match owner {
        AudioPlaybackOwner::Entity(entity) => Some(*entity),
        AudioPlaybackOwner::Room(room_id) => game
            .world_of_room(*room_id)
            .and_then(|world| world.get_room(*room_id))
            .and_then(|room| room.singleton),
        AudioPlaybackOwner::World(world_id) => game.get_world(*world_id).and_then(|world| world.singleton),
    }
}

#[cfg(test)]
mod tests {
    use super::desired_auto_loop_specs;
    use engine_core::audio::AudioPlaybackOwner;
    use engine_core::constants::paths::SFX_FOLDER;
    use engine_core::ecs::{
        AudioGroup, AudioSource, AudioStopBehavior, AudioTrigger, Entity, SoundGroupId,
        SoundId,
    };
    use engine_core::game::Game;
    use engine_core::worlds::{Room, RoomId, World, WorldId};
    use std::collections::HashMap;
    use std::path::PathBuf;

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
            .singleton = Some(entity);
    }

    fn attach_world_singleton_auto_loop(
        game: &mut Game,
        group_name: &str,
        sound_name: &str,
        sound_id: SoundId,
    ) {
        let entity = game.ecs.create_entity().finish();
        insert_auto_loop_source(
            game,
            entity,
            group_name,
            sound_id,
            AudioStopBehavior::Immediate,
        );
        register_sound(game, sound_id, sound_name);
        game.current_world_mut().unwrap().singleton = Some(entity);
    }

    #[test]
    fn desired_auto_loops_include_current_room_and_world_singletons_but_not_frontier_neighbors() {
        let mut game = two_room_game();
        {
            let world = game.current_world_mut().unwrap();
            world.get_room_mut(RoomId(1)).unwrap().adjacent_rooms = vec![RoomId(2)];
            world.get_room_mut(RoomId(2)).unwrap().adjacent_rooms = vec![RoomId(1)];
        }
        attach_room_singleton_auto_loop(
            &mut game,
            RoomId(1),
            "Ambience",
            "room_current",
            SoundId(1),
            AudioStopBehavior::Immediate,
        );
        attach_room_singleton_auto_loop(
            &mut game,
            RoomId(2),
            "Neighbor",
            "room_neighbor",
            SoundId(2),
            AudioStopBehavior::Immediate,
        );
        attach_world_singleton_auto_loop(&mut game, "WorldHum", "world_hum", SoundId(3));

        let desired = desired_auto_loop_specs(&game);

        assert!(desired
            .iter()
            .any(|spec| matches!(spec.key.owner, AudioPlaybackOwner::Room(RoomId(1)))));
        assert!(desired
            .iter()
            .any(|spec| matches!(spec.key.owner, AudioPlaybackOwner::World(WorldId(1)))));
        assert!(!desired
            .iter()
            .any(|spec| matches!(spec.key.owner, AudioPlaybackOwner::Room(RoomId(2)))));
    }
}
