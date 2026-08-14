use bishop::prelude::*;
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::module::CollapsibleComponentModule;
use engine_core::ecs::*;
use engine_core::game::GameCtxMut;
use engine_core::ui::measure_text;
use engine_core::worlds::world::WorldExitTrigger;
use engine_core::worlds::ExitDestination;
use ::widgets::constants::{colors, layout};
use ::widgets::*;

const TITLE: &str = "World Exit";
const RETURN_LABEL: &str = ExitDestination::RETURN_LABEL;
const ROW_H: f32 = 30.0;
const DROP_W_REDUCTION: f32 = 10.0;

/// Inspector module for the `WorldExit` component.
pub struct WorldExitModule {
    dest_id: WidgetId,
    entry_id: WidgetId,
    trigger_id: WidgetId,
    range_id: WidgetId,
    show_bottom_row: bool,
    show_start_warning: bool,
    warning_height: f32,
}

impl Default for WorldExitModule {
    fn default() -> Self {
        Self {
            dest_id: WidgetId::default(),
            entry_id: WidgetId::default(),
            trigger_id: WidgetId::default(),
            range_id: WidgetId::default(),
            show_bottom_row: true,
            show_start_warning: false,
            warning_height: 0.0,
        }
    }
}

impl InspectorModule for WorldExitModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(WorldExit::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<WorldExit>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let gap = layout::WIDGET_SPACING;
        let mut layout = InspectorBodyLayout::new()
            .top_padding(gap)
            .block(ROW_H).gap(gap)
            .block(ROW_H).gap(gap)
            .block(ROW_H).gap(gap)
            .block(ROW_H);
        if self.show_bottom_row {
            layout = layout.gap(gap).block(ROW_H);
        }
        if self.show_start_warning {
            layout = layout.gap(gap).block(self.warning_height);
        }
        layout
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

        let is_return = matches!(current_exit.destination, Some(ExitDestination::Return));
        let current_dest_world = match &current_exit.destination {
            Some(ExitDestination::World(id)) => Some(*id),
            _ => None,
        };

        // Only overlay worlds support Return; determine if entity is in an overlay world
        let entity_world_is_overlay = game_ctx.ecs.get::<CurrentRoom>(entity)
            .and_then(|room| game_ctx.room_world_map.get(&room.room_id).copied())
            .and_then(|wid| world_directory.iter().find(|w| w.id == wid))
            .is_some_and(|w| w.overlay);

        let mut dest_opts: Vec<String> = Vec::new();
        if entity_world_is_overlay {
            dest_opts.push(RETURN_LABEL.to_string());
        }
        dest_opts.extend(world_directory.iter().map(|w| w.name.clone()));

        let dest_overlay = current_dest_world
            .and_then(|id| world_directory.iter().find(|w| w.id == id))
            .map(|w| w.overlay);

        let current_dest_label = if is_return {
            RETURN_LABEL.to_string()
        } else {
            current_dest_world
                .and_then(|id| world_directory.iter().find(|w| w.id == id).map(|w| w.name.clone()))
                .unwrap_or_else(|| "Select World".to_string())
        };

        ctx.draw_text("Mode:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
        let mode_label = if is_return {
            "Overlay"
        } else {
            match dest_overlay {
                Some(true) => "Overlay",
                Some(false) => "Transport",
                None => "-",
            }
        };
        ctx.draw_text(mode_label, rect.x + label_w, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_MUTED_COLOR);
        y += ROW_H + gap;

        ctx.draw_text("World:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
        let dir_snap = world_directory.clone();
        if let Some(sel) = Dropdown::new(self.dest_id, Rect::new(rect.x + label_w, y, rect.w - label_w - DROP_W_REDUCTION, ROW_H), &current_dest_label, &dest_opts, |n| n.clone())
            .filterable()
            .truncate_trigger_text()
            .list_width(rect.w - label_w - DROP_W_REDUCTION)
            .suppressed(blocked)
            .show(ctx)
        {
            if sel == RETURN_LABEL {
                if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                    exit.destination = Some(ExitDestination::Return);
                    exit.entry = None;
                }
            } else if let Some(new_id) = dir_snap.iter().find(|w| w.name == sel).map(|w| w.id) {
                if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                    if exit.destination != Some(ExitDestination::World(new_id)) {
                        exit.destination = Some(ExitDestination::World(new_id));
                        exit.entry = None;
                    }
                }
            }
        }

        // Auto-clear Return destination if this world is no longer an overlay
        if is_return && !entity_world_is_overlay {
            if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                exit.destination = None;
            }
        }

        y += ROW_H + gap;

        let mut start_missing = false;
        if !is_return {
            if let Some(dest_world_id) = current_dest_world {
                let entry_data: Vec<(String, bool)> = game_ctx.ecs.get_store::<WorldEntry>().data.iter()
                    .filter(|(entry_entity, _)| {
                        game_ctx.ecs.get::<CurrentRoom>(**entry_entity)
                            .and_then(|room| game_ctx.room_world_map.get(&room.room_id).copied())
                            .is_some_and(|wid| wid == dest_world_id)
                    })
                    .map(|(_, entry)| (entry.name.clone(), entry.is_start))
                    .collect();

                let display_names: Vec<String> = entry_data.iter().map(|(name, is_start)| {
                    WorldEntry::display_name(name, *is_start).to_string()
                }).collect();

                let start_idx = entry_data.iter().position(|(_, is_start)| *is_start);
                let default_display = start_idx
                    .map(|idx| display_names[idx].clone())
                    .or_else(|| display_names.first().cloned())
                    .unwrap_or_default();

                let current_display = match current_exit.entry.as_deref() {
                    None | Some("") | Some(WorldEntry::START_NAME) => default_display.clone(),
                    Some(name) => entry_data.iter()
                        .position(|(entry_name, _)| entry_name == name)
                        .map(|idx| display_names[idx].clone())
                        .unwrap_or_else(|| name.to_string()),
                };

                start_missing = !entry_data.iter().any(|(_, is_start)| *is_start);

                ctx.draw_text("Entry:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
                if let Some(sel) = Dropdown::new(self.entry_id, Rect::new(rect.x + label_w, y, rect.w - label_w - DROP_W_REDUCTION, ROW_H), &current_display, &display_names, |s| s.clone())
                    .filterable()
                    .truncate_trigger_text()
                    .list_width(rect.w - label_w - DROP_W_REDUCTION)
                    .suppressed(blocked)
                    .show(ctx)
                {
                    if let Some(idx) = display_names.iter().position(|n| n == &sel) {
                        let (actual_name, is_start) = &entry_data[idx];
                        if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                            exit.entry = if *is_start && actual_name.is_empty() {
                                None
                            } else {
                                Some(actual_name.clone())
                            };
                        }
                    }
                }
            }
        }
        self.show_start_warning = start_missing;
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
                self.show_bottom_row = true;
                ctx.draw_text("Range:", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
                let (typed, _) = NumberInput::new(self.range_id, Rect::new(rect.x + label_w, y, rect.w - label_w - DROP_W_REDUCTION, ROW_H), *range)
                    .blocked(blocked)
                    .show(ctx);
                if typed != *range {
                    if let Some(exit) = game_ctx.ecs.get_mut::<WorldExit>(entity) {
                        exit.trigger = WorldExitTrigger::OnProximity(typed);
                    }
                }
            }
            WorldExitTrigger::OnInteract => {
                let has_interactable = game_ctx.ecs.has::<Interactable>(entity);
                self.show_bottom_row = !has_interactable;
                if !has_interactable {
                    ctx.draw_text("Add Interactable to enable", rect.x, y + 20.0, layout::FIELD_TEXT_SIZE_16 - 2.0, Color::RED);
                }
            }
        }

        if self.show_start_warning {
            y += if self.show_bottom_row { ROW_H + (2. * gap) } else { gap };
            let msg = format!("No {} entry in destination.", WorldEntry::START_NAME);
            self.warning_height = ctx.draw_text_wrapped(
                &msg,
                rect.x,
                y,
                layout::FIELD_TEXT_SIZE_16 - 2.0,
                Color::GOLD,
                rect.w,
            );
        }
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: WorldExit::TYPE_NAME,
        title: TITLE,
        factory: || Box::new(
            CollapsibleComponentModule::new(
                crate::gui::inspector::world_exit_module::WorldExitModule::default()
            ).with_title(TITLE)
        ),
        allowed_for: Some(|entity, ecs| {
            ecs.has::<engine_core::ecs::CurrentRoom>(entity)
                && !ecs.has::<engine_core::ecs::Player>(entity)
                && !ecs.has::<engine_core::ecs::PlayerProxy>(entity)
        }),
    }
}
