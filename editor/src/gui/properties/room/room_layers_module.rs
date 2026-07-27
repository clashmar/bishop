use super::super::PropertyModule;
use crate::commands::room::{SetBackLayerCompositionModeCmd, SetBackLayerEnabledCmd};
use crate::editor_global::push_command;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::room::Room;
use engine_core::worlds::LayerCompositionMode;
use std::cell::Cell;
use widgets::constants::{colors, layout};
use widgets::{Button, Dropdown, WidgetId};

const ROW_H: f32 = 30.0;
const GAP: f32 = layout::WIDGET_SPACING;

pub struct RoomLayersModule {
    composition_id: WidgetId,
    show_composition_mode: Cell<bool>,
}

impl RoomLayersModule {
    pub fn new() -> Self {
        Self {
            composition_id: WidgetId::default(),
            show_composition_mode: Cell::new(false),
        }
    }
}

impl Default for RoomLayersModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<Room> for RoomLayersModule {
    fn visible(&self, room: &Room, _game_ctx: &GameCtxMut) -> bool {
        self.show_composition_mode
            .set(room.current_variant().layers.back.is_some());
        true
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        room: &mut Room,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let room_id = room.id;
        let Some(world_id) = game_ctx.world.as_deref().map(|world| world.id) else {
            return;
        };
        let back_enabled = room.current_variant().layers.back.is_some();

        let button_label = if back_enabled {
            "- Back Layer"
        } else {
            "+ Back Layer"
        };
        let button_rect = Rect::new(rect.x, rect.y, rect.w, ROW_H);
        if Button::new(button_rect, button_label).show(ctx) {
            push_command(Box::new(SetBackLayerEnabledCmd::new(room_id, !back_enabled)));
            return;
        }

        if !self.show_composition_mode.get() {
            return;
        }

        let y = rect.y + ROW_H + GAP;
        let current_mode = room
            .current_variant()
            .layers
            .back
            .as_ref()
            .map(|back| back.composition_mode)
            .unwrap_or_default();
        let options = LayerCompositionMode::ALL
            .into_iter()
            .map(|mode| mode.ui_label().to_string())
            .collect::<Vec<_>>();
        let current_label = current_mode.ui_label();

        ctx.draw_text(
            "Composition:",
            rect.x,
            y + 20.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        if let Some(selection) = Dropdown::new(
            self.composition_id,
            Rect::new(rect.x + 110.0, y, rect.w - 120.0, ROW_H),
            current_label,
            &options,
            |value| value.clone(),
        )
        .truncate_trigger_text()
        .suppressed(false)
        .show(ctx)
        {
            let selected_mode = LayerCompositionMode::ALL
                .into_iter()
                .find(|mode| mode.ui_label() == selection)
                .expect("composition dropdown should emit a known mode");

            push_command(Box::new(SetBackLayerCompositionModeCmd::new(
                world_id,
                room_id,
                current_mode,
                selected_mode,
            )));
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let mut layout = InspectorBodyLayout::new().rows(1, GAP);
        if self.show_composition_mode.get() {
            layout = layout.gap(GAP).rows(1, GAP);
        }
        layout
    }

    fn title(&self) -> &str {
        "Layers"
    }
}
