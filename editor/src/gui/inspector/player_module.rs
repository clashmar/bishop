// editor/src/gui/inspector/player_module.rs
use bishop::prelude::*;
use engine_core::prelude::*;

#[derive(Default)]
pub struct PlayerModule {}

impl InspectorModule for PlayerModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        None
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.has::<Player>(entity) || ecs.has::<PlayerProxy>(entity)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        _blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let ecs = &game_ctx.ecs;

        if ecs.has::<Player>(entity) {
            ctx.draw_text(
                "Player Entity",
                rect.x,
                rect.y + 20.0,
                18.0,
                FIELD_TEXT_COLOR,
            );
        } else if ecs.has::<PlayerProxy>(entity) {
            ctx.draw_text(
                "Player Proxy",
                rect.x,
                rect.y + 20.0,
                18.0,
                FIELD_TEXT_COLOR,
            );
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new().top_padding(0.0).block(28.0)
    }

    fn title(&self) -> &str {
        std::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .unwrap_or("Player")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_body_layout_keeps_shared_bottom_gutter() {
        let module = PlayerModule::default();

        assert_eq!(module.body_layout().height(), 38.0);
    }
}
