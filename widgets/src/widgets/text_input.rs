use crate::clipboard::{clipboard_get_text, clipboard_set_text};
use crate::*;

/// A text input widget using the builder pattern.
pub struct TextInput<'a> {
    id: WidgetId,
    rect: Rect,
    current: &'a str,
    blocked: bool,
    start_focused: bool,
    max_len: Option<usize>,
    char_filter: Option<fn(char) -> Option<char>>,
    live: bool,
    bypass_dropdown: bool,
}

impl<'a> TextInput<'a> {
    /// Creates a new text input widget with the given id, rect, and current value.
    pub fn new(id: WidgetId, rect: impl Into<Rect>, current: &'a str) -> Self {
        Self {
            id,
            rect: rect.into(),
            current,
            blocked: false,
            start_focused: false,
            max_len: None,
            char_filter: None,
            live: false,
            bypass_dropdown: false,
        }
    }

    /// Sets whether the input is blocked from interaction.
    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    /// Sets whether the input should start focused.
    pub fn focused(mut self, focused: bool) -> Self {
        self.start_focused = focused;
        self
    }

    /// Sets the maximum character length for the input.
    pub fn max_len(mut self, max_len: usize) -> Self {
        self.max_len = Some(max_len);
        self
    }

    /// Sets a character filter that transforms or rejects input characters.
    /// Return `Some(char)` to accept (possibly transformed), or `None` to reject.
    pub fn char_filter(mut self, filter: fn(char) -> Option<char>) -> Self {
        self.char_filter = Some(filter);
        self
    }

    /// Returns the current typed text every frame without requiring Enter to confirm.
    /// Use for inputs that should filter or react in real time.
    pub fn live(mut self) -> Self {
        self.live = true;
        self
    }

    /// Allows this input to receive keyboard events even when a dropdown list is open.
    /// Use for TextInputs that are embedded inside a dropdown list.
    pub fn in_dropdown(mut self) -> Self {
        self.bypass_dropdown = true;
        self
    }

    /// Draws the widget and returns the current text and focus state.
    pub fn show<C: BishopContext>(self, ctx: &mut C) -> (String, bool) {
        tab_registry_add(self.id, self.rect, true);

        let mut just_gained_focus = false;
        let mut confirmed = false;

        let pending_focus = consume_pending_focus(self.id);

        let (
            mut text,
            mut cursor_char,
            mut focused,
            mut selection_anchor,
            mut last_key_time,
            mut repeat_key,
            mut repeat_started,
            mut dragging,
            mut scroll_offset_x,
        ) = INPUT_TEXT_STATE.with(|s| {
            let mut map = s.borrow_mut();

            if let Some(state) = map.get(&self.id) {
                let should_focus = pending_focus;
                let f = if should_focus { true } else { state.focused };
                just_gained_focus = should_focus && !state.focused;
                let cc = if should_focus && just_gained_focus {
                    state.text.chars().count()
                } else {
                    state.cursor_char
                };
                (
                    state.text.clone(),
                    cc,
                    f,
                    state.selection_anchor,
                    state.last_key_time,
                    state.repeat_key,
                    state.repeat_started,
                    state.dragging,
                    state.scroll_offset_x,
                )
            } else {
                let t = self.current.to_string();
                let cc = t.chars().count();
                just_gained_focus = self.start_focused || pending_focus;
                map.insert(self.id, TextInputState::new(t.clone()));
                (
                    t,
                    cc,
                    self.start_focused || pending_focus,
                    None,
                    0.0,
                    None,
                    false,
                    false,
                    0.0,
                )
            }
        });

        if !focused && text != self.current {
            text = self.current.to_string();
            cursor_char = text.chars().count();
            scroll_offset_x = 0.0;
        }

        ctx.draw_rectangle(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            FIELD_BACKGROUND_COLOR,
        );
        ctx.draw_rectangle_lines(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            2.,
            Color::WHITE,
        );

        let text_area_x = self.rect.x + WIDGET_PADDING / 2.;

        if let Some((start, end)) = selection_range(cursor_char, selection_anchor) {
            let start_byte = byte_offset(&text, start);
            let end_byte = byte_offset(&text, end);
            let sel_start_x = text_area_x
                + measure_text_ui(ctx, &text[..start_byte], DEFAULT_FONT_SIZE_16).width
                - scroll_offset_x;
            let sel_end_x = text_area_x
                + measure_text_ui(ctx, &text[..end_byte], DEFAULT_FONT_SIZE_16).width
                - scroll_offset_x;

            let clipped_start = sel_start_x.max(text_area_x);
            let clipped_end = sel_end_x.min(self.rect.x + self.rect.w - WIDGET_PADDING / 2.);

            if clipped_end > clipped_start {
                ctx.draw_rectangle(
                    clipped_start,
                    self.rect.y + self.rect.h * 0.2,
                    clipped_end - clipped_start,
                    self.rect.h * 0.6,
                    Color::new(0.3, 0.5, 0.8, 0.5),
                );
            }
        }

        let display = if text.is_empty() {
            PLACEHOLDER_TEXT
        } else {
            &text
        };
        draw_text_clipped(
            ctx,
            display,
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            scroll_offset_x,
            DEFAULT_FONT_SIZE_16,
            FIELD_TEXT_COLOR,
        );

        let mouse = ctx.mouse_position();
        let mouse_over = self.rect.contains(Vec2::new(mouse.0, mouse.1));

        if ctx.is_mouse_button_pressed(MouseButton::Left) && !is_click_consumed() {
            if !focused && mouse_over {
                just_gained_focus = true;
            }
            focused = mouse_over && !self.blocked;

            if !focused {
                INPUT_FOCUSED.with(|f| *f.borrow_mut() = false);
                selection_anchor = None;
            }

            if focused && mouse_over {
                if just_gained_focus {
                    clear_all_input_focus();
                }
                let click_pos = char_index_from_x(
                    ctx,
                    &text,
                    mouse.0,
                    self.rect.x,
                    DEFAULT_FONT_SIZE_16,
                    scroll_offset_x,
                );
                cursor_char = click_pos;
                selection_anchor = Some(click_pos);
                dragging = true;
            }
        }

        if dragging && ctx.is_mouse_button_down(MouseButton::Left) {
            let drag_pos = char_index_from_x(
                ctx,
                &text,
                mouse.0,
                self.rect.x,
                DEFAULT_FONT_SIZE_16,
                scroll_offset_x,
            );
            cursor_char = drag_pos;
        }

        if ctx.is_mouse_button_released(MouseButton::Left) && dragging {
            if selection_anchor == Some(cursor_char) {
                selection_anchor = None;
            }
            dragging = false;
        }

        if just_gained_focus {
            // Clear any pending character input
            let _ = ctx.chars_pressed();
        }

        if is_dropdown_open() && !self.bypass_dropdown {
            return (text, false);
        }

        if focused {
            INPUT_FOCUSED.with(|f| *f.borrow_mut() = true);
            let now = ctx.get_time();
            let shift_held =
                ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
            let ctrl_held = ctx.is_key_down(KeyCode::LeftControl)
                || ctx.is_key_down(KeyCode::RightControl)
                || ctx.is_key_down(KeyCode::LeftSuper)
                || ctx.is_key_down(KeyCode::RightSuper);

            if ctrl_held && ctx.is_key_pressed(KeyCode::A) {
                selection_anchor = Some(0);
                cursor_char = text.chars().count();
            }

            if ctrl_held
                && ctx.is_key_pressed(KeyCode::C)
                && let Some(selected) = get_selected_text(&text, cursor_char, selection_anchor)
            {
                clipboard_set_text(&selected);
            }

            if ctrl_held
                && ctx.is_key_pressed(KeyCode::V)
                && let Some(clipboard_text) = clipboard_get_text()
            {
                if selection_anchor.is_some() {
                    cursor_char = delete_selection(&mut text, cursor_char, selection_anchor);
                    selection_anchor = None;
                }

                let filtered: String = if let Some(filter) = self.char_filter {
                    clipboard_text.chars().filter_map(filter).collect()
                } else {
                    clipboard_text
                        .chars()
                        .filter(|c| c.is_ascii_graphic() || *c == ' ')
                        .collect()
                };

                let cur_len = text.chars().count();
                let available = self
                    .max_len
                    .map_or(usize::MAX, |limit| limit.saturating_sub(cur_len));
                let to_insert: String = filtered.chars().take(available).collect();

                let pos = byte_offset(&text, cursor_char);
                text.insert_str(pos, &to_insert);
                cursor_char += to_insert.chars().count();
            }

            let handle_key_action = |key: RepeatableKey,
                                     pressed: bool,
                                     down: bool,
                                     rk: &mut Option<RepeatableKey>,
                                     rs: &mut bool,
                                     lkt: &mut f64|
             -> bool {
                if pressed {
                    *rk = Some(key);
                    *lkt = now;
                    *rs = false;
                    true
                } else if down && *rk == Some(key) {
                    let elapsed = now - *lkt;
                    if (!*rs && elapsed >= HOLD_INITIAL_DELAY)
                        || (*rs && elapsed >= HOLD_REPEAT_RATE)
                    {
                        *lkt = now;
                        *rs = true;
                        true
                    } else {
                        false
                    }
                } else {
                    if *rk == Some(key) && !down {
                        *rk = None;
                    }
                    false
                }
            };

            if handle_key_action(
                RepeatableKey::Backspace,
                ctx.is_key_pressed(KeyCode::Backspace),
                ctx.is_key_down(KeyCode::Backspace),
                &mut repeat_key,
                &mut repeat_started,
                &mut last_key_time,
            ) {
                if selection_anchor.is_some() {
                    cursor_char = delete_selection(&mut text, cursor_char, selection_anchor);
                    selection_anchor = None;
                } else if cursor_char > 0 {
                    let start = byte_offset(&text, cursor_char - 1);
                    let end = byte_offset(&text, cursor_char);
                    text.drain(start..end);
                    cursor_char -= 1;
                }
            }

            if handle_key_action(
                RepeatableKey::Delete,
                ctx.is_key_pressed(KeyCode::Delete),
                ctx.is_key_down(KeyCode::Delete),
                &mut repeat_key,
                &mut repeat_started,
                &mut last_key_time,
            ) {
                if selection_anchor.is_some() {
                    cursor_char = delete_selection(&mut text, cursor_char, selection_anchor);
                    selection_anchor = None;
                } else if cursor_char < text.chars().count() {
                    let start = byte_offset(&text, cursor_char);
                    let end = byte_offset(&text, cursor_char + 1);
                    text.drain(start..end);
                }
            }

            if handle_key_action(
                RepeatableKey::Left,
                ctx.is_key_pressed(KeyCode::Left),
                ctx.is_key_down(KeyCode::Left),
                &mut repeat_key,
                &mut repeat_started,
                &mut last_key_time,
            ) && cursor_char > 0
            {
                if shift_held {
                    if selection_anchor.is_none() {
                        selection_anchor = Some(cursor_char);
                    }
                } else {
                    selection_anchor = None;
                }
                if ctrl_held {
                    cursor_char = prev_word_boundary(&text, cursor_char);
                } else {
                    cursor_char -= 1;
                }
            }

            if handle_key_action(
                RepeatableKey::Right,
                ctx.is_key_pressed(KeyCode::Right),
                ctx.is_key_down(KeyCode::Right),
                &mut repeat_key,
                &mut repeat_started,
                &mut last_key_time,
            ) && cursor_char < text.chars().count()
            {
                if shift_held {
                    if selection_anchor.is_none() {
                        selection_anchor = Some(cursor_char);
                    }
                } else {
                    selection_anchor = None;
                }
                if ctrl_held {
                    cursor_char = next_word_boundary(&text, cursor_char);
                } else {
                    cursor_char += 1;
                }
            }

            if ctx.is_key_pressed(KeyCode::Home) {
                if shift_held {
                    if selection_anchor.is_none() {
                        selection_anchor = Some(cursor_char);
                    }
                } else {
                    selection_anchor = None;
                }
                cursor_char = 0;
            }

            if ctx.is_key_pressed(KeyCode::End) {
                if shift_held {
                    if selection_anchor.is_none() {
                        selection_anchor = Some(cursor_char);
                    }
                } else {
                    selection_anchor = None;
                }
                cursor_char = text.chars().count();
            }

            if ctx.is_key_pressed(KeyCode::Tab) {
                tab_request_pending(self.id, shift_held);
            }

            for chr in ctx.chars_pressed() {
                if ctrl_held {
                    continue;
                }
                let accepted = if let Some(filter) = self.char_filter {
                    filter(chr)
                } else if chr.is_ascii_graphic() || chr == ' ' {
                    Some(chr)
                } else {
                    None
                };

                if let Some(ch) = accepted {
                    if selection_anchor.is_some() {
                        cursor_char = delete_selection(&mut text, cursor_char, selection_anchor);
                        selection_anchor = None;
                    }

                    let cur_len = text.chars().count();
                    if self.max_len.is_none_or(|limit| cur_len < limit) {
                        let pos = byte_offset(&text, cursor_char);
                        text.insert(pos, ch);
                        cursor_char += 1;
                    }
                }
            }

            if ctx.is_key_pressed(KeyCode::Escape) {
                INPUT_FOCUSED.with(|f| *f.borrow_mut() = false);
                focused = false;
                selection_anchor = None;
            }

            if ctx.is_key_down(KeyCode::Enter) {
                INPUT_FOCUSED.with(|f| *f.borrow_mut() = false);
                focused = false;
                confirmed = true;
                selection_anchor = None;
            }
        }

        scroll_offset_x = calculate_scroll_offset(
            ctx,
            &text,
            cursor_char,
            scroll_offset_x,
            self.rect.w,
            WIDGET_PADDING,
            DEFAULT_FONT_SIZE_16,
        );

        let now = ctx.get_time();
        if focused && ((now * 2.0) as i32 % 2 == 0) {
            let byte_pos = byte_offset(&text, cursor_char);
            let prefix = &text[..byte_pos];
            let cursor_x = self.rect.x
                + WIDGET_PADDING / 2.
                + measure_text_ui(ctx, prefix, DEFAULT_FONT_SIZE_16).width
                - scroll_offset_x;
            if cursor_x >= self.rect.x && cursor_x <= self.rect.x + self.rect.w {
                ctx.draw_line(
                    cursor_x,
                    self.rect.y + self.rect.h * 0.3,
                    cursor_x,
                    self.rect.y + self.rect.h * 0.8,
                    2.,
                    OUTLINE_COLOR,
                );
            }
        }

        INPUT_TEXT_STATE.with(|s| {
            let mut map = s.borrow_mut();
            map.insert(
                self.id,
                TextInputState {
                    text: text.clone(),
                    cursor_char,
                    focused,
                    selection_anchor,
                    last_key_time,
                    repeat_key,
                    repeat_started,
                    dragging,
                    scroll_offset_x,
                },
            );
        });

        if self.live || (!focused && confirmed) {
            (text, focused)
        } else {
            (self.current.to_string(), focused)
        }
    }
}

/// Resets the text input state for the given widget id.
pub fn text_input_reset(id: WidgetId) {
    INPUT_FOCUSED.with(|f| *f.borrow_mut() = false);
    INPUT_TEXT_STATE.with(|s| {
        let mut map = s.borrow_mut();
        map.remove(&id);
    });
}
