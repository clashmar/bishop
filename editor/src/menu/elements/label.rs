use crate::menu::menu_canvas::drawing::MenuCanvasFrame;
use crate::menu::menu_properties_panel::common_properties::row_visible;
use crate::menu::menu_properties_panel::{FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

impl MenuEditor {
    pub(crate) fn draw_label(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        element: &MenuElement,
        element_rect: Rect,
        is_selected: bool,
    ) {
        let MenuElementKind::Label(label) = &element.kind else {
            return;
        };
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
        let label_align = match label.alignment {
            HorizontalAlign::Left => LabelAlign::Left,
            HorizontalAlign::Center => LabelAlign::Center,
            HorizontalAlign::Right => LabelAlign::Right,
        };
        Label::new(element_rect, &label.text_key)
            .font_size(label.font_size)
            .alignment(label_align)
            .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
            .show(frame.ctx);
    }

    pub(crate) fn draw_label_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (current_text_key, current_font_size, current_alignment) = {
            let Some(element) = self.selected_element() else {
                return;
            };
            let MenuElementKind::Label(label) = &element.kind else {
                return;
            };
            (label.text_key.clone(), label.font_size, label.alignment)
        };

        // Text key field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Text Key:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);

            let (new_text_key, _) = TextInput::new(
                self.properties_panel.widget_ids.text_id,
                field_rect,
                &current_text_key,
            )
            .blocked(blocked)
            .show(ctx);

            if new_text_key != current_text_key {
                self.push_element_update(|el| {
                    if let MenuElementKind::Label(label) = &mut el.kind {
                        label.text_key = new_text_key;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Font size
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Font Size:", x, *y + 16.0, 12.0, Color::WHITE);

            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);

            let new_font_size = NumberInput::new(
                self.properties_panel.widget_ids.font_size_id,
                field_rect,
                current_font_size,
            )
            .blocked(blocked)
            .min(8.0)
            .max(72.0)
            .show(ctx);

            if (new_font_size - current_font_size).abs() > 0.01 {
                self.push_element_update(|el| {
                    if let MenuElementKind::Label(label) = &mut el.kind {
                        label.font_size = new_font_size;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Horizontal alignment
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Align:", x, *y + 16.0, 12.0, Color::WHITE);
            let h_options = ["Left", "Center", "Right"];
            let current_h = match current_alignment {
                HorizontalAlign::Left => "Left",
                HorizontalAlign::Center => "Center",
                HorizontalAlign::Right => "Right",
            };
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.label_h_align_id,
                dropdown_rect,
                current_h,
                &h_options,
                |s| s.to_string(),
            )
            .suppressed(blocked)
            .fixed_width()
            .show(ctx)
            {
                let new_align = match selected {
                    "Left" => HorizontalAlign::Left,
                    "Center" => HorizontalAlign::Center,
                    "Right" => HorizontalAlign::Right,
                    _ => current_alignment,
                };
                self.push_element_update(|el| {
                    if let MenuElementKind::Label(label) = &mut el.kind {
                        label.alignment = new_align;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;
    }
}
