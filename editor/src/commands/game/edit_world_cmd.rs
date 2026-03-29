// editor/src/commands/game/edit_world_cmd.rs
use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::prelude::*;

/// Undo-able command for editing world properties.
#[derive(Debug)]
pub struct EditWorldCmd {
    world_id: WorldId,
    /// Values before the edit
    old_name: String,
    old_sprite: Option<SpriteId>,
    /// Values after the edit
    new_name: Option<String>,
    new_sprite: Option<Option<SpriteId>>,
}

impl EditWorldCmd {
    pub fn new(
        world_id: WorldId,
        new_name: Option<String>,
        new_sprite: Option<Option<SpriteId>>,
    ) -> Self {
        Self {
            world_id,
            old_name: String::new(),
            old_sprite: None,
            new_name,
            new_sprite,
        }
    }

    /// Helper that writes the supplied values into the world.
    fn apply(
        game: &mut Game,
        world_id: WorldId,
        name: Option<&str>,
        sprite: Option<Option<SpriteId>>,
    ) {
        if let Some(world) = game.worlds.iter_mut().find(|w| w.id == world_id) {
            if let Some(name) = name {
                world.name = name.to_owned();
            }
            if let Some(sprite_opt) = sprite {
                world.meta.set_sprite(sprite_opt, &mut game.asset_manager);
            }
        }
    }

    /// Capture the current state of the world.
    fn capture_original_state(&mut self, game: &Game) {
        if let Some(world) = game.worlds.iter().find(|w| w.id == self.world_id) {
            self.old_name = world.name.clone();
            self.old_sprite = world.meta.sprite_id;
        }
    }
}

impl EditorCommand for EditWorldCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            self.capture_original_state(&editor.game);
        });

        // Apply the new values
        with_editor(|editor| {
            let game = &mut editor.game;
            Self::apply(
                game,
                self.world_id,
                self.new_name.as_deref(),
                self.new_sprite,
            );

            // Persist the change
            editor.save();
        });
    }

    fn undo(&mut self) {
        // Restore the old values
        with_editor(|editor| {
            let game = &mut editor.game;
            Self::apply(
                game,
                self.world_id,
                Some(&self.old_name),
                Some(self.old_sprite),
            );
            editor.save();
        });
    }

    fn mode(&self) -> EditorMode {
        EditorMode::Game
    }
}
