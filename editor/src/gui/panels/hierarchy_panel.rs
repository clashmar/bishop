use crate::app::EditorMode;
use crate::commands::room::{RemoveParentCmd, SetParentCmd};
use crate::editor_global::push_command;
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::prefab::prefab_editor::{is_prefab_entity, linked_prefab_display};
use crate::room::room_editor::RoomEditor;
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashSet;

const ROW_HEIGHT: f32 = 22.0;
const ROW_SPACING: f32 = 5.0;
const HEADER_HEIGHT: f32 = 18.0;
const ADD_BUTTON_HEIGHT: f32 = 26.0;
const TOP_PADDING: f32 = 8.0;
const BOTTOM_PADDING: f32 = 8.0;
const HEADER_FONT_SIZE: f32 = 15.0;

pub struct HierarchyPanel {
    expanded: HashSet<Entity>,
    prefab_seen_roots: HashSet<Entity>,
    prefab_expansion_session: Option<PrefabId>,
    dragging: Option<Entity>,
    drag_offset: Vec2,
    scroll_state: ScrollState,
}

impl HierarchyPanel {
    pub fn new() -> Self {
        Self {
            expanded: HashSet::new(),
            prefab_seen_roots: HashSet::new(),
            prefab_expansion_session: None,
            dragging: None,
            drag_offset: Vec2::ZERO,
            scroll_state: ScrollState::new(),
        }
    }
}

pub const HIERARCHY_PANEL: &str = "Hierarchy";

impl PanelDefinition for HierarchyPanel {
    fn title(&self) -> &'static str {
        HIERARCHY_PANEL
    }

    fn default_rect(&self, _ctx: &WgpuContext) -> Rect {
        Rect::new(20., 60., 260., 400.)
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        if matches!(editor.mode, EditorMode::Prefab(_)) {
            self.draw_prefab(ctx, rect, editor, blocked);
            return;
        }

        self.prefab_expansion_session = None;
        self.prefab_seen_roots.clear();

        let cur_room_id = editor.cur_room_id;

        // Get room position before borrowing ecs mutably
        let room_pos = cur_room_id.and_then(|room_id| {
            editor
                .game
                .current_world()
                .rooms
                .iter()
                .find(|r| r.id == room_id)
                .map(|r| r.position)
        });

        let game = &mut editor.game;
        let room_mode_prefab_library =
            room_mode_prefab_library(cur_room_id, &game.prefab_library);
        let ecs = &mut game.ecs;
        let room_editor = &mut editor.room_editor;
        prune_dead_hierarchy_state(ecs, &mut self.expanded, &mut self.dragging);

        let global_entities = {
            let store = ecs.get_store::<Global>();
            let all: Vec<Entity> = store.data.keys().copied().collect();
            get_root_entities(ecs, &all)
        };

        let room_entities = {
            let cur_room_store = ecs.get_store::<CurrentRoom>();
            let entities: Vec<Entity> = cur_room_store
                .data
                .iter()
                .filter_map(|(&entity, cur_room_comp)| {
                    if cur_room_comp.0 == cur_room_id.unwrap() {
                        Some(entity)
                    } else {
                        None
                    }
                })
                .collect();
            get_root_entities(ecs, &entities)
        };

        // Layout pass
        let mut layout_y = 0.0;

        layout_y += TOP_PADDING;
        layout_y += ADD_BUTTON_HEIGHT + ROW_SPACING;
        layout_y += HEADER_HEIGHT;

        for entity in &global_entities {
            layout_entity_tree(*entity, &mut layout_y, &self.expanded, ecs);
        }

        layout_y += 10.0;
        layout_y += HEADER_HEIGHT;

        // Account for proxy button if room doesn't have one
        if let Some(room_id) = cur_room_id {
            if ecs.get_player_proxy(room_id).is_none() {
                layout_y += ADD_BUTTON_HEIGHT + ROW_SPACING;
            }
        }

        for entity in &room_entities {
            layout_entity_tree(*entity, &mut layout_y, &self.expanded, ecs);
        }

        layout_y += BOTTOM_PADDING;

        let content_height = layout_y;
        let area = ScrollableArea::new(rect, content_height)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);

        // Draw pass
        let mut y = rect.y + self.scroll_state.scroll_y + TOP_PADDING;

        // Add global button
        let btn_w = area.usable_width();
        if area.is_fully_visible(y, ADD_BUTTON_HEIGHT) {
            let clicked = Button::new(
                Rect::new(rect.x + 6., y, btn_w, ADD_BUTTON_HEIGHT),
                "+ Global",
            )
            .blocked(blocked)
            .show(ctx);
            if !blocked && clicked {
                ecs.create_entity()
                    .with(Global::default())
                    .with(Name("Global Entity".into()));
            }
        }

        y += ADD_BUTTON_HEIGHT + ROW_SPACING;

        // Global header
        if area.is_visible(y, HEADER_HEIGHT) {
            ctx.draw_text(
                "Global",
                rect.x + 6.,
                y + 14.,
                HEADER_FONT_SIZE,
                Color::GREY,
            );
        }
        y += HEADER_HEIGHT;

        // Global entities use EditorMode::Game for undo scope
        let mut global_draw = EntityTreeDrawContext {
            ctx,
            panel_rect: rect,
            expanded: &mut self.expanded,
            dragging: &mut self.dragging,
            drag_offset: &mut self.drag_offset,
            room_editor,
            ecs,
            area: &area,
            blocked,
            mode: EditorMode::Game,
            prefab_library: room_mode_prefab_library,
        };

        for entity in global_entities {
            draw_entity_tree(entity, 0, &mut y, &mut global_draw);
        }

        y += ROW_SPACING;

        // Room header
        if area.is_visible(y, HEADER_HEIGHT) {
            ctx.draw_text("Room", rect.x + 6., y + 14., HEADER_FONT_SIZE, Color::GREY);
        }
        y += HEADER_HEIGHT;

        // Add proxy button if the room has none already
        if let Some(room_id) = cur_room_id {
            let has_spawn = ecs.get_player_proxy(room_id).is_some();
            if !has_spawn {
                let spawn_pos = room_pos.unwrap_or_default();
                if area.is_fully_visible(y, ADD_BUTTON_HEIGHT) {
                    let clicked = Button::new(
                        Rect::new(rect.x + 6., y, btn_w, ADD_BUTTON_HEIGHT),
                        "+ Player Proxy",
                    )
                    .blocked(blocked)
                    .show(ctx);
                    if !blocked && clicked {
                        create_spawn_point(ecs, room_id, spawn_pos);
                    }
                }
                y += ADD_BUTTON_HEIGHT + ROW_SPACING;
            }
        }

        // Room entities use EditorMode::Room for undo scope
        let room_mode = cur_room_id
            .map(EditorMode::Room)
            .unwrap_or(EditorMode::Game);

        let mut room_draw = EntityTreeDrawContext {
            ctx,
            panel_rect: rect,
            expanded: &mut self.expanded,
            dragging: &mut self.dragging,
            drag_offset: &mut self.drag_offset,
            room_editor,
            ecs,
            area: &area,
            blocked,
            mode: room_mode,
            prefab_library: room_mode_prefab_library,
        };
        
        for entity in room_entities {
            draw_entity_tree(entity, 0, &mut y, &mut room_draw);
        }

        area.draw_scrollbar(ctx, self.scroll_state.scroll_y);
        draw_drag_ghost(ctx, ecs, &mut self.dragging, self.drag_offset);
    }
}

impl HierarchyPanel {
    fn draw_prefab(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        editor: &mut Editor,
        blocked: bool,
    ) {
        let (Some(prefab_editor), Some(prefab_stage)) =
            (editor.prefab_editor.as_mut(), editor.prefab_stage.as_mut())
        else {
            return;
        };
        let EditorMode::Prefab(prefab_id) = editor.mode else {
            return;
        };
        if self.prefab_expansion_session != Some(prefab_id) {
            self.prefab_expansion_session = Some(prefab_id);
            self.prefab_seen_roots.clear();
        }
        let ecs = &mut prefab_stage.ecs;
        prune_dead_hierarchy_state(ecs, &mut self.expanded, &mut self.dragging);

        let prefab_entities = {
            let entities = ecs
                .get_store::<Transform>()
                .data
                .iter()
                .filter_map(|(&entity, _)| is_prefab_entity(ecs, entity).then_some(entity))
                .collect::<Vec<_>>();
            get_root_entities(ecs, &entities)
        };
        sync_prefab_root_expansion(
            &prefab_entities,
            &mut self.expanded,
            &mut self.prefab_seen_roots,
        );

        let mut layout_y = TOP_PADDING + HEADER_HEIGHT;
        for entity in &prefab_entities {
            layout_entity_tree(*entity, &mut layout_y, &self.expanded, ecs);
        }
        layout_y += BOTTOM_PADDING;

        let area = ScrollableArea::new(rect, layout_y)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);
        let mut y = rect.y + self.scroll_state.scroll_y + TOP_PADDING;

        if area.is_visible(y, HEADER_HEIGHT) {
            ctx.draw_text("Prefab", rect.x + 6., y + 14., HEADER_FONT_SIZE, Color::GREY);
        }
        y += HEADER_HEIGHT;

        for entity in prefab_entities {
            draw_prefab_entity_tree(
                entity,
                0,
                &mut y,
                ctx,
                rect,
                &area,
                &mut self.expanded,
                &mut self.dragging,
                &mut self.drag_offset,
                prefab_editor,
                ecs,
                blocked,
                editor.mode,
            );
        }

        area.draw_scrollbar(ctx, self.scroll_state.scroll_y);
        draw_drag_ghost(ctx, ecs, &mut self.dragging, self.drag_offset);
    }
}

fn layout_entity_tree(entity: Entity, y: &mut f32, expanded: &HashSet<Entity>, ecs: &Ecs) {
    *y += ROW_HEIGHT;
    if expanded.contains(&entity) && has_children(ecs, entity) {
        for child in get_children(ecs, entity) {
            layout_entity_tree(child, y, expanded, ecs);
        }
    }
}

struct EntityTreeDrawContext<'a> {
    ctx: &'a mut WgpuContext,
    panel_rect: Rect,
    expanded: &'a mut HashSet<Entity>,
    dragging: &'a mut Option<Entity>,
    drag_offset: &'a mut Vec2,
    room_editor: &'a mut RoomEditor,
    ecs: &'a mut Ecs,
    area: &'a ActiveScrollArea,
    blocked: bool,
    mode: EditorMode,
    prefab_library: Option<&'a PrefabLibrary>,
}

fn draw_entity_tree(
    entity: Entity,
    depth: usize,
    y: &mut f32,
    draw: &mut EntityTreeDrawContext<'_>,
) {
    let panel_rect = draw.panel_rect;
    let area = draw.area;
    let blocked = draw.blocked;
    let mode = draw.mode;
    let usable_w = area.usable_width();
    let indent = depth as f32 * 16.0;
    let row_rect = Rect::new(
        panel_rect.x + 6.0 + indent,
        *y,
        usable_w - indent,
        ROW_HEIGHT,
    );

    // Track pending parent action to execute after drawing
    let mut pending_set_parent: Option<(Entity, Entity)> = None;

    // Check visibility before drawing
    if area.is_fully_visible(row_rect.y, row_rect.h) {
        let ctx = &mut *draw.ctx;
        let ecs = &mut *draw.ecs;
        let room_editor = &mut *draw.room_editor;
        let has_children = has_children(ecs, entity);
        let is_expanded = draw.expanded.contains(&entity);
        let mouse: Vec2 = ctx.mouse_position().into();
        let mouse_over = row_rect.contains(mouse);
        let expand_button_rect = Rect::new(row_rect.x, row_rect.y, 14.0, ROW_HEIGHT);
        let expand_button_hovered = has_children && expand_button_rect.contains(mouse);

        // Selection highlight
        if room_editor.is_selected(entity) {
            ctx.draw_rectangle(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                Color::new(0.25, 0.45, 0.85, 0.35),
            );
        }

        // Expand/collapse buttons
        if has_children {
            let symbol = if is_expanded { "-" } else { "+" };
            let clicked = Button::new(expand_button_rect, symbol)
                .plain()
                .text_color(Color::WHITE)
                .hover_color(Color::GREY)
                .blocked(blocked)
                .show(ctx);
            if !blocked && clicked {
                if is_expanded {
                    draw.expanded.remove(&entity);
                } else {
                    draw.expanded.insert(entity);
                }
            }
        }

        // Selection with Shift support for multi-select
        if !blocked
            && mouse_over
            && !expand_button_hovered
            && ctx.is_mouse_button_pressed(MouseButton::Left)
            && draw.dragging.is_none()
        {
            let shift_held =
                ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
            if shift_held {
                // Toggle entity in selection
                if room_editor.is_selected(entity) {
                    room_editor.selected_entities.remove(&entity);
                    // Update inspector if we now have single or no selection
                    if room_editor.selected_entities.len() == 1 {
                        let remaining = *room_editor.selected_entities.iter().next().unwrap();
                        room_editor.inspector.set_target(Some(remaining));
                    } else {
                        room_editor.inspector.set_target(None);
                    }
                } else {
                    room_editor.add_to_selection(entity);
                }
            } else {
                room_editor.set_selected_entity(Some(entity));
            }
        }

        // Start drag
        if !blocked
            && mouse_over
            && !expand_button_hovered
            && ctx.is_mouse_button_pressed(MouseButton::Left)
            && draw.dragging.is_none()
        {
            *draw.dragging = Some(entity);
            *draw.drag_offset = mouse - row_rect.top_left();
        }

        // Drop target to parent
        if !blocked {
            if let Some(dragged) = *draw.dragging {
                if dragged != entity && mouse_over && !is_ancestor(ecs, dragged, entity) {
                    ctx.draw_rectangle(
                        row_rect.x,
                        row_rect.y,
                        row_rect.w,
                        row_rect.h,
                        Color::new(0.3, 0.7, 0.3, 0.3),
                    );
                    if ctx.is_mouse_button_released(MouseButton::Left) {
                        pending_set_parent = Some((dragged, entity));
                        draw.expanded.insert(entity);
                        *draw.dragging = None;
                    }
                }
            }
        }

        // Entity name
        ctx.draw_text(
            &get_entity_name(ecs, entity),
            row_rect.x + 18.0,
            row_rect.y + 16.0,
            14.0,
            Color::WHITE,
        );

        if let Some(prefab_library) = draw.prefab_library.as_ref() {
            if let Some(prefab_display) = linked_prefab_display(ecs, prefab_library, entity) {
                let badge_font_size = 11.0;
                let badge_padding_x = 6.0;
                let badge_padding_y = 3.0;
                let badge_text = prefab_display.label;
                let badge_dims = measure_text(ctx, &badge_text, badge_font_size);
                let badge_w = badge_dims.width + badge_padding_x * 2.0;
                let badge_h = badge_dims.height + badge_padding_y * 2.0;
                let badge_x = (row_rect.x + row_rect.w - badge_w).max(row_rect.x + 18.0);
                let badge_rect = Rect::new(badge_x, row_rect.y + 3.0, badge_w, badge_h);

                ctx.draw_rectangle(
                    badge_rect.x,
                    badge_rect.y,
                    badge_rect.w,
                    badge_rect.h,
                    Color::new(0.19, 0.24, 0.36, 0.95),
                );
                ctx.draw_rectangle_lines(
                    badge_rect.x,
                    badge_rect.y,
                    badge_rect.w,
                    badge_rect.h,
                    1.0,
                    Color::new(0.48, 0.62, 0.92, 1.0),
                );
                ctx.draw_text(
                    &badge_text,
                    badge_rect.x + badge_padding_x,
                    badge_rect.y + badge_dims.offset_y + badge_padding_y,
                    badge_font_size,
                    Color::WHITE,
                );
            }
        }
    }

    // Execute pending set_parent action as undoable command
    if let Some((child, new_parent)) = pending_set_parent {
        let ecs = &mut *draw.ecs;
        let old_parent = get_parent(ecs, child);
        push_command(Box::new(SetParentCmd::new(
            child, new_parent, old_parent, mode,
        )));
    }

    *y += ROW_HEIGHT;

    // Recursively draw children
    let should_draw_children = {
        let ecs = &*draw.ecs;
        draw.expanded.contains(&entity) && has_children(ecs, entity)
    };
    if should_draw_children {
        let children = {
            let ecs = &*draw.ecs;
            get_children(ecs, entity)
        };
        for child in children {
            draw_entity_tree(child, depth + 1, y, draw);
        }
    }

    // Unparent by dragging outside panel
    if !blocked {
        if let Some(dragged) = *draw.dragging {
            if dragged == entity {
                let ctx = &mut *draw.ctx;
                let mouse: Vec2 = ctx.mouse_position().into();
                if !panel_rect.contains(mouse) && ctx.is_mouse_button_released(MouseButton::Left) {
                    let ecs = &mut *draw.ecs;
                    let old_parent = get_parent(ecs, dragged);
                    push_command(Box::new(RemoveParentCmd::new(dragged, old_parent, mode)));
                    *draw.dragging = None;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_prefab_entity_tree(
    entity: Entity,
    depth: usize,
    y: &mut f32,
    ctx: &mut WgpuContext,
    panel_rect: Rect,
    area: &ActiveScrollArea,
    expanded: &mut HashSet<Entity>,
    dragging: &mut Option<Entity>,
    drag_offset: &mut Vec2,
    prefab_editor: &mut crate::prefab::PrefabEditor,
    ecs: &mut Ecs,
    blocked: bool,
    mode: EditorMode,
) {
    let usable_w = area.usable_width();
    let indent = depth as f32 * 16.0;
    let row_rect = Rect::new(
        panel_rect.x + 6.0 + indent,
        *y,
        usable_w - indent,
        ROW_HEIGHT,
    );

    let mut pending_set_parent: Option<(Entity, Entity)> = None;

    if area.is_fully_visible(row_rect.y, row_rect.h) {
        let has_children = has_children(ecs, entity);
        let is_expanded = expanded.contains(&entity);
        let mouse: Vec2 = ctx.mouse_position().into();
        let mouse_over = row_rect.contains(mouse);
        let expand_button_rect = Rect::new(row_rect.x, row_rect.y, 14.0, ROW_HEIGHT);
        let expand_button_hovered = has_children && expand_button_rect.contains(mouse);

        if prefab_editor.is_selected(entity) {
            ctx.draw_rectangle(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                Color::new(0.25, 0.45, 0.85, 0.35),
            );
        }

        if has_children {
            let symbol = if is_expanded { "-" } else { "+" };
            let clicked = Button::new(expand_button_rect, symbol)
                .plain()
                .text_color(Color::WHITE)
                .hover_color(Color::GREY)
                .blocked(blocked)
                .show(ctx);
            if !blocked && clicked {
                if is_expanded {
                    expanded.remove(&entity);
                } else {
                    expanded.insert(entity);
                }
            }
        }

        if !blocked
            && mouse_over
            && !expand_button_hovered
            && ctx.is_mouse_button_pressed(MouseButton::Left)
            && dragging.is_none()
        {
            let shift_held =
                ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
            if shift_held {
                if prefab_editor.is_selected(entity) {
                    prefab_editor.selected_entities.remove(&entity);
                    prefab_editor
                        .inspector
                        .set_target(prefab_editor.single_selected_entity());
                } else {
                    prefab_editor.add_to_selection(entity);
                }
            } else {
                prefab_editor.set_selected_entity(Some(entity));
            }
        }

        if !blocked
            && mouse_over
            && !expand_button_hovered
            && ctx.is_mouse_button_pressed(MouseButton::Left)
            && dragging.is_none()
        {
            *dragging = Some(entity);
            *drag_offset = mouse - row_rect.top_left();
        }

        if !blocked {
            if let Some(dragged) = *dragging {
                if dragged != entity && mouse_over && !is_ancestor(ecs, dragged, entity) {
                    ctx.draw_rectangle(
                        row_rect.x,
                        row_rect.y,
                        row_rect.w,
                        row_rect.h,
                        Color::new(0.3, 0.7, 0.3, 0.3),
                    );
                    if ctx.is_mouse_button_released(MouseButton::Left) {
                        pending_set_parent = Some((dragged, entity));
                        expanded.insert(entity);
                        *dragging = None;
                    }
                }
            }
        }

        ctx.draw_text(
            &get_entity_name(ecs, entity),
            row_rect.x + 18.0,
            row_rect.y + 16.0,
            14.0,
            Color::WHITE,
        );
    }

    if let Some((child, new_parent)) = pending_set_parent {
        let old_parent = get_parent(ecs, child);
        push_command(Box::new(SetParentCmd::new(
            child, new_parent, old_parent, mode,
        )));
    }

    *y += ROW_HEIGHT;

    if expanded.contains(&entity) && has_children(ecs, entity) {
        for child in get_children(ecs, entity) {
            draw_prefab_entity_tree(
                child,
                depth + 1,
                y,
                ctx,
                panel_rect,
                area,
                expanded,
                dragging,
                drag_offset,
                prefab_editor,
                ecs,
                blocked,
                mode,
            );
        }
    }

    if !blocked {
        if let Some(dragged) = *dragging {
            if dragged == entity {
                let mouse: Vec2 = ctx.mouse_position().into();
                if !panel_rect.contains(mouse) && ctx.is_mouse_button_released(MouseButton::Left) {
                    let old_parent = get_parent(ecs, dragged);
                    push_command(Box::new(RemoveParentCmd::new(dragged, old_parent, mode)));
                    *dragging = None;
                }
            }
        }
    }
}

fn get_entity_name(ecs: &Ecs, entity: Entity) -> String {
    ecs.get::<Name>(entity)
        .map(|n| n.to_string())
        .unwrap_or_else(|| format!("{:?}", entity))
}

fn prune_dead_hierarchy_state(
    ecs: &Ecs,
    expanded: &mut HashSet<Entity>,
    dragging: &mut Option<Entity>,
) {
    expanded.retain(|entity| entity_exists_in_hierarchy(ecs, *entity));

    if dragging.is_some_and(|entity| !entity_exists_in_hierarchy(ecs, entity)) {
        *dragging = None;
    }
}

fn sync_prefab_root_expansion(
    prefab_roots: &[Entity],
    expanded: &mut HashSet<Entity>,
    seen_roots: &mut HashSet<Entity>,
) {
    let live_roots = prefab_roots.iter().copied().collect::<HashSet<_>>();
    seen_roots.retain(|entity| live_roots.contains(entity));

    for &root in prefab_roots {
        if seen_roots.insert(root) {
            expanded.insert(root);
        }
    }
}

fn clear_drag_on_mouse_release(dragging: &mut Option<Entity>, mouse_released: bool) {
    if mouse_released {
        *dragging = None;
    }
}

fn room_mode_prefab_library(
    cur_room_id: Option<RoomId>,
    prefab_library: &PrefabLibrary,
) -> Option<&PrefabLibrary> {
    cur_room_id.map(|_| prefab_library)
}

fn draw_drag_ghost(
    ctx: &mut WgpuContext,
    ecs: &Ecs,
    dragging: &mut Option<Entity>,
    drag_offset: Vec2,
) {
    if let Some(dragged) = *dragging {
        let (mx, my) = ctx.mouse_position();
        let name = get_entity_name(ecs, dragged);
        ctx.draw_rectangle(
            mx - drag_offset.x,
            my - drag_offset.y,
            150.0,
            ROW_HEIGHT,
            Color::new(0.3, 0.5, 0.7, 0.5),
        );
        ctx.draw_text(
            &name,
            mx - drag_offset.x + 4.0,
            my - drag_offset.y + 16.0,
            14.0,
            Color::WHITE,
        );
        clear_drag_on_mouse_release(dragging, ctx.is_mouse_button_released(MouseButton::Left));
    }
}

fn entity_exists_in_hierarchy(ecs: &Ecs, entity: Entity) -> bool {
    ecs.get_store::<Transform>().contains(entity)
        || ecs.get_store::<Name>().contains(entity)
        || ecs.get_store::<Parent>().contains(entity)
        || ecs.get_store::<Children>().contains(entity)
        || ecs.get_store::<Global>().contains(entity)
        || ecs.get_store::<CurrentRoom>().contains(entity)
        || ecs.get_store::<RoomCamera>().contains(entity)
        || ecs.get_store::<PlayerProxy>().contains(entity)
        || ecs.get_store::<Player>().contains(entity)
}

/// Creates a player proxy entity at the room's origin.
fn create_spawn_point(ecs: &mut Ecs, room_id: RoomId, room_position: Vec2) {
    ecs.create_entity()
        .with(PlayerProxy)
        .with(Transform {
            position: room_position,
            ..Default::default()
        })
        .with(CurrentRoom(room_id))
        .with(Name("Player Proxy".to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Editor;
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};

    #[test]
    fn prune_dead_hierarchy_state_removes_deleted_entities() {
        let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("hierarchy_prune_dead");
        let mut stage = crate::prefab::PrefabStage::new(test_game.name());
        let live = stage
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Live".to_string()))
            .finish();
        let dead = stage
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Dead".to_string()))
            .finish();

        {
            let mut ctx = stage.ctx_mut();
            Ecs::remove_entity(&mut ctx, dead);
        }

        let mut expanded = HashSet::from([live, dead]);
        let mut dragging = Some(dead);

        prune_dead_hierarchy_state(&stage.ecs, &mut expanded, &mut dragging);

        assert_eq!(expanded, HashSet::from([live]));
        assert_eq!(dragging, None);
    }

    #[test]
    fn layout_entity_tree_includes_children_when_root_is_expanded() {
        let mut ecs = Ecs::default();
        let root = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Root".to_string()))
            .finish();
        let child = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Child".to_string()))
            .finish();
        set_parent(&mut ecs, child, root);

        let mut y = 0.0;
        layout_entity_tree(root, &mut y, &HashSet::new(), &ecs);
        assert_eq!(y, ROW_HEIGHT);

        let mut expanded_y = 0.0;
        layout_entity_tree(root, &mut expanded_y, &HashSet::from([root]), &ecs);
        assert_eq!(expanded_y, ROW_HEIGHT * 2.0);
    }

    #[test]
    fn sync_prefab_root_expansion_expands_new_roots_once() {
        let root = Entity(1);
        let mut expanded = HashSet::new();
        let mut seen_roots = HashSet::new();

        sync_prefab_root_expansion(&[root], &mut expanded, &mut seen_roots);
        assert!(expanded.contains(&root));
        assert!(seen_roots.contains(&root));

        expanded.remove(&root);
        sync_prefab_root_expansion(&[root], &mut expanded, &mut seen_roots);
        assert!(!expanded.contains(&root));
    }

    #[test]
    fn sync_prefab_root_expansion_expands_roots_when_they_first_appear() {
        let root_a = Entity(1);
        let root_b = Entity(2);
        let mut expanded = HashSet::new();
        let mut seen_roots = HashSet::new();

        sync_prefab_root_expansion(&[root_a], &mut expanded, &mut seen_roots);
        expanded.remove(&root_a);

        sync_prefab_root_expansion(&[root_a, root_b], &mut expanded, &mut seen_roots);

        assert!(!expanded.contains(&root_a));
        assert!(expanded.contains(&root_b));
    }

    #[test]
    fn clear_drag_on_mouse_release_clears_prefab_drag_on_blank_space_release() {
        let dragged = Entity(7);
        let mut dragging = Some(dragged);

        clear_drag_on_mouse_release(&mut dragging, true);

        assert_eq!(dragging, None);
    }

    #[test]
    fn room_mode_prefab_library_is_available_for_global_and_room_rows() {
        let mut editor = Editor {
            cur_room_id: Some(RoomId(1)),
            ..Default::default()
        };

        let prefab_library =
            room_mode_prefab_library(editor.cur_room_id, &editor.game.prefab_library).unwrap();
        assert!(std::ptr::eq(prefab_library, &editor.game.prefab_library));

        editor.cur_room_id = None;
        assert!(room_mode_prefab_library(editor.cur_room_id, &editor.game.prefab_library).is_none());
    }
}
