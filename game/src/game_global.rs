// game/game_global.rs
use crate::input::input_snapshot::InputSnapshot;
use crate::input::{focus_priority, InputFocusMap, VirtualInputState};
use crate::playtest::control::{ActiveControlTimeline, RuntimeControlFrame};
use crate::scripting::commands::lua_command::LuaCommand;
use crate::scripting::commands::lua_command_manager::LuaCommandManager;
use engine_core::audio::{clear_audio_commands, reset_audio_runtime_state};
use engine_core::menu::{clear_menu_events, clear_slider_events};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::vec::IntoIter;

/// Global services for the `Engine`.
#[derive(Default)]
pub struct GameServices {
    pub command_manager: RefCell<LuaCommandManager>,
    pub input_snapshot: RefCell<InputSnapshot>,
    pub virtual_input: RefCell<VirtualInputState>,
    pub active_playtest_control_timeline: RefCell<Option<ActiveControlTimeline>>,
    pub clear_completed_playtest_control_next_tick: Cell<bool>,
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

/// Returns the current virtual-input overlay state.
pub fn get_virtual_input_state() -> VirtualInputState {
    GAME_SERVICES.with(|services| services.virtual_input.borrow().clone())
}

/// Replaces the current virtual-input overlay state.
pub fn set_virtual_input_state(virtual_input: VirtualInputState) {
    GAME_SERVICES.with(|services| {
        *services.virtual_input.borrow_mut() = virtual_input;
    });
}

/// Clears the current virtual-input overlay state.
pub fn clear_virtual_input_state() {
    set_virtual_input_state(VirtualInputState::default());
}

/// Installs the active playtest control timeline.
pub fn install_active_playtest_control_timeline(timeline: ActiveControlTimeline) {
    GAME_SERVICES.with(|services| {
        *services.active_playtest_control_timeline.borrow_mut() = Some(timeline);
        services
            .clear_completed_playtest_control_next_tick
            .set(false);
    });
}

/// Clears any active playtest control timeline.
pub fn clear_active_playtest_control_timeline() {
    GAME_SERVICES.with(|services| {
        *services.active_playtest_control_timeline.borrow_mut() = None;
        services
            .clear_completed_playtest_control_next_tick
            .set(false);
    });
}

/// Advances the active playtest control timeline and returns the resolved frame, if any.
pub fn advance_active_playtest_control_timeline() -> Option<RuntimeControlFrame> {
    GAME_SERVICES.with(|services| {
        if services.clear_completed_playtest_control_next_tick.get() {
            *services.virtual_input.borrow_mut() = VirtualInputState::default();
            services
                .clear_completed_playtest_control_next_tick
                .set(false);
            return None;
        }

        let mut active = services.active_playtest_control_timeline.borrow_mut();
        let timeline = active.as_mut()?;

        match timeline.tick() {
            Some(frame) => {
                *services.virtual_input.borrow_mut() = virtual_input_from_control_frame(&frame);
                if timeline.is_complete() {
                    *active = None;
                    services
                        .clear_completed_playtest_control_next_tick
                        .set(true);
                }
                Some(frame)
            }
            None => {
                *services.virtual_input.borrow_mut() = VirtualInputState::default();
                *active = None;
                services
                    .clear_completed_playtest_control_next_tick
                    .set(false);
                None
            }
        }
    })
}

/// Clears completed playtest-control frame state after the frame has consumed it.
pub fn finalize_completed_playtest_control_frame() {
    GAME_SERVICES.with(|services| {
        if !services.clear_completed_playtest_control_next_tick.get() {
            return;
        }

        *services.virtual_input.borrow_mut() = VirtualInputState::default();
        services
            .clear_completed_playtest_control_next_tick
            .set(false);
    });
}

/// Returns whether a playtest control timeline is currently active or pending cleanup.
pub fn has_active_playtest_control_timeline() -> bool {
    GAME_SERVICES.with(|services| {
        services
            .active_playtest_control_timeline
            .borrow()
            .as_ref()
            .is_some()
    })
}

/// Returns whether a completed playtest-control frame is pending post-snapshot cleanup.
pub fn has_pending_completed_playtest_control_frame() -> bool {
    GAME_SERVICES.with(|services| services.clear_completed_playtest_control_next_tick.get())
}

/// Advances the active playtest control timeline and discards any returned camera frame.
pub fn tick_active_playtest_control_timeline() {
    let _ = advance_active_playtest_control_timeline();
}

fn virtual_input_from_control_frame(frame: &RuntimeControlFrame) -> VirtualInputState {
    let mut virtual_input = VirtualInputState::default();

    for &input in &frame.down_inputs {
        virtual_input.set_down(input, true);
    }

    for &input in &frame.pressed_inputs {
        virtual_input.set_down(input, true);
        virtual_input.set_pressed(input, true);
    }

    for &input in &frame.released_inputs {
        virtual_input.set_down(input, false);
        virtual_input.set_released(input, true);
    }

    virtual_input
}

/// Resets thread-local engine session state to fresh defaults for a new engine instance.
pub fn reset_engine_session_state() {
    GAME_SERVICES.with(|services| {
        *services.command_manager.borrow_mut() = LuaCommandManager::default();
        *services.input_snapshot.borrow_mut() = InputSnapshot::default();
        *services.virtual_input.borrow_mut() = VirtualInputState::default();
        *services.active_playtest_control_timeline.borrow_mut() = None;
        services
            .clear_completed_playtest_control_next_tick
            .set(false);
        services.menu_active.set(false);
        *services.input_focus.borrow_mut() = InputFocusMap::default();
    });
    clear_menu_events();
    clear_slider_events();
    clear_audio_commands();
    reset_audio_runtime_state();
}

/// Clears one-frame virtual-input edges while preserving held down overrides.
pub fn clear_virtual_input_edges() {
    GAME_SERVICES.with(|services| {
        services.virtual_input.borrow_mut().clear_edges();
    });
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
