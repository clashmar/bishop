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
        current_element_index: 0,
    };

    template.render_background(ctx, viewport);

    for element_index in template.sorted_element_indices() {
        let element = &template.elements[element_index];
        if !element.visible {
            continue;
        }

        env.current_element_index = element_index;
        render_element(ctx, element, &mut env);
    }

    triggered_action
}

pub(crate) struct RenderEnv<'a> {
    pub(crate) text_id: &'a str,
    pub(crate) text_manager: &'a TextManager,
    pub(crate) canvas_origin: Vec2,
    pub(crate) canvas_size: Vec2,
    pub(crate) focus: &'a MenuFocus,
    pub(crate) slider_values: &'a mut HashMap<String, f32>,
    pub(crate) triggered_action: &'a mut Option<MenuAction>,
    pub(crate) current_element_index: usize,
}

fn render_element<C: BishopContext>(ctx: &mut C, element: &MenuElement, env: &mut RenderEnv<'_>) {
    let screen_rect = normalized_rect_to_screen(element.rect, env.canvas_origin, env.canvas_size);
    let is_focused = env.focus.node == env.current_element_index && env.focus.child.is_none();

    let action = match &element.kind {
        MenuElementKind::Label(l) => l.render(ctx, element, screen_rect, false, env),
        MenuElementKind::Button(b) => b.render(ctx, element, screen_rect, is_focused, env),
        MenuElementKind::Panel(p) => p.render(ctx, element, screen_rect, false, env),
        MenuElementKind::Slider(s) => s.render(ctx, element, screen_rect, is_focused, env),
        MenuElementKind::LayoutGroup(g) => g.render(ctx, element, screen_rect, false, env),
    };

    if let Some(a) = action {
        *env.triggered_action = Some(a);
    }
}
