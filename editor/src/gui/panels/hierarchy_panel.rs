use crate::app::EditorMode;
use crate::commands::room::{RemoveParentCmd, SetParentCmd};
use crate::editor_global::push_command;
use crate::gui::panels::generic_panel::PanelDefinition;
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
    dragging: Option<Entity>,
    drag_offset: Vec2,
    scroll_state: ScrollState,
}

impl HierarchyPanel {
    pub fn new() -> Self {
        Self {
            expanded: HashSet::new(),
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

        let ecs = &mut editor.game.ecs;
        let room_editor = &mut editor.room_editor;

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
        };
        
        for entity in room_entities {
            draw_entity_tree(entity, 0, &mut y, &mut room_draw);
        }

        area.draw_scrollbar(ctx, self.scroll_state.scroll_y);

        // Drag ghost
        if let Some(dragged) = self.dragging {
            let (mx, my) = ctx.mouse_position();
            let name = get_entity_name(ecs, dragged);
            ctx.draw_rectangle(
                mx - self.drag_offset.x,
                my - self.drag_offset.y,
                150.0,
                ROW_HEIGHT,
                Color::new(0.3, 0.5, 0.7, 0.5),
            );
            ctx.draw_text(
                &name,
                mx - self.drag_offset.x + 4.0,
                my - self.drag_offset.y + 16.0,
                14.0,
                Color::WHITE,
            );
            if ctx.is_mouse_button_released(MouseButton::Left) {
                self.dragging = None;
            }
        }
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
            let btn = Rect::new(row_rect.x, row_rect.y, 14.0, ROW_HEIGHT);
            let symbol = if is_expanded { "-" } else { "+" };
            let clicked = Button::new(btn, symbol)
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

fn get_entity_name(ecs: &Ecs, entity: Entity) -> String {
    ecs.get::<Name>(entity)
        .map(|n| n.to_string())
        .unwrap_or_else(|| format!("{:?}", entity))
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
