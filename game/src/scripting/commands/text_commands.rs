// game/src/scripting/commands/text_commands.rs
use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::prelude::*;

/// Command to show a speech bubble on an entity.
pub struct ShowSpeechCmd {
    pub entity: Entity,
    pub text: String,
    pub duration: f32,
    pub color: Option<[f32; 4]>,
    pub offset: Option<(f32, f32)>,
    pub font_size: Option<f32>,
    pub max_width: Option<f32>,
    pub show_background: Option<bool>,
    pub background_color: Option<[f32; 4]>,
}

impl LuaCommand for ShowSpeechCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let config = &game_instance.game.text_manager.config;

        let bubble = SpeechBubble {
            text: self.text.clone(),
            timer: self.duration,
            color: self.color.unwrap_or(config.default_color),
            offset: self.offset.unwrap_or((0.0, config.default_offset_y)),
            font_size: self.font_size,
            max_width: self.max_width,
            show_background: self.show_background.unwrap_or(config.show_background),
            background_color: self
                .background_color
                .unwrap_or(config.default_background_color),
        };

        game_instance
            .game
            .ecs
            .add_component_to_entity(self.entity, bubble);
    }
}

/// Command to clear a speech bubble from an entity.
pub struct ClearSpeechCmd {
    pub entity: Entity,
}

impl LuaCommand for ClearSpeechCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        let ecs = &mut game_instance.game.ecs;
        engine_core::text::clear_speech(ecs, self.entity);
    }
}

/// Command to set the current text display language.
pub struct SetLanguageCmd {
    pub language: String,
}

impl LuaCommand for SetLanguageCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        game_instance.game.text_manager.set_language(&self.language);
    }
}
