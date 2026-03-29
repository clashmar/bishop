// engine_core/src/controls/controls.rs
use bishop::prelude::*;

pub struct Controls;

impl Controls {
    pub fn save(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::S)
            && (ctx.is_key_down(KeyCode::LeftSuper))
            && !(ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift))
    }

    pub fn save_as(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::S)
            && (ctx.is_key_down(KeyCode::LeftSuper))
            && (ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift))
    }

    pub fn undo(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::Z)
            && (ctx.is_key_down(KeyCode::LeftSuper))
            && !(ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift))
    }

    pub fn redo(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::Z)
            && ctx.is_key_down(KeyCode::LeftSuper)
            && (ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift))
    }

    pub fn delete(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::Backspace)
    }

    pub fn copy(ctx: &WgpuContext) -> bool {
        (ctx.is_key_down(KeyCode::LeftSuper)) && ctx.is_key_pressed(KeyCode::C)
    }

    pub fn paste(ctx: &WgpuContext) -> bool {
        (ctx.is_key_down(KeyCode::LeftSuper)) && ctx.is_key_pressed(KeyCode::V)
    }

    pub fn select_all(ctx: &WgpuContext) -> bool {
        (ctx.is_key_down(KeyCode::LeftSuper)) && ctx.is_key_pressed(KeyCode::A)
    }

    pub fn duplicate(ctx: &WgpuContext) -> bool {
        (ctx.is_key_down(KeyCode::LeftSuper)) && ctx.is_key_pressed(KeyCode::D)
    }

    pub fn escape(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::Escape) && modifier_not_pressed(ctx)
    }

    pub fn enter(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::Enter) && modifier_not_pressed(ctx)
    }

    pub fn c(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::C) && modifier_not_pressed(ctx)
    }

    pub fn d(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::D) && modifier_not_pressed(ctx)
    }

    pub fn e(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::E) && modifier_not_pressed(ctx)
    }

    pub fn g(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::G) && modifier_not_pressed(ctx)
    }

    pub fn h(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::H) && modifier_not_pressed(ctx)
    }

    pub fn m(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::M) && modifier_not_pressed(ctx)
    }

    pub fn n(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::N) && modifier_not_pressed(ctx)
    }

    pub fn r(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::R) && modifier_not_pressed(ctx)
    }

    pub fn s(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::S) && modifier_not_pressed(ctx)
    }

    pub fn t(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::T) && modifier_not_pressed(ctx)
    }

    pub fn v(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::V) && modifier_not_pressed(ctx)
    }

    pub fn f3(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::F3) && modifier_not_pressed(ctx)
    }

    pub fn tab(ctx: &WgpuContext) -> bool {
        ctx.is_key_pressed(KeyCode::Tab) && modifier_not_pressed(ctx)
    }

    /// Returns true if any key was pressed this frame.
    pub fn any_key_pressed(ctx: &WgpuContext) -> bool {
        ctx.any_key_pressed()
    }

    /// Returns true if alt key is currently held.
    pub fn alt_held(ctx: &WgpuContext) -> bool {
        ctx.is_key_down(KeyCode::LeftAlt) || ctx.is_key_down(KeyCode::RightAlt)
    }
}

fn modifier_not_pressed(ctx: &WgpuContext) -> bool {
    !ctx.is_key_down(KeyCode::LeftControl)
        && !ctx.is_key_down(KeyCode::RightControl)
        && !ctx.is_key_down(KeyCode::LeftShift)
        && !ctx.is_key_down(KeyCode::RightShift)
        && !ctx.is_key_down(KeyCode::LeftAlt)
        && !ctx.is_key_down(KeyCode::RightAlt)
        && !ctx.is_key_down(KeyCode::LeftSuper)
        && !ctx.is_key_down(KeyCode::RightSuper)
}

pub fn get_omni_input(ctx: &WgpuContext) -> Vec2 {
    let mut dir = Vec2::ZERO;

    if ctx.is_key_down(KeyCode::Right) {
        dir.x += 1.0;
    }
    if ctx.is_key_down(KeyCode::Left) {
        dir.x -= 1.0;
    }
    if ctx.is_key_down(KeyCode::Down) {
        dir.y += 1.0;
    }
    if ctx.is_key_down(KeyCode::Up) {
        dir.y -= 1.0;
    }

    if dir.length_squared() > 0.0 {
        dir.normalize()
    } else {
        dir
    }
}

pub fn get_omni_input_pressed(ctx: &WgpuContext) -> Vec2 {
    let mut dir = Vec2::ZERO;

    if ctx.is_key_pressed(KeyCode::Right) {
        dir.x += 1.0;
    }
    if ctx.is_key_pressed(KeyCode::Left) {
        dir.x -= 1.0;
    }
    if ctx.is_key_pressed(KeyCode::Down) {
        dir.y += 1.0;
    }
    if ctx.is_key_pressed(KeyCode::Up) {
        dir.y -= 1.0;
    }

    if dir.length_squared() > 0.0 {
        dir.normalize()
    } else {
        dir
    }
}

pub fn get_horizontal_input(ctx: &WgpuContext) -> f32 {
    let mut dir_x = 0.0;

    if ctx.is_key_down(KeyCode::Right) {
        dir_x += 1.0;
    }
    if ctx.is_key_down(KeyCode::Left) {
        dir_x -= 1.0;
    }

    dir_x
}
