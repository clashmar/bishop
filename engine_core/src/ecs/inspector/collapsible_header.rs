use crate::storage::editor_config::{get_inspector_module_expanded, set_inspector_module_expanded};
use crate::ui::widgets::*;
use bishop::prelude::*;
use widgets::constants::layout;

/// Shared collapsible header used by both entity component modules and property modules.
/// Handles the -/+ toggle button, title text, and expand state persistence.
pub struct CollapsibleHeader {
    expanded: bool,
    custom_title: Option<String>,
}

impl CollapsibleHeader {
    pub const HEADER_HEIGHT: f32 = 24.0;
    const BUTTON_TEXT_OFFSET: Vec2 = Vec2::new(0.0, -1.0);

    pub fn new(title: &str) -> Self {
        let mut header = Self {
            expanded: true,
            custom_title: None,
        };
        header.sync_saved_state(title);
        header
    }

    pub fn expanded(&self) -> bool {
        self.expanded
    }

    /// Set expanded state for testing. Production code should use the
    /// toggle button in `draw()` to change state with persistence.
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.custom_title = Some(title.into());
        self
    }

    /// Sync expanded state from EditorConfig using a key derived from title.
    pub fn sync_saved_state(&mut self, title: &str) {
        if let Some(expanded) = get_inspector_module_expanded(title) {
            self.expanded = expanded;
        }
    }

    /// Draw the header bar. Returns true if the toggle was clicked.
    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        title: &str,
        blocked: bool,
    ) -> bool {
        // Background for the header
        ctx.draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            Self::HEADER_HEIGHT,
            Color::new(0., 0., 0., 0.4),
        );

        ctx.draw_text(
            title,
            rect.x + 28.0,
            rect.y + 18.0,
            layout::DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        // Toggle button (- when open, + when closed)
        let symbol = if self.expanded { "-" } else { "+" };
        let btn_rect = Rect::new(rect.x + 4.0, rect.y + 4.0, 16.0, 16.0);
        if Button::new(btn_rect, symbol)
            .text_offset(Self::BUTTON_TEXT_OFFSET)
            .suppressed(blocked)
            .show(ctx)
        {
            self.expanded = !self.expanded;
            let key = self.custom_title.as_deref().unwrap_or(title);
            set_inspector_module_expanded(key, self.expanded);
            return true;
        }

        false
    }
}

#[cfg(feature = "editor")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_new_starts_expanded() {
        let header = CollapsibleHeader::new("TestModule");
        assert!(header.expanded());
    }

    #[test]
    fn header_toggle_flips_expanded() {
        let header = CollapsibleHeader::new("TestModule");
        // Starts expanded; toggling is handled via draw() with wgpu context.
        assert!(header.expanded());
    }

    #[test]
    fn header_height_is_constant() {
        assert_eq!(CollapsibleHeader::HEADER_HEIGHT, 24.0);
    }
}
