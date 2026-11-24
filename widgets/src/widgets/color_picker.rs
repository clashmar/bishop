use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::{BishopContext, Color, KeyCode, Rect, WidgetId};

#[path = "color_picker_drawing.rs"]
mod drawing;

use drawing::PickerLayout;

/// The resolved color and edit phase produced by a color input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorInputResponse {
    /// The currently resolved color.
    pub color: Color,
    /// The active popup edit phase, if any.
    pub edit: ColorInputEdit,
}

/// The current phase of a popup color edit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorInputEdit {
    /// No popup transaction is active.
    None,
    /// The popup is open and the color is being previewed.
    Preview { initial: Color },
    /// The popup closed normally.
    Commit { initial: Color },
    /// The popup edit was canceled.
    Cancel { initial: Color },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PickerControl {
    SaturationValue,
    Hue,
    Opacity,
}

#[derive(Clone, Copy)]
struct PickerState {
    initial: Color,
    color: Color,
    hue: f32,
    saturation: f32,
    value: f32,
    alpha: f32,
    control: Option<PickerControl>,
    layout: PickerLayout,
    inline_edit_rect: Rect,
}

thread_local! {
    static PICKER_STATE: RefCell<HashMap<WidgetId, PickerState>> = RefCell::new(HashMap::new());
    static PENDING_PICKER_EDITS: RefCell<HashMap<WidgetId, ColorInputResponse>> = RefCell::new(HashMap::new());
    static PICKER_DISMISSED_THIS_FRAME: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn color_picker_open() -> bool {
    PICKER_STATE.with(|state| !state.borrow().is_empty())
        || PICKER_DISMISSED_THIS_FRAME.with(Cell::get)
}

pub(crate) fn color_picker_open_for(id: WidgetId) -> bool {
    PICKER_STATE.with(|state| state.borrow().contains_key(&id))
        || (PICKER_DISMISSED_THIS_FRAME.with(Cell::get)
            && PENDING_PICKER_EDITS.with(|edits| edits.borrow().contains_key(&id)))
}

pub(crate) fn color_picker_reset(id: WidgetId) {
    PICKER_STATE.with(|state| {
        state.borrow_mut().remove(&id);
    });
    PENDING_PICKER_EDITS.with(|edits| {
        edits.borrow_mut().remove(&id);
    });
}

pub(crate) fn update_color_picker_color(id: WidgetId, color: Color) {
    PICKER_STATE.with(|state| {
        if let Some(picker) = state.borrow_mut().get_mut(&id) {
            picker.set_color(color);
        }
    });
}

pub(crate) fn show_color_picker<C: BishopContext>(
    ctx: &mut C,
    id: WidgetId,
    swatch: Rect,
    color: Color,
    blocked: bool,
    opacity_enabled: bool,
    inline_edit_rect: Rect,
) -> ColorInputResponse {
    if let Some(response) = PENDING_PICKER_EDITS.with(|edits| edits.borrow_mut().remove(&id)) {
        return response;
    }

    if PICKER_DISMISSED_THIS_FRAME.with(Cell::get) {
        return ColorInputResponse {
            color,
            edit: ColorInputEdit::None,
        };
    }

    let is_open = color_picker_open_for(id);
    if !is_open
        && !blocked
        && !color_picker_open()
        && !crate::is_dropdown_open()
        && !crate::is_context_menu_open()
        && !crate::is_click_consumed()
        && ctx.is_mouse_button_pressed(crate::MouseButton::Left)
        && ctx.is_mouse_over(swatch)
    {
        crate::close_open_dropdowns();
        crate::close_open_context_menus();
        open_picker(ctx, id, swatch, color, opacity_enabled, inline_edit_rect);
        crate::consume_click();
        return ColorInputResponse {
            color,
            edit: ColorInputEdit::Preview { initial: color },
        };
    }

    if !is_open {
        return ColorInputResponse {
            color,
            edit: ColorInputEdit::None,
        };
    }

    let mut response = ColorInputResponse {
        color,
        edit: ColorInputEdit::None,
    };
    let mut close = None;
    PICKER_STATE.with(|state| {
        let mut states = state.borrow_mut();
        let Some(picker) = states.get_mut(&id) else {
            return;
        };
        picker.set_color(color);
        if blocked {
            response = ColorInputResponse {
                color: picker.color,
                edit: ColorInputEdit::Preview {
                    initial: picker.initial,
                },
            };
            return;
        }

        if ctx.is_key_pressed(KeyCode::Escape) {
            close = Some((picker.initial, picker.initial, true));
            crate::consume_click();
            return;
        }

        if ctx.is_mouse_button_pressed(crate::MouseButton::Left) {
            let pointer = ctx.mouse_position().into();
            if picker.layout.popup.contains(pointer) {
                picker.control = active_control(picker.layout, ctx.mouse_position().into());
                if let Some(control) = picker.control {
                    picker.update_from_pointer(control, ctx.mouse_position().into());
                }
                crate::consume_click();
            } else {
                if !picker.inline_edit_rect.contains(pointer) {
                    close = Some((picker.initial, picker.color, false));
                }
                crate::consume_click();
            }
        }

        if let Some(control) = picker.control {
            let pointer = ctx.mouse_position().into();
            if ctx.is_mouse_button_down(crate::MouseButton::Left) {
                picker.update_from_pointer(control, pointer);
                crate::consume_click();
            }
            if ctx.is_mouse_button_released(crate::MouseButton::Left) {
                picker.update_from_pointer(control, pointer);
                picker.control = None;
                crate::consume_click();
            }
        }

        response = ColorInputResponse {
            color: picker.color,
            edit: ColorInputEdit::Preview {
                initial: picker.initial,
            },
        };
    });

    if let Some((initial, color, canceled)) = close {
        color_picker_reset(id);
        response = ColorInputResponse {
            color,
            edit: if canceled {
                ColorInputEdit::Cancel { initial }
            } else {
                ColorInputEdit::Commit { initial }
            },
        };
    }

    response
}

fn open_picker<C: BishopContext>(
    ctx: &C,
    id: WidgetId,
    swatch: Rect,
    color: Color,
    opacity_enabled: bool,
    inline_edit_rect: Rect,
) {
    let layout = drawing::picker_layout(ctx, swatch, opacity_enabled);
    let (hue, saturation, value) = rgb_to_hsv(color);
    PICKER_STATE.with(|state| {
        let mut states = state.borrow_mut();
        states.clear();
        states.insert(
            id,
            PickerState {
                initial: color,
                color,
                hue,
                saturation,
                value,
                alpha: color.a,
                control: None,
                layout,
                inline_edit_rect,
            },
        );
    });
}

fn active_control(layout: PickerLayout, pointer: crate::Vec2) -> Option<PickerControl> {
    if layout.saturation_value.contains(pointer) {
        Some(PickerControl::SaturationValue)
    } else if layout.hue.contains(pointer) {
        Some(PickerControl::Hue)
    } else if layout.opacity.is_some_and(|rect| rect.contains(pointer)) {
        Some(PickerControl::Opacity)
    } else {
        None
    }
}

pub(crate) fn handle_color_picker_click_away<C: BishopContext>(ctx: &C) {
    PICKER_DISMISSED_THIS_FRAME.with(|dismissed| dismissed.set(false));
    if !ctx.is_mouse_button_pressed(crate::MouseButton::Left) {
        return;
    }

    let pointer = ctx.mouse_position().into();
    let click_is_inside_popup = PICKER_STATE.with(|state| {
        state
            .borrow()
            .values()
            .any(|picker| picker.layout.popup.contains(pointer))
    });
    if click_is_inside_popup {
        crate::consume_click();
        return;
    }

    let click_is_inside_inline_field = PICKER_STATE.with(|state| {
        state
            .borrow()
            .values()
            .any(|picker| picker.inline_edit_rect.contains(pointer))
    });
    if click_is_inside_inline_field {
        return;
    }

    let dismissed = PICKER_STATE.with(|state| {
        let mut states = state.borrow_mut();
        let id = states.iter().find_map(|(id, picker)| {
            (!picker.layout.popup.contains(pointer)
                && !picker.inline_edit_rect.contains(pointer))
                .then_some(*id)
        })?;
        states.remove(&id).map(|picker| {
            (
                id,
                ColorInputResponse {
                    color: picker.color,
                    edit: ColorInputEdit::Commit {
                        initial: picker.initial,
                    },
                },
            )
        })
    });

    if let Some((id, response)) = dismissed {
        PENDING_PICKER_EDITS.with(|edits| {
            edits.borrow_mut().insert(id, response);
        });
        PICKER_DISMISSED_THIS_FRAME.with(|dismissed| dismissed.set(true));
        crate::clear_all_input_focus();
        crate::consume_click();
    }
}

pub(crate) fn flush_color_picker<C: BishopContext>(ctx: &mut C) {
    PICKER_STATE.with(|state| {
        for picker in state.borrow().values() {
            drawing::draw_picker(ctx, picker);
        }
    });
}

impl PickerState {
    fn set_color(&mut self, color: Color) {
        self.color = color;
        (self.hue, self.saturation, self.value) = rgb_to_hsv(color);
        self.alpha = color.a.clamp(0.0, 1.0);
    }

    fn update_from_pointer(&mut self, control: PickerControl, pointer: crate::Vec2) {
        match control {
            PickerControl::SaturationValue => {
                self.saturation = ((pointer.x - self.layout.saturation_value.x)
                    / self.layout.saturation_value.w)
                    .clamp(0.0, 1.0);
                self.value = (1.0
                    - (pointer.y - self.layout.saturation_value.y)
                        / self.layout.saturation_value.h)
                    .clamp(0.0, 1.0);
            }
            PickerControl::Hue => {
                self.hue = ((pointer.y - self.layout.hue.y) / self.layout.hue.h)
                    .clamp(0.0, 1.0)
                    * 360.0;
            }
            PickerControl::Opacity => {
                if let Some(opacity) = self.layout.opacity {
                    self.alpha = ((pointer.x - opacity.x) / opacity.w).clamp(0.0, 1.0);
                }
            }
        }
        self.color = hsv_to_rgb(self.hue, self.saturation, self.value, self.alpha);
    }
}

fn rgb_to_hsv(color: Color) -> (f32, f32, f32) {
    let red = color.r.clamp(0.0, 1.0);
    let green = color.g.clamp(0.0, 1.0);
    let blue = color.b.clamp(0.0, 1.0);
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let delta = max - min;

    let hue = if delta == 0.0 {
        0.0
    } else if max == red {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if max == green {
        60.0 * ((blue - red) / delta + 2.0)
    } else {
        60.0 * ((red - green) / delta + 4.0)
    };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };

    (hue, saturation, max)
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32, alpha: f32) -> Color {
    let hue = hue.rem_euclid(360.0) / 60.0;
    let saturation = saturation.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);
    let alpha = alpha.clamp(0.0, 1.0);
    let chroma = value * saturation;
    let intermediate = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let offset = value - chroma;
    let (red, green, blue) = match hue as u32 {
        0 => (chroma, intermediate, 0.0),
        1 => (intermediate, chroma, 0.0),
        2 => (0.0, chroma, intermediate),
        3 => (0.0, intermediate, chroma),
        4 => (intermediate, 0.0, chroma),
        _ => (chroma, 0.0, intermediate),
    };

    Color::new(red + offset, green + offset, blue + offset, alpha)
}

#[cfg(test)]
#[path = "color_picker_tests.rs"]
mod tests;
