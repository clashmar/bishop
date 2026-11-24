use crate::constants::layout;
use crate::theme::with_theme;
use crate::{BishopContext, Color, Rect, Vec2, draw_text_ui, measure_text_ui};
use std::cell::RefCell;

const TOOLTIP_DELAY_SECONDS: f64 = 0.5;
const TOOLTIP_MAX_TEXT_WIDTH: f32 = 320.0;
pub(crate) const TOOLTIP_PADDING: f32 = 4.0;
const TOOLTIP_LINE_HEIGHT: f32 = 20.0;
const TOOLTIP_TARGET_OFFSET: f32 = 8.0;

#[derive(Clone, PartialEq)]
struct TooltipCandidate {
    bounds: Rect,
    text: String,
}

#[derive(Clone)]
struct ActiveTooltip {
    candidate: TooltipCandidate,
    started_at: f64,
}

#[derive(Default)]
struct TooltipManager {
    candidates: Vec<TooltipCandidate>,
    active: Option<ActiveTooltip>,
}

thread_local! {
    static TOOLTIP_MANAGER: RefCell<TooltipManager> = RefCell::new(TooltipManager::default());
}

/// Offers a tooltip for a hover target with its visible bounds.
pub fn offer_tooltip(bounds: Rect, text: impl Into<String>) {
    TOOLTIP_MANAGER.with(|manager| manager.borrow_mut().offer(bounds, text));
}

pub(crate) fn begin_tooltip_frame() {
    TOOLTIP_MANAGER.with(|manager| manager.borrow_mut().begin_frame());
}

/// Draws the tooltip for the currently hovered target, if its delay has elapsed.
pub fn flush_tooltips<C: BishopContext>(ctx: &mut C) {
    let mouse = ctx.mouse_position().into();
    let candidate = TOOLTIP_MANAGER.with(|manager| {
        manager
            .borrow_mut()
            .resolve(mouse, ctx.get_time())
    });
    let Some(candidate) = candidate else {
        return;
    };

    draw_tooltip(ctx, &candidate);
}

impl TooltipManager {
    fn begin_frame(&mut self) {
        self.candidates.clear();
    }

    fn offer(&mut self, bounds: Rect, text: impl Into<String>) {
        let text = text.into();
        if !text.is_empty() {
            self.candidates.push(TooltipCandidate { bounds, text });
        }
    }

    fn resolve(&mut self, mouse: Vec2, now: f64) -> Option<TooltipCandidate> {
        let hovered = self
            .candidates
            .iter()
            .rev()
            .find(|candidate| candidate.bounds.contains(mouse))
            .cloned();

        let Some(candidate) = hovered else {
            self.active = None;
            return None;
        };

        let same_target = self
            .active
            .as_ref()
            .is_some_and(|active| active.candidate == candidate);
        if !same_target {
            self.active = Some(ActiveTooltip {
                candidate: candidate.clone(),
                started_at: now,
            });
            return None;
        }

        self.active
            .as_ref()
            .filter(|active| now - active.started_at >= TOOLTIP_DELAY_SECONDS)
            .map(|active| active.candidate.clone())
    }
}

fn draw_tooltip<C: BishopContext>(ctx: &mut C, candidate: &TooltipCandidate) {
    let padding = TOOLTIP_PADDING;
    let viewport = Rect::new(0.0, 0.0, ctx.screen_width(), ctx.screen_height());
    let max_text_width = TOOLTIP_MAX_TEXT_WIDTH.min((viewport.w - padding * 2.0).max(1.0));
    let mut lines = wrap_tooltip_text(&candidate.text, max_text_width, |line| {
        measure_text_ui(ctx, line, layout::DEFAULT_FONT_SIZE_16).width
    });
    let max_lines = ((viewport.h - padding * 2.0).max(0.0) / TOOLTIP_LINE_HEIGHT).floor() as usize;
    if max_lines == 0 {
        return;
    }
    lines.truncate(max_lines);

    let text_width = lines
        .iter()
        .map(|line| measure_text_ui(ctx, line, layout::DEFAULT_FONT_SIZE_16).width)
        .fold(0.0_f32, f32::max)
        .min(max_text_width);
    let width = (text_width + padding * 2.0).min(viewport.w);
    let height = (lines.len() as f32 * TOOLTIP_LINE_HEIGHT + padding * 2.0).min(viewport.h);
    let panel = position_tooltip(candidate.bounds, width, height, viewport);

    let background = Color::BLACK;
    let text_color = with_theme(|theme| theme.text);
    ctx.push_clip_rect(panel);
    ctx.draw_rectangle(panel.x, panel.y, panel.w, panel.h, background);
    for (line_index, line) in lines.iter().enumerate() {
        let baseline_y = panel.y + padding + 15.0 + line_index as f32 * TOOLTIP_LINE_HEIGHT;
        draw_text_ui(
            ctx,
            line,
            panel.x + padding,
            baseline_y,
            layout::DEFAULT_FONT_SIZE_16,
            text_color,
        );
    }
    ctx.pop_clip_rect();
}

fn wrap_tooltip_text(
    text: &str,
    max_width: f32,
    mut measure_width: impl FnMut(&str) -> f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if measure_width(&candidate) <= max_width {
                line = candidate;
                continue;
            }

            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            for character in word.chars() {
                let candidate = format!("{line}{character}");
                if !line.is_empty() && measure_width(&candidate) > max_width {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(character);
            }
        }
        lines.push(line);
    }
    lines
}

fn position_tooltip(target: Rect, width: f32, height: f32, viewport: Rect) -> Rect {
    let below_y = target.y + target.h + TOOLTIP_TARGET_OFFSET;
    let above_y = target.y - height - TOOLTIP_TARGET_OFFSET;
    let y = if below_y + height <= viewport.y + viewport.h || above_y < viewport.y {
        below_y
    } else {
        above_y
    };
    clamp_tooltip_rect(Rect::new(target.x, y, width, height), viewport)
}

fn clamp_tooltip_rect(rect: Rect, viewport: Rect) -> Rect {
    let width = rect.w.min(viewport.w);
    let height = rect.h.min(viewport.h);
    let max_x = (viewport.x + viewport.w - width).max(viewport.x);
    let max_y = (viewport.y + viewport.h - height).max(viewport.y);
    Rect::new(
        rect.x.clamp(viewport.x, max_x),
        rect.y.clamp(viewport.y, max_y),
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::{TooltipManager, clamp_tooltip_rect, position_tooltip, wrap_tooltip_text};
    use crate::{Rect, Vec2};

    #[test]
    fn tooltip_manager_hovering_same_target_shows_after_delay() {
        let mut manager = TooltipManager::default();
        let target = Rect::new(10.0, 10.0, 80.0, 24.0);

        manager.begin_frame();
        manager.offer(target, "sample");
        assert!(manager.resolve(Vec2::new(20.0, 20.0), 1.0).is_none());

        manager.begin_frame();
        manager.offer(target, "sample");
        assert_eq!(
            manager
                .resolve(Vec2::new(20.0, 20.0), 1.5)
                .map(|candidate| candidate.text),
            Some("sample".to_string())
        );
    }

    #[test]
    fn tooltip_manager_target_change_restarts_delay() {
        let mut manager = TooltipManager::default();
        let target = Rect::new(10.0, 10.0, 80.0, 24.0);

        manager.begin_frame();
        manager.offer(target, "first");
        assert!(manager.resolve(Vec2::new(20.0, 20.0), 1.0).is_none());

        manager.begin_frame();
        manager.offer(target, "second");
        assert!(manager.resolve(Vec2::new(20.0, 20.0), 1.4).is_none());

        manager.begin_frame();
        manager.offer(target, "second");
        assert_eq!(
            manager
                .resolve(Vec2::new(20.0, 20.0), 1.9)
                .map(|candidate| candidate.text),
            Some("second".to_string())
        );
    }

    #[test]
    fn tooltip_manager_missing_hovered_target_hides_tooltip() {
        let mut manager = TooltipManager::default();
        let target = Rect::new(10.0, 10.0, 80.0, 24.0);

        manager.begin_frame();
        manager.offer(target, "sample");
        assert!(manager.resolve(Vec2::new(20.0, 20.0), 1.0).is_none());

        manager.begin_frame();
        manager.offer(target, "sample");
        assert_eq!(
            manager
                .resolve(Vec2::new(20.0, 20.0), 1.5)
                .map(|candidate| candidate.text),
            Some("sample".to_string())
        );

        manager.begin_frame();
        manager.offer(target, "sample");
        assert!(manager.resolve(Vec2::new(200.0, 200.0), 1.6).is_none());
    }

    #[test]
    fn tooltip_manager_last_offered_overlapping_target_wins() {
        let mut manager = TooltipManager::default();
        let bounds = Rect::new(10.0, 10.0, 80.0, 24.0);

        manager.begin_frame();
        manager.offer(bounds, "first");
        manager.offer(bounds, "last");
        assert!(manager.resolve(Vec2::new(20.0, 20.0), 1.0).is_none());

        manager.begin_frame();
        manager.offer(bounds, "first");
        manager.offer(bounds, "last");
        assert_eq!(
            manager
                .resolve(Vec2::new(20.0, 20.0), 1.5)
                .map(|candidate| candidate.text),
            Some("last".to_string())
        );
    }

    #[test]
    fn tooltip_manager_ignores_empty_text() {
        let mut manager = TooltipManager::default();
        let bounds = Rect::new(10.0, 10.0, 80.0, 24.0);

        manager.begin_frame();
        manager.offer(bounds, "");

        assert!(manager.resolve(Vec2::new(20.0, 20.0), 1.0).is_none());
    }

    #[test]
    fn tooltip_text_wraps_when_maximum_width_is_exceeded() {
        let lines = wrap_tooltip_text("one two", 40.0, |text| text.len() as f32 * 10.0);

        assert_eq!(lines, vec!["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn tooltip_text_wraps_long_unbroken_words() {
        let lines = wrap_tooltip_text("abcdef", 40.0, |text| text.len() as f32 * 10.0);

        assert_eq!(lines, vec!["abcd".to_string(), "ef".to_string()]);
    }

    #[test]
    fn tooltip_panel_anchors_below_and_flips_above_when_needed() {
        let viewport = Rect::new(0.0, 0.0, 100.0, 100.0);
        let below_target = Rect::new(10.0, 10.0, 20.0, 20.0);
        let below_panel = position_tooltip(below_target, 60.0, 20.0, viewport);
        assert_eq!(below_panel, Rect::new(10.0, 38.0, 60.0, 20.0));

        let above_target = Rect::new(90.0, 90.0, 8.0, 8.0);
        let above_panel = position_tooltip(above_target, 60.0, 20.0, viewport);
        assert_eq!(above_panel, Rect::new(40.0, 62.0, 60.0, 20.0));
    }

    #[test]
    fn tooltip_panel_clamps_to_viewport_edges() {
        let viewport = Rect::new(0.0, 0.0, 100.0, 80.0);
        let panel = clamp_tooltip_rect(Rect::new(90.0, 70.0, 50.0, 30.0), viewport);

        assert_eq!(panel.x, 50.0);
        assert_eq!(panel.y, 50.0);
        assert_eq!(panel.w, 50.0);
        assert_eq!(panel.h, 30.0);
    }

    #[test]
    fn tooltip_panel_shrinks_to_fit_the_viewport() {
        let viewport = Rect::new(0.0, 0.0, 100.0, 80.0);
        let panel = clamp_tooltip_rect(Rect::new(-20.0, -10.0, 120.0, 100.0), viewport);

        assert_eq!(panel, viewport);
    }
}
