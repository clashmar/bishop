use bishop::{Text, TextureLoader, WgpuContext};
use engine_core::animation::{update_entity_animations, ClipDef, ClipId};
use engine_core::assets::sprite_manager::SpriteManager;
use engine_core::assets::AssetRegistry;
use engine_core::controls::Controls;
use engine_core::ecs::{Animation, ColliderFrameKey, Ecs, Entity};
use engine_core::ui::measure_text;
use std::cell::RefCell;
use std::collections::HashSet;
use widgets::constants::colors;
use widgets::{Button, Rect};

use super::{AnimationModule, LABEL_FONT_SIZE, LABEL_Y_OFFSET};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationFrameEditTarget {
    pub entity: Entity,
    pub clip_id: ClipId,
    pub frame: ColliderFrameKey,
}

thread_local! {
    static ACTIVE_TARGET: RefCell<Option<AnimationFrameEditTarget>> = const { RefCell::new(None) };
}

pub fn enter(entity: Entity, animation: &Animation) -> Option<AnimationFrameEditTarget> {
    let clip_id = animation.current.clone()?;
    let frame = animation
        .states
        .get(&clip_id)
        .map(|state| ColliderFrameKey {
            row: state.row,
            col: state.col,
        })
        .unwrap_or(ColliderFrameKey { row: 0, col: 0 });
    let target = AnimationFrameEditTarget {
        entity,
        clip_id,
        frame,
    };
    ACTIVE_TARGET.with(|active| *active.borrow_mut() = Some(target.clone()));
    Some(target)
}

pub fn exit(entity: Entity) {
    ACTIVE_TARGET.with(|active| {
        if active.borrow().as_ref().is_some_and(|target| target.entity == entity) {
            *active.borrow_mut() = None;
        }
    });
}

pub fn exit_all() {
    ACTIVE_TARGET.with(|active| *active.borrow_mut() = None);
}

pub fn active_target(entity: Entity) -> Option<AnimationFrameEditTarget> {
    ACTIVE_TARGET.with(|active| {
        active
            .borrow()
            .as_ref()
            .filter(|target| target.entity == entity)
            .cloned()
    })
}

pub fn active_target_for_any_entity() -> Option<AnimationFrameEditTarget> {
    ACTIVE_TARGET.with(|active| active.borrow().clone())
}

pub fn is_active_for(entity: Entity) -> bool {
    active_target(entity).is_some()
}

pub fn set_clip(entity: Entity, animation: &Animation, clip_id: ClipId) {
    let frame = clamp_frame(animation, &clip_id, ColliderFrameKey { row: 0, col: 0 });
    ACTIVE_TARGET.with(|active| {
        *active.borrow_mut() = Some(AnimationFrameEditTarget {
            entity,
            clip_id,
            frame,
        });
    });
}

pub fn step_selected_frame(entity: Entity, animation: &Animation, delta: isize) {
    ACTIVE_TARGET.with(|active| {
        let mut guard = active.borrow_mut();
        let Some(target) = guard.as_mut().filter(|target| target.entity == entity) else {
            return;
        };
        target.frame = stepped_frame(animation, &target.clip_id, target.frame, delta);
    });
}

pub(crate) fn handle_shortcuts(
    ctx: &WgpuContext,
    ecs: &Ecs,
    selected_entity: Option<Entity>,
) -> bool {
    let Some(entity) = selected_entity else {
        return false;
    };
    if !is_active_for(entity) {
        return false;
    }
    let Some(animation) = ecs.get::<Animation>(entity) else {
        return false;
    };

    if Controls::left_bracket(ctx) {
        step_selected_frame(entity, animation, -1);
        return true;
    }
    if Controls::right_bracket(ctx) {
        step_selected_frame(entity, animation, 1);
        return true;
    }

    false
}

pub fn apply_pinned_frame(
    loader: &impl TextureLoader,
    ecs: &mut Ecs,
    asset_registry: &mut AssetRegistry,
    sprite_manager: &mut SpriteManager,
) {
    let Some(target) = active_target_for_any_entity() else {
        return;
    };
    let Some(animation) = ecs.get_mut::<Animation>(target.entity) else {
        exit(target.entity);
        return;
    };
    if !animation.clips.contains_key(&target.clip_id) {
        exit(target.entity);
        return;
    }
    animation.current = Some(target.clip_id.clone());
    if let Some(state) = animation.states.get_mut(&target.clip_id) {
        state.row = target.frame.row;
        state.col = target.frame.col;
        state.timer = 0.0;
        state.finished = false;
    }

    let entities = HashSet::from([target.entity]);
    update_entity_animations(loader, ecs, asset_registry, sprite_manager, 0.0, &entities);
}

pub(super) fn sync_selected_clip(entity: Entity, animation: &Animation, clip_id: ClipId) {
    if is_active_for(entity) {
        set_clip(entity, animation, clip_id);
    }
}

pub(super) fn current_frame_key(animation: &Animation) -> Option<ColliderFrameKey> {
    let clip_id = animation.current.as_ref()?;
    animation.states.get(clip_id).map(|state| ColliderFrameKey {
        row: state.row,
        col: state.col,
    })
}

pub(super) fn frame_edit_label(clip: &ClipDef, frame: ColliderFrameKey) -> String {
    let index = frame.row.saturating_mul(clip.cols).saturating_add(frame.col) + 1;
    let total = clip.cols.saturating_mul(clip.rows).max(1);
    format!("Frame {index} / {total}")
}

pub(super) fn draw_frame_edit_controls(
    ctx: &mut WgpuContext,
    module: &mut AnimationModule,
    entity: Entity,
    animation: &Animation,
    clip: &ClipDef,
    rect: Rect,
    blocked: bool,
) {
    let active = is_active_for(entity);
    let button_w = 70.0;
    let gap = 6.0;
    let toggle_rect = Rect::new(rect.x, rect.y, button_w, rect.h);
    let prev_rect = Rect::new(toggle_rect.x + button_w + gap, rect.y, 28.0, rect.h);
    let next_rect = Rect::new(prev_rect.x + 28.0 + gap, rect.y, 28.0, rect.h);

    let toggle_label = if active { "Live" } else { "Frame" };
    if Button::new(toggle_rect, toggle_label)
        .interaction_id(module.frame_edit_toggle_id)
        .active(active)
        .suppressed(blocked)
        .show(ctx)
    {
        if active {
            exit(entity);
        } else {
            enter(entity, animation);
        }
    }

    if active && Button::new(prev_rect, "<")
        .interaction_id(module.prev_frame_id)
        .suppressed(blocked)
        .show(ctx)
    {
        step_selected_frame(entity, animation, -1);
    }
    if active && Button::new(next_rect, ">")
        .interaction_id(module.next_frame_id)
        .suppressed(blocked)
        .show(ctx)
    {
        step_selected_frame(entity, animation, 1);
    }

    let frame = active_target(entity)
        .map(|target| target.frame)
        .or_else(|| current_frame_key(animation))
        .unwrap_or(ColliderFrameKey { row: 0, col: 0 });
    let label = frame_edit_label(clip, frame);
    let label_w = measure_text(ctx, &label, LABEL_FONT_SIZE).width;
    let min_label_x = next_rect.x + 28.0 + gap;
    let label_x = (rect.x + rect.w - label_w).max(min_label_x);
    ctx.draw_text(
        &label,
        label_x,
        rect.y + LABEL_Y_OFFSET,
        LABEL_FONT_SIZE,
        colors::DEFAULT_TEXT_COLOR,
    );
}

fn stepped_frame(
    animation: &Animation,
    clip_id: &ClipId,
    frame: ColliderFrameKey,
    delta: isize,
) -> ColliderFrameKey {
    let Some(clip) = animation.clips.get(clip_id) else {
        return frame;
    };
    let frame_count = clip.cols.saturating_mul(clip.rows).max(1);
    let current = frame.row.saturating_mul(clip.cols).saturating_add(frame.col);
    let next = (current as isize + delta).rem_euclid(frame_count as isize) as usize;
    ColliderFrameKey {
        row: next / clip.cols.max(1),
        col: next % clip.cols.max(1),
    }
}

fn clamp_frame(
    animation: &Animation,
    clip_id: &ClipId,
    frame: ColliderFrameKey,
) -> ColliderFrameKey {
    let Some(clip) = animation.clips.get(clip_id) else {
        return ColliderFrameKey { row: 0, col: 0 };
    };
    ColliderFrameKey {
        row: frame.row.min(clip.rows.saturating_sub(1)),
        col: frame.col.min(clip.cols.saturating_sub(1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::animation::{ClipDef, ClipState};
    use std::collections::HashMap;

    fn animation_with_run_clip(cols: usize, rows: usize) -> Animation {
        let mut animation = Animation {
            clips: HashMap::from([(
                ClipId::Run,
                ClipDef {
                    cols,
                    rows,
                    ..Default::default()
                },
            )]),
            current: Some(ClipId::Run),
            ..Default::default()
        };
        animation.states.insert(ClipId::Run, ClipState::default());
        animation
    }

    #[test]
    fn frame_label_uses_one_based_index_and_total() {
        let clip = ClipDef {
            cols: 4,
            rows: 2,
            ..Default::default()
        };
        let frame = ColliderFrameKey { row: 1, col: 2 };

        assert_eq!(frame_edit_label(&clip, frame), "Frame 7 / 8");
    }

    #[test]
    fn selected_frame_uses_current_clip_state() {
        let mut animation = Animation {
            current: Some(ClipId::Run),
            ..Default::default()
        };
        animation.clips.insert(
            ClipId::Run,
            ClipDef {
                cols: 3,
                rows: 2,
                ..Default::default()
            },
        );
        animation.states.insert(
            ClipId::Run,
            ClipState {
                row: 1,
                col: 2,
                ..Default::default()
            },
        );

        assert_eq!(
            current_frame_key(&animation),
            Some(ColliderFrameKey { row: 1, col: 2 }),
        );
    }

    #[test]
    fn step_frame_advances_within_clip_bounds() {
        let entity = Entity(1);
        let animation = animation_with_run_clip(3, 2);
        enter(entity, &animation);

        step_selected_frame(entity, &animation, 1);
        assert_eq!(
            active_target(entity).map(|target| target.frame),
            Some(ColliderFrameKey { row: 0, col: 1 }),
        );

        step_selected_frame(entity, &animation, 4);
        assert_eq!(
            active_target(entity).map(|target| target.frame),
            Some(ColliderFrameKey { row: 1, col: 2 }),
        );

        step_selected_frame(entity, &animation, 1);
        assert_eq!(
            active_target(entity).map(|target| target.frame),
            Some(ColliderFrameKey { row: 0, col: 0 }),
        );

        exit(entity);
    }

    #[test]
    fn exit_clears_only_matching_entity() {
        let first = Entity(1);
        let second = Entity(2);
        let animation = animation_with_run_clip(2, 1);

        enter(first, &animation);
        exit(second);
        assert!(active_target(first).is_some());

        exit(first);
        assert!(active_target(first).is_none());
    }

    #[test]
    fn exit_all_clears_active_target() {
        let entity = Entity(3);
        let animation = animation_with_run_clip(2, 1);

        enter(entity, &animation);
        exit_all();

        assert!(active_target(entity).is_none());
    }
}
