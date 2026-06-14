use crate::gui::widgets::script_picker_row::draw_script_picker_row;
use crate::with_lua;
use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::game::GameCtxMut;
use engine_core::logging::omni_error;
use engine_core::ui::{measure_text, gui_toml_picker};
use ::widgets::*;
use std::collections::HashMap;
use ::widgets::constants::{colors, layout};

#[derive(Default)]
pub struct ScriptModule {
    field_ids: HashMap<String, WidgetId>,
    picker_id: WidgetId,
    fields_len: usize,
}

const SPACING: f32 = 5.0;
const FONT_SIZE: f32 = layout::DEFAULT_FONT_SIZE_16;
const MIN_LABEL_WIDTH: f32 = 80.0;
const MIN_WIDGET_WIDTH: f32 = 80.0;
const LABEL_PADDING: f32 = 10.0;

impl InspectorModule for ScriptModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(<Script>::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<Script>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        Ecs::remove_component::<Script>(game_ctx, entity);
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        if game_ctx.ecs.get::<Script>(entity).is_none() {
            return;
        }

        // Layout
        let mut y = rect.y + layout::WIDGET_SPACING;
        let full_w = rect.w - 2.0 * layout::WIDGET_PADDING;

        let picker_row_rect = Rect::new(
            rect.x + layout::WIDGET_PADDING,
            y,
            full_w,
            layout::DEFAULT_FIELD_HEIGHT,
        );

        draw_script_picker_row(
            ctx,
            picker_row_rect,
            self.picker_id,
            entity,
            game_ctx.ecs,
            game_ctx.asset_registry,
            game_ctx.script_manager,
            blocked,
        );

        let script_comp = match game_ctx.ecs.get_mut::<Script>(entity) {
            Some(comp) => comp,
            None => {
                self.fields_len = 1;
                return;
            }
        };

        if script_comp.data.fields.is_empty() {
            self.fields_len = 1;
            return;
        }

        y += picker_row_rect.h + SPACING * 2.0;

        let mut field_names: Vec<_> = script_comp.data.fields.keys().cloned().collect();

        field_names.sort();
        self.fields_len = field_names.len() + 1;

        for name in field_names {
            // Create the id for the widget
            let base_key = name.to_string();
            let base_id = *self.field_ids.entry(base_key.clone()).or_default();

            // Prepare the field label
            let display_name = parse_field_name(&name);
            let label = format!("{} :", display_name);
            let label_w = measure_text(ctx, &label, FONT_SIZE)
                .width
                .max(MIN_LABEL_WIDTH);
            let widget_x = rect.x + label_w + LABEL_PADDING;
            ctx.draw_text(
                &label,
                rect.x,
                y + 22.0,
                FONT_SIZE,
                colors::DEFAULT_TEXT_COLOR,
            );

            // Widget rectangle
            let widget_x = if widget_x > rect.x + rect.w - MIN_WIDGET_WIDTH {
                // Clamp the widget size to the min length
                rect.x + rect.w - MIN_WIDGET_WIDTH
            } else {
                widget_x
            };

            let widget_w = (rect.x + rect.w) - widget_x - 10.0;
            let widget_rect = Rect::new(
                widget_x,
                y,
                widget_w.max(MIN_WIDGET_WIDTH),
                layout::DEFAULT_FIELD_HEIGHT,
            );

            // Pull the mutable reference to the field value
            let field = match script_comp.data.fields.get_mut(&name) {
                Some(f) => f,
                None => {
                    omni_error!("Could not read field data from script component.");
                    return;
                }
            };

            // Track if any values changed to write back
            let mut changed = false;

            match field {
                ScriptField::Bool(ref mut v) => {
                    let cb_rect = Rect::new(
                        widget_rect.x,
                        widget_rect.y + 6.0,
                        layout::DEFAULT_CHECKBOX_DIMS,
                        layout::DEFAULT_CHECKBOX_DIMS,
                    );
                    if Checkbox::new(cb_rect, v).blocked(blocked).show(ctx) {
                        changed = true;
                    }
                }
                ScriptField::Int(ref mut v) => {
                    let (new, _) = NumberInput::new(base_id, widget_rect, *v as i32)
                        .blocked(blocked)
                        .show(ctx);
                    let new = new as i64;
                    if new != *v {
                        *v = new;
                        changed = true;
                    }
                }
                ScriptField::Float(ref mut v) => {
                    let (new, _) = NumberInput::new(base_id, widget_rect, *v as f32)
                        .blocked(blocked)
                        .show(ctx);
                    let new = new as f64;
                    if new != *v {
                        *v = new;
                        changed = true;
                    }
                }
                ScriptField::Text(ref mut s) => {
                    let (txt, _) = TextInput::new(base_id, widget_rect, s)
                        .blocked(blocked)
                        .show(ctx);
                    if txt != *s {
                        *s = txt;
                        changed = true;
                    }
                }
                ScriptField::Toml(ref mut toml_id) => {
                    if gui_toml_picker(ctx, widget_rect, base_id, toml_id, game_ctx.asset_registry, blocked)
                    {
                        changed = true;
                    }
                }
                ScriptField::Vec2(ref mut v) => {
                    let id_x = *self.field_ids.entry(format!("{}.x", name)).or_default();

                    let id_y = *self.field_ids.entry(format!("{}.y", name)).or_default();

                    let half = widget_rect.w / 2.0;

                    // X
                    let rect_x = Rect::new(widget_rect.x, widget_rect.y, half - 2.0, widget_rect.h);
                    let (new_x, _) = NumberInput::new(id_x, rect_x, v[0])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_x - v[0]).abs() > f32::EPSILON {
                        v[0] = new_x;
                        changed = true;
                    }

                    // Y
                    let rect_y = Rect::new(
                        widget_rect.x + half + 2.0,
                        widget_rect.y,
                        half - 2.0,
                        widget_rect.h,
                    );

                    let (new_y, _) = NumberInput::new(id_y, rect_y, v[0])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_y - v[0]).abs() > f32::EPSILON {
                        v[0] = new_y;
                        changed = true;
                    };
                }
                ScriptField::Vec3(ref mut v) => {
                    let id_x = *self.field_ids.entry(format!("{}.x", name)).or_default();

                    let id_y = *self.field_ids.entry(format!("{}.y", name)).or_default();

                    let id_z = *self.field_ids.entry(format!("{}.z", name)).or_default();

                    let third = widget_rect.w / 3.0 - SPACING / 3.0;

                    // X
                    let rect_x =
                        Rect::new(widget_rect.x, widget_rect.y, third - 2.0, widget_rect.h);
                    let (new_x, _) = NumberInput::new(id_x, rect_x, v[0])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_x - v[0]).abs() > f32::EPSILON {
                        v[0] = new_x;
                        changed = true;
                    }

                    // Y
                    let rect_y = Rect::new(
                        widget_rect.x + third + 2.0,
                        widget_rect.y,
                        third - 2.0,
                        widget_rect.h,
                    );

                    let (new_y, _) = NumberInput::new(id_y, rect_y, v[0])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_y - v[0]).abs() > f32::EPSILON {
                        v[0] = new_y;
                        changed = true;
                    };

                    // Z
                    let rect_z = Rect::new(
                        widget_rect.x + 2.0 * third + 4.0,
                        widget_rect.y,
                        third - 2.0,
                        widget_rect.h,
                    );

                    let (new_z, _) = NumberInput::new(id_z, rect_z, v[0])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_z - v[0]).abs() > f32::EPSILON {
                        v[0] = new_z;
                        changed = true;
                    };
                }
            }

            // Write back to Lua
            if changed {
                with_lua(|lua| {
                    if let Err(e) = script_comp.sync_to_lua(lua, game_ctx.script_manager, entity) {
                        omni_error!("Failed to sync script: {}", e);
                    }
                })
            }

            y += widget_rect.h + SPACING;
        }
    }

    /// Compute the body layout from the number of fields.
    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(10.0)
            .rows(self.fields_len.max(1), SPACING)
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: <engine_core::ecs::Script>::TYPE_NAME,
        title: <engine_core::ecs::Script>::TYPE_NAME,
        factory: || {
            Box::new(
                CollapsibleComponentModule::new(
                    crate::gui::inspector::script_module::ScriptModule::default()
                )
                .with_title(<engine_core::ecs::Script>::TYPE_NAME)
            )
        },
        allowed_for: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_body_layout_keeps_larger_bottom_gutter() {
        let module = ScriptModule {
            fields_len: 1,
            ..Default::default()
        };

        assert_eq!(module.body_layout().height(), 50.0);
    }
}
