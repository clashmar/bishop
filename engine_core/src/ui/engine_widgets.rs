use crate::assets::{sprite_manager::SpriteManager, AssetRegistry};
use crate::ecs::entity::Entity;
use crate::ecs::{ScriptId, SpriteId, TomlId};
use crate::prelude::{assets_folder, scripts_folder, text_folder};
use crate::scripting::script_manager::ScriptManager;
use crate::text::{TextManager, TextManifest};
use crate::*;
use bishop::prelude::*;
use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use widgets::{Button, WidgetId, WIDGET_SPACING};

pub fn gui_sprite_picker<C: BishopContext>(
    ctx: &mut C,
    rect: Rect,
    interaction_id: WidgetId,
    id: &mut SpriteId,
    asset_registry: &mut AssetRegistry,
    sprite_manager: &mut SpriteManager,
    blocked: bool,
) -> bool {
    let btn_label: Cow<str> = if id.0 == 0 {
        Cow::Borrowed("[Pick File]")
    } else {
        let filename = sprite_manager
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
        .interaction_id(interaction_id)
        .suppressed(blocked)
        .show_native_dialog(ctx)
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG images", &["png"])
                .set_directory(assets_folder())
                .pick_file()
            {
                let normalized = sprite_manager.normalize_path(path);
                match sprite_manager.get_or_load(asset_registry, ctx, &normalized) {
                    Some(new_id) => {
                        sprite_manager.change_sprite(id, new_id);
                        changed = true;
                    }
                    None => {
                        onscreen_error!("Failed to load sprite.");
                    }
                }
            }
        }
    }

    if Button::new(remove_rect, "x").suppressed(blocked).show(ctx) && id.0 != 0 {
        *id = SpriteId(0);
        changed = true;
    }

    changed
}

pub fn gui_script_picker<C: BishopContext>(
    ctx: &mut C,
    rect: Rect,
    interaction_id: WidgetId,
    selection: (Entity, &mut ScriptId),
    asset_registry: &mut AssetRegistry,
    script_manager: &mut ScriptManager,
    blocked: bool,
) -> bool {
    let (entity, script_id) = selection;
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
        .interaction_id(interaction_id)
        .suppressed(blocked)
        .show_native_dialog(ctx)
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Lua Scripts", &["lua"])
                .set_directory(scripts_folder())
                .pick_file()
            {
                let normalized = script_manager.normalize_path(path);
                match script_manager.get_or_load(asset_registry, &normalized) {
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

    if Button::new(remove_rect, "x").suppressed(blocked).show(ctx) && script_id.0 != 0 {
        script_manager.unload(entity, *script_id);
        *script_id = ScriptId(0);
        changed = true;
    }

    changed
}

pub fn gui_toml_picker<C: BishopContext>(
    ctx: &mut C,
    rect: Rect,
    interaction_id: WidgetId,
    id: &mut TomlId,
    asset_registry: &mut AssetRegistry,
    blocked: bool,
) -> bool {
    let btn_label: Cow<str> = if id.0 == 0 {
        Cow::Borrowed("[Pick File]")
    } else {
        Cow::Owned(toml_label(asset_registry, *id))
    };

    let remove_w = rect.h;
    let picker_w = rect.w - remove_w - WIDGET_SPACING;

    let picker_rect = Rect::new(rect.x, rect.y, picker_w, rect.h);
    let remove_rect = Rect::new(rect.x + rect.w - remove_w, rect.y, remove_w, rect.h);

    let mut changed = false;

    if Button::new(picker_rect, &btn_label)
        .interaction_id(interaction_id)
        .suppressed(blocked)
        .show_native_dialog(ctx)
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let text_root = text_folder();
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("TOML files", &["toml"])
                .set_directory(&text_root)
                .pick_file()
            {
                match picked_toml_relative_path(&text_root, &path).and_then(|relative_path| {
                    TextManager::new(text_root.clone())
                        .register_toml_path(asset_registry, &relative_path)
                }) {
                    Ok(new_id) => {
                        *id = new_id;
                        changed = true;
                    }
                    Err(error) => {
                        onscreen_error!("Failed to register TOML asset: {error}");
                    }
                }
            }
        }
    }

    if Button::new(remove_rect, "x").suppressed(blocked).show(ctx) && id.0 != 0 {
        *id = TomlId(0);
        changed = true;
    }

    changed
}

fn toml_label(asset_registry: &AssetRegistry, toml_id: TomlId) -> String {
    asset_registry
        .relative_path(toml_id)
        .map(|path| format!("[/{}]", path.display()))
        .unwrap_or_else(|| "[/???]".to_string())
}

fn picked_toml_relative_path(text_root: &Path, picked_path: &Path) -> Result<PathBuf, String> {
    let relative_path = picked_path
        .strip_prefix(text_root)
        .map_err(|_| format!("Selected file '{}' is outside text/", picked_path.display()))?;

    Ok(canonicalize_toml_relative_path(relative_path))
}

fn canonicalize_toml_relative_path(relative_path: &Path) -> PathBuf {
    let manifest_path = text_folder().join("_manifest.toml");
    let Some(content) = fs::read_to_string(manifest_path).ok() else {
        return relative_path.to_path_buf();
    };
    let Some(manifest) = toml::from_str::<TextManifest>(&content).ok() else {
        return relative_path.to_path_buf();
    };

    manifest
        .available
        .iter()
        .find_map(|language| relative_path.strip_prefix(language).ok())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| relative_path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_global::set_game_name;
    use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use std::fs;

    #[test]
    fn canonicalize_toml_relative_path_strips_manifest_language_prefix() {
        let _lock = game_fs_test_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let folder = TestGameFolder::new("toml_picker_language_prefix");
        set_game_name(folder.name());
        fs::create_dir_all(text_folder()).unwrap();
        fs::write(
            text_folder().join("_manifest.toml"),
            "default_language = \"en\"\navailable = [\"en\", \"es\"]\n",
        )
        .unwrap();

        assert_eq!(
            canonicalize_toml_relative_path(Path::new("en/dialogue/npcs/npc.toml")),
            PathBuf::from("dialogue/npcs/npc.toml")
        );
    }
}
