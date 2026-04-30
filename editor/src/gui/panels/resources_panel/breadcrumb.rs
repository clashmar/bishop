use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::ui::text::measure_text;
use widgets::constants::layout;

use super::navigation::Navigation;

const BREADCRUMB_FONT_SIZE: f32 = layout::DEFAULT_FONT_SIZE_16;
const HOVER_COLOR: Color = Color::new(0.706, 0.824, 1.0, 1.0);

struct BreadcrumbHit {
    rect: Rect,
    depth: usize,
}

/// Geometry and style for drawing a breadcrumb.
pub struct BreadcrumbStyle<'a> {
    pub x: f32,
    pub y: f32,
    pub max_width: f32,
    pub height: f32,
    pub root_label: &'a str,
}

/// Draws the breadcrumb bar and returns the depth the user clicked on, if any.
///
/// Each segment (`Resources/dir1/dir2/`) is drawn individually so it can be
/// highlighted on hover and clicked to navigate to that depth. Depth 0
/// corresponds to the root label prefix.
pub fn draw_breadcrumb(
    ctx: &mut WgpuContext,
    style: &BreadcrumbStyle<'_>,
    navigation: &Navigation,
    blocked: bool,
) -> Option<usize> {
    let mut cursor_x = style.x;
    let text_y = style.y + style.height * 0.7;
    let mouse: Vec2 = ctx.mouse_position().into();
    let hovered = if blocked {
        None
    } else {
        find_hovered_depth(style, navigation, ctx, mouse)
    };

    let mut hits: Vec<BreadcrumbHit> = Vec::new();

    // Root segment
    let label = format!("{}/", style.root_label);
    let dims = measure_text(ctx, &label, BREADCRUMB_FONT_SIZE);
    if cursor_x + dims.width <= style.x + style.max_width {
        let rect = Rect::new(cursor_x, style.y, dims.width, style.height);
        let color = if hovered == Some(0) {
            HOVER_COLOR
        } else {
            Color::BLACK
        };
        ctx.draw_text(&label, cursor_x, text_y, BREADCRUMB_FONT_SIZE, color);
        hits.push(BreadcrumbHit { rect, depth: 0 });
        cursor_x += dims.width;
    } else {
        // Truncated: no more segments fit
        if blocked {
            return None;
        }
        if !ctx.is_mouse_button_pressed(MouseButton::Left) {
            return None;
        }
        for hit in &hits {
            if hit.rect.contains(mouse) {
                return Some(hit.depth);
            }
        }
        return None;
    }

    // Each segment: "name/"
    for i in 0..navigation.depth() {
        let segment = navigation.segment(i).unwrap();
        let label = format!("{segment}/");
        let dims = measure_text(ctx, &label, BREADCRUMB_FONT_SIZE);
        if cursor_x + dims.width > style.x + style.max_width {
            break;
        }
        let rect = Rect::new(cursor_x, style.y, dims.width, style.height);
        let color = if hovered == Some(i + 1) {
            HOVER_COLOR
        } else {
            Color::BLACK
        };
        ctx.draw_text(&label, cursor_x, text_y, BREADCRUMB_FONT_SIZE, color);
        hits.push(BreadcrumbHit { rect, depth: i + 1 });
        cursor_x += dims.width;
    }

    if blocked {
        return None;
    }

    if !ctx.is_mouse_button_pressed(MouseButton::Left) {
        return None;
    }

    for hit in &hits {
        if hit.rect.contains(mouse) {
            return Some(hit.depth);
        }
    }

    None
}

/// Pre-measures all segment rects to find which depth the mouse is hovering over.
fn find_hovered_depth(
    style: &BreadcrumbStyle<'_>,
    navigation: &Navigation,
    ctx: &WgpuContext,
    mouse: Vec2,
) -> Option<usize> {
    let mut cursor_x = style.x;

    let dims = measure_text(ctx, &format!("{}/", style.root_label), BREADCRUMB_FONT_SIZE);
    if cursor_x + dims.width <= style.x + style.max_width {
        let rect = Rect::new(cursor_x, style.y, dims.width, style.height);
        if rect.contains(mouse) {
            return Some(0);
        }
        cursor_x += dims.width;
    } else {
        return None;
    }

    for i in 0..navigation.depth() {
        let segment = navigation.segment(i).unwrap();
        let label = format!("{segment}/");
        let dims = measure_text(ctx, &label, BREADCRUMB_FONT_SIZE);
        if cursor_x + dims.width > style.x + style.max_width {
            break;
        }
        let rect = Rect::new(cursor_x, style.y, dims.width, style.height);
        if rect.contains(mouse) {
            return Some(i + 1);
        }
        cursor_x += dims.width;
    }

    None
}
