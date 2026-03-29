// editor/src/editor_global.rs
use crate::commands::editor_command_manager::*;
use crate::gui::panels::panel_manager::PanelManager;
use crate::Editor;
use engine_core::prelude::*;
use mlua::Lua;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

/// Global services that can be reached from anywhere in the editor.
pub struct EditorServices {
    pub editor: RefCell<Option<Editor>>, // set once at startup
    pub lua: RefCell<Lua>,
    pub command_manager: RefCell<EditorCommandManager>,
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

/// Takes the pending toast, if any. Called by `Editor::draw_toast()` each frame.
pub fn take_pending_toast() -> Option<Toast> {
    PENDING_TOAST.with(|cell| cell.borrow_mut().take())
}

/// Reset the global editor services.
pub fn reset_services() {
    EDITOR_SERVICES.with(|services| {
        *services.command_manager.borrow_mut() = EditorCommandManager::new();
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
