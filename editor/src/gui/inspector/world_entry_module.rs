use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::module::CollapsibleComponentModule;
use engine_core::game::GameCtxMut;
use crate::commands::world::RenameWorldEntryCmd;
use crate::editor_global::{push_command, push_toast};
use ::widgets::constants::layout;
use ::widgets::*;

const TITLE: &str = "World Entry";
const BODY_TOP_PADDING: f32 = layout::WIDGET_SPACING;
const ROW_H: f32 = 30.0;
const ERROR_ROW_H: f32 = 20.0;

/// Inspector module for the `WorldEntry` component.
#[derive(Default)]
pub struct WorldEntryModule {
    name_id: WidgetId,
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
        InspectorBodyLayout::new()
            .top_padding(BODY_TOP_PADDING)
            .block(ROW_H)
            .block(ERROR_ROW_H)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let old_name = match game_ctx.ecs.get::<WorldEntry>(entity) {
            Some(entry) => entry.name.clone(),
            None => return,
        };

        let y = rect.y + BODY_TOP_PADDING;
        let name_rect = Rect::new(rect.x, y, rect.w, ROW_H);

        let (typed, commit) = TextInput::new(self.name_id, name_rect, &old_name)
            .blocked(blocked)
            .show(ctx);

        if commit == InputCommit::Committed && typed != old_name {
            let owning_room_id = game_ctx.ecs.get::<CurrentRoom>(entity).map(|r| r.0);
            // Scope: precise world check when dest is active; RoomId proxy otherwise.
            let owning_world_id = owning_room_id.and_then(|room_id| {
                if game_ctx.world.as_deref().is_some_and(|w| w.get_room(room_id).is_some()) {
                    game_ctx.world.as_deref().map(|w| w.id)
                } else {
                    None
                }
            });

            let collides = match owning_world_id {
                Some(world_id) => {
                    game_ctx.ecs.get_store::<WorldEntry>().data.iter().any(|(other, entry)| {
                        *other != entity
                            && entry.name == typed
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
                        game_ctx.ecs.get_store::<WorldEntry>().data.iter().any(|(other, entry)| {
                            *other != entity
                                && entry.name == typed
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
                    RenameWorldEntryCmd::new(entity, old_name, typed),
                ));
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
