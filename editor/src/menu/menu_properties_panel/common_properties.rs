use super::{FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

struct CommonState {
    name: String,
    class: Option<String>,
    style_id: Option<String>,
    rect_val: Rect,
    z_order: i32,
    visible: bool,
    type_label: &'static str,
}

impl MenuEditor {
    pub(super) fn draw_common_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        _w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let state = {
            let Some(element) = self.selected_element() else {
                return;
            };

            let type_label = element.kind.kind_name();

            CommonState {
                name: element.name.clone(),
                class: element.class.clone(),
                style_id: element.style_id.clone(),
                rect_val: element.rect,
                z_order: element.z_order,
                visible: element.visible,
                type_label,
            }
        };

        let child_is_managed = self.is_selected_child_managed();

        // Type (read-only)
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Type:", x, *y + 16.0, 12.0, Color::WHITE);
            ctx.draw_text(
                state.type_label,
                x + LABEL_WIDTH,
                *y + 16.0,
                12.0,
                Color::new(0.7, 0.7, 0.7, 1.0),
            );
        }
        *y += ROW_HEIGHT;

        // Name field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Name:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, _w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_name, commit) = TextInput::new(
                self.properties_panel.widget_ids.name_id,
                field_rect,
                &state.name,
            )
            .blocked(blocked)
            .show(ctx);
            self.push_input_update(commit, |el| el.name = new_name);
        }
        *y += ROW_HEIGHT;

        // Visible checkbox
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Visible:", x, *y + 16.0, 12.0, Color::WHITE);
            let checkbox_rect = Rect::new(x + LABEL_WIDTH, *y + 4.0, 16.0, 16.0);
            let mut visible_val = state.visible;
            if Checkbox::new(checkbox_rect, &mut visible_val).show(ctx) {
                self.push_element_update(|el| el.visible = visible_val);
            }
        }
        *y += ROW_HEIGHT;

        // Class field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Class:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 120.0, FIELD_HEIGHT);
            let class_str = state.class.as_deref().unwrap_or("");
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
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 120.0, FIELD_HEIGHT);
            let id_str = state.style_id.as_deref().unwrap_or("");
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

        // Z Order (only for top-level elements, children inherit from parent)
        if self.selected_child_index.is_none() {
            if row_visible(*y, ROW_HEIGHT, clip) {
                ctx.draw_text("Z Order:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
                let (new_z_val, commit) = NumberInput::new(
                    self.properties_panel.widget_ids.z_order_id,
                    field_rect,
                    state.z_order as f32,
                )
                .blocked(blocked)
                .show(ctx);
                let new_z = new_z_val as i32;
                self.push_input_update(commit, |el| el.z_order = new_z);
            }
            *y += ROW_HEIGHT;
        }

        if !child_is_managed {
            if row_visible(*y, 20.0, clip) {
                ctx.draw_text("Position (normalized)", x, *y + 14.0, 12.0, Color::GREY);
            }
            *y += 20.0;

            if row_visible(*y, ROW_HEIGHT, clip) {
                // Position X
                ctx.draw_text("X:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + 24.0, *y, 60.0, FIELD_HEIGHT);
                let snap_x = self
                    .field_originals
                    .entry(self.properties_panel.widget_ids.pos_x_id)
                    .or_insert(state.rect_val.x);
                let (new_x, commit_x) = NumberInput::new(
                    self.properties_panel.widget_ids.pos_x_id,
                    field_rect,
                    *snap_x,
                )
                .blocked(blocked)
                .show(ctx);
                let px_x = format!("{}px", (new_x * ui::DESIGN_RESOLUTION_WIDTH) as i32);
                ctx.draw_text(&px_x, x + 88.0, *y + 16.0, 10.0, Color::GREY);

                // Position Y
                ctx.draw_text("Y:", x + 130.0, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + 154.0, *y, 60.0, FIELD_HEIGHT);
                let snap_y = self
                    .field_originals
                    .entry(self.properties_panel.widget_ids.pos_y_id)
                    .or_insert(state.rect_val.y);
                let (new_y, commit_y) = NumberInput::new(
                    self.properties_panel.widget_ids.pos_y_id,
                    field_rect,
                    *snap_y,
                )
                .blocked(blocked)
                .show(ctx);

                // Shared element snapshot for undo (full element, first preview only)
                if self.drag_original_element.is_none()
                    && (matches!(commit_x, InputCommit::Previewing | InputCommit::Committed)
                        || matches!(commit_y, InputCommit::Previewing | InputCommit::Committed))
                {
                    let ti = self.current_template_index;
                    let ei = self.primary_selected_index();
                    if let (Some(ti), Some(ei)) = (ti, ei) {
                        if let Some(element) = self
                            .templates
                            .get(ti)
                            .and_then(|t| t.elements.get(ei).cloned())
                        {
                            self.drag_original_element = Some(element);
                            self.drag_original_indices = Some((ti, ei));
                        }
                    }
                }

                if matches!(commit_x, InputCommit::Previewing)
                    || matches!(commit_y, InputCommit::Previewing)
                {
                    self.input_active_this_frame = true;
                }

                // Apply values (mirrors inspector per-field pattern)
                match commit_x {
                    InputCommit::Previewing => {
                        if let Some(element) = element_mut(self) {
                            element.rect.x = new_x;
                        }
                    }
                    InputCommit::Committed => {
                        if let Some(element) = element_mut(self) {
                            element.rect.x = new_x;
                        }
                        self.field_originals
                            .remove(&self.properties_panel.widget_ids.pos_x_id);
                        self.commit_element_update();
                    }
                    InputCommit::Unchanged => {
                        if let Some(original) = self
                            .field_originals
                            .remove(&self.properties_panel.widget_ids.pos_x_id)
                        {
                            if let Some(element) = element_mut(self) {
                                element.rect.x = original;
                            }
                        }
                    }
                }
                match commit_y {
                    InputCommit::Previewing => {
                        if let Some(element) = element_mut(self) {
                            element.rect.y = new_y;
                        }
                    }
                    InputCommit::Committed => {
                        if let Some(element) = element_mut(self) {
                            element.rect.y = new_y;
                        }
                        self.field_originals
                            .remove(&self.properties_panel.widget_ids.pos_y_id);
                        self.commit_element_update();
                    }
                    InputCommit::Unchanged => {
                        if let Some(original) = self
                            .field_originals
                            .remove(&self.properties_panel.widget_ids.pos_y_id)
                        {
                            if let Some(element) = element_mut(self) {
                                element.rect.y = original;
                            }
                        }
                    }
                }
            }
            *y += ROW_HEIGHT;

            if row_visible(*y, ROW_HEIGHT, clip) {
                // Size W
                ctx.draw_text("W:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + 24.0, *y, 60.0, FIELD_HEIGHT);
                let snap_w = *self
                    .field_originals
                    .entry(self.properties_panel.widget_ids.size_w_id)
                    .or_insert(state.rect_val.w);
                let (new_w, commit_w) = NumberInput::new(
                    self.properties_panel.widget_ids.size_w_id,
                    field_rect,
                    snap_w,
                )
                .blocked(blocked)
                .min(0.005)
                .show(ctx);
                let px_w = format!("{}px", (new_w * ui::DESIGN_RESOLUTION_WIDTH) as i32);
                ctx.draw_text(&px_w, x + 88.0, *y + 16.0, 10.0, Color::GREY);

                // Size H
                ctx.draw_text("H:", x + 130.0, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + 154.0, *y, 60.0, FIELD_HEIGHT);
                let snap_h = *self
                    .field_originals
                    .entry(self.properties_panel.widget_ids.size_h_id)
                    .or_insert(state.rect_val.h);
                let (new_h, commit_h) = NumberInput::new(
                    self.properties_panel.widget_ids.size_h_id,
                    field_rect,
                    snap_h,
                )
                .blocked(blocked)
                .min(0.005)
                .show(ctx);

                if self.drag_original_element.is_none()
                    && (matches!(commit_w, InputCommit::Previewing | InputCommit::Committed)
                        || matches!(commit_h, InputCommit::Previewing | InputCommit::Committed))
                {
                    let ti = self.current_template_index;
                    let ei = self.primary_selected_index();
                    if let (Some(ti), Some(ei)) = (ti, ei) {
                        if let Some(element) = self
                            .templates
                            .get(ti)
                            .and_then(|t| t.elements.get(ei).cloned())
                        {
                            self.drag_original_element = Some(element);
                            self.drag_original_indices = Some((ti, ei));
                        }
                    }
                }

                if matches!(commit_w, InputCommit::Previewing)
                    || matches!(commit_h, InputCommit::Previewing)
                {
                    self.input_active_this_frame = true;
                }

                match commit_w {
                    InputCommit::Previewing => {
                        if let Some(element) = element_mut(self) {
                            element.rect.w = new_w;
                        }
                    }
                    InputCommit::Committed => {
                        if let Some(element) = element_mut(self) {
                            element.rect.w = new_w;
                        }
                        self.field_originals
                            .remove(&self.properties_panel.widget_ids.size_w_id);
                        self.commit_element_update();
                    }
                    InputCommit::Unchanged => {
                        if let Some(original) = self
                            .field_originals
                            .remove(&self.properties_panel.widget_ids.size_w_id)
                        {
                            if let Some(element) = element_mut(self) {
                                element.rect.w = original;
                            }
                        }
                    }
                }
                match commit_h {
                    InputCommit::Previewing => {
                        if let Some(element) = element_mut(self) {
                            element.rect.h = new_h;
                        }
                    }
                    InputCommit::Committed => {
                        if let Some(element) = element_mut(self) {
                            element.rect.h = new_h;
                        }
                        self.field_originals
                            .remove(&self.properties_panel.widget_ids.size_h_id);
                        self.commit_element_update();
                    }
                    InputCommit::Unchanged => {
                        if let Some(original) = self
                            .field_originals
                            .remove(&self.properties_panel.widget_ids.size_h_id)
                        {
                            if let Some(element) = element_mut(self) {
                                element.rect.h = original;
                            }
                        }
                    }
                }
            }
            *y += ROW_HEIGHT + 8.0;
        } else {
            if row_visible(*y, 20.0, clip) {
                ctx.draw_text(
                    "Position/size managed by layout",
                    x,
                    *y + 14.0,
                    10.0,
                    Color::new(0.5, 0.5, 0.5, 1.0),
                );
            }
            *y += 20.0;
        }
    }
}

/// Returns true if a row is fully visible within the clip rect.
pub(crate) fn row_visible(y: f32, h: f32, clip: &Rect) -> bool {
    y >= clip.y && y + h <= clip.y + clip.h
}

fn element_mut(editor: &mut MenuEditor) -> Option<&mut MenuElement> {
    editor.selected_element_mut()
}
