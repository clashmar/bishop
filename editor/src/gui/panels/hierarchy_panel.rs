use crate::app::EditorMode;
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::prefab::prefab_editor::selection::is_prefab_entity;
use crate::room::room_editor::RoomEditor;
use crate::shared::scene_ui::hierarchy::{
    draw_scene_entity_tree, SceneHierarchyFrame, SceneHierarchyHost, SceneHierarchySelectionAction,
    SceneUiHost,
};
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::theme::with_theme;
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

pub(crate) struct RoomHierarchyHost<'a> {
    pub(crate) room_editor: &'a mut RoomEditor,
    pub(crate) mode: EditorMode,
    pub(crate) prefab_manager: Option<&'a PrefabManager>,
}

impl SceneUiHost for RoomHierarchyHost<'_> {
    fn command_mode(&self) -> EditorMode {
        self.mode
    }

    fn prefab_manager(&self) -> Option<&PrefabManager> {
        self.prefab_manager
    }
}

impl SceneHierarchyHost for RoomHierarchyHost<'_> {
    fn is_selected(&self, entity: Entity) -> bool {
        self.room_editor.is_selected(entity)
    }

    fn apply_selection_action(&mut self, entity: Entity, action: SceneHierarchySelectionAction) {
        match action {
            SceneHierarchySelectionAction::Replace => {
                self.room_editor.set_selected_entity(Some(entity));
            }
            SceneHierarchySelectionAction::Toggle => {
                self.room_editor.toggle_entity_selection(entity);
            }
        }
    }
}

pub(crate) struct PrefabHierarchyHost<'a> {
    pub(crate) prefab_editor: &'a mut crate::prefab::prefab_editor::PrefabEditor,
    pub(crate) mode: EditorMode,
}

impl SceneUiHost for PrefabHierarchyHost<'_> {
    fn command_mode(&self) -> EditorMode {
        self.mode
    }

    fn prefab_manager(&self) -> Option<&PrefabManager> {
        None
    }
}

impl SceneHierarchyHost for PrefabHierarchyHost<'_> {
    fn is_selected(&self, entity: Entity) -> bool {
        self.prefab_editor.is_selected(entity)
    }

    fn apply_selection_action(&mut self, entity: Entity, action: SceneHierarchySelectionAction) {
        match action {
            SceneHierarchySelectionAction::Replace => {
                self.prefab_editor.set_selected_entity(Some(entity));
            }
            SceneHierarchySelectionAction::Toggle => {
                self.prefab_editor.toggle_entity_selection(entity);
            }
        }
    }
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
                .rooms()
                .iter()
                .find(|r| r.id == room_id)
                .map(|r| r.position)
        });

        let game = &mut editor.game;
        let room_mode_prefab_manager = room_mode_prefab_manager(cur_room_id, &game.prefab_manager);
        let ecs = &mut game.ecs;
        let room_editor = &mut editor.room_editor;
        prune_dead_hierarchy_state(ecs, &mut self.expanded, &mut self.dragging);

        let global_entities = {
            let store = ecs.get_store::<Global>();
            let all: Vec<Entity> = store.data.keys().copied().collect();
            get_root_entities(ecs, &all)
        };

        let room_entities = if let Some(room_id) = cur_room_id {
            let entities = ecs.entities_in_room(room_id).clone();
            let entities: Vec<Entity> = entities.into_iter().collect();
            get_root_entities(ecs, &entities)
        } else {
            Vec::new()
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
        let mut room_host = RoomHierarchyHost {
            room_editor,
            mode: EditorMode::Game,
            prefab_manager: room_mode_prefab_manager,
        };

        // Add global button
        let btn_w = area.usable_width();
        if area.is_fully_visible(y, ADD_BUTTON_HEIGHT) {
            let clicked = Button::new(
                Rect::new(rect.x + 6., y, btn_w, ADD_BUTTON_HEIGHT),
                "+ Global",
            )
            .suppressed(blocked)
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
                with_theme(|t| t.text_muted),
            );
        }
        y += HEADER_HEIGHT;

        {
            let mut global_draw = SceneHierarchyFrame {
                ctx,
                panel_rect: rect,
                area: &area,
                blocked,
                expanded: &mut self.expanded,
                dragging: &mut self.dragging,
                drag_offset: &mut self.drag_offset,
            };

            for entity in global_entities {
                draw_scene_entity_tree(entity, 0, &mut y, &mut global_draw, &mut room_host, ecs);
            }
        }

        y += ROW_SPACING;

        // Room header
        if area.is_visible(y, HEADER_HEIGHT) {
            ctx.draw_text(
                "Room",
                rect.x + 6.,
                y + 14.,
                HEADER_FONT_SIZE,
                with_theme(|t| t.text_muted),
            );
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
                    .suppressed(blocked)
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
        room_host.mode = room_mode;

        {
            let mut room_draw = SceneHierarchyFrame {
                ctx,
                panel_rect: rect,
                area: &area,
                blocked,
                expanded: &mut self.expanded,
                dragging: &mut self.dragging,
                drag_offset: &mut self.drag_offset,
            };

            for entity in room_entities {
                draw_scene_entity_tree(entity, 0, &mut y, &mut room_draw, &mut room_host, ecs);
            }
        }

        area.draw_scrollbar(ctx, &self.scroll_state);
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
        let mut prefab_host = PrefabHierarchyHost {
            prefab_editor,
            mode: editor.mode,
        };

        if area.is_visible(y, HEADER_HEIGHT) {
            ctx.draw_text(
                "Prefab",
                rect.x + 6.,
                y + 14.,
                HEADER_FONT_SIZE,
                with_theme(|t| t.text_muted),
            );
        }
        y += HEADER_HEIGHT;

        {
            let mut prefab_draw = SceneHierarchyFrame {
                ctx,
                panel_rect: rect,
                area: &area,
                blocked,
                expanded: &mut self.expanded,
                dragging: &mut self.dragging,
                drag_offset: &mut self.drag_offset,
            };

            for entity in prefab_entities {
                draw_scene_entity_tree(entity, 0, &mut y, &mut prefab_draw, &mut prefab_host, ecs);
            }
        }

        area.draw_scrollbar(ctx, &self.scroll_state);
        draw_drag_ghost(ctx, ecs, &mut self.dragging, self.drag_offset);
    }
}

pub(crate) fn layout_entity_tree(
    entity: Entity,
    y: &mut f32,
    expanded: &HashSet<Entity>,
    ecs: &Ecs,
) {
    *y += ROW_HEIGHT;
    if expanded.contains(&entity) && has_children(ecs, entity) {
        for child in get_children(ecs, entity) {
            layout_entity_tree(child, y, expanded, ecs);
        }
    }
}

fn get_entity_name(ecs: &Ecs, entity: Entity) -> String {
    ecs.get::<Name>(entity)
        .map(|n| n.to_string())
        .unwrap_or_else(|| format!("{:?}", entity))
}

pub(crate) fn prune_dead_hierarchy_state(
    ecs: &Ecs,
    expanded: &mut HashSet<Entity>,
    dragging: &mut Option<Entity>,
) {
    expanded.retain(|entity| entity_exists_in_hierarchy(ecs, *entity));

    if dragging.is_some_and(|entity| !entity_exists_in_hierarchy(ecs, entity)) {
        *dragging = None;
    }
}

pub(crate) fn sync_prefab_root_expansion(
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

pub(crate) fn clear_drag_on_mouse_release(dragging: &mut Option<Entity>, mouse_released: bool) {
    if mouse_released {
        *dragging = None;
    }
}

pub(crate) fn room_mode_prefab_manager(
    cur_room_id: Option<RoomId>,
    prefab_manager: &PrefabManager,
) -> Option<&PrefabManager> {
    cur_room_id.map(|_| prefab_manager)
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
            Color::new(0.3, 0.5, 0.7, 1.0).with_alpha(0.5),
        );
        ctx.draw_text(
            &name,
            mx - drag_offset.x + 4.0,
            my - drag_offset.y + 16.0,
            14.0,
            with_theme(|t| t.text),
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
    ecs
        .create_entity()
        .with(PlayerProxy)
        .with(Transform {
            position: room_position,
            ..Default::default()
        })
        .with(Name("Player Proxy".to_string()))
        .with_current_room(room_id)
        .finish();
}
