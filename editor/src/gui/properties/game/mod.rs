use crate::gui::gui_constants;
use crate::gui::text_input::committed_name_change;
use crate::gui::text_input::draw_labeled_text_input;
use crate::shared::scene_ui::inspector::InspectorContent;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction, InspectorOutput};
use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use engine_core::prelude::*;

/// Editable properties for the current game.
pub struct GameProperties {
    input_id: WidgetId,
}

impl GameProperties {
    /// Creates a new game properties pane.
    pub fn new() -> Self {
        Self {
            input_id: WidgetId::default(),
        }
    }
}

impl InspectorContent for GameProperties {
    fn header_height(&self) -> f32 {
        gui_constants::inspector::HEADER_HEIGHT
    }

    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _blocked: bool,
        _game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        let mut output = InspectorOutput::default();
        let Some(ref name) = insp_ctx.game_name else {
            return output;
        };

        let sub_rect = Rect::new(rect.x + 10.0, rect.y + 10.0, rect.w - 20.0, 30.0);
        let (edited, commit) =
            draw_labeled_text_input(ctx, sub_rect, "Name:", name, self.input_id);
        if let Some(new_name) = committed_name_change(name, &edited, commit) {
            output.host_action = Some(InspectorHostAction::RenameGame(new_name));
        }

        output
    }

    fn total_content_height(
        &self,
        _game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> f32 {
        30.0 + 20.0
    }
}
