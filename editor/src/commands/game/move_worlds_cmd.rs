use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::game::GameEditorSubmode;
use crate::with_editor;
use bishop::prelude::Vec2;
use engine_core::game::Game;
use engine_core::worlds::WorldId;

/// Moves multiple worlds as one undoable command.
#[derive(Debug)]
pub struct MoveWorldsCmd {
    worlds: Vec<(WorldId, Vec2, Vec2)>,
}

impl MoveWorldsCmd {
    /// Creates a command from world IDs and their before/after positions.
    pub fn new(worlds: Vec<(WorldId, Vec2, Vec2)>) -> Self {
        Self { worlds }
    }

    fn set_position(game: &mut Game, world_id: WorldId, position: Vec2) {
        if let Some(world) = game.get_world_mut(world_id) {
            world.meta.position = position;
        }
    }
}

impl EditorCommand for MoveWorldsCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            for (world_id, _, to) in &self.worlds {
                Self::set_position(&mut editor.game, *world_id, *to);
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            for (world_id, from, _) in &self.worlds {
                Self::set_position(&mut editor.game, *world_id, *from);
            }
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        current_mode == EditorMode::Game(GameEditorSubmode::Worlds)
    }
}

#[cfg(test)]
mod tests {
    use super::MoveWorldsCmd;
    use crate::commands::editor_command_manager::EditorCommandManager;
    use crate::editor_global::with_editor;
    use crate::test_utils::setup_editor;
    use crate::world::world_creation::create_new_world;
    use bishop::prelude::Vec2;

    #[test]
    fn move_worlds_cmd_execute_undo_redo_moves_selected_group_atomically() {
        let _context = setup_editor("move_worlds_cmd");
        let targets = with_editor(|editor| {
            let first_world_id = editor.game.worlds()[0].id;
            let second_world = create_new_world(&mut editor.game);
            let second_world_id = second_world.id;
            editor.game.add_world(second_world);

            [first_world_id, second_world_id]
                .map(|world_id| {
                    let from = editor
                        .game
                        .get_world(world_id)
                        .expect("created world should be present")
                        .meta
                        .position;
                    let to = from + Vec2::new(10.0, 20.0);
                    (world_id, from, to)
                })
                .to_vec()
        });

        let mut command_manager = EditorCommandManager::new();
        command_manager.push(Box::new(MoveWorldsCmd::new(targets.clone())));
        command_manager.apply_all();

        with_editor(|editor| {
            for (world_id, _, to) in &targets {
                assert_eq!(
                    editor
                        .game
                        .get_world(*world_id)
                        .expect("world should be present")
                        .meta
                        .position,
                    *to
                );
            }
        });
        assert_eq!(command_manager.undo_stack_len(), 1);

        command_manager.undo();

        with_editor(|editor| {
            for (world_id, from, _) in &targets {
                assert_eq!(
                    editor
                        .game
                        .get_world(*world_id)
                        .expect("world should be present")
                        .meta
                        .position,
                    *from
                );
            }
        });

        command_manager.redo();

        with_editor(|editor| {
            for (world_id, _, to) in &targets {
                assert_eq!(
                    editor
                        .game
                        .get_world(*world_id)
                        .expect("world should be present")
                        .meta
                        .position,
                    *to
                );
            }
        });
        assert_eq!(command_manager.undo_stack_len(), 1);
    }
}
