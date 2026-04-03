use crate::assets::asset_manager::AssetManager;
use crate::assets::sprite::SpriteId;
use crate::ecs::entity::Entity;
use crate::prelude::{assets_folder, scripts_folder};
use crate::scripting::script::ScriptId;
use crate::scripting::script_manager::ScriptManager;
use crate::*;
use bishop::prelude::*;
use std::borrow::Cow;
use widgets::{Button, WIDGET_SPACING};

pub fn gui_sprite_picker<C: BishopContext>(
    ctx: &mut C,
    rect: Rect,
    id: &mut SpriteId,
    asset_manager: &mut AssetManager,
    blocked: bool,
) -> bool {
    let btn_label: Cow<str> = if id.0 == 0 {
        Cow::Borrowed("[Pick File]")
    } else {
        let filename = asset_manager
            .sprite_id_to_path
            .get(id)
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "???".to_string());

        Cow::Owned(format!("[/{}]", filename))
    };

    let remove_w = rect.h;
    let picker_w = rect.w - remove_w - WIDGET_SPACING;

    let picker_rect = Rect::new(rect.x, rect.y, picker_w, rect.h);
    let remove_rect = Rect::new(rect.x + rect.w - remove_w, rect.y, remove_w, rect.h);

    let mut changed = false;

    if Button::new(picker_rect, &btn_label)
        .blocked(blocked)
        .show(ctx)
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG images", &["png"])
                .set_directory(assets_folder())
                .pick_file()
            {
                let normalized = asset_manager.normalize_path(path);
                match asset_manager.get_or_load(ctx, &normalized) {
                    Some(new_id) => {
                        asset_manager.change_sprite(id, new_id);
                        changed = true;
                    }
                    None => {
                        onscreen_error!("Failed to load sprite.");
                    }
                }
            }
        }
    }

    if Button::new(remove_rect, "x").blocked(blocked).show(ctx) && id.0 != 0 {
        asset_manager.decrement_ref(*id);
        *id = SpriteId(0);
        changed = true;
    }

    changed
}

pub fn gui_script_picker<C: BishopContext>(
    ctx: &mut C,
    rect: Rect,
    entity: Entity,
    script_id: &mut ScriptId,
    script_manager: &mut ScriptManager,
    blocked: bool,
) -> bool {
    let btn_label: Cow<str> = if script_id.0 == 0 {
        Cow::Borrowed("[Pick File]")
    } else {
        let filename = script_manager
            .script_id_to_path
            .get(script_id)
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "???".to_string());

        Cow::Owned(format!("[/{}]", filename))
    };

    let remove_w = rect.h;
    let picker_w = rect.w - remove_w - WIDGET_SPACING;

    let picker_rect = Rect::new(rect.x, rect.y, picker_w, rect.h);
    let remove_rect = Rect::new(rect.x + rect.w - remove_w, rect.y, remove_w, rect.h);

    let mut changed = false;

    if Button::new(picker_rect, &btn_label)
        .blocked(blocked)
        .show(ctx)
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Lua Scripts", &["lua"])
                .set_directory(scripts_folder())
                .pick_file()
            {
                let normalized = script_manager.normalize_path(path);
                match script_manager.get_or_load(&normalized) {
                    Some(new_id) => {
                        script_manager.change_script(entity, script_id, new_id);
                        changed = true;
                    }
                    None => {
                        onscreen_error!("Failed to load script.");
                    }
                }
            }
        }
    }

    if Button::new(remove_rect, "x").blocked(blocked).show(ctx) && script_id.0 != 0 {
        script_manager.unload(entity, *script_id);
        *script_id = ScriptId(0);
        changed = true;
    }

    changed
}
