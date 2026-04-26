use crate::app::{Editor, EditorMode};
use crate::commands::game::*;
use crate::editor_global::*;
use crate::gui::modals::{open_modal_with_prompt, Modal, ModalHandler, ModalResult};
use crate::gui::prompts::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static RENAME_PROMPT_RESULT: RefCell<Option<StringPromptResult>> = const { RefCell::new(None) };
}

pub struct RenameModal;

crate::register_modal!(RenameModal);

impl ModalHandler for RenameModal {
    type Result = StringPromptResult;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>> {
        &RENAME_PROMPT_RESULT
    }

    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext) {
        if editor.is_blank_prefab_mode() {
            editor.toast = Some(Toast::new("Blank prefab sessions cannot be renamed.", 2.5));
            return;
        }

        let prompt_message = match editor.mode {
            EditorMode::Game => "Rename game: ",
            EditorMode::World(_) => "Rename world: ",
            EditorMode::Room(_) => "Rename room: ",
            EditorMode::Prefab(_) => "Rename prefab: ",
            EditorMode::Menu => "Rename menu: ",
        };

        editor.modal = Modal::new(ctx, 400.0, 180.0);
        let mut prompt = StringPrompt::new(editor.modal.rect, prompt_message)
            .with_initial_value(editor.active_entity_name())
            .select_all_on_open();
        open_modal_with_prompt(
            &mut editor.modal,
            move |ctx| prompt.draw(ctx),
            &RENAME_PROMPT_RESULT,
        );
    }

    fn handle(
        &mut self,
        editor: &mut Editor,
        _ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult> {
        match result {
            StringPromptResult::Confirmed(name) => match editor.mode {
                EditorMode::Game => {
                    if !editor.duplicate_game_exists(&name) {
                        push_command(Box::new(RenameGameCmd::new(name, editor.game.name.clone())));
                    }
                }
                EditorMode::World(_) => {
                    if let Some(world) = editor.game.current_world_mut() {
                        world.name = name;
                    }
                }
                EditorMode::Room(id) => {
                    if let Some(world) = editor.game.current_world_mut() {
                        if let Some(room) = world.get_room_mut(id) {
                            room.name = name;
                        }
                    }
                }
                EditorMode::Prefab(_) => {
                    if let Some(prefab_id) = editor.active_persisted_prefab_id() {
                        if !editor.duplicate_prefab_name_exists_excluding(&name, prefab_id) {
                            if let Some(prefab_editor) = editor.prefab_editor.as_mut() {
                                prefab_editor.set_name(name);
                            }
                        }
                    }
                }
                EditorMode::Menu => {}
            },
            StringPromptResult::Cancelled => {
                editor.modal.close();
                return None;
            }
        }
        editor.modal.close();
        None
    }
}
