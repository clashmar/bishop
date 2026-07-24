use super::*;
use engine_core::tiles::apply_tile_definition_to_entity;

fn enter_room_mode() {
    with_editor(|editor| {
        let world_id = editor
            .game
            .current_world_id
            .expect("test editor should have a current world");
        let room_id = editor
            .game
            .current_world()
            .rooms()
            .first()
            .map(|room| room.id)
            .expect("test editor should have a room");
        editor
            .game
            .get_world_mut(world_id)
            .expect("test editor should resolve current world")
            .current_room_id = Some(room_id);
        editor.mode = EditorMode::Room(room_id);
        editor.cur_world_id = Some(world_id);
        editor.cur_room_id = Some(room_id);
    });
}

#[test]
fn tile_definition_commands_when_undone_then_registry_returns_to_previous_state() {
    let _ctx = setup_editor("tile_definition_cmds");
    enter_room_mode();
    let before = with_editor(|editor| editor.game.tile_registry.len());

    push_command(Box::new(CreateTileDefinitionCmd::new(TileDef {
        sprite_id: SpriteId(11),
        components: vec![tile_definition_component_snapshot(Solid(true))],
    })));
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(editor.game.tile_registry.len(), before + 1);
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(editor.game.tile_registry.len(), before);
    });
}

#[test]
fn tile_definition_commands_when_deleted_then_undo_restores_the_definition() {
    let _ctx = setup_editor("tile_definition_delete_cmd");
    enter_room_mode();

    let (tile_id, before) = with_editor(|editor| {
        let tile_id = editor.game.tile_registry.insert(TileDef {
            sprite_id: SpriteId(5),
            components: vec![tile_definition_component_snapshot(Solid(true))],
        });
        let before = editor
            .game
            .tile_registry
            .get(tile_id)
            .expect("tile should exist before delete")
            .clone();
        (tile_id, before)
    });

    push_command(Box::new(DeleteTileDefinitionCmd::new(tile_id)));
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.game.tile_registry.get(tile_id).is_none());
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        let restored = editor
            .game
            .tile_registry
            .get(tile_id)
            .expect("tile should be restored after undo");
        assert_eq!(restored.sprite_id, before.sprite_id);
        assert_eq!(restored.components, before.components);
    });
}

#[test]
fn tile_definition_commands_when_updated_then_linked_placements_reflow() {
    let _ctx = setup_editor("tile_definition_update_reflow");
    enter_room_mode();

    let (room_id, tile_id, entity, before, after) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let tile_id = editor.game.tile_registry.insert(TileDef {
            sprite_id: SpriteId(2),
            components: vec![tile_definition_component_snapshot(Solid(true))],
        });
        let entity = editor
            .game
            .ecs
            .create_entity()
            .with(TilePlacement::new(tile_id, 1, 2))
            .with_current_room(room_id)
            .finish();
        {
            let mut ctx = editor.game.ctx_mut();
            apply_tile_definition_to_entity(&mut ctx, entity, tile_id);
        }
        let before = editor
            .game
            .tile_registry
            .get(tile_id)
            .expect("tile should exist before update")
            .clone();
        let after = TileDef {
            sprite_id: SpriteId(9),
            components: Vec::new(),
        };
        (room_id, tile_id, entity, before, after)
    });

    push_command(Box::new(UpdateTileDefinitionCmd::new(
        tile_id,
        before.clone(),
        after.clone(),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(editor.cur_room_id, Some(room_id));
        assert!(!editor.game.ecs.has::<Solid>(entity));
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.game.ecs.get::<Solid>(entity).is_some_and(|solid| solid.0));
    });
}

#[test]
fn tile_definition_commands_when_updated_then_registry_uses_new_values() {
    let _ctx = setup_editor("tile_definition_update_cmd");
    enter_room_mode();

    let (tile_id, before, after) = with_editor(|editor| {
        let tile_id = editor.game.tile_registry.insert(TileDef {
            sprite_id: SpriteId(2),
            components: Vec::new(),
        });
        let before = editor
            .game
            .tile_registry
            .get(tile_id)
            .expect("tile should exist before update")
            .clone();
        let after = TileDef {
            sprite_id: SpriteId(9),
            components: vec![tile_definition_component_snapshot(Solid(true))],
        };
        (tile_id, before, after)
    });

    push_command(Box::new(UpdateTileDefinitionCmd::new(
        tile_id,
        before,
        after.clone(),
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(
            editor
                .game
                .tile_registry
                .get(tile_id)
                .expect("tile should exist after update")
                .sprite_id,
            after.sprite_id
        );
        assert_eq!(
            editor
                .game
                .tile_registry
                .get(tile_id)
                .expect("tile should exist after update")
                .components,
            after.components
        );
    });
}
