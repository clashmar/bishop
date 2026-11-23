use crate::editor_assets::assets::circle_120px;
use crate::game::world_selection::WorldSelection;
use crate::shared::selection::{
    box_selection_drag_started,
    draw_selection_box,
    rect_from_two_points,
    rects_intersect,
};
use bishop::prelude::{Rect, Vec2, WgpuContext};
use engine_core::{
    ecs::SpriteId,
    game::Game,
    worlds::WorldId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PointerDownAction {
    StartMarquee,
    StartDrag(WorldId),
    None,
}

#[derive(Default)]
pub(super) struct WorldsSubmode {
    marquee_start_world: Option<Vec2>,
    marquee_start_screen: Option<Vec2>,
    marquee_current_world: Option<Vec2>,
    world_drag_start_mouse: Option<Vec2>,
    world_drag_start_positions: Vec<(WorldId, Vec2)>,
}

impl WorldsSubmode {
    pub(super) fn begin_marquee(&mut self, world_position: Vec2, screen_position: Vec2) {
        self.marquee_start_world = Some(world_position);
        self.marquee_start_screen = Some(screen_position);
        self.marquee_current_world = Some(world_position);
    }

    pub(super) fn update_marquee(&mut self, world_position: Vec2) {
        if self.marquee_start_world.is_some() {
            self.marquee_current_world = Some(world_position);
        }
    }

    pub(super) fn is_marquee_active(&self) -> bool {
        self.marquee_start_world.is_some()
    }

    pub(super) fn finish_marquee(
        &mut self,
        selection: &mut WorldSelection,
        end_world: Vec2,
        end_screen: Vec2,
        world_bounds: &[(WorldId, Rect)],
    ) -> bool {
        let (Some(start_world), Some(start_screen)) =
            (self.marquee_start_world.take(), self.marquee_start_screen.take())
        else {
            return false;
        };
        self.marquee_current_world = None;

        apply_marquee(
            selection,
            start_world,
            end_world,
            start_screen,
            end_screen,
            world_bounds,
        )
    }

    pub(super) fn draw_marquee(&self, ctx: &mut WgpuContext, grid_size: f32) {
        if let (Some(start), Some(end)) = (self.marquee_start_world, self.marquee_current_world) {
            draw_selection_box(ctx, start, end, grid_size);
        }
    }

    pub(super) fn begin_world_drag(
        &mut self,
        mouse_position: Vec2,
        world_positions: Vec<(WorldId, Vec2)>,
    ) {
        self.world_drag_start_mouse = Some(mouse_position);
        self.world_drag_start_positions = world_positions;
    }

    pub(super) fn is_world_drag_active(&self) -> bool {
        self.world_drag_start_mouse.is_some()
    }

    pub(super) fn world_drag_delta(&self, mouse_position: Vec2) -> Option<Vec2> {
        self.world_drag_start_mouse
            .map(|start| mouse_position - start)
    }

    pub(super) fn world_drag_start_positions(&self) -> &[(WorldId, Vec2)] {
        &self.world_drag_start_positions
    }

    pub(super) fn finish_world_drag(&mut self) -> Vec<(WorldId, Vec2)> {
        self.world_drag_start_mouse = None;
        std::mem::take(&mut self.world_drag_start_positions)
    }
}

pub(super) fn handle_pointer_down(
    selection: &mut WorldSelection,
    hit_world: Option<WorldId>,
    shift_held: bool,
) -> PointerDownAction {
    match hit_world {
        Some(world_id) if shift_held => {
            selection.toggle(world_id);
            PointerDownAction::None
        }
        Some(world_id) => {
            if selection.contains(world_id) {
                selection.set_primary(world_id);
            } else {
                selection.select_only(world_id);
            }
            PointerDownAction::StartDrag(world_id)
        }
        None => {
            if !shift_held {
                selection.clear();
            }
            PointerDownAction::StartMarquee
        }
    }
}

pub(super) fn world_entry_request(
    action: PointerDownAction,
    double_clicked: bool,
) -> Option<WorldId> {
    match action {
        PointerDownAction::StartDrag(world_id) if double_clicked => Some(world_id),
        _ => None,
    }
}

pub(super) fn world_rect(position: Vec2, size: Vec2) -> Rect {
    Rect::new(position.x, position.y, size.x, size.y)
}

pub(super) fn world_bounds(ctx: &mut WgpuContext, game: &mut Game) -> Vec<(WorldId, Rect)> {
    let world_data: Vec<(WorldId, Vec2, Option<SpriteId>)> = game
        .worlds()
        .iter()
        .map(|world| (world.id, world.meta.position, world.meta.sprite_id))
        .collect();

    world_data
        .into_iter()
        .map(|(world_id, position, sprite_id)| {
            let texture = match sprite_id {
                Some(id) => game.sprite_manager.get_texture_from_id(ctx, id),
                None => circle_120px(),
            };
            (
                world_id,
                world_rect(position, Vec2::new(texture.width(), texture.height())),
            )
        })
        .collect()
}

pub(super) fn apply_marquee(
    selection: &mut WorldSelection,
    start_world: Vec2,
    end_world: Vec2,
    start_screen: Vec2,
    end_screen: Vec2,
    world_bounds: &[(WorldId, Rect)],
) -> bool {
    if !box_selection_drag_started(start_screen, end_screen) {
        return false;
    }

    let marquee = rect_from_two_points(start_world, end_world);
    let hits: Vec<WorldId> = world_bounds
        .iter()
        .filter(|(_, bounds)| rects_intersect(marquee, *bounds))
        .map(|(world_id, _)| *world_id)
        .collect();

    selection.add_all_with_minimum_primary(&hits)
}

#[cfg(test)]
mod tests;
