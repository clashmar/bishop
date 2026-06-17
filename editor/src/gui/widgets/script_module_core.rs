use crate::editor_assets::assets::refresh_icon;
use crate::with_lua;
use bishop::prelude::*;
use engine_core::assets::AssetRegistry;
use engine_core::ecs::ecs::Ecs;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::{parse_field_name, Entity, Script, ScriptField, ScriptId};
use engine_core::game::GameCtxMut;
use engine_core::logging::omni_error;
use engine_core::scripting::script_manager::ScriptManager;
use engine_core::ui::{gui_script_picker, gui_toml_picker, measure_text};
use std::collections::HashMap;
use ::widgets::constants::{colors, layout};
use ::widgets::*;

/// State and rendering for a script component's picker row and field editors.
#[derive(Default)]
pub(crate) struct ScriptModuleCore {
    picker_id: WidgetId,
    field_ids: HashMap<String, WidgetId>,
    fields_len: usize,
}

const SPACING: f32 = 5.0;
const FONT_SIZE: f32 = layout::DEFAULT_FONT_SIZE_16;
const MIN_LABEL_WIDTH: f32 = 80.0;
const MIN_WIDGET_WIDTH: f32 = 80.0;
const LABEL_PADDING: f32 = 10.0;

impl ScriptModuleCore {
    /// Creates a new empty core.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Component type name used by the framework's undo machinery.
    pub(crate) fn undo_component_type() -> Option<&'static str> {
        Some(<Script>::TYPE_NAME)
    }

    /// Layout for the picker row and all field editors, based on the cached field count.
    pub(crate) fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(10.0)
            .rows(self.fields_len.max(1), SPACING)
    }

    /// Renders the script picker row and all field editors for the given entity.
    pub(crate) fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        entity: Entity,
        game_ctx: &mut GameCtxMut,
        blocked: bool,
    ) {
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
            let base_key = name.to_string();
            let base_id = *self.field_ids.entry(base_key.clone()).or_default();

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

            let widget_x = if widget_x > rect.x + rect.w - MIN_WIDGET_WIDTH {
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

            let field = match script_comp.data.fields.get_mut(&name) {
                Some(f) => f,
                None => {
                    omni_error!("Could not read field data from script component.");
                    return;
                }
            };

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
                    if gui_toml_picker(
                        ctx,
                        widget_rect,
                        base_id,
                        toml_id,
                        game_ctx.asset_registry,
                        blocked,
                    ) {
                        changed = true;
                    }
                }
                ScriptField::Vec2(ref mut v) => {
                    let id_x = *self.field_ids.entry(format!("{}.x", name)).or_default();
                    let id_y = *self.field_ids.entry(format!("{}.y", name)).or_default();
                    let half = widget_rect.w / 2.0;

                    let rect_x =
                        Rect::new(widget_rect.x, widget_rect.y, half - 2.0, widget_rect.h);
                    let (new_x, _) = NumberInput::new(id_x, rect_x, v[0])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_x - v[0]).abs() > f32::EPSILON {
                        v[0] = new_x;
                        changed = true;
                    }

                    let rect_y = Rect::new(
                        widget_rect.x + half + 2.0,
                        widget_rect.y,
                        half - 2.0,
                        widget_rect.h,
                    );
                    let (new_y, _) = NumberInput::new(id_y, rect_y, v[1])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_y - v[1]).abs() > f32::EPSILON {
                        v[1] = new_y;
                        changed = true;
                    }
                }
                ScriptField::Vec3(ref mut v) => {
                    let id_x = *self.field_ids.entry(format!("{}.x", name)).or_default();
                    let id_y = *self.field_ids.entry(format!("{}.y", name)).or_default();
                    let id_z = *self.field_ids.entry(format!("{}.z", name)).or_default();
                    let third = widget_rect.w / 3.0 - SPACING / 3.0;

                    let rect_x =
                        Rect::new(widget_rect.x, widget_rect.y, third - 2.0, widget_rect.h);
                    let (new_x, _) = NumberInput::new(id_x, rect_x, v[0])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_x - v[0]).abs() > f32::EPSILON {
                        v[0] = new_x;
                        changed = true;
                    }

                    let rect_y = Rect::new(
                        widget_rect.x + third + 2.0,
                        widget_rect.y,
                        third - 2.0,
                        widget_rect.h,
                    );
                    let (new_y, _) = NumberInput::new(id_y, rect_y, v[1])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_y - v[1]).abs() > f32::EPSILON {
                        v[1] = new_y;
                        changed = true;
                    }

                    let rect_z = Rect::new(
                        widget_rect.x + 2.0 * third + 4.0,
                        widget_rect.y,
                        third - 2.0,
                        widget_rect.h,
                    );
                    let (new_z, _) = NumberInput::new(id_z, rect_z, v[2])
                        .blocked(blocked)
                        .show(ctx);
                    if (new_z - v[2]).abs() > f32::EPSILON {
                        v[2] = new_z;
                        changed = true;
                    }
                }
            }

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
}

fn draw_script_picker_row(
    ctx: &mut WgpuContext,
    rect: Rect,
    picker_id: WidgetId,
    entity: Entity,
    ecs: &mut Ecs,
    asset_registry: &mut AssetRegistry,
    script_manager: &mut ScriptManager,
    blocked: bool,
) -> bool {
    let full_w = rect.w;
    let button_size = layout::DEFAULT_FIELD_HEIGHT;

    let picker_rect = Rect::new(
        rect.x,
        rect.y,
        full_w - button_size - SPACING,
        layout::DEFAULT_FIELD_HEIGHT,
    );

    let refresh_rect = Rect::new(
        picker_rect.x + picker_rect.w + SPACING,
        rect.y,
        button_size,
        button_size,
    );

    let had_script = ecs.has::<Script>(entity);
    let mut script_id = ecs
        .get::<Script>(entity)
        .map(|s| s.script_id)
        .unwrap_or(ScriptId(0));

    if script_id != ScriptId(0) {
        if let Some(script_comp) = ecs.get_mut::<Script>(entity) {
            with_lua(|lua| {
                if let Err(e) = script_comp.load(lua, asset_registry, script_manager, entity) {
                    omni_error!("Failed to load script: {}", e);
                }
            });
        }
    }

    let picked = gui_script_picker(
        ctx,
        picker_rect,
        picker_id,
        (entity, &mut script_id),
        asset_registry,
        script_manager,
        blocked,
    );

    if Button::icon(refresh_rect, refresh_icon(), "refresh_script")
        .icon_padding(5.0)
        .suppressed(blocked)
        .show(ctx)
    {
        if script_id != ScriptId(0) {
            with_lua(|lua| {
                if let Err(e) = script_manager.reload(lua, entity, script_id) {
                    omni_error!("Failed to reload script: {}", e);
                } else if let Some(comp) = ecs.get_mut::<Script>(entity) {
                    if let Err(e) = comp.load(lua, asset_registry, script_manager, entity) {
                        omni_error!("Failed to reload script data: {}", e);
                    }
                }
            });
        }
    }

    if !picked {
        return false;
    }

    if script_id == ScriptId(0) {
        ecs.get_store_mut::<Script>().remove(entity);
    } else if let Some(comp) = ecs.get_mut::<Script>(entity) {
        comp.script_id = script_id;
        with_lua(|lua| {
            if let Err(e) = comp.load(lua, asset_registry, script_manager, entity) {
                omni_error!("Failed to load script: {}", e);
            }
        });
    } else if !had_script {
        ecs.add_component_to_entity(entity, Script { script_id, ..Default::default() });
        if let Some(comp) = ecs.get_mut::<Script>(entity) {
            with_lua(|lua| {
                if let Err(e) = comp.load(lua, asset_registry, script_manager, entity) {
                    omni_error!("Failed to load script: {}", e);
                }
            });
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_layout_with_one_field_matches_expected_height() {
        let core = ScriptModuleCore {
            fields_len: 1,
            ..Default::default()
        };
        assert_eq!(core.body_layout().height(), 50.0);
    }
}
