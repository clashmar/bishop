use crate::gui::gui_constants;
use crate::gui::text_input::committed_name_change;
use crate::gui::text_input::draw_labeled_text_input;
use crate::shared::scene_ui::inspector::InspectorContent;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction, InspectorOutput};
use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use widgets::constants::{colors, layout};
use widgets::*;

/// Editable properties for the current game.
pub struct GameProperties {
    input_id: WidgetId,
    start_world_id: WidgetId,
}

impl GameProperties {
    /// Creates a new game properties pane.
    pub fn new() -> Self {
        Self {
            input_id: WidgetId::default(),
            start_world_id: WidgetId::default(),
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
        blocked: bool,
        game_ctx: &mut GameCtxMut,
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

        let options: Vec<String> = game_ctx.world_directory.iter()
            .map(|world| format!("{} ({})", world.name, world.id.0))
            .collect();
        let current_label = game_ctx.world_directory.iter()
            .position(|world| Some(world.id) == insp_ctx.start_world_id)
            .and_then(|index| options.get(index))
            .map(String::as_str)
            .unwrap_or("Select start world");
        ctx.draw_text(
            "Start World:",
            rect.x + 10.0,
            rect.y + 53.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        if let Some(selection) = Dropdown::new(
            self.start_world_id,
            Rect::new(rect.x + 10.0, rect.y + 76.0, rect.w - 20.0, 30.0),
            current_label,
            &options,
            |label| label.clone(),
        )
        .truncate_trigger_text()
        .suppressed(blocked)
        .show(ctx)
        {
            if let Some(index) = options.iter().position(|option| option == &selection) {
                let id = game_ctx.world_directory[index].id;
                if Some(id) != insp_ctx.start_world_id {
                    output.host_action = Some(InspectorHostAction::SetStartWorld(id));
                }
            }
        }

        output
    }

    fn total_content_height(
        &self,
        _game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> f32 {
        120.0
    }
}
