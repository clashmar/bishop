use crate::constants::colors;
use crate::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// Result of a slider interaction this frame.
#[derive(Debug, Clone, PartialEq)]
pub enum SliderState {
    /// No interaction or value change.
    Unchanged,
    /// Value is changing during a drag.
    Previewing,
    /// Drag ended or track was clicked.
    Committed { initial_value: f32 },
}

/// A horizontal slider widget using the builder pattern.
pub struct Slider {
    id: WidgetId,
    rect: Rect,
    min: f32,
    max: f32,
    value: f32,
    label: Option<String>,
    focused: bool,
    base: WidgetBase,
}

impl Slider {
    pub fn new(id: WidgetId, rect: impl Into<Rect>, min: f32, max: f32, value: f32) -> Self {
        Self {
            id,
            rect: rect.into(),
            min,
            max,
            value,
            label: None,
            focused: false,
            base: WidgetBase {
                blocked: false,
                overrides: WidgetTheme::default(),
                ..WidgetBase::default()
            },
        }
    }

    /// Sets an optional label text displayed to the left of the track.
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self
    }

    /// Sets whether the slider is visually focused (highlight outline).
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn show<C: BishopContext>(self, ctx: &mut C) -> (f32, SliderState) {
        let class = self.base.class_name.as_deref();
        let id = self.base.style_id.as_deref();
        let widget_theme = resolve_theme_for::<Self>(class, id);
        let rect = self.rect;
        let widget_id = self.id;
        let min = self.min;
        let max = self.max;
        let value = self.value;

        thread_local! {
            static STATE: RefCell<HashMap<WidgetId, (bool, f32, f32)>> =
                RefCell::new(HashMap::new());
        }

        let mut was_dragging = false;
        let mut drag_offset = 0.0_f32;
        let mut initial_value = value;
        STATE.with(|s| {
            let map = s.borrow();
            if let Some(&(d, off, init)) = map.get(&widget_id) {
                was_dragging = d;
                drag_offset = off;
                initial_value = init;
            }
        });

        let (track_rect, label_rect) = if let Some(ref label_text) = self.label {
            let split = rect.w * 0.4;
            let lr = Rect::new(rect.x, rect.y, split, rect.h);
            let tr = Rect::new(rect.x + split, rect.y, rect.w - split, rect.h);
            (tr, Some((lr, label_text.as_str())))
        } else {
            (rect, None)
        };

        if let Some((lr, label_text)) = &label_rect {
            let label_bg = resolve_with_theme(
                None,
                if self.focused {
                    widget_theme.hover
                } else {
                    widget_theme.background
                },
                colors::DEFAULT_BACKGROUND_COLOR,
            );
            ctx.draw_rectangle(lr.x, lr.y, lr.w, lr.h, label_bg);

            let text_color =
                resolve_with_theme(self.base.overrides.text, widget_theme.text, Color::WHITE);
            let label_font_size = 14.0;
            let text_dims = ctx.measure_text(label_text, label_font_size);
            let text_x = lr.x + (lr.w - text_dims.width) * 0.5;
            let text_y = lr.y + (lr.h - text_dims.height) * 0.5 + text_dims.offset_y;
            draw_text_ui(ctx, label_text, text_x, text_y, label_font_size, text_color);

            let divider_color =
                resolve_with_theme(None, widget_theme.border, colors::DEFAULT_BORDER_COLOR);
            ctx.draw_line(
                track_rect.x,
                rect.y,
                track_rect.x,
                rect.y + rect.h,
                4.0,
                divider_color,
            );
        }

        let track_h = track_rect.h * 0.2;
        let track_y = track_rect.y + (track_rect.h - track_h) * 0.5;
        let handle_sz = track_rect.h;
        let range = max - min;
        let norm = ((value - min) / range).clamp(0.0, 1.0);
        let handle_x = track_rect.x + norm * (track_rect.w - handle_sz);

        let track_color = Color::new(0.2, 0.2, 0.2, 0.8);
        let handle_color_idle = Color::new(0.4, 0.4, 0.8, 1.0);

        ctx.draw_rectangle(
            track_rect.x,
            track_rect.y,
            track_rect.w,
            track_rect.h,
            resolve_with_theme(
                self.base.overrides.background,
                widget_theme.background,
                colors::DEFAULT_BACKGROUND_COLOR,
            ),
        );
        ctx.draw_rectangle(
            track_rect.x,
            track_y,
            track_rect.w,
            track_h,
            resolve_with_theme(
                self.base.overrides.secondary,
                widget_theme.secondary,
                track_color,
            ),
        );

        let outline_color = if self.focused {
            resolve_with_theme(
                self.base.overrides.highlight,
                widget_theme.highlight,
                colors::DEFAULT_HIGHLIGHT_COLOR,
            )
        } else {
            resolve_with_theme(
                self.base.overrides.border,
                widget_theme.border,
                colors::DEFAULT_BORDER_COLOR,
            )
        };
        ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2., outline_color);

        let resolved_primary = resolve_with_theme(
            self.base.overrides.primary,
            widget_theme.primary,
            handle_color_idle,
        );

        let handle_col =
            if was_dragging && !is_dropdown_open() && !is_context_menu_open() && !self.base.blocked
            {
                Color::new(
                    resolved_primary.r * 0.8,
                    resolved_primary.g * 0.8,
                    resolved_primary.b * 0.8,
                    resolved_primary.a,
                )
            } else {
                resolved_primary
            };
        ctx.draw_rectangle(handle_x, track_rect.y, handle_sz, track_rect.h, handle_col);
        ctx.draw_rectangle_lines(
            handle_x,
            track_rect.y,
            handle_sz,
            track_rect.h,
            2.,
            resolve_with_theme(
                self.base.overrides.border,
                widget_theme.border,
                Color::WHITE,
            ),
        );

        if self.base.blocked || is_dropdown_open() || is_context_menu_open() {
            return (value, SliderState::Unchanged);
        }

        let mouse = ctx.mouse_position();
        let mouse_vec = Vec2::new(mouse.0, mouse.1);
        let mouse_over_handle =
            Rect::new(handle_x, track_rect.y, handle_sz, track_rect.h).contains(mouse_vec);
        let mouse_over_track = track_rect.contains(mouse_vec);

        let mut dragging = was_dragging;

        if ctx.is_mouse_button_pressed(MouseButton::Left) && mouse_over_handle {
            dragging = true;
            drag_offset = mouse.0 - handle_x;
            initial_value = value;
        }

        if ctx.is_mouse_button_released(MouseButton::Left) {
            dragging = false;
            drag_offset = 0.0;
        }

        let mut new_value = value;
        let mut state = SliderState::Unchanged;

        if was_dragging && !dragging {
            if (value - initial_value).abs() > f32::EPSILON {
                state = SliderState::Committed { initial_value };
            }
        } else if dragging {
            let handle_center = mouse.0 - drag_offset;
            let rel = ((handle_center - track_rect.x) / (track_rect.w - handle_sz)).clamp(0.0, 1.0);
            new_value = min + rel * range;
            if (new_value - value).abs() > f32::EPSILON {
                state = SliderState::Previewing;
            }
        } else if mouse_over_track && ctx.is_mouse_button_pressed(MouseButton::Left) {
            let rel = ((mouse.0 - track_rect.x) / (track_rect.w - handle_sz)).clamp(0.0, 1.0);
            new_value = min + rel * range;
            state = SliderState::Committed {
                initial_value: value,
            };
        }

        STATE.with(|s| {
            let mut map = s.borrow_mut();
            map.insert(widget_id, (dragging, drag_offset, initial_value));
        });

        (new_value, state)
    }
}

impl Widget for Slider {
    fn widget_type() -> WidgetType {
        WidgetType::Slider
    }
    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
}
