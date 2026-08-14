use bishop::prelude::*;
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::ecs::*;
use engine_core::worlds::*;

#[derive(Clone, Copy, Debug)]
enum CreateSceneEntityKind {
    RoomEntity {
        room_id: RoomId,
        layer: RoomLayer,
        position: Vec2,
        parent: Option<Entity>,
    },
    GlobalEntity,
    PlayerProxy {
        room_id: RoomId,
        layer: RoomLayer,
        position: Vec2,
    },
    RoomCamera {
        room_id: RoomId,
        layer: RoomLayer,
        position: Vec2,
        grid_size: f32,
    },
}

/// Undo-able command for creating a scene entity from editor UI actions.
#[derive(Debug)]
pub struct CreateSceneEntityCmd {
    mode: EditorMode,
    kind: CreateSceneEntityKind,
    created_entity: Option<Entity>,
}

impl CreateSceneEntityCmd {
    pub const ROOM_ENTITY_NAME: &'static str = "Entity";
    pub const GLOBAL_ENTITY_NAME: &'static str = "Global Entity";
    pub const PLAYER_PROXY_NAME: &'static str = "Player Proxy";

    pub fn new_room_entity(room_id: RoomId, layer: RoomLayer, position: Vec2, parent: Option<Entity>) -> Self {
        Self {
            mode: EditorMode::Room(room_id),
            kind: CreateSceneEntityKind::RoomEntity {
                room_id,
                layer,
                position,
                parent,
            },
            created_entity: None,
        }
    }

    pub fn new_global_entity(room_id: RoomId) -> Self {
        Self {
            mode: EditorMode::Room(room_id),
            kind: CreateSceneEntityKind::GlobalEntity,
            created_entity: None,
        }
    }

    pub fn new_player_proxy(room_id: RoomId, layer: RoomLayer, position: Vec2) -> Self {
        Self {
            mode: EditorMode::Room(room_id),
            kind: CreateSceneEntityKind::PlayerProxy { room_id, layer, position },
            created_entity: None,
        }
    }

    pub fn new_room_camera(room_id: RoomId, layer: RoomLayer, position: Vec2, grid_size: f32) -> Self {
        Self {
            mode: EditorMode::Room(room_id),
            kind: CreateSceneEntityKind::RoomCamera {
                room_id,
                layer,
                position,
                grid_size,
            },
            created_entity: None,
        }
    }

    fn sync_room_selection(editor: &mut crate::app::Editor) {
        let room_editor = &mut editor.room_editor;
        if let Some(entity) = room_editor.single_selected_entity() {
            room_editor.inspector.select_entity(entity);
        } else {
            room_editor.inspector.select_room();
        }
    }
}

impl EditorCommand for CreateSceneEntityCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let ecs = &mut editor.game.ecs;
            let entity = match self.kind {
                CreateSceneEntityKind::RoomEntity {
                    room_id,
                    layer,
                    position,
                    parent,
                } => {
                    let entity = ecs
                        .create_entity()
                        .with(Transform {
                            position,
                            ..Default::default()
                        })
                        .with(Name(Self::ROOM_ENTITY_NAME.to_string()))
                        .with_current_room_layer(room_id, layer)
                        .finish();

                    if let Some(parent) = parent {
                        set_parent(ecs, entity, parent);
                    }

                    editor.room_editor.set_selected_entity(Some(entity));
                    entity
                }
                CreateSceneEntityKind::GlobalEntity => ecs
                    .create_entity()
                    .with(Global::default())
                    .with(Name(Self::GLOBAL_ENTITY_NAME.to_string()))
                    .finish(),
                CreateSceneEntityKind::PlayerProxy { room_id, layer, position } => ecs
                    .create_entity()
                    .with(PlayerProxy)
                    .with(Transform {
                        position,
                        ..Default::default()
                    })
                    .with(Name(Self::PLAYER_PROXY_NAME.to_string()))
                    .with_current_room_layer(room_id, layer)
                    .finish(),
                CreateSceneEntityKind::RoomCamera {
                    room_id,
                    layer,
                    position,
                    grid_size,
                } => {
                    let entity = Room::create_camera_entity(ecs, room_id, position, grid_size);
                    ecs.set_entity_layer(entity, layer);
                    editor.room_editor.set_selected_entity(Some(entity));
                    entity
                }
            };

            self.created_entity = Some(entity);
        });
    }

    fn undo(&mut self) {
        let Some(entity) = self.created_entity.take() else {
            return;
        };

        with_editor(|editor| {
            let ctx = &mut editor.game.ctx_mut();
            Ecs::remove_entity(ctx, entity);

            if editor.room_editor.selected_entities.remove(&entity) {
                Self::sync_room_selection(editor);
            }
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        match self.kind {
            CreateSceneEntityKind::GlobalEntity => matches!(current_mode, EditorMode::Room(_)),
            _ => self.mode == current_mode,
        }
    }
}
