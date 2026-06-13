use bishop::prelude::*;
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::module::CollapsibleComponentModule;
use engine_core::ecs::*;
use engine_core::game::GameCtxMut;
use engine_core::ui::measure_text;
use engine_core::worlds::world::{WorldExitTrigger, WorldTransitionMode};
use strum::IntoEnumIterator;
use ::widgets::constants::{colors, layout};
use ::widgets::*;

const TITLE: &str = "World Exit";
const WORLD_START_LABEL: &str = "[ World Start ]";
const ROW_H: f32 = 30.0;
const DROP_W_REDUCTION: f32 = 10.0;

/// Inspector module for the `WorldExit` component.
#[derive(Default)]
pub struct WorldExitModule {
    dest_id: WidgetId,
    entry_id: WidgetId,
    mode_id: WidgetId,
    trigger_id: WidgetId,
    range_id: WidgetId,
}

impl InspectorModule for WorldExitModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(WorldExit::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<WorldExit>(entity).is_some()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let gap = layout::WIDGET_SPACING;
        InspectorBodyLayout::new()
            .top_padding(gap)
            .block(ROW_H).gap(gap) // world
            .block(ROW_H).gap(gap) // entry
            .block(ROW_H).gap(gap) // mode
            .block(ROW_H).gap(gap) // trigger
            .block(ROW_H)          // range / warning / empty
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let world_directory = game_ctx.world_directory.clone();
        let current_exit = match game_ctx.ecs.get::<WorldExit>(entity) {
            Some(exit) => exit.clone(),
            None => return,
        };

        let label_w = measure_text(ctx, "Trigger: ", layout::FIELD_TEXT_SIZE_16).width;
        let gap = layout::WIDGET_SPACING;
        let mut y = rect.y + gap;

        let entry_names: Vec<String> = current_exit.destination_world.map(|dest_world_id| {
            game_ctx.ecs.get_store::<WorldEntry>().data.iter()
                .filter(|(entry_entity, _)| {
                    game_ctx.ecs.get::<CurrentRoom>(**entry_entity)
                        .and_then(|r| game_ctx.room_world_map.get(&r.0).copied())
                        .is_some_and(|wid| wid == dest_world_id)
                })
                .map(|(_, entry)| entry.name.clone())
                .collect()
        }).unwrap_or_default();

        let world_names: Vec<String> = world_directory.iter().map(|(_, n)| n.clone()).collect();
        let current_world_label = current_exit.destination_world
            .and_then(|id| world_directory.iter().find(|(wid, _)| *wid == id).map(|(_, n)| n.clone()))
            .unwrap_or_else(|| "(select world)".to_string());
        ctx.draw_text("World:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
        let dir_snap = world_directory.clone();
        if let Some(sel) = Dropdown::new(self.dest_id, Rect::new(rect.x + label_w, y, rect.w - label_w - DROP_W_REDUCTION, ROW_H), &current_world_label, &world_names, |n| n.clone())
            .filterable()
            .truncate_trigger_text()
            .list_width(rect.w - label_w - DROP_W_REDUCTION)
            .suppressed(blocked)
            .show(ctx)
        {
            if let Some(new_id) = dir_snap.iter().find(|(_, n)| n == &sel).map(|(id, _)| *id) {
                if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                    if exit.destination_world != Some(new_id) {
                        exit.destination_world = Some(new_id);
                        exit.entry = None;
                    }
                }
            }
        }
        y += ROW_H + gap;

        if current_exit.destination_world.is_some() {
            ctx.draw_text("Entry:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
            if !entry_names.is_empty() {
                let current_entry = current_exit.entry.clone().unwrap_or_else(|| WORLD_START_LABEL.to_string());
                let mut opts: Vec<String> = vec![String::new()];
                opts.extend(entry_names.iter().cloned());
                if let Some(sel) = Dropdown::new(self.entry_id, Rect::new(rect.x + label_w, y, rect.w - label_w - DROP_W_REDUCTION, ROW_H), &current_entry, &opts, |s| if s.is_empty() { WORLD_START_LABEL.to_string() } else { s.clone() })
                    .filterable()
                    .truncate_trigger_text()
                    .list_width(rect.w - label_w - DROP_W_REDUCTION)
                    .suppressed(blocked)
                    .show(ctx)
                {
                    if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                        exit.entry = if sel.is_empty() { None } else { Some(sel) };
                    }
                }
            } else {
                ctx.draw_text(WORLD_START_LABEL, rect.x + label_w, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_MUTED_COLOR);
            }
        }
        y += ROW_H + gap;

        ctx.draw_text("Mode:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
        let mode_options: Vec<WorldTransitionMode> = WorldTransitionMode::iter().collect();
        if let Some(new_mode) = Dropdown::new(self.mode_id, Rect::new(rect.x + label_w, y, rect.w - label_w - DROP_W_REDUCTION, ROW_H), &current_exit.mode.to_string(), &mode_options, |m| m.to_string())
            .truncate_trigger_text()
            .list_width(rect.w - label_w - DROP_W_REDUCTION)
            .suppressed(blocked)
            .show(ctx)
        {
            if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                if exit.mode != new_mode { exit.mode = new_mode; }
            }
        }
        y += ROW_H + gap;

        ctx.draw_text("Trigger:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
        let trigger_kinds = vec!["On Interact".to_string(), "On Proximity".to_string()];
        if let Some(kind) = Dropdown::new(self.trigger_id, Rect::new(rect.x + label_w, y, rect.w - label_w - DROP_W_REDUCTION, ROW_H), &current_exit.trigger.to_string(), &trigger_kinds, |s| s.clone())
            .truncate_trigger_text()
            .list_width(rect.w - label_w - DROP_W_REDUCTION)
            .suppressed(blocked)
            .show(ctx)
        {
            if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                match kind.as_str() {
                    "On Interact" => exit.trigger = WorldExitTrigger::OnInteract,
                    "On Proximity" => {
                        if !matches!(exit.trigger, WorldExitTrigger::OnProximity(_)) {
                            exit.trigger = WorldExitTrigger::OnProximity(16.0);
                        }
                    }
                    _ => {}
                }
            }
        }
        y += ROW_H + gap;

        match &current_exit.trigger {
            WorldExitTrigger::OnProximity(range) => {
                ctx.draw_text("Range:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
                let (typed, _) = NumberInput::new(self.range_id, Rect::new(rect.x + label_w, y, rect.w - label_w, ROW_H), *range)
                    .blocked(blocked)
                    .show(ctx);
                if typed != *range {
                    if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                        exit.trigger = WorldExitTrigger::OnProximity(typed);
                    }
                }
            }
            WorldExitTrigger::OnInteract => {
                if !game_ctx.ecs.has::<Interactable>(entity) {
                    ctx.draw_text("Add Interactable to enable", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16 - 2.0, Color::RED);
                }
            }
        }
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        title: WorldExit::TYPE_NAME,
        factory: || Box::new(
            CollapsibleComponentModule::new(
                crate::gui::inspector::world_exit_module::WorldExitModule::default()
            ).with_title(TITLE)
        ),
    }
}
