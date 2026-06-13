use crate::editor_assets::assets::refresh_icon;
use crate::with_lua;
use bishop::prelude::*;
use engine_core::assets::AssetRegistry;
use engine_core::ecs::{Entity, Script, ScriptId};
use engine_core::ecs::ecs::Ecs;
use engine_core::logging::omni_error;
use engine_core::scripting::script_manager::ScriptManager;
use engine_core::ui::gui_script_picker;
use widgets::*;
use ::widgets::constants::layout;

const SPACING: f32 = 5.0;

/// Draws a script picker dropdown and refresh button for the given entity, returning `true` if the assignment changed.
pub fn draw_script_picker_row(
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

    // Ensure ScriptData is loaded
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

    // Refresh button
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

    // Apply the change
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
