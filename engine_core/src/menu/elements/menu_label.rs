use reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

/// Static text label component.
#[derive(Debug, Clone, Serialize, Deserialize, Default, Reflect)]
pub struct MenuLabel {
    pub text: String,
    pub font_size: f32,
}
