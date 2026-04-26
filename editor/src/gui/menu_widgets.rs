use crate::gui::gui_constants::*;
use crate::gui::modals::is_modal_open;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

const MENU_BUTTON_TARGET_SALT: u64 = 0x4D45_4E55_4254_4E31;
const MENU_ENTRY_TARGET_SALT: u64 = 0x4D45_4E55_454E_5452;

thread_local! {
    /// Holds the `WidgetId` of the dropdown that is currently open, if any.
    static CURRENT_OPEN: RefCell<Option<WidgetId>> = const { RefCell::new(None) };
}

/// Draws a menu-styled dropdown button and returns the selected option, if any.
pub(crate) fn menu_dropdown<T: Clone + PartialEq + Display>(
    ctx: &mut WgpuContext,
    id: WidgetId,
    rect: Rect,
    label: &str,
    options: &[T],
    to_string: impl Fn(&T) -> String,
    shortcut: impl Fn(&T) -> Option<&str>,
) -> Option<T> {
    const W_PADDING: f32 = 8.0;
    const DROPDOWN_Y_OFFSET: f32 = 7.5;

    let mut state = dropdown_state::get(id);

    let mouse_pos: Vec2 = ctx.mouse_position().into();
    let hovered = rect.contains(mouse_pos);

    if hovered {
        let any_open = DROPDOWN_OPEN.with(|f| *f.borrow());
        if any_open {
            CURRENT_OPEN.with(|c| {
                let current_id = *c.borrow();
                if current_id != Some(id) {
                    if let Some(prev_id) = current_id {
                        let mut prev_state = dropdown_state::get(prev_id);
                        prev_state.open = false;
                        dropdown_state::set(prev_id, prev_state);
                    }

                    state.open = true;
                    *c.borrow_mut() = Some(id);
                }
            });
        }
    }

    let button_clicked = menu_button(ctx, rect, label, state.open);

    if button_clicked {
        state.open = !state.open;
        if state.open {
            CURRENT_OPEN.with(|c| *c.borrow_mut() = Some(id));
        } else {
            CURRENT_OPEN.with(|c| *c.borrow_mut() = None);
        }
    }

    let list_is_open = state.open;

    DROPDOWN_OPEN.with(|f| {
        let was = *f.borrow();
        *f.borrow_mut() = was || list_is_open;
    });

    let mut max_opt_width = 0.0_f32;
    for opt in options {
        let label_w = measure_text(ctx, &to_string(opt), DEFAULT_FONT_SIZE_16).width;
        let shortcut_w = shortcut(opt)
            .map(|s| measure_text(ctx, s, DEFAULT_FONT_SIZE_16).width + SPACING)
            .unwrap_or(0.0);
        let total_w = label_w + shortcut_w;
        if total_w > max_opt_width {
            max_opt_width = total_w;
        }
    }

    let list_width = rect.w.max(max_opt_width + 2.0 * W_PADDING);
    let total_height = rect.h * options.len() as f32;

    let list_rect = Rect::new(
        rect.x,
        rect.y + rect.h + DROPDOWN_Y_OFFSET,
        list_width,
        total_height,
    );

    if list_is_open {
        state.rect = list_rect;
    }

    if list_is_open {
        let mouse_pos = ctx.mouse_position().into();

        ctx.draw_rectangle(
            list_rect.x,
            list_rect.y,
            list_rect.w,
            list_rect.h,
            PANEL_COLOR,
        );

        for (i, opt) in options.iter().enumerate() {
            let entry_y = list_rect.y + i as f32 * rect.h;
            let entry_rect = Rect::new(list_rect.x, entry_y, list_rect.w, rect.h);

            let hovered = entry_rect.contains(mouse_pos);
            if activate_on_release(
                MouseButton::Left,
                menu_entry_click_target(id, i),
                hovered,
                true,
                ctx.is_mouse_button_pressed(MouseButton::Left),
                ctx.is_mouse_button_released(MouseButton::Left),
            ) {
                state.open = false;
                dropdown_state::set(id, state);
                update_global_dropdown_flag();
                return Some(opt.clone());
            }

            if hovered {
                ctx.draw_rectangle(
                    entry_rect.x,
                    entry_rect.y,
                    entry_rect.w,
                    entry_rect.h,
                    Color::new(0.2, 0.2, 0.2, 0.9),
                );
            }

            ctx.draw_text(
                &to_string(opt),
                entry_rect.x + 5.0,
                entry_rect.y + entry_rect.h * 0.7,
                DEFAULT_FONT_SIZE_16,
                Color::BLACK,
            );

            if let Some(shortcut) = shortcut(opt) {
                let sc_width = measure_text(ctx, shortcut, DEFAULT_FONT_SIZE_16).width;
                let sc_x = entry_rect.x + entry_rect.w - sc_width - 5.0;
                ctx.draw_text(
                    shortcut,
                    sc_x,
                    entry_rect.y + entry_rect.h * 0.7,
                    DEFAULT_FONT_SIZE_16,
                    Color::WHITE,
                );
            }

            ctx.draw_rectangle_lines(
                list_rect.x,
                list_rect.y,
                list_rect.w,
                list_rect.h,
                2.0,
                Color::BLACK,
            );
        }
    }

    let mouse_pos = ctx.mouse_position().into();
    if ctx.is_mouse_button_pressed(MouseButton::Left)
        && !rect.contains(mouse_pos)
        && !(state.open && state.rect.contains(mouse_pos))
    {
        state.open = false;
        CURRENT_OPEN.with(|c| *c.borrow_mut() = None);
    }

    dropdown_state::set(id, state);
    update_global_dropdown_flag();
    None
}

/// Draws a menu-styled button and returns `true` when clicked.
pub fn menu_button(ctx: &mut WgpuContext, rect: Rect, label: &str, is_dropdown_open: bool) -> bool {
    let txt_dims = ctx.measure_text(label, HEADER_FONT_SIZE_20);
    let (txt_x, txt_y) = menu_button_text_position(rect, txt_dims);

    let mouse = ctx.mouse_position();
    let hovered = rect.contains(vec2(mouse.0, mouse.1));

    if (hovered || is_dropdown_open)
        && !is_modal_open()
        && !is_context_menu_open()
        && !ctx.is_mouse_button_down(MouseButton::Left)
    {
        ctx.draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
    }

    ctx.draw_text(label, txt_x, txt_y, HEADER_FONT_SIZE_20, Color::BLACK);

    activate_on_release(
        MouseButton::Left,
        menu_button_click_target(rect, label),
        hovered,
        !is_modal_open() && !is_context_menu_open(),
        ctx.is_mouse_button_pressed(MouseButton::Left),
        ctx.is_mouse_button_released(MouseButton::Left),
    )
}

/// Returns the centered text position used by menu-styled buttons.
pub(crate) fn menu_button_text_position(rect: Rect, txt_dims: TextDimensions) -> (f32, f32) {
    let txt_x = rect.x + (rect.w - txt_dims.width) / 2.0;
    let txt_y = rect.y + (rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y - 1.0;
    (txt_x, txt_y)
}

fn menu_button_click_target(rect: Rect, label: &str) -> ClickTargetId {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    rect.x.to_bits().hash(&mut hasher);
    rect.y.to_bits().hash(&mut hasher);
    rect.w.to_bits().hash(&mut hasher);
    rect.h.to_bits().hash(&mut hasher);
    MENU_BUTTON_TARGET_SALT.hash(&mut hasher);
    ClickTargetId(hasher.finish())
}

fn menu_entry_click_target(id: WidgetId, index: usize) -> ClickTargetId {
    ClickTargetId(((id.0 as u64) << 32) ^ index as u64 ^ MENU_ENTRY_TARGET_SALT)
}
