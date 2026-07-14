pub(crate) mod groups;
pub(crate) mod layout;
pub(crate) mod preview;

pub use self::preview::clear_active_audio_preview;
use crate::gui::widgets::audio_source_module_core::AudioSourceModuleCore;
use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::game::GameCtxMut;

const TITLE: &str = "Audio Source";

/// Editor inspector module for the `AudioSource` component.
/// Thin wrapper that delegates authoring to the shared `AudioSourceModuleCore`.
#[derive(Default)]
pub struct AudioSourceModule {
    core: AudioSourceModuleCore,
}

impl InspectorModule for AudioSourceModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        AudioSourceModuleCore::undo_component_type()
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<AudioSource>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        clear_active_audio_preview();
        Ecs::remove_component::<AudioSource>(game_ctx, entity);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.core.body_layout()
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        self.core.draw(ctx, blocked, rect, game_ctx, entity);
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: <AudioSource>::TYPE_NAME,
        title: TITLE,
        factory: || {
            Box::new(
                CollapsibleComponentModule::new(
                    crate::gui::inspector::audio_source_module::AudioSourceModule::default()
                )
                .with_title(TITLE)
            )
        },
        allowed_for: None,
    }
}

#[cfg(test)]
mod tests;
