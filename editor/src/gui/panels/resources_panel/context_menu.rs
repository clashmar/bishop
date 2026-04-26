use crate::commands::asset::{DeleteAssetCmd, DeleteDirectoryCmd, DeleteUnregisteredFileCmd};
use crate::editor_global::{push_command, with_editor};
use crate::gui::modals::{
    new_resource_folder::{NewResourceFolderModal, NEW_FOLDER_TARGET},
    rename_resource::{ResourceRenameModal, RENAME_RESOURCE_TARGET},
    rename_resource_folder::{ResourceFolderRenameModal, RENAME_FOLDER_TARGET},
    ModalHandler,
};
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::path::PathBuf;
use widgets::{ContextMenu, ContextMenuItem, WidgetId};

use super::Entry;

/// Classification of a resource entry for context-menu dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntryKind {
    Parent,
    Directory,
    RegisteredFile,
    UnregisteredFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResourceMenuAction {
    NewFolder,
    Rename,
    Delete,
    Open,
    Reveal,
}

impl ResourceMenuAction {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::NewFolder => "New Folder",
            Self::Rename => "Rename",
            Self::Delete => "Delete",
            Self::Open => "Open",
            Self::Reveal => "Reveal",
        }
    }
}

const DIRECTORY_MENU_ACTIONS: &[ResourceMenuAction] = &[
    ResourceMenuAction::Rename,
    ResourceMenuAction::Delete,
    ResourceMenuAction::Reveal,
];
const REGISTERED_FILE_MENU_ACTIONS: &[ResourceMenuAction] = &[
    ResourceMenuAction::Rename,
    ResourceMenuAction::Delete,
    ResourceMenuAction::Open,
    ResourceMenuAction::Reveal,
];
const UNREGISTERED_FILE_MENU_ACTIONS: &[ResourceMenuAction] = &[
    ResourceMenuAction::Delete,
    ResourceMenuAction::Open,
    ResourceMenuAction::Reveal,
];
const PARENT_MENU_ACTIONS: &[ResourceMenuAction] = &[];
pub(super) const BACKGROUND_MENU_ACTIONS: &[ResourceMenuAction] = &[ResourceMenuAction::NewFolder];

pub(super) fn context_menu_actions_for(kind: EntryKind) -> &'static [ResourceMenuAction] {
    match kind {
        EntryKind::Parent => PARENT_MENU_ACTIONS,
        EntryKind::Directory => DIRECTORY_MENU_ACTIONS,
        EntryKind::RegisteredFile => REGISTERED_FILE_MENU_ACTIONS,
        EntryKind::UnregisteredFile => UNREGISTERED_FILE_MENU_ACTIONS,
    }
}

#[derive(Clone)]
pub(super) struct ContextTarget {
    pub(super) entry_index: usize,
    pub(super) position: Vec2,
    pub(super) actions: Vec<ResourceMenuAction>,
}

#[derive(Clone)]
pub(super) enum ActiveMenu {
    Entry(ContextTarget),
    Background(Vec2),
}

pub(super) fn context_target_for_background(position: Vec2) -> ActiveMenu {
    ActiveMenu::Background(position)
}

pub(super) fn pending_action_for_background(
    current_dir: &std::path::Path,
) -> PendingResourceAction {
    PendingResourceAction::CreateDirectory(current_dir.to_path_buf())
}

pub(super) enum PendingResourceAction {
    RenameFile(AssetKey),
    RenameDirectory(PathBuf),
    DeleteRegisteredFile(AssetKey),
    DeleteUnregisteredFile(PathBuf),
    DeleteDirectory(PathBuf),
    CreateDirectory(PathBuf),
    Open(PathBuf),
    Reveal(PathBuf),
}

pub(super) fn context_target_for_entry(
    entry_index: usize,
    entry: &Entry,
    position: Vec2,
) -> Option<ContextTarget> {
    let actions = context_menu_actions_for(entry.kind).to_vec();
    (!actions.is_empty()).then_some(ContextTarget {
        entry_index,
        position,
        actions,
    })
}

pub(super) fn pending_action_for(
    entry: &Entry,
    action: ResourceMenuAction,
) -> Option<PendingResourceAction> {
    match (action, entry.kind) {
        (ResourceMenuAction::Rename, EntryKind::Directory) => {
            Some(PendingResourceAction::RenameDirectory(entry.path.clone()))
        }
        (ResourceMenuAction::Rename, EntryKind::RegisteredFile) => Some(
            PendingResourceAction::RenameFile(asset_key_for_entry(entry)?),
        ),
        (ResourceMenuAction::Delete, EntryKind::Directory) => {
            Some(PendingResourceAction::DeleteDirectory(entry.path.clone()))
        }
        (ResourceMenuAction::Delete, EntryKind::RegisteredFile) => Some(
            PendingResourceAction::DeleteRegisteredFile(asset_key_for_entry(entry)?),
        ),
        (ResourceMenuAction::Delete, EntryKind::UnregisteredFile) => Some(
            PendingResourceAction::DeleteUnregisteredFile(entry.path.clone()),
        ),
        (ResourceMenuAction::Open, _) => Some(PendingResourceAction::Open(entry.path.clone())),
        (ResourceMenuAction::Reveal, _) => Some(PendingResourceAction::Reveal(entry.path.clone())),
        _ => None,
    }
}

pub(super) fn handle_pending_action(
    pending: Option<PendingResourceAction>,
    editor: &mut Editor,
    ctx: &mut WgpuContext,
) -> Option<PendingResourceAction> {
    match pending {
        Some(PendingResourceAction::DeleteRegisteredFile(key)) => {
            push_command(Box::new(DeleteAssetCmd::new(key)));
            None
        }
        Some(PendingResourceAction::DeleteUnregisteredFile(path)) => {
            push_command(Box::new(DeleteUnregisteredFileCmd::new(path)));
            None
        }
        Some(PendingResourceAction::DeleteDirectory(path)) => {
            push_command(Box::new(DeleteDirectoryCmd::new(path)));
            None
        }
        Some(PendingResourceAction::Open(path)) => {
            open_resource(&path, editor);
            None
        }
        Some(PendingResourceAction::Reveal(path)) => {
            reveal_in_system_browser(&path, editor);
            None
        }
        Some(PendingResourceAction::RenameFile(key)) => {
            let old_relative = editor
                .game
                .asset_registry
                .relative_path(key)
                .unwrap_or_default();
            RENAME_RESOURCE_TARGET.with(|t| *t.borrow_mut() = Some((key, old_relative)));
            ResourceRenameModal.open(editor, ctx);
            None
        }
        Some(PendingResourceAction::RenameDirectory(path)) => {
            RENAME_FOLDER_TARGET.with(|t| *t.borrow_mut() = Some(path));
            ResourceFolderRenameModal.open(editor, ctx);
            None
        }
        Some(PendingResourceAction::CreateDirectory(path)) => {
            NEW_FOLDER_TARGET.with(|t| *t.borrow_mut() = Some(path));
            NewResourceFolderModal.open(editor, ctx);
            None
        }
        other => other,
    }
}

fn asset_key_for_entry(entry: &Entry) -> Option<AssetKey> {
    with_editor(|editor| editor.game.asset_registry.key_for_full_path(&entry.path))
}

pub(crate) fn open_resource(path: &std::path::Path, editor: &mut Editor) {
    if path
        .extension()
        .is_some_and(|ext| ext == extensions::PREFAB)
    {
        if let Some(AssetKey::Prefab(prefab_id)) =
            editor.game.asset_registry.key_for_full_path(path)
        {
            let already_open = editor
                .prefab_editor
                .as_ref()
                .is_some_and(|pe| pe.prefab_id == prefab_id);
            if !already_open {
                editor.open_prefab_editor_for_id(prefab_id);
            }
            return;
        }
        editor.toast = Some(Toast::new("Unregistered prefab file", 3.0));
        return;
    }

    open_file_with_default(path, editor);
}

fn open_file_with_default(path: &std::path::Path, editor: &mut Editor) {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(path).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer").arg(path).status()
    } else {
        std::process::Command::new("xdg-open").arg(path).status()
    };

    if result.is_err() {
        editor.toast = Some(Toast::new("Could not open resource.", 3.0));
    }
}

fn reveal_in_system_browser(path: &std::path::Path, editor: &mut Editor) {
    let is_dir = path.is_dir();

    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
    } else if cfg!(target_os = "windows") {
        if is_dir {
            std::process::Command::new("explorer").arg(path).status()
        } else {
            std::process::Command::new("explorer")
                .arg(format!("/select,\"{}\"", path.display()))
                .status()
        }
    } else if is_dir {
        std::process::Command::new("xdg-open").arg(path).status()
    } else {
        let parent = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open").arg(parent).status()
    };

    if result.is_err() {
        editor.toast = Some(Toast::new(
            "Could not reveal resource in system browser.",
            3.0,
        ));
    }
}

pub(super) fn draw_context_menu(
    context_menu_id: WidgetId,
    active_menu: &ActiveMenu,
    ctx: &mut WgpuContext,
    blocked: bool,
) -> Option<ResourceMenuAction> {
    let (position, actions): (Vec2, &[ResourceMenuAction]) = match active_menu {
        ActiveMenu::Entry(target) => (target.position, &target.actions),
        ActiveMenu::Background(pos) => (*pos, BACKGROUND_MENU_ACTIONS),
    };

    let items: Vec<ContextMenuItem<ResourceMenuAction>> = actions
        .iter()
        .copied()
        .map(|action| ContextMenuItem {
            label: action.label().to_string(),
            value: action,
        })
        .collect();

    ContextMenu::new(context_menu_id, position, &items)
        .blocked(blocked)
        .show(ctx)
}
