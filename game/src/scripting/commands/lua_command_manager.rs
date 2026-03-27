// game/src/scripting/commands/lua_command_manager.rs
use crate::scripting::commands::lua_command::LuaCommand;
use std::mem::take;
use std::vec::IntoIter;

/// Command queue that can be mutated through the global services.
#[derive(Default)]
pub struct LuaCommandManager {
    queue: Vec<Box<dyn LuaCommand>>,
}

impl LuaCommandManager {
    // Pushes a command to the command queue.
    pub fn push(&mut self, cmd: Box<dyn LuaCommand>) {
        self.queue.push(cmd);
    }

    /// Consumes the current contents of the command queue and returns an iterator.
    pub fn drain(&mut self) -> IntoIter<Box<dyn LuaCommand>> {
        take(&mut self.queue).into_iter()
    }
}
