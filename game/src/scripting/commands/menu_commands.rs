// game/src/scripting/commands/menu_commands.rs
use crate::engine::Engine;
use crate::game_global::set_menu_active;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::menu::MenuManager;

fn open_menu_and_sync_runtime_state(menu_manager: &mut MenuManager, menu_id: &str) {
    menu_manager.open_menu(menu_id);
    set_menu_active(menu_manager.has_active_menu());
}

fn close_menu_and_sync_runtime_state(menu_manager: &mut MenuManager) {
    menu_manager.close_menu();
    set_menu_active(menu_manager.has_active_menu());
}

/// Command to open a menu by id.
pub struct OpenMenuCmd {
    pub menu_id: String,
}

impl LuaCommand for OpenMenuCmd {
    fn execute(&mut self, engine: &mut Engine) {
        open_menu_and_sync_runtime_state(&mut engine.menu_manager, &self.menu_id);
    }
}

/// Command to close the current menu.
pub struct CloseMenuCmd;

impl LuaCommand for CloseMenuCmd {
    fn execute(&mut self, engine: &mut Engine) {
        close_menu_and_sync_runtime_state(&mut engine.menu_manager);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_global::{
        in_input_control, is_menu_active, reset_engine_session_state, set_menu_active,
    };
    use engine_core::input::input_constants;

    #[test]
    fn opening_a_menu_syncs_global_menu_state_immediately() {
        reset_engine_session_state();
        let mut menu_manager = MenuManager::new();

        open_menu_and_sync_runtime_state(&mut menu_manager, "pause");

        assert!(menu_manager.has_active_menu());
        assert!(is_menu_active());
        assert!(in_input_control(input_constants::MENU));
    }

    #[test]
    fn closing_a_menu_syncs_global_menu_state_immediately() {
        reset_engine_session_state();
        let mut menu_manager = MenuManager::new();
        open_menu_and_sync_runtime_state(&mut menu_manager, "pause");
        set_menu_active(true);

        close_menu_and_sync_runtime_state(&mut menu_manager);

        assert!(!menu_manager.has_active_menu());
        assert!(!is_menu_active());
        assert!(!in_input_control(input_constants::MENU));
    }
}
