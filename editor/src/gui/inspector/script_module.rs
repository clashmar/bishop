// editor/src/gui/inspector/script_module.rs
use crate::editor_assets::assets::refresh_icon;
use crate::with_lua;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct ScriptModule {
    field_ids: HashMap<String, WidgetId>,
    fields_len: usize,
}

const SPACING: f32 = 5.0;
const FONT_SIZE: f32 = DEFAULT_FONT_SIZE_16;
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
        let ecs = &mut game_ctx.ecs;
        let script_manager = &mut game_ctx.script_manager;

        let script_comp = if let Some(comp) = ecs.get_mut::<Script>(entity) {
            comp
        } else {
            return;
        };

        // Ensure ScriptData is loaded if it exists
        if script_comp.script_id != ScriptId(0) {
            with_lua(|lua| {
                if let Err(e) = script_comp.load(lua, script_manager, entity) {
                    onscreen_error!("Failed to load script: {}", e);
                }
            });
        }

        // Layout
        let mut y = rect.y + WIDGET_SPACING;
        let full_w = rect.w - 2.0 * WIDGET_PADDING;

        // Picker
        let button_size = DEFAULT_FIELD_HEIGHT;

        let picker_rect = Rect::new(
            rect.x + WIDGET_PADDING,
            y,
            full_w - button_size - SPACING,
            DEFAULT_FIELD_HEIGHT,
        );

        let refresh_rect = Rect::new(
            picker_rect.x + picker_rect.w + SPACING,
            y,
            button_size,
            button_size,
        );

        // Script picker
        if gui_script_picker(
            ctx,
            picker_rect,
            entity,
            &mut script_comp.script_id,
            script_manager,
            blocked,
        ) {
            with_lua(|lua| {
                if let Err(e) = script_comp.load(lua, script_manager, entity) {
                    onscreen_error!("Failed to load script: {}", e);
                }
            })
        }

        // Refresh button
        if Button::icon(refresh_rect, refresh_icon(), "refresh_script")
            .icon_padding(5.0)
            .blocked(blocked)
            .show(ctx)
        {
            if script_comp.script_id == ScriptId(0) {
                return;
            }

            with_lua(|lua| {
                if let Err(e) = script_manager.reload(lua, entity, script_comp.script_id) {
                    onscreen_error!("Failed to reload script: {}", e);
                } else if let Err(e) = script_comp.load(lua, script_manager, entity) {
                    onscreen_error!("Failed to reload script data: {}", e);
                }
            });
        }

        if script_comp.data.fields.is_empty() {
            self.fields_len = 1;
            return;
        }

        y += picker_rect.h + SPACING * 2.0;

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
            ctx.draw_text(&label, rect.x, y + 22.0, FONT_SIZE, FIELD_TEXT_COLOR);

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
                DEFAULT_FIELD_HEIGHT,
            );

            // Pull the mutable reference to the field value
            let field = match script_comp.data.fields.get_mut(&name) {
                Some(f) => f,
                None => {
                    onscreen_error!("Could not read field data from script component.");
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
                        DEFAULT_CHECKBOX_DIMS,
                        DEFAULT_CHECKBOX_DIMS,
                    );
                    if gui_checkbox(ctx, cb_rect, v) && !blocked {
                        changed = true;
                    }
                }
                ScriptField::Int(ref mut v) => {
                    let new = NumberInput::new(base_id, widget_rect, *v as i32)
                        .blocked(blocked)
                        .show(ctx) as i64;
                    if new != *v {
                        *v = new;
                        changed = true;
                    }
                }
                ScriptField::Float(ref mut v) => {
                    let new = NumberInput::new(base_id, widget_rect, *v as f32)
                        .blocked(blocked)
                        .show(ctx) as f64;
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
                ScriptField::Vec2(ref mut v) => {
                    let id_x = *self.field_ids.entry(format!("{}.x", name)).or_default();

                    let id_y = *self.field_ids.entry(format!("{}.y", name)).or_default();

                    let half = widget_rect.w / 2.0;

                    // X
                    let rect_x = Rect::new(widget_rect.x, widget_rect.y, half - 2.0, widget_rect.h);
                    let new_x = NumberInput::new(id_x, rect_x, v[0])
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

                    let new_y = NumberInput::new(id_y, rect_y, v[0])
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
                    let new_x = NumberInput::new(id_x, rect_x, v[0])
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

                    let new_y = NumberInput::new(id_y, rect_y, v[0])
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

                    let new_z = NumberInput::new(id_z, rect_z, v[0])
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
                    if let Err(e) = script_comp.sync_to_lua(lua, script_manager, entity) {
                        onscreen_error!("Failed to sync script: {}", e);
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
        title: <engine_core::scripting::script::Script>::TYPE_NAME,
        factory: || {
            Box::new(
                CollapsibleModule::new(
                    crate::gui::inspector::script_module::ScriptModule::default()
                )
                .with_title(<engine_core::scripting::script::Script>::TYPE_NAME)
            )
        },
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
