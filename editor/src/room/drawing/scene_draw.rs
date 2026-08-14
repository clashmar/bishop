use std::collections::HashSet;

use bishop::prelude::*;
use engine_core::camera::get_room_camera_by_id;
use engine_core::ecs::Pivot;
use engine_core::game::GameCtxMut;
use engine_core::prefab::PrefabAsset;
use engine_core::rendering::{render_room, RenderSystem, RoomRenderState};
use engine_core::worlds::{RoomId, World};

use crate::app::SubEditor;
use crate::canvas::grid;
use crate::canvas::grid_shader::GridRenderer;
use crate::room::drawing::scene_overlays::{
    draw_editor_collider,
    draw_entity_interaction_guides,
    draw_entity_interaction_guides_in_room,
    draw_entity_placeholders,
    draw_exit_placeholders,
    draw_pivot_marker,
    draw_prefab_stamp_ghost,
    highlight_selected_entity,
    is_pure_placeholder,
};
use crate::room::room_editor::{RoomEditor, RoomSceneSubMode};
use crate::shared::selection::draw_selection_box;
use crate::world::coord;

impl RoomEditor {
    pub(crate) fn draw_scene_mode(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        room_id: RoomId,
        game_ctx: &mut GameCtxMut,
        render_system: &mut RenderSystem,
        grid_renderer: &GridRenderer,
        active_prefab: Option<&PrefabAsset>,
        active_prefab_snap_pivot: Pivot,
    ) {
        let Some(grid_size) = game_ctx.world.as_deref().map(|world| world.grid_size) else {
            return;
        };
        let room_camera = get_room_camera_by_id(
            ctx,
            &*game_ctx.ecs,
            room_id,
            self.active_layer_state.active_layer,
            grid_size,
            self.preview_camera_id,
        );

        let view_preview = self.view_preview;
        let render_cam = if view_preview && room_camera.is_some() {
            room_camera.as_ref().map(|camera| &camera.camera).unwrap_or(camera)
        } else {
            camera
        };

        if view_preview {
            render_system.resize_for_camera(render_cam.zoom);
            render_system.begin_scene(ctx);
        } else {
            render_system.resize_to_window(ctx);
        }

        render_room(
            ctx,
            game_ctx,
            render_cam,
            RoomRenderState {
                current_layer: self.active_layer_state.active_layer,
                viewpoint_position: Some(render_cam.target),
                show_all_back_bounds: !view_preview,
            },
            0.0,
            None,
        );

        if view_preview {
            render_system.end_scene(ctx);
            render_system.present_game(ctx);
            return;
        }

        self.draw_scene_editor_overlays(
            ctx,
            camera,
            room_id,
            game_ctx,
            grid_renderer,
            active_prefab,
            active_prefab_snap_pivot,
            grid_size,
        );
    }

    fn draw_scene_editor_overlays(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        room_id: RoomId,
        game_ctx: &mut GameCtxMut,
        grid_renderer: &GridRenderer,
        active_prefab: Option<&PrefabAsset>,
        active_prefab_snap_pivot: Pivot,
        grid_size: f32,
    ) {
        let Some(room) = game_ctx
            .world
            .as_deref_mut()
            .and_then(World::current_room_mut)
        else {
            return;
        };

        ctx.set_camera(camera);

        if self.show_grid {
            grid::draw_grid(ctx, grid_renderer, camera, grid_size);
        }

        let ecs = &*game_ctx.ecs;
        let asset_registry = &mut *game_ctx.asset_registry;
        let sprite_manager = &mut *game_ctx.sprite_manager;

        draw_exit_placeholders(
            ctx,
            &room.exits,
            room.position,
            self.active_layer_state.active_layer,
            grid_size,
        );

        draw_entity_placeholders(
            ctx,
            ecs,
            sprite_manager,
            room_id,
            self.active_layer_state.active_layer,
            grid_size,
        );

        let selected_helpers_redraw_late = self.scene_sub_mode != RoomSceneSubMode::Zones;
        let empty_selected_entities = HashSet::new();
        let interaction_guide_exclusions = if selected_helpers_redraw_late {
            &self.selected_entities
        } else {
            &empty_selected_entities
        };

        draw_entity_interaction_guides_in_room(
            ctx,
            ecs,
            room_id,
            self.active_layer_state.active_layer,
            grid_size,
            interaction_guide_exclusions,
        );

        if self.show_interior_zones {
            self.draw_interior_zones_overlay(ctx, camera, room, grid_size);
        }
        
        if self.scene_sub_mode == RoomSceneSubMode::Stamp && !self.should_block_canvas(ctx) {
            if let Some(prefab) = active_prefab {
                draw_prefab_stamp_ghost(
                    ctx,
                    camera,
                    asset_registry,
                    sprite_manager,
                    prefab,
                    grid_size,
                    active_prefab_snap_pivot,
                );
            }
        }

        if self.scene_sub_mode != RoomSceneSubMode::Zones {
            for &selected_entity in &self.selected_entities {
                if !is_pure_placeholder(ecs, selected_entity) {
                    highlight_selected_entity(
                        ctx,
                        ecs,
                        selected_entity,
                        sprite_manager,
                        grid_size,
                    );
                }
                self.draw_camera_viewports(
                    ctx,
                    camera,
                    ecs,
                    selected_entity,
                    room_id,
                    self.active_layer_state.active_layer,
                );
                draw_pivot_marker(ctx, ecs, selected_entity);
                draw_entity_interaction_guides(ctx, ecs, selected_entity, grid_size);
            }

            if let Some(selected_entity) = self.single_selected_entity() {
                draw_editor_collider(ctx, ecs, selected_entity, grid_size);
            }

            if self.drag_state.box_select_active {
                if let Some(start) = self.drag_state.box_select_start {
                    let mouse_world = coord::mouse_world_pos(ctx, camera);
                    draw_selection_box(ctx, start, mouse_world, grid_size);
                }
            }
        }
    }
}
