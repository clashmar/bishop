use crate::engine::Engine;

/// All mutating Lua actions implement this.
pub trait LuaCommand {
    /// Execute the command, mutating the supplied `GameState`.
    fn execute(&mut self, engine: &mut Engine);
}
