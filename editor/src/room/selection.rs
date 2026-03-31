// editor/src/room/selection.rs
use crate::app::SubEditor;
use crate::room::room_editor::*;
use crate::world::coord;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashSet;

/// Stores the original drag state before switching to copy mode.
pub(crate) struct PreCopyDragState {
    pub anchor_entity: Option<Entity>,
    pub selected_entities: HashSet<Entity>,
}

/// All transient mouse-interaction state for entity dragging and box selection.
#[derive(Default)]
pub(crate) struct DragState {
    /// Whether an entity drag is currently active.
    pub dragging: bool,
    /// The entity that was clicked to start the drag.
    pub drag_anchor_entity: Option<Entity>,
    /// Offset from the anchor entity's position to the mouse at drag start.
    pub drag_offset: Vec2,
    /// Start positions of all dragged entities at the moment dragging began.
    pub drag_start_positions: Vec<(Entity, Vec2)>,
    /// The very first start positions when the drag began, used for undo commands.
    pub drag_initial_start_positions: Vec<(Entity, Vec2)>,
    /// Start position of a box selection in world coordinates.
    pub box_select_start: Option<Vec2>,
    /// Whether a box selection drag is currently active.
    pub box_select_active: bool,
    /// Whether the current drag is an alt+drag copy operation.
    pub alt_copy_mode: bool,
    /// Entities created during an alt+drag copy, for the undo command.
    pub alt_copied_entities: Vec<Entity>,
    /// Original drag state before entering copy mode, used to revert on alt release.
    pub pre_copy_drag_state: Option<PreCopyDragState>,
}

impl RoomEditor {
    /// Sets a single selected entity for the room editor, clearing any previous selection.
    pub fn set_selected_entity(&mut self, entity: Option<Entity>) {
        self.selected_entities.clear();
        if let Some(e) = entity {
            self.selected_entities.insert(e);
        }
        self.inspector.set_target(entity);
    }

    /// Adds an entity to the current selection.
    pub fn add_to_selection(&mut self, entity: Entity) {
        self.selected_entities.insert(entity);
        // Update inspector only if this is now the only selection
        if self.selected_entities.len() == 1 {
            self.inspector.set_target(Some(entity));
        } else {
            self.inspector.set_target(None);
        }
    }

    /// Clears the entire selection.
    pub fn clear_selection(&mut self) {
        self.selected_entities.clear();
        self.inspector.set_target(None);
    }

    /// Returns whether the given entity is currently selected.
    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }

    /// Returns the single selected entity if exactly one is selected.
    pub fn single_selected_entity(&self) -> Option<Entity> {
        if self.selected_entities.len() == 1 {
            self.selected_entities.iter().next().copied()
        } else {
            None
        }
    }

    /// Selects all entities in the specified room.
    pub fn select_all_in_room(&mut self, ecs: &Ecs, room_id: RoomId) {
        self.selected_entities.clear();
        for (entity, _) in ecs.get_store::<Transform>().data.iter() {
            if can_select_entity_in_room(ecs, *entity, room_id) {
                self.selected_entities.insert(*entity);
            }
        }
        // Clear inspector since we likely have multiple selected
        if self.selected_entities.len() != 1 {
            self.inspector.set_target(None);
        } else if let Some(e) = self.selected_entities.iter().next() {
            self.inspector.set_target(Some(*e));
        }
    }

    #[inline]
    pub fn register_rect(&mut self, rect: Rect) -> Rect {
        self.active_rects.push(rect);
        rect
    }

    pub(crate) fn ui_was_clicked(&self, ctx: &mut WgpuContext) -> bool {
        ctx.is_mouse_button_pressed(MouseButton::Left) && self.should_block_canvas(ctx)
    }

    pub(crate) fn handle_mouse_cursor(&self, ctx: &mut WgpuContext) {
        if self.should_block_canvas(ctx) {
            ctx.set_cursor_icon(CursorIcon::Default);
        } else {
            match self.mode {
                RoomEditorMode::Scene => {
                    ctx.set_cursor_icon(CursorIcon::Default);
                }
                RoomEditorMode::Tilemap => {
                    ctx.set_cursor_icon(CursorIcon::Crosshair);
                }
            }
        }
    }
}

/// Returns a `Rect` hitbox for an entity based on its sprite if it has one,
/// otherwise it returns a hitbox based on the default sprite dimensions.
pub fn entity_hitbox(
    ctx: &WgpuContext,
    entity: Entity,
    position: Vec2,
    camera: &Camera2D,
    ecs: &Ecs,
    asset_manager: &mut AssetManager,
    grid_size: f32,
) -> Rect {
    let size = entity_dimensions(ecs, asset_manager, entity, grid_size);

    // Only use the center-offset for pure placeholder entities (Camera/Light without sprites)
    let is_pure_placeholder = ecs.has::<RoomCamera>(entity)
        || (ecs.has::<Light>(entity) && !ecs.has_any::<(Sprite, Animation, CurrentFrame)>(entity));

    let corrected_pos = if is_pure_placeholder {
        position - vec2(grid_size * 0.5, grid_size * 0.5)
    } else {
        // Apply pivot offset for regular entities
        let pivot = ecs
            .get_store::<Transform>()
            .get(entity)
            .map(|t| t.pivot)
            .unwrap_or(Pivot::TopLeft);
        pivot_adjusted_position(position, size, pivot)
    };

    // Convert the two opposite corners of the entity to screen coords
    let top_left = coord::world_to_screen(ctx, camera, corrected_pos);
    let bottom_right = coord::world_to_screen(ctx, camera, corrected_pos + size);

    // Build the rectangle from those screen‑space points
    let rect_x = top_left.x.min(bottom_right.x);
    let rect_y = top_left.y.min(bottom_right.y);
    let rect_w = (bottom_right.x - top_left.x).abs();
    let rect_h = (bottom_right.y - top_left.y).abs();

    Rect::new(rect_x, rect_y, rect_w, rect_h)
}

/// Returns a world-space Rect for an entity based on its sprite or placeholder size.
pub fn entity_world_rect(
    entity: Entity,
    position: Vec2,
    ecs: &Ecs,
    asset_manager: &mut AssetManager,
    grid_size: f32,
) -> Rect {
    let size = entity_dimensions(ecs, asset_manager, entity, grid_size);

    let is_placeholder = ecs.has::<RoomCamera>(entity)
        || (ecs.has::<Light>(entity) && !ecs.has_any::<(Sprite, Animation, CurrentFrame)>(entity));

    let corrected_pos = if is_placeholder {
        position - vec2(grid_size * 0.5, grid_size * 0.5)
    } else {
        let pivot = ecs
            .get_store::<Transform>()
            .get(entity)
            .map(|t| t.pivot)
            .unwrap_or(Pivot::TopLeft);
        pivot_adjusted_position(position, size, pivot)
    };

    Rect::new(corrected_pos.x, corrected_pos.y, size.x, size.y)
}

/// Returns true if an entity can be selected in a room (is in the room).
pub fn can_select_entity_in_room(ecs: &Ecs, entity: Entity, room_id: RoomId) -> bool {
    // Make sure the entity is in the requested room
    match ecs.get_store::<CurrentRoom>().get(entity) {
        Some(CurrentRoom(id)) => *id == room_id,
        None => false,
    }
}
