use std::cell::RefCell;

/// Trait for handling custom menu actions.
///
/// Engine-level actions (Resume, OpenMenu, CloseMenu, QuitGame) are handled
/// automatically by MenuManager. Custom actions are forwarded to implementations
/// of this trait, allowing game-specific logic to respond to menu events.
pub trait MenuActionHandler {
    /// Handles a custom menu action.
    ///
    /// Returns true if the action was handled, false otherwise.
    fn handle_action(&mut self, action: &str) -> bool;
}

/// Default no-op implementation.
pub struct NoOpActionHandler;

impl MenuActionHandler for NoOpActionHandler {
    fn handle_action(&mut self, _action: &str) -> bool {
        false
    }
}

thread_local! {
    static MENU_EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Handles custom menu actions by queuing events to be emitted to Lua.
pub struct GameMenuHandler;

impl MenuActionHandler for GameMenuHandler {
    fn handle_action(&mut self, action: &str) -> bool {
        MENU_EVENTS.with(|events| {
            events.borrow_mut().push(action.to_string());
        });
        true
    }
}

/// Drains all pending menu events and returns them.
pub fn drain_menu_events() -> Vec<String> {
    MENU_EVENTS.with(|events| events.borrow_mut().drain(..).collect())
}

thread_local! {
    static SLIDER_EVENTS: RefCell<Vec<(String, f32)>> = const { RefCell::new(Vec::new()) };
}

/// Queues a slider value-change event.
pub fn push_slider_event(key: String, value: f32) {
    SLIDER_EVENTS.with(|events| {
        events.borrow_mut().push((key, value));
    });
}

/// Drains all pending slider events and returns them.
pub fn drain_slider_events() -> Vec<(String, f32)> {
    SLIDER_EVENTS.with(|events| events.borrow_mut().drain(..).collect())
}
