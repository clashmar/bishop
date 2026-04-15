pub mod constants;
pub mod diagnostics;
pub mod engine;
pub mod game_global;
pub mod input;
pub mod physics;
#[path = "lib_playtest.rs"]
pub mod playtest;
pub mod scripting;
pub mod startup;
pub mod transitions;

#[cfg(test)]
#[path = "playtest/control_tests.rs"]
mod control_tests;
