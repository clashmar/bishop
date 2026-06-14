use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::with_editor;
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::worlds::*;

/// Undo-able command for editing world properties.
#[derive(Debug)]
pub struct EditWorldCmd {
    world_id: WorldId,
    old_name: String,
    old_sprite: Option<SpriteId>,
    old_overlay: bool,
    new_name: Option<String>,
    new_sprite: Option<Option<SpriteId>>,
    new_overlay: Option<bool>,
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
            old_overlay: false,
            new_name,
            new_sprite,
            new_overlay: None,
        }
    }

    /// Sets the `overlay` flag to change.
    pub fn with_overlay(mut self, overlay: bool) -> Self {
        self.new_overlay = Some(overlay);
        self
    }

    fn apply(
        game: &mut Game,
        world_id: WorldId,
        name: Option<&str>,
        sprite: Option<Option<SpriteId>>,
        overlay: Option<bool>,
    ) {
        if let Some(world) = game.get_world_mut(world_id) {
            if let Some(name) = name {
                world.name = name.to_owned();
            }
            if let Some(o) = overlay {
                world.overlay = o;
            }
        }
        if let Some(sprite_opt) = sprite {
            game.set_world_sprite(world_id, sprite_opt);
        }
    }

    fn capture_original_state(&mut self, game: &Game) {
        if let Some(world) = game.get_world(self.world_id) {
            self.old_name = world.name.clone();
            self.old_sprite = world.meta.sprite_id;
            self.old_overlay = world.overlay;
        }
    }
}

impl EditorCommand for EditWorldCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            self.capture_original_state(&editor.game);
        });

        with_editor(|editor| {
            Self::apply(
                &mut editor.game,
                self.world_id,
                self.new_name.as_deref(),
                self.new_sprite,
                self.new_overlay,
            );
        });
    }

    fn undo(&mut self) {
        with_editor(|editor| {
            Self::apply(
                &mut editor.game,
                self.world_id,
                Some(&self.old_name),
                Some(self.old_sprite),
                Some(self.old_overlay),
            );
        });
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Game | EditorMode::World(_))
    }
}
