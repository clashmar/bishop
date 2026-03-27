use crate::ui::widgets::DEFAULT_FIELD_HEIGHT;

const DEFAULT_TOP_PADDING: f32 = 10.0;
const DEFAULT_BOTTOM_GUTTER: f32 = 10.0;

/// Shared body-height builder for inspector module bodies.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectorBodyLayout {
    /// Default row height used by [`Self::rows`].
    row_height: f32,
    /// Vertical padding before the first body section.
    top_padding: f32,
    /// Vertical padding after the last body section.
    bottom_gutter: f32,
    /// Total height of all content sections.
    content_height: f32,
    /// Tracks whether any visible section has been added.
    has_content: bool,
}

impl Default for InspectorBodyLayout {
    fn default() -> Self {
        Self {
            row_height: DEFAULT_FIELD_HEIGHT,
            top_padding: DEFAULT_TOP_PADDING,
            bottom_gutter: DEFAULT_BOTTOM_GUTTER,
            content_height: 0.0,
            has_content: false,
        }
    }
}

impl InspectorBodyLayout {
    /// Create a shared body layout with the default inspector metrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Override only the outer top padding.
    pub fn top_padding(mut self, top: f32) -> Self {
        self.top_padding = top;
        self
    }

    /// Override only the outer bottom gutter.
    pub fn bottom_gutter(mut self, bottom: f32) -> Self {
        self.bottom_gutter = bottom;
        self
    }

    /// Override the outer top and bottom padding.
    pub fn padding(mut self, top: f32, bottom: f32) -> Self {
        self.top_padding = top;
        self.bottom_gutter = bottom;
        self
    }

    /// Add a fixed-height content block.
    pub fn block(mut self, height: f32) -> Self {
        if height <= 0.0 {
            return self;
        }

        self.content_height += height;
        self.has_content = true;
        self
    }

    /// Add a contiguous run of rows separated by a fixed gap.
    pub fn rows(mut self, count: usize, row_spacing: f32) -> Self {
        if count == 0 {
            return self;
        }

        let row_height = self.row_height;
        self = self.block(count as f32 * row_height);
        self.block((count.saturating_sub(1)) as f32 * row_spacing)
    }

    /// Add an explicit gap between content sections.
    pub fn gap(mut self, height: f32) -> Self {
        if self.has_content && height > 0.0 {
            self.content_height += height;
        }
        self
    }

    /// Resolve the final body height including shared outer padding.
    pub fn height(&self) -> f32 {
        if !self.has_content {
            return self.top_padding + self.bottom_gutter;
        }

        self.top_padding + self.content_height + self.bottom_gutter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_adds_rows_gaps_and_shared_bottom_gutter() {
        let height = InspectorBodyLayout::new()
            .rows(2, 7.0)
            .gap(12.0)
            .rows(1, 9.0)
            .height();

        assert_eq!(
            height,
            DEFAULT_TOP_PADDING
                + DEFAULT_FIELD_HEIGHT * 3.0
                + 7.0
                + 12.0
                + DEFAULT_BOTTOM_GUTTER
        );
    }

    #[test]
    fn layout_supports_mixed_fixed_blocks_and_rows() {
        let height = InspectorBodyLayout::new()
            .padding(0.0, 0.0)
            .block(28.0)
            .gap(12.0)
            .rows(2, 9.0)
            .height();

        assert_eq!(height, 28.0 + 12.0 + DEFAULT_FIELD_HEIGHT * 2.0 + 9.0);
    }

    #[test]
    fn top_padding_override_keeps_default_bottom_gutter() {
        let height = InspectorBodyLayout::new()
            .top_padding(0.0)
            .block(28.0)
            .height();

        assert_eq!(height, 28.0 + DEFAULT_BOTTOM_GUTTER);
    }
}
