use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::module::CollapsibleComponentModule;
use engine_core::game::GameCtxMut;
use crate::commands::world::RenameWorldEntryCmd;
use crate::editor_global::{push_command, push_toast};
use ::widgets::constants::{colors, layout};
use ::widgets::*;

const TITLE: &str = "World Entry";
const BODY_TOP_PADDING: f32 = layout::WIDGET_SPACING;
const ROW_H: f32 = 30.0;
const ERROR_ROW_H: f32 = 20.0;

/// Inspector module for the `WorldEntry` component.
#[derive(Default)]
pub struct WorldEntryModule {
    name_id: WidgetId,
    is_start: bool,
    show_checkbox: bool,
}

impl InspectorModule for WorldEntryModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(WorldEntry::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<WorldEntry>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let mut layout = InspectorBodyLayout::new()
            .top_padding(BODY_TOP_PADDING);
        if self.show_checkbox {
            layout = layout.block(ROW_H);
        }
        if !self.is_start {
            layout = layout.block(ROW_H);
        }
        layout.block(ERROR_ROW_H)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let entry = match game_ctx.ecs.get::<WorldEntry>(entity) {
            Some(entry) => entry.clone(),
            None => return,
        };

        self.is_start = entry.is_start;

        let owning_room_id = game_ctx.ecs.get::<CurrentRoom>(entity).map(|r| r.0);
        let owning_world_id = owning_room_id.and_then(|room_id| {
            if game_ctx.world.as_deref().is_some_and(|w| w.get_room(room_id).is_some()) {
                game_ctx.world.as_deref().map(|w| w.id)
            } else {
                None
            }
        });

        let other_is_start = match owning_world_id {
            Some(world_id) => {
                game_ctx.ecs.get_store::<WorldEntry>().data.iter().any(|(other, e)| {
                    *other != entity
                        && e.is_start
                        && game_ctx
                            .ecs
                            .get::<CurrentRoom>(*other)
                            .and_then(|r| game_ctx.world.as_deref().map(|w| (r, w)))
                            .is_some_and(|(r, w)| {
                                w.get_room(r.0).is_some() && w.id == world_id
                            })
                })
            }
            None => false,
        };

        self.show_checkbox = !other_is_start || entry.is_start;
        let mut y = rect.y + BODY_TOP_PADDING;

        if self.show_checkbox {
            let cb_rect = Rect::new(rect.x, y + 6.0, layout::DEFAULT_CHECKBOX_DIMS, layout::DEFAULT_CHECKBOX_DIMS);
            let mut checked = entry.is_start;
            Checkbox::new(cb_rect, &mut checked).show(ctx);
            ctx.draw_text(
                WorldEntry::START_NAME,
                rect.x + layout::DEFAULT_CHECKBOX_DIMS + 6.0,
                y + 20.0,
                layout::FIELD_TEXT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            if checked != entry.is_start {
                if let Some(e) = game_ctx.ecs.get_mut::<WorldEntry>(entity) {
                    e.is_start = checked;
                }
            }
            y += ROW_H + layout::WIDGET_SPACING;
        }

        if !entry.is_start {
            let name_rect = Rect::new(rect.x, y, rect.w, ROW_H);
            let (typed, commit) = TextInput::new(self.name_id, name_rect, &entry.name)
                .blocked(blocked)
                .show(ctx);

            if commit == InputCommit::Committed && typed != entry.name {
                if typed.eq_ignore_ascii_case(WorldEntry::START_NAME) {
                    let msg = format!("{} is a reserved term", WorldEntry::START_NAME);
                    ctx.draw_text(
                        &msg,
                        rect.x,
                        y + ROW_H + 4.0,
                        layout::FIELD_TEXT_SIZE_16 - 2.0,
                        Color::RED,
                    );
                    push_toast(format!("'{}' is a reserved term", WorldEntry::START_NAME), 3.0);
                } else {
                    let collides = match owning_world_id {
                        Some(world_id) => {
                            game_ctx.ecs.get_store::<WorldEntry>().data.iter().any(|(other, e)| {
                                *other != entity
                                    && e.name == typed
                                    && game_ctx
                                        .ecs
                                        .get::<CurrentRoom>(*other)
                                        .and_then(|r| game_ctx.world.as_deref().map(|w| (r, w)))
                                        .is_some_and(|(r, w)| {
                                            w.get_room(r.0).is_some() && w.id == world_id
                                        })
                            })
                        }
                        None => {
                            owning_room_id.is_some_and(|room_id| {
                                game_ctx.ecs.get_store::<WorldEntry>().data.iter().any(|(other, e)| {
                                    *other != entity
                                        && e.name == typed
                                        && game_ctx.ecs.get::<CurrentRoom>(*other).map(|r| r.0)
                                            == Some(room_id)
                                })
                            })
                        }
                    };

                    if collides {
                        ctx.draw_text(
                            "Name already used in this world",
                            rect.x,
                            y + ROW_H + 4.0,
                            layout::FIELD_TEXT_SIZE_16 - 2.0,
                            Color::RED,
                        );
                        push_toast(format!("'{}' is already used in this world", typed), 3.0);
                    } else {
                        push_command(Box::new(
                            RenameWorldEntryCmd::new(entity, entry.name.clone(), typed),
                        ));
                    }
                }
            }
        }
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: WorldEntry::TYPE_NAME,
        title: TITLE,
        factory: || {
            Box::new(
                CollapsibleComponentModule::new(
                    crate::gui::inspector::world_entry_module::WorldEntryModule::default()
                )
                .with_title(TITLE)
            )
        },
        allowed_for: Some(|entity, ecs| {
            ecs.has::<engine_core::ecs::CurrentRoom>(entity)
                && !ecs.has::<engine_core::ecs::Player>(entity)
                && !ecs.has::<engine_core::ecs::PlayerProxy>(entity)
        }),
    }
}
