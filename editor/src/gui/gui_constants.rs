// Dimensions
pub const PADDING: f32 = 20.0;
pub const SPACING: f32 = 10.0;
pub const INSET: f32 = 10.0;
pub const BTN_HEIGHT: f32 = 30.0;
pub const INPUT_HEIGHT: f32 = 30.0;
pub const MARGIN: f32 = 30.0;
pub const CHECKBOX_SIZE: f32 = 20.0;
pub const MENU_PANEL_HEIGHT: f32 = 48.0;
pub const INSPECTOR_HEADER_BUTTON_Y: f32 = INSET;
pub const INSPECTOR_HEADER_HEIGHT: f32 = BTN_HEIGHT + INSET * 2.0;
pub const INSPECTOR_CONTENT_TOP_OFFSET: f32 = INSPECTOR_HEADER_HEIGHT + INSET;

/// Style class names used by editor widgets.
pub mod classes {
    /// Text drawn on panel-colored surfaces.
    pub const PANEL_TEXT: &str = "panel-text";
}
