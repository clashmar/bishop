use super::super::PropertyModule;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::{CurrentRoom, Entity, WorldEntry};
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;
use ::widgets::constants::layout;
use ::widgets::*;

const ROW_H: f32 = 24.0;

/// Read-only list of all `WorldEntry` entities in the current world.
pub struct WorldEntriesModule {
    button_ids: Vec<WidgetId>,
    pending_selection: Option<Entity>,
    entry_count: usize,
}

impl WorldEntriesModule {
    pub fn new() -> Self {
        Self { button_ids: Vec::new(), pending_selection: None, entry_count: 0 }
    }
}

impl Default for WorldEntriesModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldEntriesModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _world: &mut World,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let Some(world) = game_ctx.world.as_deref() else { return };
        let room_world_map = game_ctx.room_world_map.clone();
        let world_id = world.id;

        let entries: Vec<(Entity, String, String)> = game_ctx.ecs.get_store::<WorldEntry>()
            .data.iter()
            .filter(|(entity, _)| {
                game_ctx.ecs.get::<CurrentRoom>(**entity)
                    .and_then(|r| room_world_map.get(&r.0).copied())
                    .is_some_and(|wid| wid == world_id)
            })
            .map(|(entity, entry)| {
                let room_name = game_ctx.ecs.get::<CurrentRoom>(*entity)
                    .and_then(|r| world.get_room(r.0))
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| "?".to_string());
                (*entity, entry.name.clone(), room_name)
            })
            .collect();

        while self.button_ids.len() < entries.len() {
            self.button_ids.push(WidgetId::default());
        }
        self.entry_count = entries.len();

        let mut y = rect.y + layout::WIDGET_SPACING;
        for (i, (entity, name, room)) in entries.iter().enumerate() {
            let row_rect = Rect::new(rect.x, y, rect.w, ROW_H);
            let label = format!("{name}  ({room})");
            if Button::new(row_rect, &label)
                .interaction_id(self.button_ids[i])
                .show(ctx)
            {
                self.pending_selection = Some(*entity);
            }
            y += ROW_H + 2.0;
        }
    }

    fn take_host_action(&mut self) -> Option<InspectorHostAction> {
        self.pending_selection
            .take()
            .map(InspectorHostAction::SelectEntity)
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let count = self.entry_count.max(1);
        InspectorBodyLayout::new().rows(count, 2.0)
    }

    fn title(&self) -> &str {
        "Entries"
    }
}
