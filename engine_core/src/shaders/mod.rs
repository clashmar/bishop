// engine_core/src/shaders/mod.rs

pub const VERTEX_SHADER: &str = include_str!("vertex.vert");
pub const GLOW_FRAGMENT_SHADER: &str = include_str!("glow.frag");
pub const AMB_FRAGMENT_SHADER: &str = include_str!("amb.frag");
pub const SPOT_FRAGMENT_SHADER: &str = include_str!("spot.frag");
pub const SCENE_FRAGMENT_SHADER: &str = include_str!("scene.frag");
pub const COMPOSITE_FRAGMENT_SHADER: &str = include_str!("composite.frag");
pub const UNDARKENED_FRAGMENT_SHADER: &str = include_str!("undarkened.frag");
