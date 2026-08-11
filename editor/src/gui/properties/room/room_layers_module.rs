use super::super::PropertyModule;
use crate::commands::room::{
    SetBackLayerCompositionModeCmd,
    SetBackLayerEnabledCmd,
    UpdateInteriorZonesCmd,
};
use crate::editor_assets::assets::{edit_icon, eye_icon};
use crate::editor_global::push_command;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::room::Room;
use engine_core::worlds::{InteriorZoneBounds, LayerCompositionMode};
use std::cell::Cell;
use widgets::constants::{colors, layout};
use widgets::{Button, Dropdown, InputCommit, NumberInput, WidgetId};

const ROW_H: f32 = 30.0;
const GAP: f32 = layout::WIDGET_SPACING;
const ICON_BUTTON_SIZE: f32 = ROW_H;
const WARNING_TOP_GAP: f32 = 8.0;
const WARNING_HEIGHT: f32 = 28.0;

#[derive(Clone, Copy, Default)]
struct ZoneWidgetIds {
    delete_id: WidgetId,
    x_id: WidgetId,
    y_id: WidgetId,
    w_id: WidgetId,
    h_id: WidgetId,
}

pub struct RoomLayersModule {
    composition_id: WidgetId,
    zone_widget_ids: Vec<ZoneWidgetIds>,
    pending_host_action: Option<InspectorHostAction>,
    show_composition_mode: Cell<bool>,
    zone_count: Cell<usize>,
}

impl RoomLayersModule {
    pub fn new() -> Self {
        Self {
            composition_id: WidgetId::default(),
            zone_widget_ids: Vec::new(),
            pending_host_action: None,
            show_composition_mode: Cell::new(false),
            zone_count: Cell::new(0),
        }
    }
}

impl Default for RoomLayersModule {
    fn default() -> Self {
        Self::new()
    }
}

impl RoomLayersModule {
    fn sync_zone_widget_ids(&mut self, zone_count: usize) {
        while self.zone_widget_ids.len() < zone_count {
            self.zone_widget_ids.push(ZoneWidgetIds::default());
        }
        self.zone_widget_ids.truncate(zone_count);
    }
}

impl PropertyModule<Room> for RoomLayersModule {
    fn visible(&self, room: &Room, _game_ctx: &GameCtxMut) -> bool {
        let back = room.current_variant().layers.back.as_ref();
        self.show_composition_mode.set(back.is_some());
        self.zone_count
            .set(back.map(|back| back.interior_zones.len()).unwrap_or(0));
        true
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        room: &mut Room,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) {
        let room_id = room.id;
        let Some(world_id) = game_ctx
            .world
            .as_deref()
            .map(|world| world.id)
        else {
            return;
        };
        let back_enabled = room.current_variant().layers.back.is_some();

        let button_label = if back_enabled {
            "- Back Layer"
        } else {
            "+ Back Layer"
        };
        let visibility_icon_rect = Rect::new(
            rect.x + rect.w - ICON_BUTTON_SIZE,
            rect.y,
            ICON_BUTTON_SIZE,
            ICON_BUTTON_SIZE,
        );
        let button_rect = if back_enabled {
            Rect::new(rect.x, rect.y, rect.w - ICON_BUTTON_SIZE - GAP, ROW_H)
        } else {
            Rect::new(rect.x, rect.y, rect.w, ROW_H)
        };
        if Button::new(button_rect, button_label).show(ctx) {
            push_command(Box::new(SetBackLayerEnabledCmd::new(room_id, !back_enabled)));
            return;
        }
        if back_enabled {
            let tooltip = if insp_ctx.room_zones_visible {
                "Hide Interior Zones"
            } else {
                "Show Interior Zones"
            };
            if Button::icon(visibility_icon_rect, eye_icon(), tooltip)
                .active(insp_ctx.room_zones_visible)
                .icon_padding(5.0)
                .show(ctx)
            {
                self.pending_host_action = Some(InspectorHostAction::ToggleRoomZoneVisibility);
            }
        }

        if !self.show_composition_mode.get() {
            return;
        }

        let current_zones = room
            .current_variant()
            .layers
            .back
            .as_ref()
            .map(|back| back.interior_zones.clone())
            .unwrap_or_default();
        self.sync_zone_widget_ids(current_zones.len());

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

        let composition_label_w = 110.0;
        let icon_rect = Rect::new(rect.x + rect.w - ICON_BUTTON_SIZE, y, ICON_BUTTON_SIZE, ICON_BUTTON_SIZE);
        let dropdown_rect = Rect::new(
            rect.x + composition_label_w,
            y,
            icon_rect.x - rect.x - composition_label_w - GAP,
            ROW_H,
        );

        ctx.draw_text(
            "Composition:",
            rect.x,
            y + 20.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        if let Some(selection) = Dropdown::new(
            self.composition_id,
            dropdown_rect,
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
            return;
        }

        if Button::icon(icon_rect, edit_icon(), "Edit Interior Zones")
            .active(insp_ctx.room_zone_tool_active)
            .icon_padding(5.0)
            .show(ctx)
        {
            self.pending_host_action = Some(InspectorHostAction::ToggleRoomZoneTool);
        }

        let mut zone_y = y + ROW_H + GAP;
        if self.zone_widget_ids.is_empty() {
            zone_y += WARNING_TOP_GAP;
            ctx.draw_text_wrapped(
                "No interior zones yet. Use the zone tool to drag them on the canvas. Back bounds currently use the full room.",
                rect.x,
                zone_y,
                layout::FIELD_TEXT_SIZE_16 - 2.0,
                Color::GOLD,
                rect.w,
            );
            return;
        }

        for (index, zone) in current_zones.iter().enumerate() {
            let ids = self.zone_widget_ids[index];
            let title = format!("Zone {}", zone.id.0);
            ctx.draw_text(
                &title,
                rect.x,
                zone_y + 20.0,
                layout::FIELD_TEXT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            let delete_rect = Rect::new(rect.x + rect.w - 90.0, zone_y, 90.0, ROW_H);
            if Button::new(delete_rect, "Delete")
                .interaction_id(ids.delete_id)
                .show(ctx)
            {
                let mut new_zones = current_zones.clone();
                new_zones.remove(index);
                push_command(Box::new(UpdateInteriorZonesCmd::new(
                    world_id,
                    room_id,
                    current_zones.clone(),
                    new_zones,
                )));
                return;
            }
            zone_y += ROW_H + GAP;

            let half_w = (rect.w - GAP) * 0.5;
            let label_w = 18.0;
            ctx.draw_text("X", rect.x, zone_y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
            ctx.draw_text(
                "Y",
                rect.x + half_w + GAP,
                zone_y + 20.0,
                layout::FIELD_TEXT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            let x_rect = Rect::new(rect.x + label_w, zone_y, half_w - label_w, ROW_H);
            let y_rect = Rect::new(rect.x + half_w + GAP + label_w, zone_y, half_w - label_w, ROW_H);
            let (new_x, commit_x) = NumberInput::new(ids.x_id, x_rect, zone.bounds.x).show(ctx);
            let (new_y, commit_y) = NumberInput::new(ids.y_id, y_rect, zone.bounds.y).show(ctx);
            zone_y += ROW_H + GAP;

            ctx.draw_text("W", rect.x, zone_y + 20.0, layout::FIELD_TEXT_SIZE_16, colors::DEFAULT_TEXT_COLOR);
            ctx.draw_text(
                "H",
                rect.x + half_w + GAP,
                zone_y + 20.0,
                layout::FIELD_TEXT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            let w_rect = Rect::new(rect.x + label_w, zone_y, half_w - label_w, ROW_H);
            let h_rect = Rect::new(rect.x + half_w + GAP + label_w, zone_y, half_w - label_w, ROW_H);
            let (new_w, commit_w) = NumberInput::new(ids.w_id, w_rect, zone.bounds.w)
                .min(1)
                .show(ctx);
            let (new_h, commit_h) = NumberInput::new(ids.h_id, h_rect, zone.bounds.h)
                .min(1)
                .show(ctx);
            zone_y += ROW_H + GAP;

            let committed = matches!(commit_x, InputCommit::Committed)
                || matches!(commit_y, InputCommit::Committed)
                || matches!(commit_w, InputCommit::Committed)
                || matches!(commit_h, InputCommit::Committed);
            let edited_bounds = committed_zone_bounds(new_x, new_y, new_w, new_h);
            if committed && edited_bounds != zone.bounds {
                let mut new_zones = current_zones.clone();
                new_zones[index].bounds = edited_bounds;
                push_command(Box::new(UpdateInteriorZonesCmd::new(
                    world_id,
                    room_id,
                    current_zones.clone(),
                    new_zones,
                )));
                return;
            }
        }
    }

    fn take_host_action(&mut self) -> Option<InspectorHostAction> {
        self.pending_host_action.take()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let mut layout = InspectorBodyLayout::new().rows(1, GAP);
        if self.show_composition_mode.get() {
            layout = layout.gap(GAP).rows(1, GAP);
            if self.zone_count.get() == 0 {
                layout = layout.gap(GAP + WARNING_TOP_GAP).block(WARNING_HEIGHT);
            } else {
                for _ in 0..self.zone_count.get() {
                    layout = layout.gap(GAP).rows(3, GAP);
                }
            }
        }
        layout
    }

    fn title(&self) -> &str {
        "Layers"
    }
}

fn committed_zone_bounds(x: i32, y: i32, w: i32, h: i32) -> InteriorZoneBounds {
    InteriorZoneBounds::new(x, y, w.max(1), h.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::worlds::{InteriorZone, InteriorZoneBounds, InteriorZoneId};

    fn next_interior_zone_id(zones: &[InteriorZone]) -> InteriorZoneId {
        InteriorZoneId(
            zones
                .iter()
                .map(|zone| zone.id.0)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        )
    }

    #[test]
    fn next_interior_zone_id_returns_max_plus_one() {
        let zones = vec![
            InteriorZone {
                id: InteriorZoneId(3),
                bounds: InteriorZoneBounds::new(0, 0, 16, 16),
            },
            InteriorZone {
                id: InteriorZoneId(8),
                bounds: InteriorZoneBounds::new(16, 0, 16, 16),
            },
        ];

        assert_eq!(next_interior_zone_id(&zones), InteriorZoneId(9));
    }

    #[test]
    fn committed_zone_bounds_clamps_width_and_height_to_one() {
        assert_eq!(
            committed_zone_bounds(4, 5, 0, -2),
            InteriorZoneBounds::new(4, 5, 1, 1),
        );
    }
}
