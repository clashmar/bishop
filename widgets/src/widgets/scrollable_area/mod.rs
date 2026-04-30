use crate::*;

const DEFAULT_SCROLL_SPEED: f32 = 24.0;
const DEFAULT_SCROLLBAR_W: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 2.0;
const CONTENT_MARGIN: f32 = 12.0;

/// Persistent scroll state stored by the caller.
pub struct ScrollState {
    pub scroll_y: f32,
    pub auto_scroll: bool,
    pub dragging_thumb: bool,
    pub thumb_drag_offset: f32,
}

impl ScrollState {
    /// Creates a new scroll state starting at the top.
    pub fn new() -> Self {
        Self {
            scroll_y: 0.0,
            auto_scroll: false,
            dragging_thumb: false,
            thumb_drag_offset: 0.0,
        }
    }

    /// Creates a new scroll state that auto-scrolls to the bottom on new content.
    pub fn with_auto_scroll() -> Self {
        Self {
            scroll_y: 0.0,
            auto_scroll: true,
            dragging_thumb: false,
            thumb_drag_offset: 0.0,
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
    visuals: WidgetVisuals,
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
            visuals: WidgetVisuals::default(),
        }
    }

    /// Sets the scroll speed per mouse wheel tick.
    pub fn scroll_speed(mut self, speed: f32) -> Self {
        self.scroll_speed = speed;
        self
    }

    /// Sets visual overrides for the scrollable area.
    pub fn visuals(mut self, visuals: WidgetVisuals) -> Self {
        self.visuals = visuals;
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

        let ratio = self.rect.h / self.content_height;
        let bar_h = self.rect.h * ratio;
        let bar_x = self.rect.x + self.rect.w - self.scrollbar_w - SCROLLBAR_MARGIN;
        let thumb_y = if scroll_range > 0.0 {
            let t = (-state.scroll_y) / scroll_range;
            self.rect.y + t * (self.rect.h - bar_h)
        } else {
            self.rect.y
        };
        let thumb_rect = Rect::new(bar_x, thumb_y, self.scrollbar_w, bar_h);
        let track_rect = Rect::new(bar_x, self.rect.y, self.scrollbar_w, self.rect.h);

        if !self.blocked && scroll_range > 0.0 {
            // Wheel scroll
            if self.rect.contains(mouse) {
                let (_, wheel_y) = ctx.mouse_wheel();
                if wheel_y.abs() > 0.0 {
                    state.scroll_y += wheel_y * self.scroll_speed;
                    state.auto_scroll = false;
                }
            }

            // Thumb press
            if ctx.is_mouse_button_pressed(MouseButton::Left) && thumb_rect.contains(mouse) {
                state.dragging_thumb = true;
                state.thumb_drag_offset = mouse.y - thumb_y;
                state.auto_scroll = false;
            }

            // Track press (outside thumb)
            if ctx.is_mouse_button_pressed(MouseButton::Left)
                && track_rect.contains(mouse)
                && !thumb_rect.contains(mouse)
            {
                let t =
                    ((mouse.y - self.rect.y - bar_h / 2.0) / (self.rect.h - bar_h)).clamp(0.0, 1.0);
                state.scroll_y = -t * scroll_range;
                state.auto_scroll = false;
            }

            // Active drag
            if state.dragging_thumb && ctx.is_mouse_button_down(MouseButton::Left) {
                let new_thumb_y = mouse.y - state.thumb_drag_offset;
                let t = ((new_thumb_y - self.rect.y) / (self.rect.h - bar_h)).clamp(0.0, 1.0);
                state.scroll_y = -t * scroll_range;
            }

            // End drag
            if ctx.is_mouse_button_released(MouseButton::Left) {
                state.dragging_thumb = false;
                state.thumb_drag_offset = 0.0;
            }
        }

        // Auto-scroll to bottom
        if state.auto_scroll && !state.dragging_thumb {
            state.scroll_y = -scroll_range;
        }

        state.scroll_y = state.scroll_y.clamp(-scroll_range, 0.0);

        // Re-enable auto-scroll if scrolled near bottom
        if scroll_range > 0.0 && state.scroll_y <= -scroll_range + 1.0 && !state.dragging_thumb {
            state.auto_scroll = true;
        }

        ActiveScrollArea {
            rect: self.rect,
            scroll_range,
            content_height: self.content_height,
            scrollbar_w: self.scrollbar_w,
            visuals: self.visuals,
        }
    }
}

/// Active scroll area returned by `begin()`. Provides visibility helpers and scrollbar drawing.
pub struct ActiveScrollArea {
    rect: Rect,
    scroll_range: f32,
    content_height: f32,
    scrollbar_w: f32,
    visuals: WidgetVisuals,
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
    pub fn draw_scrollbar<C: BishopContext>(&self, ctx: &mut C, state: &ScrollState) {
        if self.scroll_range <= 0.0 {
            return;
        }

        let ratio = self.rect.h / self.content_height;
        let bar_h = self.rect.h * ratio;
        let t = (-state.scroll_y) / self.scroll_range;
        let bar_x = self.rect.x + self.rect.w - self.scrollbar_w - SCROLLBAR_MARGIN;
        let bar_y = self.rect.y + t * (self.rect.h - bar_h);

        let mouse: Vec2 = ctx.mouse_position().into();
        let thumb_rect = Rect::new(bar_x, bar_y, self.scrollbar_w, bar_h);
        let mouse_over_thumb = thumb_rect.contains(mouse);

        const TRACK_COLOR: Color = Color::new(0.15, 0.15, 0.15, 0.6);
        const THUMB_IDLE: Color = Color::new(0.7, 0.7, 0.7, 0.9);
        const THUMB_HOVER: Color = Color::new(0.85, 0.85, 0.85, 0.9);
        const THUMB_DRAG: Color = Color::new(0.9, 0.9, 0.9, 1.0);

        // Track
        ctx.draw_rectangle(
            bar_x,
            self.rect.y,
            self.scrollbar_w,
            self.rect.h,
            resolve(self.visuals.surface, TRACK_COLOR),
        );

        // Thumb
        let thumb_col = if state.dragging_thumb {
            resolve(self.visuals.text, THUMB_DRAG)
        } else if mouse_over_thumb {
            resolve(self.visuals.text_muted, THUMB_HOVER)
        } else {
            resolve(self.visuals.text_muted, THUMB_IDLE)
        };
        ctx.draw_rectangle(bar_x, bar_y, self.scrollbar_w, bar_h, thumb_col);
    }
}

const DRAG_EDGE_AUTOSCROLL_BAND: f32 = 24.0;
const DRAG_EDGE_AUTOSCROLL_MAX_STEP: f32 = DEFAULT_SCROLL_SPEED;

impl ActiveScrollArea {
    /// Applies edge-band autoscroll when a drag is active and the pointer is
    /// within the top or bottom edge band of the scroll area. Returns `true`
    /// when `state.scroll_y` changed.
    pub fn apply_drag_edge_autoscroll<C: BishopContext>(
        &self,
        ctx: &C,
        state: &mut ScrollState,
        drag_active: bool,
    ) -> bool {
        if !drag_active || self.scroll_range <= 0.0 {
            return false;
        }

        let mouse: Vec2 = ctx.mouse_position().into();
        if !self.rect.contains(mouse) {
            return false;
        }

        let top_band_end = self.rect.y + DRAG_EDGE_AUTOSCROLL_BAND;
        let bottom_band_start = self.rect.y + self.rect.h - DRAG_EDGE_AUTOSCROLL_BAND;

        let delta = if mouse.y < top_band_end {
            let t = ((top_band_end - mouse.y) / DRAG_EDGE_AUTOSCROLL_BAND).clamp(0.0, 1.0);
            DRAG_EDGE_AUTOSCROLL_MAX_STEP * t
        } else if mouse.y > bottom_band_start {
            let t = ((mouse.y - bottom_band_start) / DRAG_EDGE_AUTOSCROLL_BAND).clamp(0.0, 1.0);
            -DRAG_EDGE_AUTOSCROLL_MAX_STEP * t
        } else {
            0.0
        };

        if delta == 0.0 {
            return false;
        }

        let previous = state.scroll_y;
        state.scroll_y = (state.scroll_y + delta).clamp(-self.scroll_range, 0.0);
        if state.scroll_y != previous {
            state.auto_scroll = false;
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests;
