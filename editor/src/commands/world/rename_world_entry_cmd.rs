use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::worlds::WorldId;

/// Undoable rename of a `WorldEntry` that cascades to every referencing `WorldExit`.
#[derive(Debug)]
pub struct RenameWorldEntryCmd {
    entity: Entity,
    old_name: String,
    new_name: String,
}

impl RenameWorldEntryCmd {
    /// Creates a command renaming `entity`'s `WorldEntry` from `old_name` to `new_name` and cascading to all referencing `WorldExit`s.
    pub fn new(entity: Entity, old_name: String, new_name: String) -> Self {
        Self { entity, old_name, new_name }
    }

    /// Sets the entry name and rewrites all `WorldExit`s pointing at it.
    pub(super) fn apply(game: &mut Game, entity: Entity, from: &str, to: &str) {
        let owning_world: Option<WorldId> = game
            .ecs
            .get::<CurrentRoom>(entity)
            .and_then(|room| game.world_of_room(room.0))
            .map(|world| world.id);

        if let Some(entry) = game.ecs.get_mut::<WorldEntry>(entity) {
            entry.name = to.to_string();
        }

        let Some(owning_world) = owning_world else { return };

        let exits: Vec<Entity> =
            game.ecs.get_store::<WorldExit>().data.keys().copied().collect();
        for exit_entity in exits {
            let matches = game.ecs.get::<WorldExit>(exit_entity).is_some_and(|exit| {
                exit.destination_world == Some(owning_world) && exit.entry.as_deref() == Some(from)
            });
            if matches {
                if let Some(exit) = game.ecs.get_mut::<WorldExit>(exit_entity) {
                    exit.entry = Some(to.to_string());
                }
            }
        }
    }
}

impl EditorCommand for RenameWorldEntryCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            Self::apply(&mut editor.game, self.entity, &self.old_name, &self.new_name);
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            Self::apply(&mut editor.game, self.entity, &self.new_name, &self.old_name);
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Game
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::worlds::world::WorldExitTrigger;
    use engine_core::worlds::*;

    const ORIGINAL_NAME: &str = "Door";
    const RENAMED_NAME: &str = "Gate";

    fn world_with_room(id: usize, room: usize) -> World {
        let mut world = World::new(WorldId(id), String::new(), 16.0);
        world.add_room(Room { id: RoomId(room), ..Default::default() });
        world
    }

    #[test]
    fn cascade_rewrites_only_matching_exits() {
        let mut game = Game::default();
        game.add_world(world_with_room(1, 1));
        game.add_world(world_with_room(2, 2));
        game.select_world(WorldId(1));

        let entry = game
            .ecs
            .create_entity()
            .with(WorldEntry { name: ORIGINAL_NAME.to_string() })
            .with_current_room(RoomId(1))
            .finish();

        // Points at (world 1, ORIGINAL_NAME) — should be rewritten.
        let exit_a = game.ecs.create_entity()
            .with(WorldExit {
                destination_world: Some(WorldId(1)),
                entry: Some(ORIGINAL_NAME.to_string()),
                mode: WorldTransitionMode::Transport,
                trigger: WorldExitTrigger::OnInteract,
            })
            .finish();
        // Points at (world 2, ORIGINAL_NAME) — different world, must NOT change.
        let exit_b = game.ecs.create_entity()
            .with(WorldExit {
                destination_world: Some(WorldId(2)),
                entry: Some(ORIGINAL_NAME.to_string()),
                mode: WorldTransitionMode::Transport,
                trigger: WorldExitTrigger::OnInteract,
            })
            .finish();

        RenameWorldEntryCmd::apply(&mut game, entry, ORIGINAL_NAME, RENAMED_NAME);

        assert_eq!(game.ecs.get::<WorldEntry>(entry).map(|e| e.name.clone()), Some(RENAMED_NAME.to_string()));
        assert_eq!(game.ecs.get::<WorldExit>(exit_a).and_then(|e| e.entry.clone()), Some(RENAMED_NAME.to_string()));
        assert_eq!(game.ecs.get::<WorldExit>(exit_b).and_then(|e| e.entry.clone()), Some(ORIGINAL_NAME.to_string()));
    }
}
