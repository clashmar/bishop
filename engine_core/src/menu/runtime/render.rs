use crate::menu::*;
use crate::text::TextManager;
use bishop::prelude::*;
use std::collections::HashMap;
use widgets::*;

/// Renders the currently active menu and returns a triggered button action.
pub(crate) fn render_active_menu<C: BishopContext>(
    ctx: &mut C,
    template: &MenuTemplate,
    menu_id: &str,
    viewport: Rect,
    focus: &MenuFocus,
    slider_values: &mut HashMap<String, f32>,
    text_manager: &TextManager,
) -> Option<MenuAction> {
    widgets_frame_start(ctx);
    let action = render_menu_elements(
        ctx,
        template,
        menu_id,
        viewport,
        focus,
        slider_values,
        text_manager,
    );
    widgets_frame_end(ctx);
    action
}

/// Renders all menu elements into the given viewport rect using screen-space coordinates.
pub fn render_menu_elements<C: BishopContext>(
    ctx: &mut C,
    template: &MenuTemplate,
    menu_id: &str,
    viewport: Rect,
    focus: &MenuFocus,
    slider_values: &mut HashMap<String, f32>,
    text_manager: &TextManager,
) -> Option<MenuAction> {
    let text_id = format!("ui/{}", menu_id);
    let mut triggered_action = None;
    let mut env = RenderEnv {
        text_id: &text_id,
        text_manager,
        canvas_origin: Vec2::new(viewport.x, viewport.y),
        canvas_size: Vec2::new(viewport.w, viewport.h),
        focus,
        slider_values,
        triggered_action: &mut triggered_action,
    };

    template.render_background(ctx, viewport);

    for element_index in template.sorted_element_indices() {
        let element = &template.elements[element_index];
        if !element.visible {
            continue;
        }

        render_element(ctx, template, element_index, element, &mut env);
    }

    triggered_action
}

struct RenderEnv<'a> {
    text_id: &'a str,
    text_manager: &'a TextManager,
    canvas_origin: Vec2,
    canvas_size: Vec2,
    focus: &'a MenuFocus,
    slider_values: &'a mut HashMap<String, f32>,
    triggered_action: &'a mut Option<MenuAction>,
}

fn render_element<C: BishopContext>(
    ctx: &mut C,
    template: &MenuTemplate,
    element_index: usize,
    element: &MenuElement,
    env: &mut RenderEnv<'_>,
) {
    match &element.kind {
        MenuElementKind::Label(label) => {
            let display_text = env
                .text_manager
                .resolve_ui_text(env.text_id, &label.text_key);
            let screen_rect =
                normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
            let label_align = match label.alignment {
                HorizontalAlign::Left => LabelAlign::Left,
                HorizontalAlign::Center => LabelAlign::Center,
                HorizontalAlign::Right => LabelAlign::Right,
            };
            Label::new(screen_rect, display_text)
                .font_size(label.font_size)
                .alignment(label_align)
                .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
                .show(ctx);
        }
        MenuElementKind::Button(button) => {
            let display_text = env
                .text_manager
                .resolve_ui_text(env.text_id, &button.text_key);
            let is_focused = env.focus.node == element_index && env.focus.child.is_none();
            let screen_rect =
                normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
            let widget = Button::new(screen_rect, &display_text)
                .blocked(!element.enabled)
                .focused(is_focused)
                .apply_selectors(element.class.as_deref(), element.style_id.as_deref());
            if widget.show(ctx) {
                *env.triggered_action = Some(button.action.clone());
            }
        }
        MenuElementKind::Panel(_panel) => {
            let screen_rect =
                normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
            Panel::new(screen_rect)
                .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
                .show(ctx);
        }
        MenuElementKind::LayoutGroup(group) => {
            render_layout_group(ctx, template, group, element_index, element, env);
        }
        MenuElementKind::Slider(slider) => {
            let screen_rect =
                normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
            let is_focused = env.focus.node == element_index && env.focus.child.is_none();
            let display_text = env
                .text_manager
                .resolve_ui_text(env.text_id, &slider.text_key);
            render_slider(
                ctx,
                slider,
                screen_rect,
                &display_text,
                env.slider_values,
                is_focused,
                element,
            );
        }
    }
}

fn render_layout_group<C: BishopContext>(
    ctx: &mut C,
    _template: &MenuTemplate,
    group: &LayoutGroupElement,
    element_index: usize,
    element: &MenuElement,
    env: &mut RenderEnv<'_>,
) {
    if let Some(bg) = &group.background {
        let screen_rect =
            normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
        ctx.draw_rectangle(
            screen_rect.x,
            screen_rect.y,
            screen_rect.w,
            screen_rect.h,
            bg.render_color(),
        );
    }

    let resolved = resolve_layout(group, element.rect);
    let mut focusable_idx = 0;

    for (child, rect) in group.children.iter().zip(resolved.iter()) {
        if !child.element.visible {
            continue;
        }

        let screen_rect = normalized_rect_to_screen(*rect, env.canvas_origin, env.canvas_size);
        match &child.element.kind {
            MenuElementKind::Label(label) => {
                let display_text = env
                    .text_manager
                    .resolve_ui_text(env.text_id, &label.text_key);
                let label_align = match label.alignment {
                    HorizontalAlign::Left => LabelAlign::Left,
                    HorizontalAlign::Center => LabelAlign::Center,
                    HorizontalAlign::Right => LabelAlign::Right,
                };
                Label::new(screen_rect, display_text)
                    .font_size(label.font_size)
                    .alignment(label_align)
                    .apply_selectors(
                        child.element.class.as_deref(),
                        child.element.style_id.as_deref(),
                    )
                    .show(ctx);
            }
            MenuElementKind::Button(button) => {
                let display_text = env
                    .text_manager
                    .resolve_ui_text(env.text_id, &button.text_key);
                let is_focused =
                    env.focus.node == element_index && env.focus.child == Some(focusable_idx);
                let widget = Button::new(screen_rect, &display_text)
                    .blocked(!child.element.enabled)
                    .focused(is_focused)
                    .apply_selectors(
                        child.element.class.as_deref(),
                        child.element.style_id.as_deref(),
                    );
                if widget.show(ctx) {
                    *env.triggered_action = Some(button.action.clone());
                }
                if child.element.enabled {
                    focusable_idx += 1;
                }
            }
            MenuElementKind::Slider(slider) => {
                let is_focused =
                    env.focus.node == element_index && env.focus.child == Some(focusable_idx);
                let display_text = env
                    .text_manager
                    .resolve_ui_text(env.text_id, &slider.text_key);
                render_slider(
                    ctx,
                    slider,
                    screen_rect,
                    &display_text,
                    env.slider_values,
                    is_focused,
                    &child.element,
                );
                if child.element.enabled {
                    focusable_idx += 1;
                }
            }
            _ => {}
        }
    }
}

fn render_slider<C: BishopContext>(
    ctx: &mut C,
    slider: &SliderElement,
    screen_rect: Rect,
    display_text: &str,
    slider_values: &mut HashMap<String, f32>,
    is_focused: bool,
    element: &MenuElement,
) {
    let value = slider_values
        .get(&slider.key)
        .copied()
        .unwrap_or(slider.default_value);
    let split = screen_rect.w * 0.4;
    let label_rect = Rect::new(screen_rect.x, screen_rect.y, split, screen_rect.h);
    let slider_rect = Rect::new(
        screen_rect.x + split,
        screen_rect.y,
        screen_rect.w - split,
        screen_rect.h,
    );
    let label_bg = with_theme(|t| if is_focused { t.hover } else { t.background });
    ctx.draw_rectangle(
        label_rect.x,
        label_rect.y,
        label_rect.w,
        label_rect.h,
        label_bg,
    );
    Label::new(label_rect, display_text)
        .font_size(14.0)
        .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
        .show(ctx);

    let (new_value, state) =
        Slider::new(slider.widget_id, slider_rect, slider.min, slider.max, value)
            .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
            .show(ctx);
    if !matches!(state, SliderState::Unchanged) {
        slider_values.insert(slider.key.clone(), new_value);
        push_slider_event(slider.key.clone(), new_value);
    }

    let outline_color = with_theme(|t| if is_focused { t.highlight } else { t.border });
    ctx.draw_rectangle_lines(
        screen_rect.x,
        screen_rect.y,
        screen_rect.w,
        screen_rect.h,
        2.0,
        outline_color,
    );
}
