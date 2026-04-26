use crate::commands::asset::{DeleteAssetCmd, DeleteDirectoryCmd, DeleteUnregisteredFileCmd};
use crate::editor_global::{push_command, push_toast, with_editor};
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
    Reveal,
}

impl ResourceMenuAction {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::NewFolder => "New Folder",
            Self::Rename => "Rename",
            Self::Delete => "Delete",
            Self::Reveal => "Reveal",
        }
    }
}

const DIRECTORY_MENU_ACTIONS: &[ResourceMenuAction] = &[
    ResourceMenuAction::NewFolder,
    ResourceMenuAction::Rename,
    ResourceMenuAction::Delete,
];
const REGISTERED_FILE_MENU_ACTIONS: &[ResourceMenuAction] = &[
    ResourceMenuAction::Rename,
    ResourceMenuAction::Delete,
    ResourceMenuAction::Reveal,
];
const UNREGISTERED_FILE_MENU_ACTIONS: &[ResourceMenuAction] =
    &[ResourceMenuAction::Delete, ResourceMenuAction::Reveal];
const PARENT_MENU_ACTIONS: &[ResourceMenuAction] = &[];

pub(super) fn context_menu_actions_for(kind: EntryKind) -> &'static [ResourceMenuAction] {
    match kind {
        EntryKind::Parent => PARENT_MENU_ACTIONS,
        EntryKind::Directory => DIRECTORY_MENU_ACTIONS,
        EntryKind::RegisteredFile => REGISTERED_FILE_MENU_ACTIONS,
        EntryKind::UnregisteredFile => UNREGISTERED_FILE_MENU_ACTIONS,
    }
}

pub(super) struct ContextTarget {
    pub(super) entry_index: usize,
    pub(super) position: Vec2,
    pub(super) actions: Vec<ResourceMenuAction>,
}

pub(super) enum PendingResourceAction {
    RenameFile(AssetKey),
    RenameDirectory(PathBuf),
    DeleteRegisteredFile(AssetKey),
    DeleteUnregisteredFile(PathBuf),
    DeleteDirectory(PathBuf),
    CreateDirectory(PathBuf),
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
        (ResourceMenuAction::NewFolder, EntryKind::Directory) => {
            Some(PendingResourceAction::CreateDirectory(entry.path.clone()))
        }
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
        Some(PendingResourceAction::Reveal(path)) => {
            reveal_in_system_browser(&path, editor);
            None
        }
        Some(PendingResourceAction::RenameFile(_key)) => {
            open_resource_rename_modal(editor, ctx);
            None
        }
        Some(PendingResourceAction::RenameDirectory(_path)) => {
            open_directory_rename_modal(editor, ctx);
            None
        }
        Some(PendingResourceAction::CreateDirectory(_path)) => {
            open_new_folder_modal(editor, ctx);
            None
        }
        other => other,
    }
}

fn asset_key_for_entry(entry: &Entry) -> Option<AssetKey> {
    with_editor(|editor| editor.game.asset_registry.key_for_full_path(&entry.path))
}

fn open_resource_rename_modal(_editor: &mut Editor, _ctx: &mut WgpuContext) {
    push_toast("Resource rename coming soon.", 3.0);
}

fn open_directory_rename_modal(_editor: &mut Editor, _ctx: &mut WgpuContext) {
    push_toast("Directory rename coming soon.", 3.0);
}

fn open_new_folder_modal(_editor: &mut Editor, _ctx: &mut WgpuContext) {
    push_toast("New folder coming soon.", 3.0);
}

fn reveal_in_system_browser(path: &std::path::Path, editor: &mut Editor) {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(path).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer").arg(path).status()
    } else {
        push_toast("Reveal is unsupported on this platform.", 3.0);
        return;
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
    context_target: &ContextTarget,
    ctx: &mut WgpuContext,
    blocked: bool,
) -> Option<ResourceMenuAction> {
    let items: Vec<ContextMenuItem<ResourceMenuAction>> = context_target
        .actions
        .iter()
        .copied()
        .map(|action| ContextMenuItem {
            label: action.label().to_string(),
            value: action,
        })
        .collect();

    ContextMenu::new(context_menu_id, context_target.position, &items)
        .blocked(blocked)
        .show(ctx)
}
