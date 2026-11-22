use super::super::PropertyModule;
use super::super::collapsible::COLLAPSIBLE_CONTENT_INSET;
use crate::gui::asset_import::import_asset_from_dialog;
use crate::gui::widgets::font_selector::draw_font_dropdown;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use bishop::prelude::*;
use engine_core::assets::AssetKind;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::ui::measure_text;
use ::widgets::constants::{colors, layout};
use widgets::{Button, WidgetId};

/// Edits game-wide text presentation settings.
pub struct GameTextSettingsModule {
    font_dropdown_id: WidgetId,
    pending_host_action: Option<InspectorHostAction>,
}

impl GameTextSettingsModule {
    pub fn new() -> Self {
        Self {
            font_dropdown_id: WidgetId::default(),
            pending_host_action: None,
        }
    }
}

impl Default for GameTextSettingsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<()> for GameTextSettingsModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _target: &mut (),
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) {
        let padding = COLLAPSIBLE_CONTENT_INSET;
        let row_gap = layout::WIDGET_SPACING;
        let row_height = layout::DEFAULT_FIELD_HEIGHT;
        let label = "Default Font:";
        let label_x = rect.x + padding;
        let label_y = rect.y + row_gap;
        let label_width = measure_text(ctx, label, layout::FIELD_TEXT_SIZE_16).width;
        ctx.draw_text(
            label,
            label_x,
            label_y + 20.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        let dropdown_x = label_x + label_width + row_gap;
        let dropdown_y = label_y;
        let dropdown_width = (rect.x + rect.w - padding - dropdown_x).max(0.0);
        let dropdown_rect = Rect::new(dropdown_x, dropdown_y, dropdown_width, row_height);
        if let Some(font) = draw_font_dropdown(
            ctx,
            self.font_dropdown_id,
            dropdown_rect,
            game_ctx.asset_registry,
            insp_ctx.game_default_font,
            false,
        ) {
            if font != insp_ctx.game_default_font {
                self.pending_host_action = Some(InspectorHostAction::SetGameDefaultFont(font));
            }
        }

        let import_rect = Rect::new(
            rect.x + padding,
            dropdown_y + row_height + row_gap,
            rect.w - padding * 2.0,
            row_height,
        );
        if Button::new(import_rect, "Import Font…").show(ctx) {
            import_asset_from_dialog(Some(AssetKind::Font), None);
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(layout::WIDGET_SPACING)
            .rows(2, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "Text Settings"
    }

    fn take_host_action(&mut self) -> Option<InspectorHostAction> {
        self.pending_host_action.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_layout_when_showing_all_controls_accounts_for_two_rows() {
        let module = GameTextSettingsModule::new();
        let expected = InspectorBodyLayout::new()
            .top_padding(layout::WIDGET_SPACING)
            .rows(2, layout::WIDGET_SPACING)
            .height();

        assert_eq!(module.body_layout().height(), expected);
    }
}
