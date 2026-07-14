use super::super::PropertyModule;
use crate::gui::widgets::audio_source_module_core::AudioSourceModuleCore;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;

/// Authors an AudioSource on the world's singleton entity.
pub struct WorldAudioSourceModule {
    core: AudioSourceModuleCore,
}

impl WorldAudioSourceModule {
    /// Creates a new world audio source module.
    pub fn new() -> Self {
        Self {
            core: AudioSourceModuleCore::default(),
        }
    }
}

impl Default for WorldAudioSourceModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldAudioSourceModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _world: &mut World,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let entity = game_ctx
            .world
            .as_deref()
            .expect("world must exist")
            .singleton;
        self.core.draw(ctx, false, rect, game_ctx, entity);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.core.body_layout()
    }

    fn title(&self) -> &str {
        "Audio"
    }
}
