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

/// Draws a horizontal slider widget.
///
/// Returns the new value and a `SliderState` indicating the interaction phase.
pub fn gui_slider<C: BishopContext>(
    ctx: &mut C,
    id: WidgetId,
    rect: impl Into<Rect>,
    min: f32,
    max: f32,
    value: f32,
) -> (f32, SliderState) {
    let rect = rect.into();
    // (is_dragging, drag_offset, initial_value)
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

    ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, FIELD_BACKGROUND_COLOR);
    ctx.draw_rectangle(
        rect.x,
        track_y,
        rect.w,
        track_h,
        Color::new(0.2, 0.2, 0.2, 0.8),
    );
    ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2., OUTLINE_COLOR);

    let handle_col = if was_dragging && !is_dropdown_open() {
        Color::new(0.6, 0.6, 0.9, 1.0)
    } else {
        Color::new(0.4, 0.4, 0.8, 1.0)
    };
    ctx.draw_rectangle(handle_x, rect.y, handle_sz, rect.h, handle_col);
    ctx.draw_rectangle_lines(handle_x, rect.y, handle_sz, rect.h, 2., Color::WHITE);

    if is_dropdown_open() {
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
        // Drag just ended
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
