use crate::*;

/// Truncates text to fit within a given pixel width, appending "…" if needed.
pub fn truncate_to_width<C: BishopContext>(
    ctx: &mut C,
    text: &str,
    max_width: f32,
    font_size: f32,
) -> String {
    if measure_text_ui(ctx, text, font_size).width <= max_width {
        return text.to_string();
    }
    let ellipsis = "…";
    let ellipsis_w = measure_text_ui(ctx, ellipsis, font_size).width;
    let target = max_width - ellipsis_w;
    let mut end_byte = 0;
    for (byte_idx, ch) in text.char_indices() {
        let w = measure_text_ui(ctx, &text[..byte_idx + ch.len_utf8()], font_size).width;
        if w > target {
            break;
        }
        end_byte = byte_idx + ch.len_utf8();
    }
    format!("{}{}", &text[..end_byte], ellipsis)
}

pub fn byte_offset(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or_else(|| s.len())
}

/// Draws text in an input field at the standard position.
pub fn draw_input_field_text<C: BishopContext>(ctx: &mut C, text: &str, rect: impl Into<Rect>) {
    let rect = rect.into();
    draw_text_ui(
        ctx,
        text,
        rect.x + WIDGET_PADDING / 2.,
        rect.y + rect.h * 0.7,
        DEFAULT_FONT_SIZE_16,
        FIELD_TEXT_COLOR,
    );
}

/// Centers text horizontally and returns the x position and width.
pub fn center_text_field<C: BishopContext>(ctx: &C, x: f32, text: &str) -> (f32, f32) {
    let text_to_measure = if text.is_empty() {
        PLACEHOLDER_TEXT
    } else {
        text
    };
    let text_size = measure_text_ui(ctx, text_to_measure, DEFAULT_FONT_SIZE_16);
    let new_x = x - (text_size.width / 2.);
    (
        new_x - WIDGET_PADDING / 2.,
        text_size.width + WIDGET_PADDING,
    )
}

/// Returns the rectangle width needed to fit the given text.
pub fn rect_width_for_text<C: BishopContext>(ctx: &C, text: &str, font_size: f32) -> f32 {
    measure_text_ui(ctx, text, font_size).width + WIDGET_PADDING * 2.0
}

/// Returns the selection range as (start, end) where start <= end.
pub fn selection_range(cursor: usize, anchor: Option<usize>) -> Option<(usize, usize)> {
    anchor.map(|a| if cursor < a { (cursor, a) } else { (a, cursor) })
}

/// Gets the selected text from a string given cursor position and optional anchor.
pub fn get_selected_text(text: &str, cursor: usize, anchor: Option<usize>) -> Option<String> {
    selection_range(cursor, anchor).map(|(start, end)| {
        let start_byte = byte_offset(text, start);
        let end_byte = byte_offset(text, end);
        text[start_byte..end_byte].to_string()
    })
}

/// Deletes the selected text and returns the new cursor position.
pub fn delete_selection(text: &mut String, cursor: usize, anchor: Option<usize>) -> usize {
    if let Some((start, end)) = selection_range(cursor, anchor) {
        let start_byte = byte_offset(text, start);
        let end_byte = byte_offset(text, end);
        text.drain(start_byte..end_byte);
        start
    } else {
        cursor
    }
}

/// Filters pasted text for numeric input, keeping only valid numeric characters.
pub fn filter_numeric_paste(
    input: &str,
    is_float: bool,
    allow_negative: bool,
    has_decimal: bool,
) -> String {
    let mut result = String::new();
    let mut seen_decimal = has_decimal;

    for (i, ch) in input.chars().enumerate() {
        if ch == '-' && i == 0 && allow_negative && result.is_empty() {
            result.push(ch);
        } else if ch == '.' && is_float && !seen_decimal {
            result.push(ch);
            seen_decimal = true;
        } else if ch.is_ascii_digit() {
            result.push(ch);
        }
    }

    result
}

/// Calculates the character index from a mouse x-coordinate within the text field.
pub fn char_index_from_x<C: BishopContext>(
    ctx: &C,
    text: &str,
    mouse_x: f32,
    field_x: f32,
    font_size: f32,
    scroll_offset: f32,
) -> usize {
    let text_start_x = field_x + WIDGET_PADDING / 2.;
    let relative_x = mouse_x - text_start_x + scroll_offset;

    if relative_x <= 0.0 {
        return 0;
    }

    let mut prev_width = 0.0;
    for (i, _) in text.char_indices() {
        let char_idx = text[..i].chars().count();
        let prefix = &text[..i];
        let width = measure_text_ui(ctx, prefix, font_size).width;

        if relative_x < width {
            let mid = (prev_width + width) / 2.0;
            if relative_x < mid {
                return char_idx.saturating_sub(1);
            } else {
                return char_idx;
            }
        }
        prev_width = width;
    }

    text.chars().count()
}

/// Calculates scroll offset to ensure cursor stays visible within field bounds.
pub fn calculate_scroll_offset<C: BishopContext>(
    ctx: &C,
    text: &str,
    cursor_char: usize,
    current_offset: f32,
    field_width: f32,
    padding: f32,
    font_size: f32,
) -> f32 {
    let cursor_byte = byte_offset(text, cursor_char);
    let cursor_x = measure_text_ui(ctx, &text[..cursor_byte], font_size).width;
    let total_text_width = measure_text_ui(ctx, text, font_size).width;
    let usable_width = field_width - padding;

    let mut offset = current_offset;

    if cursor_x < offset + padding {
        offset = (cursor_x - padding).max(0.0);
    }

    if cursor_x > offset + usable_width - padding {
        offset = cursor_x - usable_width + padding;
    }

    let max_offset = (total_text_width - usable_width + padding).max(0.0);
    offset.clamp(0.0, max_offset)
}

/// Finds the start of the previous word from the given character position.
pub fn prev_word_boundary(text: &str, cursor_char: usize) -> usize {
    if cursor_char == 0 {
        return 0;
    }

    let chars: Vec<char> = text.chars().collect();
    let mut pos = cursor_char - 1;

    while pos > 0 && !chars[pos].is_alphanumeric() {
        pos -= 1;
    }

    while pos > 0 && chars[pos - 1].is_alphanumeric() {
        pos -= 1;
    }

    pos
}

/// Finds the end of the next word from the given character position.
pub fn next_word_boundary(text: &str, cursor_char: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    if cursor_char >= len {
        return len;
    }

    let mut pos = cursor_char;

    while pos < len && !chars[pos].is_alphanumeric() {
        pos += 1;
    }

    while pos < len && chars[pos].is_alphanumeric() {
        pos += 1;
    }

    pos
}

/// Sorts targets by position (Y then X) for consistent tab ordering.
fn sort_targets_by_position(targets: &[TabTarget]) -> Vec<TabTarget> {
    let mut sorted: Vec<TabTarget> = targets.to_vec();
    sorted.sort_by(|a, b| {
        let ay = a.rect.y + a.rect.h / 2.0;
        let by = b.rect.y + b.rect.h / 2.0;
        ay.partial_cmp(&by)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.rect
                    .x
                    .partial_cmp(&b.rect.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    sorted
}

/// Finds a widget's index by ID, falling back to position matching.
fn find_widget_index(
    sorted: &[TabTarget],
    current_id: WidgetId,
    current_rect: Rect,
) -> Option<usize> {
    if let Some(idx) = sorted.iter().position(|t| t.id == current_id) {
        return Some(idx);
    }

    sorted.iter().position(|t| {
        (t.rect.x - current_rect.x).abs() < 2.0
            && (t.rect.y - current_rect.y).abs() < 2.0
            && (t.rect.w - current_rect.w).abs() < 2.0
            && (t.rect.h - current_rect.h).abs() < 2.0
    })
}

/// Finds the next tab target when pressing Tab.
/// Navigates through widgets sorted by position (top-to-bottom, left-to-right).
/// If wrap is true and at the last widget, wraps to the first.
pub fn find_next_tab_target(
    current_rect: Rect,
    targets: &[TabTarget],
    current_id: WidgetId,
    wrap: bool,
) -> Option<TabTarget> {
    if targets.is_empty() {
        return None;
    }

    let sorted = sort_targets_by_position(targets);
    let current_idx = find_widget_index(&sorted, current_id, current_rect);

    match current_idx {
        Some(idx) => {
            if idx + 1 < sorted.len() {
                Some(sorted[idx + 1])
            } else if wrap && !sorted.is_empty() {
                Some(sorted[0])
            } else {
                None
            }
        }
        None => sorted.first().copied(),
    }
}

/// Finds the previous tab target when pressing Shift+Tab.
/// Navigates through widgets sorted by position (top-to-bottom, left-to-right).
/// If wrap is true and at the first widget, wraps to the last.
pub fn find_prev_tab_target(
    current_rect: Rect,
    targets: &[TabTarget],
    current_id: WidgetId,
    wrap: bool,
) -> Option<TabTarget> {
    if targets.is_empty() {
        return None;
    }

    let sorted = sort_targets_by_position(targets);
    let current_idx = find_widget_index(&sorted, current_id, current_rect);

    match current_idx {
        Some(idx) => {
            if idx > 0 {
                Some(sorted[idx - 1])
            } else if wrap && !sorted.is_empty() {
                Some(sorted[sorted.len() - 1])
            } else {
                None
            }
        }
        None => sorted.last().copied(),
    }
}
