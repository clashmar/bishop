// editor/src/gui/panels/console_panel.rs
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;

const ROW_HEIGHT: f32 = 18.0;
const TOP_PADDING: f32 = 4.0;
const BOTTOM_PADDING: f32 = 8.0;
const CLEAR_BUTTON_HEIGHT: f32 = 20.0;
const HEADER_ROW_HEIGHT: f32 = 28.0;

/// Selection position within the console text.
#[derive(Clone, Copy, PartialEq)]
struct SelectionPos {
    line: usize,
    char_idx: usize,
}

pub struct ConsolePanel {
    scroll_state: ScrollState,
    last_total_pushed: usize,
    selection_anchor: Option<SelectionPos>,
    selection_end: Option<SelectionPos>,
    dragging: bool,
    cached_wrapped: Vec<(log::Level, Vec<String>)>,
}

impl ConsolePanel {
    pub fn new() -> Self {
        Self {
            scroll_state: ScrollState::with_auto_scroll(),
            last_total_pushed: 0,
            selection_anchor: None,
            selection_end: None,
            dragging: false,
            cached_wrapped: Vec::new(),
        }
    }

    fn level_color(level: log::Level) -> Color {
        match level {
            log::Level::Error => Color::RED,
            log::Level::Warn => Color::YELLOW,
            log::Level::Info => Color::WHITE,
            log::Level::Debug => Color::GREY,
            log::Level::Trace => Color::DARKGRAY,
        }
    }

    /// Wraps text to fit within the given pixel width.
    fn wrap_text(ctx: &WgpuContext, text: &str, max_width: f32, font_size: f32) -> Vec<String> {
        if text.is_empty() {
            return vec![String::new()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width: f32 = 0.0;

        // Split into tokens that preserve delimiters as separate tokens
        let tokens = Self::tokenize_for_wrap(text);

        for token in tokens {
            let token_width = measure_text(ctx, &token, font_size).width;

            // If this token alone exceeds max_width, break it character by character
            if token_width > max_width {
                // First, push current line if non-empty
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                    current_width = 0.0;
                }

                // Break the long token character by character
                for c in token.chars() {
                    let char_str = c.to_string();
                    let char_width = measure_text(ctx, &char_str, font_size).width;

                    if current_width + char_width > max_width && !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = String::new();
                        current_width = 0.0;
                    }

                    current_line.push(c);
                    current_width += char_width;
                }
            } else if current_width + token_width > max_width {
                // Token doesn't fit on current line, start a new line
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                // Skip leading whitespace on new lines
                let trimmed = token.trim_start();
                current_line = trimmed.to_string();
                current_width = measure_text(ctx, &current_line, font_size).width;
            } else {
                // Token fits, append it
                current_line.push_str(&token);
                current_width += token_width;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    /// Splits text into tokens for wrapping, keeping delimiters as break points.
    fn tokenize_for_wrap(text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();

        for c in text.chars() {
            // These characters are good break points
            if c == ' ' || c == '/' || c == '\\' || c == '-' || c == '_' || c == '.' {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
                tokens.push(c.to_string());
            } else {
                current.push(c);
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    /// Returns ordered selection range (start, end) where start comes before end.
    fn ordered_selection(&self) -> Option<(SelectionPos, SelectionPos)> {
        match (self.selection_anchor, self.selection_end) {
            (Some(anchor), Some(end)) if anchor != end => {
                if anchor.line < end.line
                    || (anchor.line == end.line && anchor.char_idx < end.char_idx)
                {
                    Some((anchor, end))
                } else {
                    Some((end, anchor))
                }
            }
            _ => None,
        }
    }

    /// Converts mouse position to line and character index.
    fn pos_from_mouse(
        ctx: &WgpuContext,
        mouse: Vec2,
        content_rect: Rect,
        scroll_y: f32,
        all_lines: &[String],
        font_size: f32,
    ) -> Option<SelectionPos> {
        let relative_y = mouse.y - content_rect.y - scroll_y;
        let line = (relative_y / ROW_HEIGHT).floor() as usize;

        if line >= all_lines.len() {
            return Some(SelectionPos {
                line: all_lines.len().saturating_sub(1),
                char_idx: all_lines.last().map_or(0, |l| l.chars().count()),
            });
        }

        let line_text = &all_lines[line];
        let text_start_x = content_rect.x + 6.0;
        let relative_x = mouse.x - text_start_x;

        if relative_x <= 0.0 {
            return Some(SelectionPos { line, char_idx: 0 });
        }

        let mut prev_width = 0.0;
        for (byte_idx, ch) in line_text.char_indices() {
            let char_idx = line_text[..byte_idx].chars().count();
            let width = measure_text(ctx, &line_text[..byte_idx + ch.len_utf8()], font_size).width;

            if relative_x < width {
                let mid = (prev_width + width) / 2.0;
                if relative_x < mid {
                    return Some(SelectionPos { line, char_idx });
                } else {
                    return Some(SelectionPos {
                        line,
                        char_idx: char_idx + 1,
                    });
                }
            }
            prev_width = width;
        }

        Some(SelectionPos {
            line,
            char_idx: line_text.chars().count(),
        })
    }

    /// Extracts selected text from all lines given a selection range.
    fn extract_selected_text(
        all_lines: &[String],
        start: SelectionPos,
        end: SelectionPos,
    ) -> String {
        if start.line == end.line {
            let line = &all_lines[start.line];
            let start_byte = line
                .char_indices()
                .nth(start.char_idx)
                .map(|(b, _)| b)
                .unwrap_or(line.len());
            let end_byte = line
                .char_indices()
                .nth(end.char_idx)
                .map(|(b, _)| b)
                .unwrap_or(line.len());
            return line[start_byte..end_byte].to_string();
        }

        let mut result = String::new();

        // First line from start.char_idx to end
        let first_line = &all_lines[start.line];
        let start_byte = first_line
            .char_indices()
            .nth(start.char_idx)
            .map(|(b, _)| b)
            .unwrap_or(first_line.len());
        result.push_str(&first_line[start_byte..]);
        result.push('\n');

        // Middle lines in full
        for line in all_lines.iter().take(end.line).skip(start.line + 1) {
            result.push_str(line);
            result.push('\n');
        }

        // Last line from start to end.char_idx
        let last_line = &all_lines[end.line];
        let end_byte = last_line
            .char_indices()
            .nth(end.char_idx)
            .map(|(b, _)| b)
            .unwrap_or(last_line.len());
        result.push_str(&last_line[..end_byte]);

        result
    }

    /// Clears current selection.
    fn clear_selection(&mut self) {
        self.selection_anchor = None;
        self.selection_end = None;
        self.dragging = false;
    }
}

pub const CONSOLE_PANEL: &str = "Console";

impl PanelDefinition for ConsolePanel {
    fn title(&self) -> &'static str {
        CONSOLE_PANEL
    }

    fn default_rect(&self, _ctx: &WgpuContext) -> Rect {
        Rect::new(20., 460., 550., 250.)
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, _editor: &mut Editor, blocked: bool) {
        let mouse: Vec2 = ctx.mouse_position().into();

        // Clear button centered in header row
        let btn_width = 50.0;
        let btn_y = rect.y + TOP_PADDING + (HEADER_ROW_HEIGHT - CLEAR_BUTTON_HEIGHT) / 2.0;
        let clear_btn_rect = Rect::new(
            rect.x + rect.w - btn_width - 6.,
            btn_y,
            btn_width,
            CLEAR_BUTTON_HEIGHT,
        );

        let clicked = Button::new(clear_btn_rect, "Clear")
            .blocked(blocked)
            .show(ctx);
        if !blocked && clicked {
            if let Ok(mut history) = LOG_HISTORY.lock() {
                history.clear();
            }
            self.scroll_state.scroll_y = 0.0;
            self.clear_selection();
            self.cached_wrapped.clear();
        }

        // Content area below the header row
        let content_y = rect.y + TOP_PADDING + HEADER_ROW_HEIGHT;
        let content_h = rect.h - TOP_PADDING - HEADER_ROW_HEIGHT - BOTTOM_PADDING;
        let content_rect = Rect::new(rect.x, content_y, rect.w, content_h.max(0.0));

        let font_size = 14.0;

        // Check if cache needs update using counter
        let (_entry_count, total_pushed) = LOG_HISTORY
            .lock()
            .map(|h| (h.entries().len(), h.total_pushed()))
            .unwrap_or((0, 0));

        let needs_update = total_pushed != self.last_total_pushed;

        if needs_update {
            // Rebuild cache from current LOG_HISTORY entries
            let entries: Vec<(log::Level, String, String, &'static str, u32, u32)> = LOG_HISTORY
                .lock()
                .map(|history| {
                    history
                        .entries()
                        .iter()
                        .map(|e| {
                            (
                                e.level,
                                e.message.clone(),
                                e.time.clone(),
                                e.file,
                                e.line,
                                e.count,
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Calculate usable width for wrapping (account for potential scrollbar)
            let usable_w = content_rect.w - 6.0 - 12.0;

            self.cached_wrapped = entries
                .iter()
                .map(|(level, message, time, file, line, count)| {
                    let count_suffix = if *count > 1 {
                        format!(" ×{}", count)
                    } else {
                        String::new()
                    };
                    let full_message = if *line > 0 {
                        format!("{} [{}:{}] {}{}", time, file, line, message, count_suffix)
                    } else {
                        format!("{} [{}] {}{}", time, file, message, count_suffix)
                    };
                    let lines = Self::wrap_text(ctx, &full_message, usable_w, font_size);
                    (*level, lines)
                })
                .collect();

            self.last_total_pushed = total_pushed;
        }

        let wrapped_entries = &self.cached_wrapped;

        // Calculate total line count for content height
        let total_lines: usize = wrapped_entries.iter().map(|(_, lines)| lines.len()).sum();
        let content_height = total_lines as f32 * ROW_HEIGHT;

        // Auto-scroll when new entries arrive
        if needs_update && self.scroll_state.auto_scroll {
            let scroll_range = (content_height - content_h).max(0.0);
            self.scroll_state.scroll_y = -scroll_range;
        }

        let area = ScrollableArea::new(content_rect, content_height)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);

        // Create flat list of all lines with their colors for selection handling
        let all_lines: Vec<(String, Color)> = wrapped_entries
            .iter()
            .flat_map(|(level, lines)| {
                let color = Self::level_color(*level);
                lines.iter().map(move |line| (line.clone(), color))
            })
            .collect();

        let all_line_texts: Vec<String> = all_lines.iter().map(|(text, _)| text.clone()).collect();

        // Handle mouse selection
        if !blocked
            && content_rect.contains(mouse)
            && ctx.is_mouse_button_pressed(MouseButton::Left)
        {
            if let Some(pos) = Self::pos_from_mouse(
                ctx,
                mouse,
                content_rect,
                self.scroll_state.scroll_y,
                &all_line_texts,
                font_size,
            ) {
                self.selection_anchor = Some(pos);
                self.selection_end = Some(pos);
                self.dragging = true;
            }
        }

        if self.dragging && ctx.is_mouse_button_down(MouseButton::Left) {
            if let Some(pos) = Self::pos_from_mouse(
                ctx,
                mouse,
                content_rect,
                self.scroll_state.scroll_y,
                &all_line_texts,
                font_size,
            ) {
                self.selection_end = Some(pos);
            }
        }

        if ctx.is_mouse_button_released(MouseButton::Left) && self.dragging {
            self.dragging = false;
            if self.selection_anchor == self.selection_end {
                self.clear_selection();
            }
        }

        // Handle copy with Ctrl+C
        if !blocked && Controls::copy(ctx) {
            if let Some((start, end)) = self.ordered_selection() {
                let selected_text = Self::extract_selected_text(&all_line_texts, start, end);
                clipboard_set_text(&selected_text);
            }
        }

        // Draw entries with cumulative Y tracking and selection highlights
        let selection = self.ordered_selection();

        for (global_line_idx, (line_text, color)) in all_lines.iter().enumerate() {
            let line_y =
                content_rect.y + self.scroll_state.scroll_y + global_line_idx as f32 * ROW_HEIGHT;

            if !area.is_fully_visible(line_y, ROW_HEIGHT) {
                continue;
            }

            // Draw selection highlight for this line
            if let Some((start, end)) = selection {
                if global_line_idx >= start.line && global_line_idx <= end.line {
                    let line_chars = line_text.chars().count();
                    let sel_start_char = if global_line_idx == start.line {
                        start.char_idx
                    } else {
                        0
                    };
                    let sel_end_char = if global_line_idx == end.line {
                        end.char_idx
                    } else {
                        line_chars
                    };

                    if sel_start_char < sel_end_char {
                        let start_byte = line_text
                            .char_indices()
                            .nth(sel_start_char)
                            .map(|(b, _)| b)
                            .unwrap_or(0);
                        let end_byte = line_text
                            .char_indices()
                            .nth(sel_end_char)
                            .map(|(b, _)| b)
                            .unwrap_or(line_text.len());

                        let sel_start_x = content_rect.x
                            + 6.0
                            + measure_text(ctx, &line_text[..start_byte], font_size).width;
                        let sel_end_x = content_rect.x
                            + 6.0
                            + measure_text(ctx, &line_text[..end_byte], font_size).width;

                        ctx.draw_rectangle(
                            sel_start_x,
                            line_y,
                            sel_end_x - sel_start_x,
                            ROW_HEIGHT,
                            Color::new(0.3, 0.5, 0.8, 0.5),
                        );
                    }
                }
            }

            ctx.draw_text(
                line_text,
                content_rect.x + 6.,
                line_y + ROW_HEIGHT * 0.75,
                font_size,
                *color,
            );
        }

        area.draw_scrollbar(ctx, self.scroll_state.scroll_y);
    }
}
