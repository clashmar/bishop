use crate::app::EditorMode;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::storage::editor_storage::PrefabPaletteState;
use engine_core::prelude::*;

#[derive(Debug)]
pub struct DeletePrefabCmd {
    prefab_id: PrefabId,
    mode: EditorMode,
    deleted_prefab: Option<PrefabAsset>,
    deleted_snapshots: Vec<GroupSnapshot>,
    previous_palette_state: Option<PrefabPaletteState>,
}

impl DeletePrefabCmd {
    pub fn new(prefab_id: PrefabId, mode: EditorMode) -> Self {
        Self {
            prefab_id,
            mode,
            deleted_prefab: None,
            deleted_snapshots: Vec::new(),
            previous_palette_state: None,
        }
    }
}

impl EditorCommand for DeletePrefabCmd {
    fn execute(&mut self) {
        with_editor(|editor| {
            let Some(prefab) = editor
                .game
                .prefab_library
                .prefabs
                .get(&self.prefab_id)
                .cloned()
            else {
                return;
            };

            self.deleted_prefab = Some(prefab.clone());
            self.previous_palette_state = Some(editor.room_editor.prefab_palette_state());
            self.deleted_snapshots =
                capture_linked_prefab_instance_snapshots(&mut editor.game.ecs, self.prefab_id);

            if let Err(error) = delete_prefab(&editor.game.name, self.prefab_id) {
                onscreen_error!("Could not delete prefab: {error}");
                return;
            }

            editor.game.prefab_library.prefabs.remove(&self.prefab_id);
            if let Err(error) = sync_prefabs_lua_file(&editor.game) {
                onscreen_error!("Could not write prefabs.lua: {error}");
                return;
            }

            remove_linked_prefab_instances(editor, &self.deleted_snapshots);
            if !editor.reconcile_prefab_palette_after_library_change() {
                return;
            }
            editor.enter_required_blank_prefab_mode();
        });
    }

    fn undo(&mut self) {
        let Some(prefab) = self.deleted_prefab.as_ref() else {
            return;
        };

        with_editor(|editor| {
            if let Err(error) = save_prefab(&editor.game.name, prefab) {
                onscreen_error!("Could not restore prefab: {error}");
                return;
            }

            editor
                .game
                .prefab_library
                .prefabs
                .insert(self.prefab_id, prefab.clone());
            if let Err(error) = sync_prefabs_lua_file(&editor.game) {
                onscreen_error!("Could not write prefabs.lua: {error}");
                return;
            }
            restore_linked_prefab_instances(editor, &self.deleted_snapshots);
            editor.open_prefab_editor_for_id(self.prefab_id);
            restore_prefab_palette(editor, self.previous_palette_state.as_ref());
        });
    }

    fn mode(&self) -> EditorMode {
        self.mode
    }

    fn applies_in_mode(&self, current_mode: EditorMode) -> bool {
        matches!(current_mode, EditorMode::Prefab(prefab_id) if prefab_id == self.prefab_id || prefab_id == crate::prefab::BLANK_PREFAB_ID)
    }
}

fn capture_linked_prefab_instance_snapshots(
    ecs: &mut Ecs,
    prefab_id: PrefabId,
) -> Vec<GroupSnapshot> {
    linked_prefab_instance_roots(ecs, prefab_id)
        .into_iter()
        .map(|root| capture_subtree(ecs, root))
        .collect()
}

fn restore_linked_prefab_instances(editor: &mut crate::Editor, snapshots: &[GroupSnapshot]) {
    for snapshot in snapshots {
        let Some(root_entity) = snapshot.first().map(|entity| entity.entity) else {
            continue;
        };

        if editor.game.ecs.has::<Transform>(root_entity) {
            let mut ctx = editor.game.ctx_mut();
            Ecs::remove_entity(&mut ctx, root_entity);
        }

        let mut ctx = editor.game.ctx_mut();
        restore_subtree(&mut ctx, snapshot);
    }
}

fn remove_linked_prefab_instances(editor: &mut crate::Editor, snapshots: &[GroupSnapshot]) {
    let removed_entities = snapshots
        .iter()
        .flat_map(|snapshot| snapshot.iter().map(|entity| entity.entity))
        .collect::<Vec<_>>();

    for snapshot in snapshots {
        let Some(root_entity) = snapshot.first().map(|entity| entity.entity) else {
            continue;
        };

        let mut ctx = editor.game.ctx_mut();
        Ecs::remove_entity(&mut ctx, root_entity);
    }

    editor
        .room_editor
        .selected_entities
        .retain(|entity| !removed_entities.contains(entity));
    if editor
        .room_editor
        .inspector
        .target
        .is_some_and(|entity| removed_entities.contains(&entity))
    {
        editor.room_editor.inspector.set_target(None);
    }
}

fn restore_prefab_palette(editor: &mut crate::Editor, state: Option<&PrefabPaletteState>) {
    let Some(state) = state.cloned() else {
        return;
    };

    editor
        .room_editor
        .load_prefab_palette_state(&editor.game.prefab_library, state);
    let _ = editor.save_prefab_palette_state();
}

fn linked_prefab_instance_roots(ecs: &Ecs, prefab_id: PrefabId) -> Vec<Entity> {
    ecs.get_store::<PrefabInstanceRoot>()
        .data
        .iter()
        .filter_map(|(&entity, root)| (root.prefab_id == prefab_id).then_some(entity))
        .collect()
}

fn sync_prefabs_lua_file(game: &Game) -> std::io::Result<()> {
    let prefab_names = crate::storage::editor_storage::collect_prefab_names(&game.prefab_library)?;
    crate::editor_assets::write_prefabs_lua(&scripts_folder(), &prefab_names)
}
