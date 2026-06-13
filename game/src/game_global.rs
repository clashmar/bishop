use crate::input::input_snapshot::InputSnapshot;
use crate::input::{focus_priority, InputFocusMap};
use crate::scripting::commands::lua_command::LuaCommand;
use crate::scripting::commands::lua_command_manager::LuaCommandManager;
use crate::transitions::world_transitions::WorldTransitionRequest;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::vec::IntoIter;

/// Global services for the `Engine`.
#[derive(Default)]
pub struct GameServices {
    pub command_manager: RefCell<LuaCommandManager>,
    pub input_snapshot: RefCell<InputSnapshot>,
    pub menu_active: Cell<bool>,
    pub input_focus: RefCell<InputFocusMap>,
    pub pending_world_transition: RefCell<Option<WorldTransitionRequest>>,
}

thread_local! {
    static GAME_SERVICES: Rc<GameServices> = Rc::new(GameServices::default());
}

/// Push an `LuaCommand` to the global command queue.
pub fn push_command(cmd: Box<dyn LuaCommand>) {
    GAME_SERVICES.with(|services| {
        services.command_manager.borrow_mut().push(cmd);
    });
}

/// Consumes the current contents of the global command queue and returns an iterator.
pub fn drain_commands() -> IntoIter<Box<dyn LuaCommand>> {
    GAME_SERVICES.with(|services| {
        return services.command_manager.borrow_mut().drain();
    })
}

/// Records a world transition to apply at end of frame. First request per frame wins.
pub fn set_pending_world_transition(request: WorldTransitionRequest) {
    GAME_SERVICES.with(|services| {
        let mut pending = services.pending_world_transition.borrow_mut();
        if pending.is_some() {
            engine_core::omni_warn!(
                "Ignoring world transition to '{}': another transition is already pending this frame",
                request.world_name
            );
            return;
        }
        *pending = Some(request);
    });
}

/// Takes the pending world transition, if any.
pub fn take_pending_world_transition() -> Option<WorldTransitionRequest> {
    GAME_SERVICES.with(|services| services.pending_world_transition.borrow_mut().take())
}

/// Returns a fresh copy of the current `InputSnapshot`.
pub fn get_input_snapshot() -> InputSnapshot {
    GAME_SERVICES.with(|services| services.input_snapshot.borrow().clone())
}

/// Sets whether a menu is currently active.
pub fn set_menu_active(active: bool) {
    GAME_SERVICES.with(|services| {
        services.menu_active.set(active);
        let mut focus = services.input_focus.borrow_mut();
        if active {
            focus.take_control("menu", focus_priority::MENU);
        } else {
            focus.release_control("menu");
        }
    });
}

/// Registers `name` with `priority` in the input focus map.
pub fn take_input_control(name: &str, priority: u8) {
    GAME_SERVICES.with(|services| {
        services
            .input_focus
            .borrow_mut()
            .take_control(name, priority);
    });
}

/// Removes `name` from the input focus map.
pub fn release_input_control(name: &str) {
    GAME_SERVICES.with(|services| {
        services.input_focus.borrow_mut().release_control(name);
    });
}

/// Returns `true` if `name` currently holds the highest priority.
pub fn in_input_control(name: &str) -> bool {
    GAME_SERVICES.with(|services| services.input_focus.borrow().in_control(name))
}

/// Returns true if any menu is currently active.
pub fn is_menu_active() -> bool {
    GAME_SERVICES.with(|services| services.menu_active.get())
}
