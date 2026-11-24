use crate::engine::game_instance::GameInstance;
use crate::game_global::{get_input_snapshot, set_input_snapshot};
use crate::input::input_snapshot::InputSnapshot;
use bishop::input::GamepadState;
use bishop::prelude::PlatformContext;
use engine_core::storage::InputSettings;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn capture_input_snapshot(
    ctx: &PlatformContext,
    game_instance: &Rc<RefCell<GameInstance>>,
    settings: &InputSettings,
) {
    let mut snapshot: InputSnapshot = get_input_snapshot();
    let instance = game_instance.borrow();
    let gamepad = current_gamepad_state(ctx);
    snapshot.capture_input_state(
        ctx,
        &instance.game.input_actions,
        settings,
        instance.game.id,
        &gamepad,
    );
    drop(instance);
    set_input_snapshot(snapshot);
}

#[cfg(all(
    feature = "wgpu",
    any(target_os = "macos", target_os = "windows", target_os = "linux")
))]
fn current_gamepad_state(ctx: &PlatformContext) -> GamepadState {
    ctx.borrow().gamepad_state().clone()
}

#[cfg(not(all(
    feature = "wgpu",
    any(target_os = "macos", target_os = "windows", target_os = "linux")
)))]
fn current_gamepad_state(_ctx: &PlatformContext) -> GamepadState {
    GamepadState::default()
}
