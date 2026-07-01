use super::super::PropertyModule;
use crate::gui::widgets::audio_source_module_core::AudioSourceModuleCore;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::room::Room;

/// Authors an AudioSource on the room's singleton entity.
pub struct RoomAudioSourceModule {
    core: AudioSourceModuleCore,
}

impl RoomAudioSourceModule {
    /// Creates a new room audio source module.
    pub fn new() -> Self {
        Self {
            core: AudioSourceModuleCore::default(),
        }
    }
}

impl Default for RoomAudioSourceModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<Room> for RoomAudioSourceModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        room: &mut Room,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        self.core.draw(ctx, false, rect, game_ctx, room.singleton);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.core.body_layout()
    }

    fn title(&self) -> &str {
        "Audio"
    }
}
