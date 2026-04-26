use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::ui::text::measure_text;

use super::navigation::Navigation;
use super::BREADCRUMB_HEIGHT;

const BREADCRUMB_FONT_SIZE: f32 = DEFAULT_FONT_SIZE_16;
const HOVER_COLOR: Color = Color::new(0.706, 0.824, 1.0, 1.0);

struct BreadcrumbHit {
    rect: Rect,
    depth: usize,
}

/// Draws the breadcrumb bar and returns the depth the user clicked on, if any.
///
/// Each segment (`/dir1/dir2/`) is drawn individually so it can be
/// highlighted on hover and clicked to navigate to that depth. Depth 0
/// corresponds to the root `/` prefix.
pub fn draw_breadcrumb(
    ctx: &mut WgpuContext,
    x: f32,
    y: f32,
    navigation: &Navigation,
    blocked: bool,
) -> Option<usize> {
    let mut cursor_x = x;
    let text_y = y + BREADCRUMB_HEIGHT * 0.7;
    let mouse: Vec2 = ctx.mouse_position().into();
    let hovered = if blocked {
        None
    } else {
        find_hovered_depth(cursor_x, y, navigation, ctx, mouse)
    };

    let mut hits: Vec<BreadcrumbHit> = Vec::new();

    // Root segment
    let label = "root/";
    let dims = measure_text(ctx, label, BREADCRUMB_FONT_SIZE);
    let rect = Rect::new(cursor_x, y, dims.width, BREADCRUMB_HEIGHT);
    let color = if hovered == Some(0) {
        HOVER_COLOR
    } else {
        Color::WHITE
    };
    ctx.draw_text(label, cursor_x, text_y, BREADCRUMB_FONT_SIZE, color);
    hits.push(BreadcrumbHit { rect, depth: 0 });
    cursor_x += dims.width;

    // Each segment: "name/"
    for i in 0..navigation.depth() {
        let segment = navigation.segment(i).unwrap();
        let label = format!("{segment}/");
        let dims = measure_text(ctx, &label, BREADCRUMB_FONT_SIZE);
        let rect = Rect::new(cursor_x, y, dims.width, BREADCRUMB_HEIGHT);
        let color = if hovered == Some(i + 1) {
            HOVER_COLOR
        } else {
            Color::WHITE
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
    start_x: f32,
    y: f32,
    navigation: &Navigation,
    ctx: &WgpuContext,
    mouse: Vec2,
) -> Option<usize> {
    let mut cursor_x = start_x;

    let dims = measure_text(ctx, "root/", BREADCRUMB_FONT_SIZE);
    let rect = Rect::new(cursor_x, y, dims.width, BREADCRUMB_HEIGHT);
    if rect.contains(mouse) {
        return Some(0);
    }
    cursor_x += dims.width;

    for i in 0..navigation.depth() {
        let segment = navigation.segment(i).unwrap();
        let label = format!("{segment}/");
        let dims = measure_text(ctx, &label, BREADCRUMB_FONT_SIZE);
        let rect = Rect::new(cursor_x, y, dims.width, BREADCRUMB_HEIGHT);
        if rect.contains(mouse) {
            return Some(i + 1);
        }
        cursor_x += dims.width;
    }

    None
}
