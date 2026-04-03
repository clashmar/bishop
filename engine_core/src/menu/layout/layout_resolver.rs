use super::{HorizontalAlign, LayoutConfig, LayoutDirection, VerticalAlign};
use crate::menu::elements::layout_group::LayoutGroupElement;
use bishop::prelude::{Rect, Vec2};

/// Computes absolute rects for all children in a layout group.
///
/// `group_rect` is the normalized rect of the group element itself.
/// Returns one `Rect` per child in the same order as `group.children`.
pub fn resolve_layout(group: &LayoutGroupElement, group_rect: Rect) -> Vec<Rect> {
    let layout = &group.layout;

    // Padding is stored in pixels in LayoutConfig — normalize relative to group size
    let pad_left = layout.padding.left / 1920.0;
    let pad_top = layout.padding.top / 1080.0;
    let pad_right = layout.padding.right / 1920.0;
    let pad_bottom = layout.padding.bottom / 1080.0;

    let item_w = layout.item_width / 1920.0;
    let item_h = layout.item_height / 1080.0;

    let inner_w = group_rect.w - pad_left - pad_right;
    let inner_h = group_rect.h - pad_top - pad_bottom;

    let managed_count = group.children.iter().filter(|c| c.managed).count();

    let managed_rects =
        compute_managed_positions(*layout, managed_count, Vec2::new(inner_w, inner_h));

    let mut managed_index = 0;
    group
        .children
        .iter()
        .map(|child| {
            if child.managed {
                let base = if managed_index < managed_rects.len() {
                    managed_rects[managed_index]
                } else {
                    Rect::new(0.0, 0.0, item_w, item_h)
                };
                managed_index += 1;
                Rect::new(
                    group_rect.x + pad_left + base.x,
                    group_rect.y + pad_top + base.y,
                    base.w,
                    base.h,
                )
            } else {
                Rect::new(
                    group_rect.x + child.element.rect.x,
                    group_rect.y + child.element.rect.y,
                    child.element.rect.w,
                    child.element.rect.h,
                )
            }
        })
        .collect()
}

/// Computes positions for managed children relative to inner area origin (0,0).
fn compute_managed_positions(layout: LayoutConfig, count: usize, inner_size: Vec2) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }

    let item_w = layout.item_width / 1920.0;
    let item_h = layout.item_height / 1080.0;
    let spacing_x = layout.spacing / 1920.0;
    let spacing_y = layout.spacing / 1080.0;

    match layout.direction {
        LayoutDirection::Vertical => {
            let total_h = count as f32 * item_h + (count as f32 - 1.0) * spacing_y;
            let start_y = align_offset(layout.alignment.vertical, inner_size.y, total_h);
            let start_x = align_offset_h(layout.alignment.horizontal, inner_size.x, item_w);

            (0..count)
                .map(|i| {
                    let y = start_y + i as f32 * (item_h + spacing_y);
                    Rect::new(start_x, y, item_w, item_h)
                })
                .collect()
        }
        LayoutDirection::Horizontal => {
            let total_w = count as f32 * item_w + (count as f32 - 1.0) * spacing_x;
            let start_x = align_offset_h(layout.alignment.horizontal, inner_size.x, total_w);
            let start_y = align_offset(layout.alignment.vertical, inner_size.y, item_h);

            (0..count)
                .map(|i| {
                    let x = start_x + i as f32 * (item_w + spacing_x);
                    Rect::new(x, start_y, item_w, item_h)
                })
                .collect()
        }
        LayoutDirection::Grid { columns } => {
            let cols = columns.max(1) as usize;
            let rows = count.div_ceil(cols);
            let total_w = cols as f32 * item_w + (cols as f32 - 1.0) * spacing_x;
            let total_h = rows as f32 * item_h + (rows as f32 - 1.0) * spacing_y;
            let start_x = align_offset_h(layout.alignment.horizontal, inner_size.x, total_w);
            let start_y = align_offset(layout.alignment.vertical, inner_size.y, total_h);

            (0..count)
                .map(|i| {
                    let col = i % cols;
                    let row = i / cols;
                    let x = start_x + col as f32 * (item_w + spacing_x);
                    let y = start_y + row as f32 * (item_h + spacing_y);
                    Rect::new(x, y, item_w, item_h)
                })
                .collect()
        }
    }
}

fn align_offset(v_align: VerticalAlign, container: f32, content: f32) -> f32 {
    match v_align {
        VerticalAlign::Top => 0.0,
        VerticalAlign::Middle => (container - content).max(0.0) / 2.0,
        VerticalAlign::Bottom => (container - content).max(0.0),
    }
}

fn align_offset_h(h_align: HorizontalAlign, container: f32, content: f32) -> f32 {
    match h_align {
        HorizontalAlign::Left => 0.0,
        HorizontalAlign::Center => (container - content).max(0.0) / 2.0,
        HorizontalAlign::Right => (container - content).max(0.0),
    }
}
