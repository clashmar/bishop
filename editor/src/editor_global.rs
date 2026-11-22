use crate::commands::editor_command_manager::*;
use crate::gui::panels::panel_manager::PanelManager;
use crate::Editor;
use engine_core::ecs::*;
use engine_core::ui::{Toast};
use mlua::Lua;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

type DeferredEditorAction = Box<dyn FnOnce(&mut Editor)>;

/// Global services that can be reached from anywhere in the editor.
pub struct EditorServices {
    pub editor: RefCell<Option<Editor>>, // set once at startup
    pub lua: RefCell<Lua>,
    pub command_manager: RefCell<EditorCommandManager>,
    pending_editor_actions: RefCell<Vec<DeferredEditorAction>>,
    pub pending_undo: Cell<bool>,
    pub pending_redo: Cell<bool>,
    pub entity_clipboard: RefCell<Option<GroupSnapshot>>,
    pub panel_manager: RefCell<PanelManager>,
}

impl EditorServices {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            editor: RefCell::new(None),
            lua: RefCell::new(Lua::new()),
            command_manager: RefCell::new(EditorCommandManager::new()),
            pending_editor_actions: RefCell::new(Vec::new()),
            pending_undo: Cell::new(false),
            pending_redo: Cell::new(false),
            entity_clipboard: RefCell::new(None),
            panel_manager: RefCell::new(PanelManager::new()),
        })
    }
}

thread_local! {
    /// Single instance of services used by the whole program.
    pub static EDITOR_SERVICES: Rc<EditorServices> = EditorServices::new();
}

thread_local! {
    static PENDING_TOAST: RefCell<Option<Toast>> = const { RefCell::new(None) };
}

/// Queue a toast notification for display by the Editor's centralized toast system.
pub fn push_toast<S: Into<String>>(msg: S, duration: f32) {
    PENDING_TOAST.with(|cell| {
        *cell.borrow_mut() = Some(Toast::new(msg, duration));
    });
}

/// Queue a persistent, throb-animating toast. Stays active until replaced by another toast.
pub fn push_throbbing_toast<S: Into<String>>(msg: S) {
    PENDING_TOAST.with(|cell| {
        *cell.borrow_mut() = Some(Toast::new_throbbing(msg));
    });
}

/// Takes the pending toast, if any. Called by `Editor::draw_toast()` each frame.
pub fn take_pending_toast() -> Option<Toast> {
    PENDING_TOAST.with(|cell| cell.borrow_mut().take())
}

/// Reset the global editor services.
pub fn reset_services() {
    EDITOR_SERVICES.with(|services| {
        *services.command_manager.borrow_mut() = EditorCommandManager::new();
        services.pending_editor_actions.borrow_mut().clear();
        services.pending_undo.set(false);
        services.pending_redo.set(false);
        *services.entity_clipboard.borrow_mut() = None;
    });
}

/// Store the `Editor` in global services.
pub fn set_editor(editor: Editor) {
    EDITOR_SERVICES.with(|services| {
        *services.editor.borrow_mut() = Some(editor);
    });
}

/// Gets mutable access to the `Editor`.
pub fn with_editor<F, R>(f: F) -> R
where
    F: FnOnce(&mut Editor) -> R,
{
    EDITOR_SERVICES.with(|services| {
        let mut opt = services.editor.borrow_mut();
        let editor = opt.as_mut().expect("Editor not initialised");
        f(editor)
    })
}

/// Gets immutable access to the Lua VM.
pub fn with_lua<F, R>(f: F) -> R
where
    F: FnOnce(&Lua) -> R,
{
    EDITOR_SERVICES.with(|services| {
        let lua = services.lua.borrow();
        f(&lua)
    })
}

/// Queue a one-off editor action to run after the active editor borrow ends.
pub fn push_editor_action(action: impl FnOnce(&mut Editor) + 'static) {
    EDITOR_SERVICES.with(|services| {
        services
            .pending_editor_actions
            .borrow_mut()
            .push(Box::new(action));
    });
}

/// Runs queued one-off actions against the active editor.
pub fn apply_pending_editor_actions() {
    let actions = EDITOR_SERVICES.with(|services| {
        std::mem::take(&mut *services.pending_editor_actions.borrow_mut())
    });
    if actions.is_empty() {
        return;
    }

    with_editor(|editor| {
        for action in actions {
            action(editor);
        }
    });
}

/// Push an `EditorCommand` to the global command queue.
pub fn push_command(command: Box<dyn EditorCommand>) {
    EDITOR_SERVICES.with(|services| {
        services.command_manager.borrow_mut().push(command);
    });
}

/// Apply all `EditorCommand`'s in the global command queue.
pub fn apply_pending_commands() {
    EDITOR_SERVICES.with(|s| {
        // Execute normal commands
        s.command_manager.borrow_mut().apply_all();

        if s.pending_undo.replace(false) {
            s.command_manager.borrow_mut().undo();
        }

        if s.pending_redo.replace(false) {
            s.command_manager.borrow_mut().redo();
        }
    });
}

/// Requests an undo for the current frame.
pub fn request_undo() {
    EDITOR_SERVICES.with(|s| s.pending_undo.set(true));
}

/// Requests an redo for the current frame.
pub fn request_redo() {
    EDITOR_SERVICES.with(|s| s.pending_redo.set(true));
}

/// Gets mutable access to the `PanelManager`.
pub fn with_panel_manager<F, R>(f: F) -> R
where
    F: FnOnce(&mut PanelManager) -> R,
{
    EDITOR_SERVICES.with(|services| {
        let mut pm = services.panel_manager.borrow_mut();
        f(&mut pm)
    })
}

/// Gets immutable access to the `EditorCommandManager`.
pub fn with_command_manager<F, R>(f: F) -> R
where
    F: FnOnce(&EditorCommandManager) -> R,
{
    EDITOR_SERVICES.with(|services| {
        let cm = services.command_manager.borrow();
        f(&cm)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        apply_pending_editor_actions, push_editor_action, reset_services, set_editor,
        with_command_manager, with_editor,
    };
    use crate::app::Editor;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn deferred_editor_actions_execute_once_without_entering_undo_history() {
        reset_services();
        set_editor(Editor::default());

        let executions = Rc::new(Cell::new(0));
        let action_executions = Rc::clone(&executions);
        with_editor(|editor| {
            push_editor_action(move |editor| {
                action_executions.set(action_executions.get() + 1);
                editor.room_editor.view_preview = true;
            });
            assert!(!editor.room_editor.view_preview);
        });

        apply_pending_editor_actions();
        apply_pending_editor_actions();

        assert_eq!(executions.get(), 1);
        assert!(with_editor(|editor| editor.room_editor.view_preview));
        assert_eq!(with_command_manager(|manager| manager.undo_stack_len()), 0);
        reset_services();
    }
}
