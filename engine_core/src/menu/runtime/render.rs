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

    widgets_frame_end(ctx);
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
            let display_text = env.text_manager.resolve_ui_text(env.text_id, &label.text_key);
            let screen_rect =
                normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
            MenuTemplate::render_label(ctx, label, screen_rect, &display_text);
        }
        MenuElementKind::Button(button) => {
            let display_text = env.text_manager.resolve_ui_text(env.text_id, &button.text_key);
            let is_focused = env.focus.node == element_index && env.focus.child.is_none();
            let screen_rect =
                normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
            let widget = Button::new(screen_rect, &display_text)
                .blocked(!element.enabled)
                .focused(is_focused);
            if widget.show(ctx) {
                *env.triggered_action = Some(button.action.clone());
            }
        }
        MenuElementKind::Panel(panel) => {
            let screen_rect =
                normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
            ctx.draw_rectangle(
                screen_rect.x,
                screen_rect.y,
                screen_rect.w,
                screen_rect.h,
                panel.background.render_color(),
            );
        }
        MenuElementKind::LayoutGroup(group) => {
            render_layout_group(ctx, template, group, element_index, element, env);
        }
        MenuElementKind::Slider(slider) => {
            let screen_rect =
                normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
            let is_focused = env.focus.node == element_index && env.focus.child.is_none();
            render_slider(
                ctx,
                slider,
                screen_rect,
                env.text_manager,
                env.text_id,
                env.slider_values,
                is_focused,
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

        let screen_rect =
            normalized_rect_to_screen(*rect, env.canvas_origin, env.canvas_size);
        match &child.element.kind {
            MenuElementKind::Label(label) => {
                let display_text = env.text_manager.resolve_ui_text(env.text_id, &label.text_key);
                MenuTemplate::render_label(ctx, label, screen_rect, &display_text);
            }
            MenuElementKind::Button(button) => {
                let display_text = env.text_manager.resolve_ui_text(env.text_id, &button.text_key);
                let is_focused =
                    env.focus.node == element_index && env.focus.child == Some(focusable_idx);
                let widget = Button::new(screen_rect, &display_text)
                    .blocked(!child.element.enabled)
                    .focused(is_focused);
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
                render_slider(
                    ctx,
                    slider,
                    screen_rect,
                    env.text_manager,
                    env.text_id,
                    env.slider_values,
                    is_focused,
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
    text_manager: &TextManager,
    text_id: &str,
    slider_values: &mut HashMap<String, f32>,
    is_focused: bool,
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
    let label_bg = if is_focused {
        HOVER_COLOR
    } else {
        FIELD_BACKGROUND_COLOR
    };
    ctx.draw_rectangle(
        label_rect.x,
        label_rect.y,
        label_rect.w,
        label_rect.h,
        label_bg,
    );
    let display_text = text_manager.resolve_ui_text(text_id, &slider.text_key);
    let label = LabelElement::default();
    MenuTemplate::render_label(ctx, &label, label_rect, &display_text);

    let (new_value, state) =
        gui_slider(ctx, slider.widget_id, slider_rect, slider.min, slider.max, value);
    if !matches!(state, SliderState::Unchanged) {
        slider_values.insert(slider.key.clone(), new_value);
        push_slider_event(slider.key.clone(), new_value);
    }

    let outline_color = if is_focused {
        Color::WHITE
    } else {
        Color::new(0.5, 0.5, 0.5, 1.0)
    };
    ctx.draw_rectangle_lines(
        screen_rect.x,
        screen_rect.y,
        screen_rect.w,
        screen_rect.h,
        2.0,
        outline_color,
    );
}
