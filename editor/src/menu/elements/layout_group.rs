use crate::menu::menu_canvas::drawing::{draw_reorder_indicator, MenuCanvasFrame};
use crate::menu::menu_properties_panel::common_properties::row_visible;
use crate::menu::menu_properties_panel::nav_section::NavSectionStyle;
use crate::menu::menu_properties_panel::{FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::resize_handle::draw_resize_handles;
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::theme::with_theme;

impl MenuEditor {
    pub(crate) fn draw_layout_group(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        element: &MenuElement,
        element_rect: Rect,
        is_selected: bool,
    ) {
        let MenuElementKind::LayoutGroup(group) = &element.kind else {
            return;
        };
        let has_child_selected = is_selected && self.selected_child_index.is_some();

        for child in group.children.iter().filter(|c| !c.managed) {
            if let MenuElementKind::Panel(_) = &child.element.kind {
                Panel::new(element_rect)
                    .apply_selectors(
                        child.element.class.as_deref(),
                        child.element.style_id.as_deref(),
                    )
                    .show(frame.ctx);
            }
        }

        if !frame.preview {
            let outline_color = if is_selected {
                with_theme(|t| t.highlight)
            } else {
                Color::new(0., 0., 0., 0.)
            };
            let thickness = if is_selected { 2.0 } else { 1.0 };
            frame.ctx.draw_rectangle_lines(
                element_rect.x,
                element_rect.y,
                element_rect.w,
                element_rect.h,
                thickness,
                outline_color,
            );
            let group_label = if !element.name.is_empty() {
                format!("[{}]", element.name)
            } else {
                "[Layout Group]".to_string()
            };
            frame.ctx.draw_text(
                &group_label,
                element_rect.x + 4.0,
                element_rect.y + 12.0,
                10.0,
                outline_color,
            );
        }

        let resolved = resolve_layout(group, element.rect);
        let reorder_info = self
            .reorder_drag
            .as_ref()
            .filter(|r| self.selected_element_indices.contains(&r.group_index));
        let dragged_child_idx = reorder_info.map(|r| r.child_index);
        let drop_target = reorder_info.and_then(|r| r.drop_target);

        for (child_idx, (child, resolved_rect)) in
            group.children.iter().zip(resolved.iter()).enumerate()
        {
            let child_screen =
                normalized_rect_to_screen(*resolved_rect, frame.canvas_origin, frame.canvas_size);
            let is_child_selected = is_selected && self.selected_child_index == Some(child_idx);
            let child_allow_resize = !child.managed;

            if dragged_child_idx == Some(child_idx) {
                frame.ctx.draw_rectangle(
                    child_screen.x,
                    child_screen.y,
                    child_screen.w,
                    child_screen.h,
                    Color::new(0.0, 0.0, 0.0, 0.3),
                );
            }

            self.draw_element(
                frame,
                &child.element,
                child_screen,
                is_child_selected,
                child_allow_resize,
            );
        }

        if let Some(target) = drop_target {
            let managed_rects: Vec<(usize, Rect)> = group
                .children
                .iter()
                .zip(resolved.iter())
                .enumerate()
                .filter(|(_, (child, _))| child.managed)
                .map(|(idx, (_, rect))| (idx, *rect))
                .collect();
            let managed_slot = group
                .children
                .iter()
                .take(target)
                .filter(|c| c.managed)
                .count();
            draw_reorder_indicator(
                frame.ctx,
                &managed_rects,
                managed_slot,
                &group.layout,
                frame.canvas_origin,
                frame.canvas_size,
            );
        }

        if is_selected && !has_child_selected {
            draw_resize_handles(frame.ctx, element_rect);
        }
    }

    pub(crate) fn draw_layout_group_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (
            has_bg_panel,
            direction,
            grid_cols,
            spacing,
            padding,
            h_align,
            v_align,
            item_w,
            item_h,
        ) = {
            let Some(element) = self.selected_element() else {
                return;
            };
            let MenuElementKind::LayoutGroup(group) = &element.kind else {
                return;
            };
            let cols = match group.layout.direction {
                LayoutDirection::Grid { columns } => columns,
                _ => 2,
            };
            let has_bg_panel = group
                .children
                .first()
                .is_some_and(|c| !c.managed && matches!(c.element.kind, MenuElementKind::Panel(_)));
            (
                has_bg_panel,
                group.layout.direction,
                cols,
                group.layout.spacing,
                group.layout.padding,
                group.layout.alignment.horizontal,
                group.layout.alignment.vertical,
                group.layout.item_width,
                group.layout.item_height,
            )
        };

        // Background Panel section
        if row_visible(*y, 20.0, clip) {
            ctx.draw_text("Background Panel", x, *y + 14.0, 12.0, Color::GREY);
        }
        *y += 20.0;

        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Enabled:", x, *y + 16.0, 12.0, Color::WHITE);
            let checkbox_rect = Rect::new(x + LABEL_WIDTH, *y + 4.0, 16.0, 16.0);
            let mut enabled = has_bg_panel;
            if Checkbox::new(checkbox_rect, &mut enabled).show(ctx) {
                self.push_element_update(|el| {
                    if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                        let has_bg = !group.children.is_empty()
                            && !group.children[0].managed
                            && matches!(group.children[0].element.kind, MenuElementKind::Panel(_));
                        if enabled && !has_bg {
                            let child = LayoutChild {
                                element: MenuElement::new(
                                    MenuElementKind::Panel(PanelElement),
                                    Rect::new(0.0, 0.0, 0.0, 0.0),
                                ),
                                managed: false,
                            };
                            group.children.insert(0, child);
                        } else if !enabled && has_bg {
                            group.children.remove(0);
                        }
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Children
        let child_count = {
            let Some(element) = self.selected_element() else {
                return;
            };
            let MenuElementKind::LayoutGroup(group) = &element.kind else {
                return;
            };
            group.children.len()
        };

        *y += 4.0;
        if row_visible(*y, 20.0, clip) {
            ctx.draw_text(
                &format!("Children ({})", child_count),
                x,
                *y + 14.0,
                12.0,
                Color::GREY,
            );
        }
        *y += 20.0;

        let selected_idx = self.selected_child_index;
        let children_item_h = 24.0;
        let children_item_pad = 4.0;

        for i in 0..child_count {
            let (child_label, managed, is_selected, is_background) = {
                let Some(element) = self.selected_element() else {
                    break;
                };
                let MenuElementKind::LayoutGroup(group) = &element.kind else {
                    break;
                };
                let child = &group.children[i];
                let label = if !child.element.name.is_empty() {
                    child.element.name.clone()
                } else {
                    let kind_name = child.element.kind.kind_name();
                    match &child.element.kind {
                        MenuElementKind::Label(l) => format!("{}: {}", kind_name, l.text_key),
                        MenuElementKind::Button(b) => format!("{}: {}", kind_name, b.text_key),
                        MenuElementKind::Slider(s) => format!("{}: {}", kind_name, s.text_key),
                        _ => kind_name.to_string(),
                    }
                };
                let is_bg = i == 0
                    && !child.managed
                    && matches!(child.element.kind, MenuElementKind::Panel(_));
                (label, child.managed, selected_idx == Some(i), is_bg)
            };

            if row_visible(*y, children_item_h, clip) {
                let mouse: Vec2 = ctx.mouse_position().into();
                let item_rect = Rect::new(x, *y, w - 40.0, children_item_h);
                let hovering = item_rect.contains(mouse) && !blocked;

                let bg = if is_selected {
                    with_theme(|theme| theme.primary)
                } else if hovering {
                    with_theme(|theme| theme.hover)
                } else {
                    with_theme(|theme| theme.background)
                };
                ctx.draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, bg);

                let text_color = if is_selected {
                    Color::WHITE
                } else {
                    Color::new(0.8, 0.8, 0.8, 1.0)
                };
                ctx.draw_text(&child_label, item_rect.x + 8.0, *y + 16.0, 12.0, text_color);

                // Managed checkbox (background panel managed flag is immutable)
                if !is_background {
                    let checkbox_rect =
                        Rect::new(item_rect.x + item_rect.w + 6.0, *y + 4.0, 16.0, 16.0);
                    let mut managed_val = managed;
                    if Checkbox::new(checkbox_rect, &mut managed_val).show(ctx) {
                        self.push_element_update(|el| {
                            if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                                if let Some(child) = group.children.get_mut(i) {
                                    child.managed = managed_val;
                                }
                            }
                        });
                    }
                }

                if hovering && ctx.is_mouse_button_pressed(MouseButton::Left) {
                    self.selected_child_index = Some(i);
                    return;
                }
            }
            *y += children_item_h + children_item_pad;
        }

        *y += 6.0;

        // Direction dropdown
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Direction:", x, *y + 16.0, 12.0, Color::WHITE);
            let dir_options = ["Vertical", "Horizontal", "Grid"];
            let current_dir = match direction {
                LayoutDirection::Vertical => "Vertical",
                LayoutDirection::Horizontal => "Horizontal",
                LayoutDirection::Grid { .. } => "Grid",
            };
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.layout_direction_id,
                dropdown_rect,
                current_dir,
                &dir_options,
                |s| s.to_string(),
            )
            .suppressed(blocked)
            .fixed_width()
            .show(ctx)
            {
                let new_dir = match selected {
                    "Vertical" => LayoutDirection::Vertical,
                    "Horizontal" => LayoutDirection::Horizontal,
                    "Grid" => LayoutDirection::Grid { columns: grid_cols },
                    _ => direction,
                };
                self.push_element_update(|el| {
                    if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                        group.layout.direction = new_dir;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Grid columns (only if Grid)
        if matches!(direction, LayoutDirection::Grid { .. }) {
            if row_visible(*y, ROW_HEIGHT, clip) {
                ctx.draw_text("Columns:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
                let new_cols = NumberInput::new(
                    self.properties_panel.widget_ids.layout_grid_cols_id,
                    field_rect,
                    grid_cols as f32,
                )
                .blocked(blocked)
                .min(1.0)
                .max(20.0)
                .show(ctx);
                let new_cols = new_cols as u32;
                if new_cols != grid_cols {
                    self.push_element_update(|el| {
                        if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                            group.layout.direction = LayoutDirection::Grid { columns: new_cols };
                        }
                    });
                }
            }
            *y += ROW_HEIGHT;
        }

        // Spacing
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Spacing:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
            let new_spacing = NumberInput::new(
                self.properties_panel.widget_ids.layout_spacing_id,
                field_rect,
                spacing,
            )
            .blocked(blocked)
            .min(0.0)
            .show(ctx);
            if (new_spacing - spacing).abs() > 0.01 {
                self.push_element_update(|el| {
                    if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                        group.layout.spacing = new_spacing;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Padding
        *y += 4.0;
        if row_visible(*y, 20.0, clip) {
            ctx.draw_text("Padding", x, *y + 14.0, 12.0, Color::GREY);
        }
        *y += 20.0;

        let pad_fields = [
            (
                "Top:",
                self.properties_panel.widget_ids.layout_pad_top_id,
                padding.top,
            ),
            (
                "Right:",
                self.properties_panel.widget_ids.layout_pad_right_id,
                padding.right,
            ),
            (
                "Bottom:",
                self.properties_panel.widget_ids.layout_pad_bottom_id,
                padding.bottom,
            ),
            (
                "Left:",
                self.properties_panel.widget_ids.layout_pad_left_id,
                padding.left,
            ),
        ];

        for (label, id, current_val) in pad_fields {
            if row_visible(*y, ROW_HEIGHT, clip) {
                ctx.draw_text(label, x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
                let new_val = NumberInput::new(id, field_rect, current_val)
                    .blocked(blocked)
                    .min(0.0)
                    .show(ctx);
                if (new_val - current_val).abs() > 0.01 {
                    let label_str = label.to_string();
                    self.push_element_update(|el| {
                        if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                            match label_str.as_str() {
                                "Top:" => group.layout.padding.top = new_val,
                                "Right:" => group.layout.padding.right = new_val,
                                "Bottom:" => group.layout.padding.bottom = new_val,
                                "Left:" => group.layout.padding.left = new_val,
                                _ => {}
                            }
                        }
                    });
                }
            }
            *y += ROW_HEIGHT;
        }

        // Alignment
        *y += 4.0;
        if row_visible(*y, 20.0, clip) {
            ctx.draw_text("Alignment", x, *y + 14.0, 12.0, Color::GREY);
        }
        *y += 20.0;

        // Horizontal alignment
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("H Align:", x, *y + 16.0, 12.0, Color::WHITE);
            let h_options = ["Left", "Center", "Right"];
            let current_h = match h_align {
                HorizontalAlign::Left => "Left",
                HorizontalAlign::Center => "Center",
                HorizontalAlign::Right => "Right",
            };
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.layout_h_align_id,
                dropdown_rect,
                current_h,
                &h_options,
                |s| s.to_string(),
            )
            .suppressed(blocked)
            .fixed_width()
            .show(ctx)
            {
                let new_align = match selected {
                    "Left" => HorizontalAlign::Left,
                    "Center" => HorizontalAlign::Center,
                    "Right" => HorizontalAlign::Right,
                    _ => h_align,
                };
                self.push_element_update(|el| {
                    if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                        group.layout.alignment.horizontal = new_align;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Vertical alignment
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("V Align:", x, *y + 16.0, 12.0, Color::WHITE);
            let v_options = ["Top", "Middle", "Bottom"];
            let current_v = match v_align {
                VerticalAlign::Top => "Top",
                VerticalAlign::Middle => "Middle",
                VerticalAlign::Bottom => "Bottom",
            };
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.layout_v_align_id,
                dropdown_rect,
                current_v,
                &v_options,
                |s| s.to_string(),
            )
            .suppressed(blocked)
            .fixed_width()
            .show(ctx)
            {
                let new_align = match selected {
                    "Top" => VerticalAlign::Top,
                    "Middle" => VerticalAlign::Middle,
                    "Bottom" => VerticalAlign::Bottom,
                    _ => v_align,
                };
                self.push_element_update(|el| {
                    if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                        group.layout.alignment.vertical = new_align;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Item size
        *y += 4.0;
        if row_visible(*y, 20.0, clip) {
            ctx.draw_text("Item Size", x, *y + 14.0, 12.0, Color::GREY);
        }
        *y += 20.0;

        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Width:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
            let new_item_w = NumberInput::new(
                self.properties_panel.widget_ids.layout_item_w_id,
                field_rect,
                item_w,
            )
            .blocked(blocked)
            .min(1.0)
            .show(ctx);
            if (new_item_w - item_w).abs() > 0.01 {
                self.push_element_update(|el| {
                    if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                        group.layout.item_width = new_item_w;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Height:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
            let new_item_h = NumberInput::new(
                self.properties_panel.widget_ids.layout_item_h_id,
                field_rect,
                item_h,
            )
            .blocked(blocked)
            .min(1.0)
            .show(ctx);
            if (new_item_h - item_h).abs() > 0.01 {
                self.push_element_update(|el| {
                    if let MenuElementKind::LayoutGroup(group) = &mut el.kind {
                        group.layout.item_height = new_item_h;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Navigation section
        *y += 8.0;
        if row_visible(*y, 20.0, clip) {
            ctx.draw_text("Navigation", x, *y + 14.0, 12.0, Color::GREY);
        }
        *y += 20.0;

        let nav_ids = self.properties_panel.widget_ids.layout_nav_ids;

        self.draw_nav_section::<LayoutGroupElement>(
            ctx,
            y,
            NavSectionStyle {
                x,
                w,
                blocked,
                clip,
            },
            &nav_ids,
        );
    }
}
