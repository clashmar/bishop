use crate::menu::menu_canvas::drawing::MenuCanvasFrame;
use crate::menu::menu_properties_panel::common_properties::row_visible;
use crate::menu::menu_properties_panel::{FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

impl MenuEditor {
    pub(crate) fn draw_panel(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        element: &MenuElement,
        element_rect: Rect,
        is_selected: bool,
    ) {
        Panel::new(element_rect)
            .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
            .show(frame.ctx);
        if !frame.preview {
            let outline_color = if is_selected {
                with_theme(|theme| theme.highlight)
            } else {
                Color::new(0., 0., 0., 0.)
            };
            let thickness = if is_selected { 2.0 } else { 1.0 };
            frame.ctx.draw_rectangle_lines(
                element_rect.x,
                element_rect.y,
                element_rect.w,
                element_rect.h,
                thickness,
                outline_color,
            );
        }
    }

    pub(crate) fn draw_panel_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (current_class, current_style_id) = {
            let Some(element) = self.selected_element() else {
                return;
            };
            (element.class.clone(), element.style_id.clone())
        };

        // Class field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Class:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let class_str = current_class.as_deref().unwrap_or("");
            let (new_class_val, commit) = TextInput::new(
                self.properties_panel.widget_ids.class_id,
                field_rect,
                class_str,
            )
            .blocked(blocked)
            .show(ctx);
            let new_class = if new_class_val.is_empty() {
                None
            } else {
                Some(new_class_val)
            };
            self.push_input_update(commit, |el| el.class = new_class);
        }
        *y += ROW_HEIGHT;

        // Style ID field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Style ID:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let id_str = current_style_id.as_deref().unwrap_or("");
            let (new_id_val, commit) = TextInput::new(
                self.properties_panel.widget_ids.style_id,
                field_rect,
                id_str,
            )
            .blocked(blocked)
            .show(ctx);
            let new_id = if new_id_val.is_empty() {
                None
            } else {
                Some(new_id_val)
            };
            self.push_input_update(commit, |el| el.style_id = new_id);
        }
        *y += ROW_HEIGHT;
    }
}
