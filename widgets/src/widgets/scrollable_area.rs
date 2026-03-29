use crate::*;

const DEFAULT_SCROLL_SPEED: f32 = 24.0;
const DEFAULT_SCROLLBAR_W: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 2.0;
const CONTENT_MARGIN: f32 = 12.0;

/// Persistent scroll state stored by the caller.
pub struct ScrollState {
    pub scroll_y: f32,
    pub auto_scroll: bool,
}

impl ScrollState {
    /// Creates a new scroll state starting at the top.
    pub fn new() -> Self {
        Self {
            scroll_y: 0.0,
            auto_scroll: false,
        }
    }

    /// Creates a new scroll state that auto-scrolls to the bottom on new content.
    pub fn with_auto_scroll() -> Self {
        Self {
            scroll_y: 0.0,
            auto_scroll: true,
        }
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for a scrollable area.
pub struct ScrollableArea {
    rect: Rect,
    content_height: f32,
    scroll_speed: f32,
    scrollbar_w: f32,
    blocked: bool,
}

impl ScrollableArea {
    /// Creates a new scrollable area for the given rect and total content height.
    pub fn new(rect: Rect, content_height: f32) -> Self {
        Self {
            rect,
            content_height,
            scroll_speed: DEFAULT_SCROLL_SPEED,
            scrollbar_w: DEFAULT_SCROLLBAR_W,
            blocked: false,
        }
    }

    /// Sets the scroll speed per mouse wheel tick.
    pub fn scroll_speed(mut self, speed: f32) -> Self {
        self.scroll_speed = speed;
        self
    }

    /// Sets whether interaction is blocked.
    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    /// Processes scroll input and returns an active area for content drawing.
    pub fn begin<C: BishopContext>(self, ctx: &mut C, state: &mut ScrollState) -> ActiveScrollArea {
        let mouse: Vec2 = ctx.mouse_position().into();
        let scroll_range = (self.content_height - self.rect.h).max(0.0);

        if !self.blocked && self.rect.contains(mouse) {
            let (_, wheel_y) = ctx.mouse_wheel();
            if wheel_y.abs() > 0.0 {
                state.scroll_y += wheel_y * self.scroll_speed;
                state.auto_scroll = false;
            }
        }

        // Auto-scroll to bottom
        if state.auto_scroll {
            state.scroll_y = -scroll_range;
        }

        state.scroll_y = state.scroll_y.clamp(-scroll_range, 0.0);

        // Re-enable auto-scroll if scrolled near bottom
        if scroll_range > 0.0 && state.scroll_y <= -scroll_range + 1.0 {
            state.auto_scroll = true;
        }

        ActiveScrollArea {
            rect: self.rect,
            scroll_range,
            content_height: self.content_height,
            scrollbar_w: self.scrollbar_w,
        }
    }
}

/// Active scroll area returned by `begin()`. Provides visibility helpers and scrollbar drawing.
pub struct ActiveScrollArea {
    rect: Rect,
    scroll_range: f32,
    content_height: f32,
    scrollbar_w: f32,
}

impl ActiveScrollArea {
    /// The rect of the scrollable area, with width reduced to account for the scrollbar when present.
    pub fn content_rect(&self) -> Rect {
        if self.scroll_range > 0.0 {
            Rect::new(
                self.rect.x,
                self.rect.y,
                self.rect.w - self.scrollbar_w - SCROLLBAR_MARGIN,
                self.rect.h,
            )
        } else {
            self.rect
        }
    }

    /// The scroll range (0 means content fits, >0 means scrollable).
    pub fn scroll_range(&self) -> f32 {
        self.scroll_range
    }

    /// Width available for content, accounting for scrollbar when present.
    pub fn usable_width(&self) -> f32 {
        if self.scroll_range > 0.0 {
            self.rect.w - CONTENT_MARGIN - self.scrollbar_w
        } else {
            self.rect.w - CONTENT_MARGIN
        }
    }

    /// Returns true if an item is at least partially visible.
    pub fn is_visible(&self, item_y: f32, item_height: f32) -> bool {
        item_y + item_height > self.rect.y && item_y < self.rect.y + self.rect.h
    }

    /// Returns true if an item is fully visible within the scroll area.
    pub fn is_fully_visible(&self, item_y: f32, item_height: f32) -> bool {
        item_y >= self.rect.y && item_y + item_height <= self.rect.y + self.rect.h
    }

    /// Draws the scrollbar. Call after all content is drawn.
    pub fn draw_scrollbar<C: BishopContext>(&self, ctx: &mut C, scroll_y: f32) {
        if self.scroll_range <= 0.0 {
            return;
        }

        let ratio = self.rect.h / self.content_height;
        let bar_h = self.rect.h * ratio;
        let t = (-scroll_y) / self.scroll_range;
        let bar_x = self.rect.x + self.rect.w - self.scrollbar_w - SCROLLBAR_MARGIN;
        let bar_y = self.rect.y + t * (self.rect.h - bar_h);

        // Track
        ctx.draw_rectangle(
            bar_x,
            self.rect.y,
            self.scrollbar_w,
            self.rect.h,
            Color::new(0.15, 0.15, 0.15, 0.6),
        );

        // Thumb
        ctx.draw_rectangle(
            bar_x,
            bar_y,
            self.scrollbar_w,
            bar_h,
            Color::new(0.7, 0.7, 0.7, 0.9),
        );
    }
}
