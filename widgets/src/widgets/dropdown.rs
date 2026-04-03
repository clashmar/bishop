use crate::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;

/// Offset added to a dropdown's WidgetId to derive its filter TextInput's WidgetId.
const FILTER_ID_OFFSET: usize = usize::MAX / 2 + 1;
const ENTRY_ID_SALT: u64 = 0x4452_4F50_444F_574E;
const FILTERED_ENTRY_ID_SALT: u64 = 0x0046_494C_5445_5244;

/// Data for deferred dropdown rendering.
struct DeferredDropdownRender {
    list_rect: Rect,
    row_height: f32,
    scroll_offset: f32,
    labels: Vec<String>,
    option_count: usize,
}

thread_local! {
    static DEFERRED_DROPDOWN_RENDERS: RefCell<Vec<DeferredDropdownRender>> =
        const { RefCell::new(Vec::new()) };
    /// Per-dropdown filter text, keyed by the dropdown's WidgetId.
    static DROPDOWN_FILTER_STATE: RefCell<HashMap<WidgetId, String>> =
        RefCell::new(HashMap::new());
}

fn get_filter(id: WidgetId) -> String {
    DROPDOWN_FILTER_STATE.with(|s| s.borrow().get(&id).cloned().unwrap_or_default())
}

fn set_filter(id: WidgetId, filter: String) {
    DROPDOWN_FILTER_STATE.with(|s| {
        s.borrow_mut().insert(id, filter);
    });
}

fn clear_filter(id: WidgetId) {
    DROPDOWN_FILTER_STATE.with(|s| {
        s.borrow_mut().remove(&id);
    });
}

fn dropdown_entry_click_target(id: WidgetId, index: usize, salt: u64) -> ClickTargetId {
    ClickTargetId(((id.0 as u64) << 32) ^ index as u64 ^ salt)
}

/// Flushes all deferred dropdown list renders.
///
/// Call this after drawing all modules to ensure dropdown lists render on top.
pub fn flush_dropdown_lists<C: BishopContext>(ctx: &mut C) {
    DEFERRED_DROPDOWN_RENDERS.with(|renders| {
        for render in renders.borrow_mut().drain(..) {
            render_dropdown_list(
                ctx,
                render.list_rect,
                render.row_height,
                render.scroll_offset,
                &render.labels,
                render.option_count,
            );
        }
    });
}

/// The visual style of a dropdown trigger button.
#[derive(Clone, Copy)]
pub enum DropDownStyle {
    /// Standard dropdown with background and border.
    Default,
    /// Minimal dropdown with no background.
    Plain,
}

/// Horizontal alignment for the dropdown list relative to its trigger button.
#[derive(Clone, Copy)]
pub enum DropDownAlignment {
    /// Align the dropdown list's left edge with the trigger button's left edge.
    Left,
    /// Align the dropdown list's right edge with the trigger button's right edge.
    Right,
}

/// A dropdown widget using the builder pattern.
pub struct Dropdown<'a, T> {
    id: WidgetId,
    rect: Rect,
    label: &'a str,
    options: &'a [T],
    to_string: Box<dyn Fn(&T) -> String + 'a>,
    style: DropDownStyle,
    text_color: Color,
    label_font_size: f32,
    y_offset: f32,
    blocked: bool,
    fixed_width: bool,
    filterable: bool,
    alignment: DropDownAlignment,
    list_width: Option<f32>,
    truncate_trigger: bool,
}

impl<'a, T: Clone + PartialEq + Display + 'static> Dropdown<'a, T> {
    /// Creates a new dropdown with the given parameters.
    pub fn new(
        id: WidgetId,
        rect: impl Into<Rect>,
        label: &'a str,
        options: &'a [T],
        to_string: impl Fn(&T) -> String + 'a,
    ) -> Self {
        Self {
            id,
            rect: rect.into(),
            label,
            options,
            to_string: Box::new(to_string),
            style: DropDownStyle::Default,
            text_color: Color::WHITE,
            label_font_size: FIELD_TEXT_SIZE_16,
            y_offset: 0.0,
            blocked: false,
            fixed_width: false,
            filterable: false,
            alignment: DropDownAlignment::Left,
            list_width: None,
            truncate_trigger: false,
        }
    }

    /// Sets the dropdown to use the plain style (no background).
    pub fn plain(mut self) -> Self {
        self.style = DropDownStyle::Plain;
        self
    }

    /// Renders the trigger button using the menu bar style: transparent at rest,
    /// dark overlay on hover or when open, black text at 20pt.
    pub fn menu_style(mut self) -> Self {
        self.style = DropDownStyle::Plain;
        self.text_color = Color::BLACK;
        self.label_font_size = HEADER_FONT_SIZE_20;
        self
    }

    /// Sets the text color.
    pub fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.text_color = color.into();
        self
    }

    /// Sets the font size for the trigger button label.
    pub fn font_size(mut self, size: f32) -> Self {
        self.label_font_size = size;
        self
    }

    /// Sets the vertical offset for the dropdown list.
    pub fn y_offset(mut self, offset: f32) -> Self {
        self.y_offset = offset;
        self
    }

    /// Sets whether the dropdown is blocked from interaction.
    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    /// Clamps the dropdown list width to match the parent button.
    pub fn fixed_width(mut self) -> Self {
        self.fixed_width = true;
        self
    }

    /// Shows a filter TextInput at the top of the dropdown list for case-insensitive search.
    /// The list renders inline (non-deferred) when filtering is active.
    pub fn filterable(mut self) -> Self {
        self.filterable = true;
        self
    }

    /// Sets an explicit width for the dropdown list without changing trigger button sizing.
    pub fn list_width(mut self, width: f32) -> Self {
        self.list_width = Some(width.max(0.0));
        self
    }

    /// Truncates the trigger button label to fit within the trigger width.
    pub fn truncate_trigger_text(mut self) -> Self {
        self.truncate_trigger = true;
        self
    }

    /// Aligns the dropdown list to the trigger button's right edge.
    pub fn right_aligned(mut self) -> Self {
        self.alignment = DropDownAlignment::Right;
        self
    }

    /// Draws the dropdown and returns the selected option if one was clicked.
    pub fn show<C: BishopContext>(self, ctx: &mut C) -> Option<T> {
        const MAX_VISIBLE_ROWS: usize = 8;
        const SCROLL_SPEED: f32 = 5.0;
        const W_PADDING: f32 = 8.0;
        const SCROLLBAR_WIDTH: f32 = 6.0;

        let mut state = dropdown_state::get(self.id);

        let prev_state = state.open;
        state.open = false;
        dropdown_state::set(self.id, state);
        update_global_dropdown_flag();

        let truncated;
        let display_label = if self.fixed_width || self.truncate_trigger {
            truncated = truncate_to_width(
                ctx,
                self.label,
                self.rect.w - WIDGET_PADDING,
                DEFAULT_FONT_SIZE_16,
            );
            &truncated
        } else {
            self.label
        };

        let button_clicked = match self.style {
            DropDownStyle::Default => {
                Button::new(self.rect, display_label)
                    .blocked(self.blocked)
                    .show(ctx)
                    && !self.blocked
            }
            DropDownStyle::Plain => {
                Button::new(self.rect, display_label)
                    .plain()
                    .text_color(self.text_color)
                    .font_size(self.label_font_size)
                    .blocked(self.blocked)
                    .show(ctx)
                    && !self.blocked
            }
        };

        state.open = prev_state;
        dropdown_state::set(self.id, state);
        update_global_dropdown_flag();

        if button_clicked {
            consume_click();
            state.open = !state.open;
            if self.filterable && !state.open {
                clear_filter(self.id);
            }
        }

        let list_is_open = state.open;

        DROPDOWN_OPEN.with(|r| {
            let was = *r.borrow();
            *r.borrow_mut() = was || list_is_open;
        });

        let mut max_opt_width = 0.0_f32;
        for opt in self.options.iter() {
            let txt = (self.to_string)(opt);
            let width = measure_text_ui(ctx, &txt, DEFAULT_FONT_SIZE_16).width;
            if width > max_opt_width {
                max_opt_width = width;
            }
        }

        let list_width = if let Some(list_width) = self.list_width {
            list_width
        } else if self.fixed_width {
            self.rect.w
        } else {
            self.rect
                .w
                .max(max_opt_width + 2.0 * W_PADDING + SCROLLBAR_WIDTH)
        };

        let mut result: Option<T> = None;

        if list_is_open {
            if self.filterable {
                result = self.show_filterable_list(ctx, &mut state, list_width);
            } else {
                let visible_rows = MAX_VISIBLE_ROWS.min(self.options.len());
                let list_h = self.rect.h * visible_rows as f32;
                let drop_down_y = self.rect.y + self.rect.h + self.y_offset;
                let drop_up_y = self.rect.y - list_h - self.y_offset;
                let drops_below_screen = drop_down_y + list_h > ctx.screen_height();
                let list_y = if drops_below_screen && drop_up_y >= 0.0 {
                    drop_up_y
                } else {
                    drop_down_y
                };
                let list_x = self.list_x(list_width);
                let list_rect = Rect::new(list_x, list_y, list_width, list_h);

                state.rect = list_rect;

                let total_height = self.rect.h * self.options.len() as f32;
                let max_offset = (total_height - list_rect.h).max(0.0);

                let mouse_pos = ctx.mouse_position();
                let mouse_vec = Vec2::new(mouse_pos.0, mouse_pos.1);

                if list_rect.contains(mouse_vec) {
                    let (_, wheel_y) = ctx.mouse_wheel();
                    if wheel_y != 0.0 {
                        let delta = wheel_y * SCROLL_SPEED;
                        state.scroll_offset = (state.scroll_offset - delta).clamp(0.0, max_offset);
                    }
                }

                let mut hovered_entry = false;
                for (i, opt) in self.options.iter().enumerate() {
                    let entry_y = list_rect.y + i as f32 * self.rect.h;
                    let draw_y = entry_y - state.scroll_offset;

                    if draw_y + self.rect.h < list_rect.y + self.rect.h
                        || draw_y > list_rect.y + list_rect.h - self.rect.h
                    {
                        continue;
                    }

                    let entry_rect = Rect::new(list_rect.x, draw_y, list_rect.w, self.rect.h);

                    let hovered = entry_rect.contains(mouse_vec);
                    hovered_entry |= hovered;
                    if activate_on_release(
                        MouseButton::Left,
                        dropdown_entry_click_target(self.id, i, ENTRY_ID_SALT),
                        hovered,
                        true,
                        ctx.is_mouse_button_pressed(MouseButton::Left),
                        ctx.is_mouse_button_released(MouseButton::Left),
                    ) {
                        state.open = false;
                        dropdown_state::set(self.id, state);
                        update_global_dropdown_flag();
                        result = Some(opt.clone());
                        break;
                    }
                }

                if list_rect.contains(mouse_vec)
                    && ctx.is_mouse_button_pressed(MouseButton::Left)
                    && !hovered_entry
                    && !is_click_consumed()
                {
                    consume_click();
                }

                let scroll_offset = state.scroll_offset;
                let row_height = self.rect.h;
                let labels: Vec<String> =
                    self.options.iter().map(|o| (self.to_string)(o)).collect();
                let option_count = self.options.len();

                DEFERRED_DROPDOWN_RENDERS.with(|renders| {
                    renders.borrow_mut().push(DeferredDropdownRender {
                        list_rect,
                        row_height,
                        scroll_offset,
                        labels,
                        option_count,
                    });
                });
            }
        }

        let mouse_pos = ctx.mouse_position();
        let mouse_vec = Vec2::new(mouse_pos.0, mouse_pos.1);
        if ctx.is_mouse_button_pressed(MouseButton::Left)
            && !self.rect.contains(mouse_vec)
            && !(state.open && state.rect.contains(mouse_vec))
        {
            state.open = false;
            if self.filterable {
                clear_filter(self.id);
            }
        }

        dropdown_state::set(self.id, state);
        update_global_dropdown_flag();
        result
    }

    /// Draws the filterable list inline (non-deferred) and returns a selected option if any.
    fn show_filterable_list<C: BishopContext>(
        &self,
        ctx: &mut C,
        state: &mut dropdown_state::DropState,
        list_width: f32,
    ) -> Option<T> {
        const MAX_VISIBLE_ROWS: usize = 8;
        const SCROLL_SPEED: f32 = 5.0;

        let prev_filter = get_filter(self.id);
        let filter_lower = prev_filter.to_lowercase();

        let filtered: Vec<&T> = if filter_lower.is_empty() {
            self.options.iter().collect()
        } else {
            self.options
                .iter()
                .filter(|opt| (self.to_string)(opt).to_lowercase().contains(&filter_lower))
                .collect()
        };

        let visible_rows = MAX_VISIBLE_ROWS.min(filtered.len());
        let row_h = self.rect.h;
        let filter_h = row_h;
        let entries_h = row_h * visible_rows as f32;
        let popup_h = filter_h + entries_h;

        let drop_down_y = self.rect.y + self.rect.h + self.y_offset;
        let drop_up_y = self.rect.y - popup_h - self.y_offset;
        let drops_below = drop_down_y + popup_h > ctx.screen_height();
        let popup_y = if drops_below && drop_up_y >= 0.0 {
            drop_up_y
        } else {
            drop_down_y
        };

        let popup_x = self.list_x(list_width);
        let popup_rect = Rect::new(popup_x, popup_y, list_width, popup_h);
        state.rect = popup_rect;

        // Background
        ctx.draw_rectangle(
            popup_rect.x,
            popup_rect.y,
            popup_rect.w,
            popup_rect.h,
            FIELD_BACKGROUND_COLOR,
        );

        // Filter TextInput
        let filter_rect = Rect::new(popup_rect.x, popup_y, list_width, filter_h);
        let filter_id = WidgetId(self.id.0.wrapping_add(FILTER_ID_OFFSET));
        let (new_filter, _) = TextInput::new(filter_id, filter_rect, &prev_filter)
            .in_dropdown()
            .live()
            .show(ctx);

        // Reset scroll when filter changes so the user is never stranded past new results
        if new_filter != prev_filter {
            state.scroll_offset = 0.0;
        }
        set_filter(self.id, new_filter);

        // Scroll offset
        let total_entries_h = row_h * filtered.len() as f32;
        let max_offset = (total_entries_h - entries_h).max(0.0);

        let entries_y = popup_y + filter_h;
        let mouse_pos: Vec2 = ctx.mouse_position().into();
        let entries_rect = Rect::new(popup_rect.x, entries_y, list_width, entries_h);

        if entries_rect.contains(mouse_pos) {
            let (_, wheel_y) = ctx.mouse_wheel();
            if wheel_y != 0.0 {
                state.scroll_offset =
                    (state.scroll_offset - wheel_y * SCROLL_SPEED).clamp(0.0, max_offset);
            }
        }

        // Entries
        ctx.push_clip_rect(entries_rect);
        let mut result = None;

        for (i, opt) in filtered.iter().enumerate() {
            let draw_y = entries_y + i as f32 * row_h - state.scroll_offset;

            if draw_y + row_h <= entries_y || draw_y >= entries_y + entries_h {
                continue;
            }

            let entry_rect = Rect::new(popup_rect.x, draw_y, list_width, row_h);
            let hovered = entry_rect.contains(mouse_pos);

            if hovered {
                ctx.draw_rectangle(
                    entry_rect.x,
                    entry_rect.y,
                    entry_rect.w,
                    entry_rect.h,
                    Color::new(0.2, 0.2, 0.2, 0.9),
                );
            }

            draw_text_clipped(
                ctx,
                &(self.to_string)(opt),
                entry_rect,
                0.0,
                DEFAULT_FONT_SIZE_16,
                FIELD_TEXT_COLOR,
            );

            if activate_on_release(
                MouseButton::Left,
                dropdown_entry_click_target(self.id, i, FILTERED_ENTRY_ID_SALT),
                hovered,
                true,
                ctx.is_mouse_button_pressed(MouseButton::Left),
                ctx.is_mouse_button_released(MouseButton::Left),
            ) {
                state.open = false;
                clear_filter(self.id);
                dropdown_state::set(self.id, *state);
                update_global_dropdown_flag();
                result = Some((*opt).clone());
                break;
            }
        }

        ctx.pop_clip_rect();

        // Scrollbar when content overflows visible area
        if total_entries_h > entries_h {
            let thumb_h = (entries_h / total_entries_h) * entries_h;
            let thumb_y = entries_y + (state.scroll_offset / max_offset) * (entries_h - thumb_h);

            ctx.draw_rectangle(
                popup_rect.x + popup_rect.w - 6.,
                entries_y,
                6.,
                entries_h,
                Color::new(0.2, 0.2, 0.2, 0.5),
            );
            ctx.draw_rectangle(
                popup_rect.x + popup_rect.w - 6.,
                thumb_y,
                6.,
                thumb_h,
                Color::new(0.6, 0.6, 0.6, 0.9),
            );
        }

        ctx.draw_rectangle_lines(
            popup_rect.x,
            popup_rect.y,
            popup_rect.w,
            popup_rect.h,
            2.,
            OUTLINE_COLOR,
        );

        result
    }

    fn list_x(&self, list_width: f32) -> f32 {
        match self.alignment {
            DropDownAlignment::Left => self.rect.x,
            DropDownAlignment::Right => self.rect.x + self.rect.w - list_width,
        }
    }
}

/// Renders the dropdown list (called from deferred queue).
fn render_dropdown_list<C: BishopContext>(
    ctx: &mut C,
    list_rect: Rect,
    row_height: f32,
    scroll_offset: f32,
    labels: &[String],
    option_count: usize,
) {
    ctx.draw_rectangle(
        list_rect.x,
        list_rect.y,
        list_rect.w,
        list_rect.h,
        FIELD_BACKGROUND_COLOR,
    );

    let mouse_pos = ctx.mouse_position();
    let mouse_vec = Vec2::new(mouse_pos.0, mouse_pos.1);

    for (i, label) in labels.iter().enumerate() {
        let entry_y = list_rect.y + i as f32 * row_height;
        let draw_y = entry_y - scroll_offset;

        if draw_y + row_height < list_rect.y + row_height
            || draw_y > list_rect.y + list_rect.h - row_height
        {
            continue;
        }

        let entry_rect = Rect::new(list_rect.x, draw_y, list_rect.w, row_height);

        let hovered = entry_rect.contains(mouse_vec);
        if hovered {
            ctx.draw_rectangle(
                entry_rect.x,
                entry_rect.y,
                entry_rect.w,
                entry_rect.h,
                Color::new(0.2, 0.2, 0.2, 0.9),
            );
        }

        draw_text_clipped(
            ctx,
            label,
            entry_rect,
            0.0,
            DEFAULT_FONT_SIZE_16,
            FIELD_TEXT_COLOR,
        );
    }

    let total_height = row_height * option_count as f32;
    if total_height > list_rect.h {
        let thumb_h = (list_rect.h / total_height) * list_rect.h;
        let thumb_y =
            list_rect.y + (scroll_offset / (total_height - list_rect.h)) * (list_rect.h - thumb_h);

        ctx.draw_rectangle(
            list_rect.x + list_rect.w - 6.,
            list_rect.y,
            6.,
            list_rect.h,
            Color::new(0.2, 0.2, 0.2, 0.5),
        );
        ctx.draw_rectangle(
            list_rect.x + list_rect.w - 6.,
            thumb_y,
            6.,
            thumb_h,
            Color::new(0.6, 0.6, 0.6, 0.9),
        );
    }

    ctx.draw_rectangle_lines(
        list_rect.x,
        list_rect.y,
        list_rect.w,
        list_rect.h,
        2.,
        OUTLINE_COLOR,
    );
}

/// Internal module for managing dropdown state.
pub mod dropdown_state {
    use crate::{Rect, WidgetId};
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        pub static STATE: RefCell<HashMap<WidgetId, DropState>> =
            RefCell::new(HashMap::new());
    }

    /// The state of a dropdown widget.
    #[derive(Clone, Copy)]
    pub struct DropState {
        pub open: bool,
        pub rect: Rect,
        pub scroll_offset: f32,
    }

    impl Default for DropState {
        fn default() -> Self {
            Self {
                open: false,
                rect: Rect::default(),
                scroll_offset: 0.,
            }
        }
    }

    /// Gets the state for a dropdown by id.
    pub fn get(key: WidgetId) -> DropState {
        STATE.with(|s| *s.borrow().get(&key).unwrap_or(&DropState::default()))
    }

    /// Sets the state for a dropdown by id.
    pub fn set(key: WidgetId, value: DropState) {
        STATE.with(|s| {
            s.borrow_mut().insert(key, value);
        });
    }
}

/// Updates the global flag indicating whether any dropdown is open.
pub fn update_global_dropdown_flag() {
    dropdown_state::STATE.with(|s| {
        let any = s.borrow().values().any(|st| st.open);
        DROPDOWN_OPEN.with(|f| *f.borrow_mut() = any);
    });
}

/// Returns true if the mouse is over any open dropdown list.
pub fn is_mouse_over_dropdown_list<C: BishopContext>(ctx: &C) -> bool {
    dropdown_state::STATE.with(|s| {
        let mouse_pos = ctx.mouse_position();
        let mouse_vec = Vec2::new(mouse_pos.0, mouse_pos.1);
        s.borrow()
            .values()
            .any(|st| st.open && st.rect.contains(mouse_vec))
    })
}
