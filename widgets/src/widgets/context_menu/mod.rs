use crate::constants::{colors, layout};
use crate::*;
use std::cell::RefCell;

pub mod state;
pub use state as context_menu_state;

const W_PADDING: f32 = 8.0;
const ROW_HEIGHT: f32 = layout::DEFAULT_FIELD_HEIGHT;

pub struct ContextMenuItem<T> {
    pub label: String,
    pub value: T,
}

struct DeferredContextMenuRender {
    rect: Rect,
    row_height: f32,
    labels: Vec<String>,
    hovered_index: Option<usize>,
    font_size: f32,
    overrides: WidgetTheme,
}

thread_local! {
    static DEFERRED_CONTEXT_MENU_RENDERS: RefCell<Vec<DeferredContextMenuRender>> =
        const { RefCell::new(Vec::new()) };
}

fn context_menu_entry_click_target(id: WidgetId, index: usize) -> ClickTargetId {
    const ENTRY_SALT: u64 = 0x434F4E544558544D;
    ClickTargetId(((id.0 as u64) << 32) ^ index as u64 ^ ENTRY_SALT)
}

pub struct ContextMenu<'a, T> {
    id: WidgetId,
    position: Vec2,
    items: &'a [ContextMenuItem<T>],
    suppressed: bool,
    font_size: f32,
    base: WidgetBase,
}

impl<'a, T: Clone + PartialEq + 'static> ContextMenu<'a, T> {
    pub fn new(id: WidgetId, position: Vec2, items: &'a [ContextMenuItem<T>]) -> Self {
        Self {
            id,
            position,
            items,
            suppressed: false,
            font_size: layout::DEFAULT_FONT_SIZE_16,
            base: WidgetBase {
                blocked: false,
                overrides: WidgetTheme::default(),
                ..WidgetBase::default()
            },
        }
    }

    pub fn suppressed(mut self, suppressed: bool) -> Self {
        self.suppressed = suppressed;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.base.overrides.text = Some(color.into());
        self
    }

    pub fn show<C: BishopContext>(self, ctx: &mut C) -> Option<T> {
        let class = self.base.class_name.as_deref();
        let id = self.base.style_id.as_deref();
        let widget_theme = resolve_theme_for::<Self>(class, id);
        if self.items.is_empty() {
            return None;
        }

        let mut state = context_menu_state::get(self.id);
        let interactive = !self.base.blocked && !self.suppressed && !is_modal_open();
        let mouse_pos: Vec2 = ctx.mouse_position().into();

        if ctx.is_mouse_button_pressed(MouseButton::Right) && interactive {
            if state.open && !state.rect.contains(mouse_pos) {
                state.open = false;
                consume_click();
            } else if !state.open {
                state.open = true;
                state.just_opened = true;
                close_open_dropdowns();
                consume_click();
            }
        }

        if ctx.is_mouse_button_pressed(MouseButton::Left) && state.open {
            let inside = state.rect.contains(mouse_pos);
            if !inside {
                state.open = false;
                consume_click();
            }
        }

        let mut result: Option<T> = None;

        if state.open {
            if state.just_opened {
                state.just_opened = false;
                context_menu_state::set(self.id, state);
                update_global_context_menu_flag();
                return None;
            }

            let popup_rect = self.compute_rect(ctx);
            state.rect = popup_rect;

            let mut hovered_index: Option<usize> = None;
            for (i, item) in self.items.iter().enumerate() {
                let entry_y = popup_rect.y + i as f32 * ROW_HEIGHT;
                let entry_rect = Rect::new(popup_rect.x, entry_y, popup_rect.w, ROW_HEIGHT);
                if entry_rect.contains(mouse_pos) {
                    hovered_index = Some(i);
                }
                if interactive
                    && activate_on_release(
                        MouseButton::Left,
                        context_menu_entry_click_target(self.id, i),
                        entry_rect.contains(mouse_pos),
                        true,
                        ctx.is_mouse_button_pressed(MouseButton::Left),
                        ctx.is_mouse_button_released(MouseButton::Left),
                    )
                {
                    state.open = false;
                    result = Some(item.value.clone());
                    consume_click();
                    break;
                }
            }

            let labels: Vec<String> = self.items.iter().map(|i| i.label.clone()).collect();
            context_menu_state::set(self.id, state);
            update_global_context_menu_flag();
            push_deferred_render(
                popup_rect,
                ROW_HEIGHT,
                labels,
                hovered_index,
                self.font_size,
                self.base.overrides.merge(widget_theme),
            );
            return result;
        }

        context_menu_state::set(self.id, state);
        update_global_context_menu_flag();
        result
    }

    fn compute_rect<C: BishopContext>(&self, ctx: &C) -> Rect {
        let mut max_width = 0.0_f32;
        for item in self.items.iter() {
            let w = measure_text_ui(ctx, &item.label, self.font_size).width;
            if w > max_width {
                max_width = w;
            }
        }
        let width = max_width + 2.0 * W_PADDING;
        let height = ROW_HEIGHT * self.items.len() as f32;

        let mut x = self.position.x;
        let mut y = self.position.y;

        if x + width > ctx.screen_width() {
            x = (self.position.x - width).max(0.0);
        }
        if y + height > ctx.screen_height() {
            y = (self.position.y - height).max(0.0);
        }

        Rect::new(x, y, width, height)
    }
}

impl<T> Widget for ContextMenu<'_, T> {
    fn widget_type() -> WidgetType {
        WidgetType::ContextMenu
    }
    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
}

fn push_deferred_render(
    rect: Rect,
    row_height: f32,
    labels: Vec<String>,
    hovered_index: Option<usize>,
    font_size: f32,
    overrides: WidgetTheme,
) {
    DEFERRED_CONTEXT_MENU_RENDERS.with(|renders| {
        renders.borrow_mut().push(DeferredContextMenuRender {
            rect,
            row_height,
            labels,
            hovered_index,
            font_size,
            overrides,
        });
    });
}

fn update_global_context_menu_flag() {
    let any = context_menu_state::any_open();
    set_context_menu_open(any);
}

pub fn close_open_context_menus() {
    context_menu_state::close_all();
    update_global_context_menu_flag();
}

pub fn flush_context_menu<C: BishopContext>(ctx: &mut C) {
    DEFERRED_CONTEXT_MENU_RENDERS.with(|renders| {
        for render in renders.borrow_mut().drain(..) {
            render_context_menu(ctx, render);
        }
    });
}

fn render_context_menu<C: BishopContext>(ctx: &mut C, render: DeferredContextMenuRender) {
    const CONTEXT_HOVER: Color = Color::new(0.35, 0.35, 0.35, 0.9);

    ctx.draw_rectangle(
        render.rect.x,
        render.rect.y,
        render.rect.w,
        render.rect.h,
        resolve(
            render.overrides.background,
            colors::DEFAULT_BACKGROUND_COLOR,
        ),
    );

    let mouse_pos: Vec2 = ctx.mouse_position().into();

    for (i, label) in render.labels.iter().enumerate() {
        let entry_y = render.rect.y + i as f32 * render.row_height;
        let entry_rect = Rect::new(render.rect.x, entry_y, render.rect.w, render.row_height);

        if render.hovered_index == Some(i) || entry_rect.contains(mouse_pos) {
            ctx.draw_rectangle(
                entry_rect.x,
                entry_rect.y,
                entry_rect.w,
                entry_rect.h,
                resolve(render.overrides.hover, CONTEXT_HOVER),
            );
        }

        draw_text_ui(
            ctx,
            label,
            entry_rect.x + W_PADDING,
            entry_rect.y + entry_rect.h * 0.7,
            render.font_size,
            resolve(render.overrides.text, colors::DEFAULT_TEXT_COLOR),
        );
    }

    ctx.draw_rectangle_lines(
        render.rect.x,
        render.rect.y,
        render.rect.w,
        render.rect.h,
        2.,
        resolve(render.overrides.border, colors::DEFAULT_BORDER_COLOR),
    );
}

pub fn is_mouse_over_context_menu<C: BishopContext>(ctx: &C) -> bool {
    let mouse_pos: Vec2 = ctx.mouse_position().into();
    context_menu_state::STATE.with(|s| {
        s.borrow()
            .values()
            .any(|st| st.open && st.rect.contains(mouse_pos))
    })
}

#[cfg(test)]
mod tests;
