/// Widget layout constants.
pub mod layout {
    pub const WIDGET_PADDING: f32 = 10.0;
    pub const WIDGET_SPACING: f32 = 10.0;
    pub const DEFAULT_FONT_SIZE_16: f32 = 16.0;
    pub const HEADER_FONT_SIZE_20: f32 = 20.0;
    pub const FIELD_TEXT_SIZE_16: f32 = 16.0;
    pub const DEFAULT_FIELD_HEIGHT: f32 = 30.0;
    pub const DEFAULT_CHECKBOX_DIMS: f32 = 20.0;
}

// Theme default colors.
pub mod colors {
    use bishop::Color;

    pub const DEFAULT_PRIMARY_COLOR: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    pub const DEFAULT_SECONDARY_COLOR: Color = Color::new(0.2, 0.2, 0.2, 0.8);
    pub const DEFAULT_BACKGROUND_COLOR: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    pub const DEFAULT_SURFACE_COLOR: Color = Color::new(0.08, 0.08, 0.08, 0.9);
    pub const DEFAULT_TEXT_COLOR: Color = Color::WHITE;
    pub const DEFAULT_TEXT_MUTED_COLOR: Color = Color::new(0.65, 0.65, 0.65, 0.9);
    pub const DEFAULT_ACCENT_COLOR: Color = Color::new(0.1647059, 1.0, 0.0, 1.0);
    pub const DEFAULT_BORDER_COLOR: Color = Color::WHITE;
    pub const DEFAULT_HOVER_COLOR: Color = Color::new(0.2, 0.2, 0.2, 0.8);
    pub const DEFAULT_DANGER_COLOR: Color = Color::RED;
    pub const DEFAULT_INPUT_SELECTION_COLOR: Color = Color::new(0.3, 0.5, 0.8, 0.5);
    pub const DEFAULT_SELECTION_COLOR: Color = Color::new(0.706, 0.824, 1.0, 0.25);
    pub const DEFAULT_HIGHLIGHT_COLOR: Color = Color::YELLOW;
    pub const DEFAULT_PLACEHOLDER_COLOR: Color = Color::new(0.2, 0.85, 0.35, 0.22);
    pub const DEFAULT_CARD_COLOR: Color = Color::new(0.18, 0.18, 0.20, 1.0);
    pub const DEFAULT_GRID_COLOR: Color = Color::new(0.5, 0.5, 0.5, 0.2);
    pub const DEFAULT_OVERLAY_COLOR: Color = Color::BLACK;
    pub const DEFAULT_PANEL_COLOR: Color = Color::GREY;
    pub const DEFAULT_PANEL_TEXT_COLOR: Color = Color::BLACK;
}

pub mod input_repeat {
    pub const HOLD_INITIAL_DELAY: f64 = 0.50;
    pub const HOLD_REPEAT_RATE: f64 = 0.05;
}

pub const PLACEHOLDER_TEXT: &str = "<type here>";
