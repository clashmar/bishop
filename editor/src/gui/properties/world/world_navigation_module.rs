use super::super::PropertyModule;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::{CurrentRoom, Entity, WorldEntry, WorldExit};
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;
use engine_core::worlds::ExitDestination;
use ::widgets::constants::{colors, layout};
use ::widgets::*;

const ROW_H: f32 = 24.0;
const ROW_GAP: f32 = 8.0;

/// Lists WorldEntry and WorldExit entities in the current world.
pub struct WorldNavigationModule {
    /// Whether entries and exits are drawn on the world editor canvas.
    pub show_icons: bool,
    entry_ids: Vec<WidgetId>,
    exit_ids: Vec<WidgetId>,
    entry_count: usize,
    exit_count: usize,
    pending_focus: Option<Entity>,
    show_start_warning: bool,
    warning_height: f32,
}

impl WorldNavigationModule {
    /// Creates a new navigation module with icon visibility enabled.
    pub fn new() -> Self {
        Self {
            show_icons: true,
            entry_ids: Vec::new(),
            exit_ids: Vec::new(),
            entry_count: 0,
            exit_count: 0,
            pending_focus: None,
            show_start_warning: false,
            warning_height: 0.0,
        }
    }
}

impl Default for WorldNavigationModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldNavigationModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _world: &mut World,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let Some(world) = game_ctx.world.as_deref() else { return };
        let room_world_map = &game_ctx.room_world_map;
        let world_directory = &game_ctx.world_directory;
        let world_id = world.id;

        let mut entries: Vec<(Entity, String, bool)> = game_ctx
            .ecs
            .get_store::<WorldEntry>()
            .data
            .iter()
            .filter(|(entity, _)| {
                game_ctx.ecs.get::<CurrentRoom>(**entity)
                    .and_then(|r| room_world_map.get(&r.0).copied())
                    .is_some_and(|wid| wid == world_id)
            })
            .map(|(entity, entry)| (*entity, entry.name.clone(), entry.is_start))
            .collect();
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let has_start = entries.iter().any(|(_, _, is_start)| *is_start);
        self.show_start_warning = !has_start;

        let mut exits: Vec<(Entity, String)> = game_ctx
            .ecs
            .get_store::<WorldExit>()
            .data
            .iter()
            .filter(|(entity, _)| {
                game_ctx.ecs.get::<CurrentRoom>(**entity)
                    .and_then(|r| room_world_map.get(&r.0).copied())
                    .is_some_and(|wid| wid == world_id)
            })
            .map(|(entity, exit)| {
                let dest_label = match &exit.destination {
                    Some(ExitDestination::World(id)) => world_directory
                        .iter()
                        .find(|w| w.id == *id)
                        .map(|w| w.name.clone())
                        .unwrap_or_else(|| "?".to_string()),
                    Some(ExitDestination::Return) => "Return".to_string(),
                    None => "-".to_string(),
                };
                (*entity, dest_label)
            })
            .collect();
        exits.sort_by(|a, b| a.1.cmp(&b.1));

        while self.entry_ids.len() < entries.len() {
            self.entry_ids.push(WidgetId::default());
        }
        while self.exit_ids.len() < exits.len() {
            self.exit_ids.push(WidgetId::default());
        }

        self.entry_count = entries.len();
        self.exit_count = exits.len();

        let mut y = rect.y + layout::WIDGET_SPACING;

        let cb_rect = Rect::new(rect.x, y + 4.0, layout::DEFAULT_CHECKBOX_DIMS, layout::DEFAULT_CHECKBOX_DIMS);
        Checkbox::new(cb_rect, &mut self.show_icons).show(ctx);
        ctx.draw_text(
            "Show in editor",
            rect.x + layout::DEFAULT_CHECKBOX_DIMS + 6.0,
            y + 20.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        y += ROW_H + ROW_GAP;

        ctx.draw_text(
            "Entries",
            rect.x,
            y + 16.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        y += ROW_H + ROW_GAP;

        for (i, (entity, name, is_start)) in entries.iter().enumerate() {
            if Button::new(Rect::new(rect.x, y, rect.w, ROW_H), WorldEntry::display_name(name, *is_start))
                .interaction_id(self.entry_ids[i])
                .show(ctx)
            {
                self.pending_focus = Some(*entity);
            }
            y += ROW_H + ROW_GAP;
        }

        ctx.draw_text(
            "Exits",
            rect.x,
            y + 16.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        y += ROW_H + ROW_GAP;

        for (i, (entity, dest)) in exits.iter().enumerate() {
            if Button::new(Rect::new(rect.x, y, rect.w, ROW_H), dest)
                .interaction_id(self.exit_ids[i])
                .show(ctx)
            {
                self.pending_focus = Some(*entity);
            }
            y += ROW_H + ROW_GAP;
        }

        if self.show_start_warning {
            y += ROW_GAP;
            let msg = format!("No '{}' entry in this world.", WorldEntry::START_NAME);
            self.warning_height = ctx.draw_text_wrapped(
                &msg,
                rect.x,
                y,
                layout::FIELD_TEXT_SIZE_16 - 2.0,
                Color::GOLD,
                rect.w,
            );
        } else {
            self.warning_height = 0.0;
        }
    }

    fn take_host_action(&mut self) -> Option<InspectorHostAction> {
        self.pending_focus
            .take()
            .map(InspectorHostAction::FocusWorldEditor)
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let rows = 1 + 1 + self.entry_count + 1 + self.exit_count;
        InspectorBodyLayout::new()
            .rows(rows.max(3), ROW_GAP)
            .block(self.warning_height)
    }

    fn title(&self) -> &str {
        "Navigation"
    }
}
