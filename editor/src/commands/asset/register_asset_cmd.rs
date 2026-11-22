use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::push_toast;
use crate::with_editor;
use crate::Editor;
use bishop::text::FontId;
use engine_core::assets::{AssetKey, AssetKind, AssetRegistry};
use engine_core::ecs::{SoundId, SpriteId};
use engine_core::prefab::PrefabAsset;
use engine_core::storage::path_utils::resources_folder_current;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct RegisterAssetCmd {
    kind: AssetKind,
    path: PathBuf,
    registered_key: Option<AssetKey>,
    newly_registered: bool,
}

impl RegisterAssetCmd {
    pub fn new(kind: AssetKind, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            path: path.into(),
            registered_key: None,
            newly_registered: false,
        }
    }

    fn register(&mut self) -> io::Result<()> {
        let (key, newly_registered) =
            with_editor(|editor| register_asset(editor, self.kind, &self.path))?;
        self.registered_key = Some(key);
        self.newly_registered = newly_registered;
        Ok(())
    }
}

impl EditorCommand for RegisterAssetCmd {
    fn execute(&mut self) {
        if let Err(error) = self.register() {
            push_toast(format!("Register failed: {error}"), 3.0);
        }
    }

    fn undo(&mut self) {
        if !self.newly_registered {
            return;
        }
        let Some(key) = self.registered_key.take() else {
            return;
        };
        with_editor(|editor| {
            editor.game.asset_registry.remove_record(key);
            if matches!(key, AssetKey::Prefab(_)) {
                editor.game.reload_prefab_manager();
            }
            editor.save();
        });
        self.newly_registered = false;
    }

    fn applies_in_mode(&self, _current_mode: EditorMode) -> bool {
        true
    }
}

pub(crate) fn register_asset(
    editor: &mut Editor,
    kind: AssetKind,
    full_path: &Path,
) -> io::Result<(AssetKey, bool)> {
    if !full_path.is_file() {
        return Err(io::Error::new(ErrorKind::NotFound, "resource file does not exist"));
    }
    let relative_path = full_path
        .strip_prefix(resources_folder_current())
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "resource must be inside Resources"))?;
    let kind_folder = AssetRegistry::asset_folder(kind);
    let kind_relative_path = relative_path
        .strip_prefix(&kind_folder)
        .map_err(|_| {
            io::Error::new(
                ErrorKind::InvalidInput,
                format!("{kind:?} must be inside '{}'", kind_folder.display()),
            )
        })?;

    if let Some(existing_key) = editor.game.asset_registry.key_for_path(relative_path) {
        if AssetRegistry::kind_for_key(existing_key) != kind {
            return Err(io::Error::new(
                ErrorKind::AlreadyExists,
                "resource path is already registered as another kind",
            ));
        }
        editor.save();
        return Ok((existing_key, false));
    }

    let key = match kind {
        AssetKind::Script => {
            let id = editor
                .game
                .script_manager
                .get_or_load(&mut editor.game.asset_registry, kind_relative_path)
                .ok_or_else(|| {
                    io::Error::new(ErrorKind::InvalidData, "script could not be registered")
                })?;
            AssetKey::Script(id)
        }
        AssetKind::Toml => {
            let id = editor
                .game
                .text_manager
                .register_toml_path(&mut editor.game.asset_registry, kind_relative_path)
                .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
            AssetKey::Toml(id)
        }
        AssetKind::Prefab => {
            let contents = fs::read_to_string(full_path)?;
            let prefab: PrefabAsset = ron::from_str(&contents)
                .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
            if prefab.id.0 == 0 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "prefab id must be greater than zero",
                ));
            }
            AssetKey::Prefab(prefab.id)
        }
        AssetKind::Sprite => AssetKey::Sprite(SpriteId(next_numeric_id(
            &editor.game.asset_registry,
            |id| AssetKey::Sprite(SpriteId(id)),
        ))),
        AssetKind::Sound => AssetKey::Sound(SoundId(next_numeric_id(
            &editor.game.asset_registry,
            |id| AssetKey::Sound(SoundId(id)),
        ))),
        AssetKind::Font => AssetKey::Font(FontId(next_numeric_id(
            &editor.game.asset_registry,
            |id| AssetKey::Font(FontId(id)),
        ))),
    };

    if let Some(record) = editor.game.asset_registry.record(key) {
        if record.path != relative_path {
            return Err(io::Error::new(
                ErrorKind::AlreadyExists,
                "asset identity is already registered to another path",
            ));
        }
    } else {
        editor
            .game
            .asset_registry
            .register_asset_relative_path(key, kind_relative_path)?;
    }
    if matches!(key, AssetKey::Prefab(_)) {
        editor.game.reload_prefab_manager();
    }
    editor.save();
    Ok((key, true))
}

fn next_numeric_id(registry: &AssetRegistry, key_for_id: impl Fn(usize) -> AssetKey) -> usize {
    let mut candidate = 1;
    while registry.record(key_for_id(candidate)).is_some() {
        candidate += 1;
    }
    candidate
}
