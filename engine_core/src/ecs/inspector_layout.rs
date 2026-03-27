use crate::ui::widgets::DEFAULT_FIELD_HEIGHT;

const DEFAULT_TOP_PADDING: f32 = 10.0;
const DEFAULT_BOTTOM_GUTTER: f32 = 5.0;

/// Shared body-height builder for row-based inspector modules.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectorBodyLayout {
    row_height: f32,
    top_padding: f32,
    bottom_gutter: f32,
    content_height: f32,
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

    /// Add a contiguous run of rows separated by a fixed gap.
    pub fn rows(mut self, count: usize, row_spacing: f32) -> Self {
        if count == 0 {
            return self;
        }

        self.content_height += count as f32 * self.row_height;
        self.content_height += (count.saturating_sub(1)) as f32 * row_spacing;
        self.has_content = true;
        self
    }

    /// Add an explicit gap between content sections.
    pub fn gap(mut self, height: f32) -> Self {
        if self.has_content && height > 0.0 {
            self.content_height += height;
        }
        self
    }

    /// Resolve the final body height including shared outer padding.
    pub fn height(self) -> f32 {
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
}
