// game/game_global.rs
use crate::input::input_snapshot::InputSnapshot;
use crate::input::{focus_priority, InputFocusMap};
use crate::scripting::commands::lua_command::LuaCommand;
use crate::scripting::commands::lua_command_manager::LuaCommandManager;
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
