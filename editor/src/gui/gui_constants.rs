pub const PADDING: f32 = 20.0;
pub const SPACING: f32 = 10.0;
pub const INSET: f32 = 10.0;
pub const BTN_HEIGHT: f32 = 30.0;
pub const INPUT_HEIGHT: f32 = 30.0;
pub const MARGIN: f32 = 30.0;
pub const CHECKBOX_SIZE: f32 = 20.0;
pub const MENU_PANEL_HEIGHT: f32 = 48.0;

/// Constants used by the inspector shell and property panes.
pub mod inspector {
    pub const HEADER_BUTTON_Y: f32 = super::INSET;
    pub const HEADER_HEIGHT: f32 = super::BTN_HEIGHT + super::INSET * 2.0;
    pub const CONTENT_TOP_OFFSET: f32 = HEADER_HEIGHT + super::INSET;
    pub const WIDTH: f32 = 325.0;
}

/// Style class names used by editor widgets.
pub mod classes {
    /// Text drawn on panel-colored surfaces.
    pub const PANEL_TEXT: &str = "panel-text";
}
