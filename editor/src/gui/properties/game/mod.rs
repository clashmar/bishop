pub mod game_text_settings_module;

use crate::gui::gui_constants;
use crate::gui::text_input::committed_name_change;
use crate::gui::text_input::draw_labeled_text_input;
use crate::shared::scene_ui::inspector::InspectorContent;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction, InspectorOutput};
use super::collapsible::{CollapsiblePropertyModule, COLLAPSIBLE_CONTENT_INSET};
use super::PropertyModule;
use game_text_settings_module::GameTextSettingsModule;
use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use widgets::constants::{colors, layout};
use widgets::*;

const FIELD_HEIGHT: f32 = layout::DEFAULT_FIELD_HEIGHT;
const START_WORLD_FIELD_TOP: f32 = 90.0;
const GAME_FIELDS_HEIGHT: f32 = START_WORLD_FIELD_TOP + FIELD_HEIGHT;
const TEXT_SETTINGS_MODULE_INSET: f32 = layout::WIDGET_PADDING - COLLAPSIBLE_CONTENT_INSET;

/// Editable properties for the current game.
pub struct GameProperties {
    input_id: WidgetId,
    start_world_id: WidgetId,
    text_settings: CollapsiblePropertyModule<(), GameTextSettingsModule>,
}

impl GameProperties {
    /// Creates a new game properties pane.
    pub fn new() -> Self {
        Self {
            input_id: WidgetId::default(),
            start_world_id: WidgetId::default(),
            text_settings: CollapsiblePropertyModule::new(GameTextSettingsModule::new()),
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

        let sub_rect = Rect::new(
            rect.x + 10.0,
            rect.y + 10.0,
            rect.w - 20.0,
            FIELD_HEIGHT,
        );
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
            rect.y + 67.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        if let Some(selection) = Dropdown::new(
            self.start_world_id,
            Rect::new(
                rect.x + 10.0,
                rect.y + START_WORLD_FIELD_TOP,
                rect.w - 20.0,
                FIELD_HEIGHT,
            ),
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

        let settings_height = self.text_settings.height();
        let settings_rect = text_settings_module_rect(
            rect,
            rect.y + GAME_FIELDS_HEIGHT + layout::WIDGET_SPACING,
            settings_height,
        );
        self.text_settings
            .draw(ctx, settings_rect, &mut (), game_ctx, insp_ctx);
        let settings_action = self.text_settings.take_host_action();
        if output.host_action.is_none() {
            output.host_action = settings_action;
        }

        output
    }

    fn total_content_height(
        &self,
        _game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> f32 {
        GAME_FIELDS_HEIGHT + layout::WIDGET_SPACING + self.text_settings.height()
    }
}

fn text_settings_module_rect(pane: Rect, y: f32, height: f32) -> Rect {
    Rect::new(
        pane.x + TEXT_SETTINGS_MODULE_INSET,
        y,
        pane.w - TEXT_SETTINGS_MODULE_INSET * 2.0,
        height,
    )
}

#[cfg(test)]
mod tests {
    use bishop::Rect;
    use widgets::constants::layout;

    use super::{text_settings_module_rect, COLLAPSIBLE_CONTENT_INSET};

    #[test]
    fn text_settings_content_matches_game_field_width() {
        let pane = Rect::new(20.0, 0.0, 350.0, 400.0);
        let module = text_settings_module_rect(pane, 150.0, 100.0);
        let content_x = module.x + COLLAPSIBLE_CONTENT_INSET;
        let content_width = module.w - COLLAPSIBLE_CONTENT_INSET * 2.0;

        assert_eq!(content_x, pane.x + layout::WIDGET_PADDING);
        assert_eq!(
            content_width,
            pane.w - layout::WIDGET_PADDING * 2.0
        );
    }
}
