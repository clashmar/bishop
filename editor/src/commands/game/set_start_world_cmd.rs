use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::worlds::WorldId;

/// Undoable change to the world entered on game startup.
#[derive(Debug)]
pub struct SetStartWorldCmd {
    new_world: WorldId,
    old_world: Option<WorldId>,
}

impl SetStartWorldCmd {
    /// Creates a start-world change from the current editor selection.
    pub fn new(new_world: WorldId, old_world: Option<WorldId>) -> Self {
        Self { new_world, old_world }
    }
}

impl EditorCommand for SetStartWorldCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            if editor.game.set_start_world(self.new_world).is_ok() {
                editor.save();
            }
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            editor.game.start_world_id = self.old_world;
            editor.save();
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Game(_))
    }
}

#[cfg(test)]
mod tests {
    use super::SetStartWorldCmd;
    use crate::app::{Editor, EditorMode};
    use crate::commands::editor_command_manager::EditorCommand;
    use crate::editor_global::with_editor;
    use crate::storage::game_io::create_new_game;
    use crate::test_utils::{game_fs_test_lock, EditorServicesGuard, TestGameFolder};
    use crate::world::world_creation::create_new_world;
    use engine_core::engine_global::set_game_name;
    use engine_core::storage::{load_full_game_from_folder, resources_folder};

    #[test]
    fn changing_start_world_saves_and_undo_restores_the_previous_choice() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let folder = TestGameFolder::new("set_start_world_cmd");
        set_game_name(folder.name());
        let mut game = create_new_game(folder.name().to_string());
        let original = game.start_world_id;
        let world = create_new_world(&mut game);
        let other = world.id;
        game.add_world(world);
        let editor = Editor {
            game,
            mode: EditorMode::Game(crate::game::GameEditorSubmode::Worlds),
            ..Default::default()
        };
        let _guard = EditorServicesGuard::install(editor);
        let mut cmd = SetStartWorldCmd::new(other, original);

        cmd.execute();
        with_editor(|editor| assert_eq!(editor.game.start_world_id, Some(other)));
        let saved = load_full_game_from_folder(&resources_folder(folder.name())).unwrap();
        assert_eq!(saved.start_world_id, Some(other));
        cmd.undo();
        with_editor(|editor| assert_eq!(editor.game.start_world_id, original));
    }
}
