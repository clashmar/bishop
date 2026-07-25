use crate::ui::text::*;
use crate::ui::widgets::*;
use std::time::Instant;
use widgets::constants::layout;
use widgets::theme::with_theme;

const PADDING: f32 = 20.0;

/// A simple toast that disappears after a short delay.
pub struct Toast {
    /// Text that will be shown.
    pub msg: String,
    /// When the toast was created.
    start: Instant,
    /// How long the toast stays visible (seconds).
    pub duration: f32,
    /// Whether the toast is currently visible.
    pub active: bool,
    /// If true, the toast never auto-expires from the timer.
    persistent: bool,
    /// When set, cycles `.`/`..`/`...` after the base text.
    throb_base_msg: Option<String>,
}

impl Toast {
    /// Create a new toast that lives for `duration` seconds.
    pub fn new<S: Into<String>>(msg: S, duration: f32) -> Self {
        Self {
            msg: msg.into(),
            start: Instant::now(),
            duration,
            active: true,
            persistent: false,
            throb_base_msg: None,
        }
    }

    /// Create a persistent toast that throb-animates with cycling dots and never auto-expires.
    pub fn new_throbbing<S: Into<String>>(msg: S) -> Self {
        let msg_string = msg.into();
        Self {
            persistent: true,
            throb_base_msg: Some(msg_string.clone()),
            duration: f32::MAX,
            ..Self::new(msg_string, 0.0)
        }
    }

    /// Reset the internal timer. Useful with a non-zero `duration` as a safety-net expiry.
    pub fn refresh(&mut self) {
        self.start = Instant::now();
    }

    /// Call each frame. Draws the toast if it is still alive.
    pub fn update<C: BishopContext>(&mut self, ctx: &mut C) {
        if !self.active {
            return;
        }
        if !self.persistent && self.start.elapsed().as_secs_f32() >= self.duration {
            self.active = false;
            return;
        }

        let display_msg = self.throb_display_msg();
        let measure_msg = self.throb_measure_msg();
        let txt = measure_text(ctx, &measure_msg, layout::DEFAULT_FONT_SIZE_16);

        // Bottom left
        let bg_rect = Rect::new(
            PADDING,
            ctx.screen_height() - PADDING - (txt.height + PADDING),
            txt.width + PADDING * 2.0,
            txt.height + PADDING,
        );

        // Background
        ctx.draw_rectangle(
            bg_rect.x,
            bg_rect.y,
            bg_rect.w,
            bg_rect.h,
            with_theme(|t| t.overlay.with_alpha(0.7)),
        );

        // Text
        draw_text_ui(
            ctx,
            &display_msg,
            bg_rect.x + PADDING,
            bg_rect.y + (bg_rect.h - txt.height) / 2.0 + txt.offset_y,
            layout::DEFAULT_FONT_SIZE_16,
            with_theme(|t| t.text),
        );
    }

    /// Returns the text to draw, including the throb dot suffix if applicable.
    fn throb_display_msg(&self) -> String {
        if let Some(ref base) = self.throb_base_msg {
            let dot_count = (self.start.elapsed().as_secs_f32() * 2.0) as usize % 3 + 1;
            format!("{}{}", base, ".".repeat(dot_count))
        } else {
            self.msg.clone()
        }
    }

    /// Returns the widest possible text (for rect measurement) to avoid jitter.
    fn throb_measure_msg(&self) -> String {
        if let Some(ref base) = self.throb_base_msg {
            format!("{}...", base)
        } else {
            self.msg.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bishop::prelude::*;
    use widgets::test_utils::WidgetTestContext;

    #[test]
    fn toast_centers_text_using_baseline_offset() {
        let text_dims = TextDimensions {
            width: 80.0,
            height: 16.0,
            offset_y: 12.0,
        };
        let mut ctx = WidgetTestContext::new();
        ctx.screen_height = 200.0;
        ctx.text_dims = Some(text_dims);
        let mut toast = Toast::new("Saved", 5.0);

        toast.update(&mut ctx);

        let bg = ctx.rect_calls[0];
        let text_call = &ctx.draw_text_calls[0];
        let expected_baseline_y = bg.y + (bg.h - text_dims.height) / 2.0 + text_dims.offset_y;

        assert_eq!(text_call.text, "Saved");
        assert_eq!(text_call.font_size, layout::DEFAULT_FONT_SIZE_16);
        assert_eq!(text_call.color, Color::WHITE);
        assert!((text_call.x - (bg.x + PADDING)).abs() < f32::EPSILON);
        assert!((text_call.y - expected_baseline_y).abs() < f32::EPSILON);
    }

    #[test]
    fn new_throbbing_sets_persistent_and_throb_base() {
        let toast = Toast::new_throbbing("Building playtest");
        assert!(toast.persistent);
        assert_eq!(toast.throb_base_msg.as_deref(), Some("Building playtest"));
        assert!(toast.active);
    }

    #[test]
    fn persistent_toast_does_not_expire() {
        let text_dims = TextDimensions {
            width: 80.0,
            height: 16.0,
            offset_y: 12.0,
        };
        let mut ctx = WidgetTestContext::new();
        ctx.screen_height = 200.0;
        ctx.text_dims = Some(text_dims);
        let mut toast = Toast::new_throbbing("Working");
        std::thread::sleep(std::time::Duration::from_millis(10));
        toast.update(&mut ctx);
        assert!(
            toast.active,
            "persistent toast should still be active after duration expires"
        );
    }

    #[test]
    fn throb_display_msg_cycles_dots() {
        let toast = Toast::new_throbbing("Build");
        let msg = toast.throb_display_msg();
        assert!(
            msg == "Build." || msg == "Build.." || msg == "Build...",
            "throb display should end with 1-3 dots, got: {msg}"
        );
    }

    #[test]
    fn throb_measure_msg_is_widest_variant() {
        let toast = Toast::new_throbbing("Build");
        assert_eq!(toast.throb_measure_msg(), "Build...");
    }

    #[test]
    fn throb_measure_msg_falls_back_to_msg_when_no_throb() {
        let toast = Toast::new("Saved", 5.0);
        assert_eq!(toast.throb_measure_msg(), "Saved");
    }

    #[test]
    fn refresh_resets_start_time() {
        let mut toast = Toast::new_throbbing("Working");
        std::thread::sleep(std::time::Duration::from_millis(10));
        toast.refresh();
        let text_dims = TextDimensions {
            width: 80.0,
            height: 16.0,
            offset_y: 12.0,
        };
        let mut ctx = WidgetTestContext::new();
        ctx.screen_height = 200.0;
        ctx.text_dims = Some(text_dims);
        toast.update(&mut ctx);
        assert!(toast.active, "refreshed toast should still be active");
    }
}
