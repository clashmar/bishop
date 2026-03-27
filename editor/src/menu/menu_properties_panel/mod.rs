// editor/src/menu_editor/menu_properties_panel/mod.rs
mod common_properties;
mod element_properties;
mod layout_properties;
mod menu_properties;
mod nav_section;

use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

pub(crate) const ROW_HEIGHT: f32 = 28.0;
pub(crate) const LABEL_WIDTH: f32 = 80.0;
pub(crate) const FIELD_HEIGHT: f32 = 24.0;

/// Widget IDs for the properties module.
#[derive(Default)]
pub struct PropertiesWidgetIds {
    pub(crate) name_id: WidgetId,
    pub(crate) text_id: WidgetId,
    pub(crate) font_size_id: WidgetId,
    pub(crate) action_id: WidgetId,
    pub(crate) action_param_id: WidgetId,
    pub(crate) z_order_id: WidgetId,
    pub(crate) pos_x_id: WidgetId,
    pub(crate) pos_y_id: WidgetId,
    pub(crate) size_w_id: WidgetId,
    pub(crate) size_h_id: WidgetId,
    pub(crate) button_nav_ids: NavWidgetIds,
    pub(crate) layout_direction_id: WidgetId,
    pub(crate) layout_grid_cols_id: WidgetId,
    pub(crate) layout_spacing_id: WidgetId,
    pub(crate) layout_pad_top_id: WidgetId,
    pub(crate) layout_pad_right_id: WidgetId,
    pub(crate) layout_pad_bottom_id: WidgetId,
    pub(crate) layout_pad_left_id: WidgetId,
    pub(crate) layout_h_align_id: WidgetId,
    pub(crate) layout_v_align_id: WidgetId,
    pub(crate) layout_item_w_id: WidgetId,
    pub(crate) layout_item_h_id: WidgetId,
    pub(crate) layout_nav_ids: NavWidgetIds,
    pub(crate) label_h_align_id: WidgetId,
    pub(crate) panel_color_id: WidgetId,
    pub(crate) panel_opacity_id: WidgetId,
    pub(crate) layout_bg_color_id: WidgetId,
    pub(crate) layout_bg_opacity_id: WidgetId,
    pub(crate) bg_type_id: WidgetId,
    pub(crate) bg_color_id: WidgetId,
    pub(crate) bg_alpha_id: WidgetId,
    pub(crate) mode_id: WidgetId,
    pub(crate) menu_name_id: WidgetId,
    pub(crate) slider_text_id: WidgetId,
    pub(crate) slider_key_id: WidgetId,
    pub(crate) slider_min_id: WidgetId,
    pub(crate) slider_max_id: WidgetId,
    pub(crate) slider_step_id: WidgetId,
    pub(crate) slider_default_id: WidgetId,
}

/// Widget IDs for nav dropdowns.
#[derive(Default, Clone, Copy)]
pub struct NavWidgetIds {
    pub(crate) up: WidgetId,
    pub(crate) down: WidgetId,
    pub(crate) left: WidgetId,
    pub(crate) right: WidgetId,
}

/// Groups property panel data.
pub struct MenuPropertiesPanel {
    pub(crate) scroll_state: ScrollState,
    pub(crate) widget_ids: PropertiesWidgetIds,
    pub(crate) last_content_height: f32,
}

impl MenuPropertiesPanel {
    /// Creates a new properties panel.
    pub fn new() -> Self {
        Self {
            scroll_state: ScrollState::new(),
            widget_ids: PropertiesWidgetIds::default(),
            last_content_height: 0.0,
        }
    }
}

impl Default for MenuPropertiesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuEditor {
    /// Renders the properties panel and handles editing.
    pub fn draw_properties_panel(&mut self, ctx: &mut WgpuContext, rect: Rect, blocked: bool) {
        let content_height = self.properties_panel.last_content_height;

        let area = ScrollableArea::new(rect, content_height)
            .scroll_speed(20.0)
            .blocked(blocked)
            .begin(ctx, &mut self.properties_panel.scroll_state);

        let start_y = rect.y + self.properties_panel.scroll_state.scroll_y + 8.0;
        let mut y = start_y;
        let content_x = rect.x + 8.0;
        let content_w = area.content_rect().w - 16.0;

        if area.is_fully_visible(y, 24.0) {
            ctx.draw_text("Properties", content_x, y + 14.0, 14.0, Color::GREY);
        }
        y += 24.0;

        if self.primary_selected_index().is_none() {
            self.draw_menu_properties(ctx, &mut y, content_x, content_w, blocked, &rect);
            self.properties_panel.last_content_height = y - start_y + 16.0;
            area.draw_scrollbar(ctx, self.properties_panel.scroll_state.scroll_y);
            return;
        }

        let element_kind = self.selected_element().map(|e| e.kind.clone());

        let Some(kind) = element_kind else {
            self.properties_panel.last_content_height = y - start_y + 16.0;
            area.draw_scrollbar(ctx, self.properties_panel.scroll_state.scroll_y);
            return;
        };

        self.draw_common_properties(ctx, &mut y, content_x, content_w, blocked, &rect);
        y += 8.0;

        match kind {
            MenuElementKind::Label(_) => {
                self.draw_label_properties(ctx, &mut y, content_x, content_w, blocked, &rect);
            }
            MenuElementKind::Button(_) => {
                self.draw_button_properties(ctx, &mut y, content_x, content_w, blocked, &rect);
            }
            MenuElementKind::Panel(_) => {
                self.draw_panel_properties(ctx, &mut y, content_x, content_w, blocked, &rect);
            }
            MenuElementKind::LayoutGroup(_) => {
                self.draw_layout_group_properties(
                    ctx, &mut y, content_x, content_w, blocked, &rect,
                );
            }
            MenuElementKind::Slider(_) => {
                self.draw_slider_properties(ctx, &mut y, content_x, content_w, blocked, &rect);
            }
        }

        self.properties_panel.last_content_height = y - start_y + 16.0;
        area.draw_scrollbar(ctx, self.properties_panel.scroll_state.scroll_y);
    }
}
