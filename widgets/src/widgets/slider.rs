use crate::constants::colors;
use crate::theme::WidgetThemeMapper;
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
    blocked: bool,
    visuals: WidgetVisuals,
    class_name: Option<String>,
    style_id: Option<String>,
}

impl Slider {
    pub fn new(id: WidgetId, rect: impl Into<Rect>, min: f32, max: f32, value: f32) -> Self {
        Self {
            id,
            rect: rect.into(),
            min,
            max,
            value,
            blocked: false,
            visuals: WidgetVisuals::default(),
            class_name: None,
            style_id: None,
        }
    }

    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    pub fn visuals(mut self, visuals: WidgetVisuals) -> Self {
        self.visuals = visuals;
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class_name = Some(class.into());
        self
    }

    pub fn style_id(mut self, id: impl Into<String>) -> Self {
        self.style_id = Some(id.into());
        self
    }

    pub fn maybe_class(mut self, class: Option<&str>) -> Self {
        if let Some(c) = class {
            self.class_name = Some(c.to_string());
        }
        self
    }

    pub fn maybe_style_id(mut self, id: Option<&str>) -> Self {
        if let Some(i) = id {
            self.style_id = Some(i.to_string());
        }
        self
    }

    pub fn apply_selectors(mut self, class: Option<&str>, style_id: Option<&str>) -> Self {
        if let Some(c) = class {
            self.class_name = Some(c.to_string());
        }
        if let Some(i) = style_id {
            self.style_id = Some(i.to_string());
        }
        self
    }

    pub fn show<C: BishopContext>(self, ctx: &mut C) -> (f32, SliderState) {
        let class = self.class_name.as_deref();
        let id = self.style_id.as_deref();
        let theme_vs = themed_visuals_for::<Self>(class, id);
        let rect = self.rect;
        let id = self.id;
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
            if let Some(&(d, off, init)) = map.get(&id) {
                was_dragging = d;
                drag_offset = off;
                initial_value = init;
            }
        });

        let track_h = rect.h * 0.2;
        let track_y = rect.y + (rect.h - track_h) * 0.5;
        let handle_sz = rect.h;
        let range = max - min;
        let norm = ((value - min) / range).clamp(0.0, 1.0);
        let handle_x = rect.x + norm * (rect.w - handle_sz);

        let track_color = Color::new(0.2, 0.2, 0.2, 0.8);
        let handle_color_dragging = Color::new(0.6, 0.6, 0.9, 1.0);
        let handle_color_idle = Color::new(0.4, 0.4, 0.8, 1.0);

        ctx.draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            resolve_with_theme(
                self.visuals.background,
                theme_vs.background,
                colors::DEFAULT_BACKGROUND_COLOR,
            ),
        );
        ctx.draw_rectangle(
            rect.x,
            track_y,
            rect.w,
            track_h,
            resolve_with_theme(self.visuals.secondary, theme_vs.secondary, track_color),
        );
        ctx.draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.,
            resolve_with_theme(
                self.visuals.border,
                theme_vs.border,
                colors::DEFAULT_BORDER_COLOR,
            ),
        );

        let handle_col =
            if was_dragging && !is_dropdown_open() && !is_context_menu_open() && !self.blocked {
                resolve_with_theme(self.visuals.hover, theme_vs.hover, handle_color_dragging)
            } else {
                resolve_with_theme(self.visuals.primary, theme_vs.primary, handle_color_idle)
            };
        ctx.draw_rectangle(handle_x, rect.y, handle_sz, rect.h, handle_col);
        ctx.draw_rectangle_lines(
            handle_x,
            rect.y,
            handle_sz,
            rect.h,
            2.,
            resolve_with_theme(self.visuals.border, theme_vs.border, Color::WHITE),
        );

        if self.blocked || is_dropdown_open() || is_context_menu_open() {
            return (value, SliderState::Unchanged);
        }

        let mouse = ctx.mouse_position();
        let mouse_vec = Vec2::new(mouse.0, mouse.1);
        let mouse_over_handle = Rect::new(handle_x, rect.y, handle_sz, rect.h).contains(mouse_vec);
        let mouse_over_track = rect.contains(mouse_vec);

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
            let rel = ((handle_center - rect.x) / (rect.w - handle_sz)).clamp(0.0, 1.0);
            new_value = min + rel * range;
            if (new_value - value).abs() > f32::EPSILON {
                state = SliderState::Previewing;
            }
        } else if mouse_over_track && ctx.is_mouse_button_pressed(MouseButton::Left) {
            let rel = ((mouse.0 - rect.x) / (rect.w - handle_sz)).clamp(0.0, 1.0);
            new_value = min + rel * range;
            state = SliderState::Committed {
                initial_value: value,
            };
        }

        STATE.with(|s| {
            let mut map = s.borrow_mut();
            map.insert(id, (dragging, drag_offset, initial_value));
        });

        (new_value, state)
    }
}

impl WidgetThemeMapper for Slider {
    fn type_kind() -> WidgetType {
        WidgetType::Slider
    }
    fn theme_visuals(theme: &Theme) -> WidgetVisuals {
        WidgetVisuals {
            primary: Some(theme.primary),
            background: Some(theme.background),
            surface: Some(theme.surface),
            border: Some(theme.border),
            hover: Some(theme.hover),
            ..Default::default()
        }
    }
}
