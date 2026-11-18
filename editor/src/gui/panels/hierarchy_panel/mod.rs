use crate::app::EditorMode;
use crate::game::GameEditorSubmode;
use crate::commands::scene::CreateSceneEntityCmd;
use crate::editor_global::push_command;
use crate::gui::panels::generic_panel::{PanelDefinition, PanelResizeLimits};
use self::search::{filter_hierarchy, visible_roots, HierarchySearchState};
use crate::prefab::prefab_editor::selection::is_prefab_entity;
use crate::room::room_editor::RoomEditor;
use crate::shared::scene_ui::hierarchy::{
    draw_scene_entity_tree, SceneHierarchyFrame, SceneHierarchyHost, SceneHierarchySelectionAction,
    SceneUiHost, ENTITY_NAME_FONT_SIZE, HIERARCHY_HORIZONTAL_PADDING,
};
use crate::Editor;
use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::ui::measure_text;
use widgets::*;
use engine_core::theme::with_theme;
use std::collections::HashSet;

pub(crate) mod search;

const ROW_HEIGHT: f32 = 22.0;
const ROW_SPACING: f32 = 5.0;
const HEADER_HEIGHT: f32 = 18.0;
const ADD_BUTTON_HEIGHT: f32 = 26.0;
const TOP_PADDING: f32 = 8.0;
const BOTTOM_PADDING: f32 = 8.0;
const HEADER_FONT_SIZE: f32 = 15.0;
const SEARCH_FIELD_HEIGHT: f32 = 26.0;
const SEARCH_WIDGET_ID: WidgetId = WidgetId(0x4849_4552);

pub struct HierarchyPanel {
    expanded: HashSet<Entity>,
    prefab_seen_roots: HashSet<Entity>,
    prefab_expansion_session: Option<PrefabId>,
    search: HierarchySearchState,
    drag_candidate: Option<(Entity, Vec2)>,
    dragging: Option<Entity>,
    scroll_state: ScrollState,
}

pub(crate) struct RoomHierarchyHost<'a> {
    pub(crate) room_editor: &'a mut RoomEditor,
    pub(crate) mode: EditorMode,
}

impl SceneUiHost for RoomHierarchyHost<'_> {
    fn command_mode(&self) -> EditorMode {
        self.mode
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
            search: HierarchySearchState::default(),
            drag_candidate: None,
            dragging: None,
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

    fn resize_limits(&self) -> Option<PanelResizeLimits> {
        Some(PanelResizeLimits {
            min_width: 180.0,
            min_height: 150.0,
        })
    }

    fn has_active_escape_interaction(&self) -> bool {
        self.drag_candidate.is_some() || self.dragging.is_some()
    }

    fn cancel_escape_interaction(&mut self) {
        self.drag_candidate = None;
        self.dragging = None;
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        advance_entity_drag_candidate(
            &mut self.drag_candidate,
            &mut self.dragging,
            ctx.mouse_position().into(),
            ctx.is_mouse_button_down(MouseButton::Left),
        );
        let search_rect = hierarchy_search_rect(rect);
        let (query, _) = TextInput::new(SEARCH_WIDGET_ID, search_rect, self.search.query())
            .blocked(blocked)
            .show(ctx);
        self.search.set_query(&query, &mut self.expanded);

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
        let ecs = &mut game.ecs;
        let room_editor = &mut editor.room_editor;
        prune_dead_hierarchy_state(
            ecs,
            &mut self.expanded,
            &mut self.dragging,
            &mut self.drag_candidate,
        );

        let global_entities = {
            let store = ecs.get_store::<Global>();
            let all: Vec<Entity> = store.data.keys().copied().collect();
            get_root_entities(ecs, &all)
        };

        let active_room_layer = room_editor.active_layer_state.active_layer;
        let room_entities = if let Some(room_id) = cur_room_id {
            let entities = ecs.entities_in_room_layer(room_id, active_room_layer).clone();
            let entities: Vec<Entity> = entities
                .into_iter()
                .filter(|e| !ecs.has::<Singleton>(*e))
                .collect();
            get_root_entities(ecs, &entities)
        } else {
            Vec::new()
        };

        let visible_entities = self.search.is_active().then(|| {
            let mut visible = filter_hierarchy(&global_entities, self.search.query(), ecs);
            visible.extend(filter_hierarchy(&room_entities, self.search.query(), ecs));
            visible
        });
        let visible_global_roots = visible_roots(&global_entities, visible_entities.as_ref());
        let visible_room_roots = visible_roots(&room_entities, visible_entities.as_ref());
        let show_global_section = visible_entities.is_none() || !visible_global_roots.is_empty();
        let show_room_section = visible_entities.is_none() || !visible_room_roots.is_empty();
        let mut expanded_for_view = self.expanded.clone();
        if let Some(visible) = &visible_entities {
            expanded_for_view.extend(visible.iter().copied());
        }

        // Layout pass
        let mut layout_y = 0.0;
        if visible_entities.is_none() {
            layout_y += ADD_BUTTON_HEIGHT + ROW_SPACING;
        }
        if show_global_section {
            layout_y += HEADER_HEIGHT;
            for entity in &visible_global_roots {
                layout_entity_tree(
                    *entity,
                    &mut layout_y,
                    &expanded_for_view,
                    visible_entities.as_ref(),
                    ecs,
                );
            }
        }
        if show_global_section && show_room_section {
            layout_y += 10.0;
        }
        if show_room_section {
            layout_y += HEADER_HEIGHT;
            if visible_entities.is_none() {
                if let Some(room_id) = cur_room_id {
                    if ecs.get_player_proxy(room_id).is_none() {
                        layout_y += ADD_BUTTON_HEIGHT + ROW_SPACING;
                    }
                }
            }
            for entity in &visible_room_roots {
                layout_entity_tree(
                    *entity,
                    &mut layout_y,
                    &expanded_for_view,
                    visible_entities.as_ref(),
                    ecs,
                );
            }
        }
        if visible_entities.as_ref().is_some_and(HashSet::is_empty) {
            layout_y += ROW_HEIGHT;
        }
        layout_y += BOTTOM_PADDING;

        let content_height = layout_y;
        let scroll_rect = hierarchy_scroll_rect(rect);
        let area = ScrollableArea::new(scroll_rect, content_height)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);

        // Draw pass
        let mut y = scroll_rect.y + self.scroll_state.scroll_y;
        let mut room_host = RoomHierarchyHost {
            room_editor,
            mode: EditorMode::Game(GameEditorSubmode::Worlds),
        };

        let content_rect = area.content_rect();
        let button_width = (content_rect.w - HIERARCHY_HORIZONTAL_PADDING * 2.0).max(0.0);
        if visible_entities.is_none() {
            if area.is_fully_visible(y, ADD_BUTTON_HEIGHT) {
                let clicked = Button::new(
                    Rect::new(
                        rect.x + HIERARCHY_HORIZONTAL_PADDING,
                        y,
                        button_width,
                        ADD_BUTTON_HEIGHT,
                    ),
                    "+ Global",
                )
                .suppressed(blocked)
                .show(ctx);
                if !blocked && clicked {
                    if let Some(room_id) = cur_room_id {
                        push_command(Box::new(CreateSceneEntityCmd::new_global_entity(room_id)));
                    }
                }
            }
            y += ADD_BUTTON_HEIGHT + ROW_SPACING;
        }

        if show_global_section {
            if area.is_fully_visible(y, HEADER_HEIGHT) {
                ctx.draw_text(
                    "Global",
                    rect.x + HIERARCHY_HORIZONTAL_PADDING,
                    y + 14.,
                    HEADER_FONT_SIZE,
                    with_theme(|t| t.text_muted),
                );
            }
            y += HEADER_HEIGHT;

            let mut global_draw = SceneHierarchyFrame {
                ctx,
                panel_rect: rect,
                area: &area,
                blocked,
                expanded: &mut expanded_for_view,
                visible_entities: visible_entities.as_ref(),
                dragging: &mut self.dragging,
                drag_candidate: &mut self.drag_candidate,
            };
            for entity in visible_global_roots.iter().copied() {
                draw_scene_entity_tree(entity, 0, &mut y, &mut global_draw, &mut room_host, ecs);
            }
        }

        if show_global_section && show_room_section {
            y += 10.0;
        }
        if show_room_section {
            if area.is_fully_visible(y, HEADER_HEIGHT) {
                ctx.draw_text(
                    "Room",
                    rect.x + HIERARCHY_HORIZONTAL_PADDING,
                    y + 14.,
                    HEADER_FONT_SIZE,
                    with_theme(|t| t.text_muted),
                );
            }
            y += HEADER_HEIGHT;

            if visible_entities.is_none() {
                if let Some(room_id) = cur_room_id {
                    let has_spawn = ecs.get_player_proxy(room_id).is_some();
                    if !has_spawn {
                        let spawn_pos = room_pos.unwrap_or_default();
                        if area.is_fully_visible(y, ADD_BUTTON_HEIGHT) {
                            let clicked = Button::new(
                                Rect::new(
                                    rect.x + HIERARCHY_HORIZONTAL_PADDING,
                                    y,
                                    button_width,
                                    ADD_BUTTON_HEIGHT,
                                ),
                                "+ Player Proxy",
                            )
                            .suppressed(blocked)
                            .show(ctx);
                            if !blocked && clicked {
                                push_command(Box::new(CreateSceneEntityCmd::new_player_proxy(
                                    room_id,
                                    active_room_layer,
                                    spawn_pos,
                                )));
                            }
                        }
                        y += ADD_BUTTON_HEIGHT + ROW_SPACING;
                    }
                }
            }

            let room_mode = cur_room_id
                .map(EditorMode::Room)
                .unwrap_or(EditorMode::Game(GameEditorSubmode::Worlds));
            room_host.mode = room_mode;
            let mut room_draw = SceneHierarchyFrame {
                ctx,
                panel_rect: rect,
                area: &area,
                blocked,
                expanded: &mut expanded_for_view,
                visible_entities: visible_entities.as_ref(),
                dragging: &mut self.dragging,
                drag_candidate: &mut self.drag_candidate,
            };
            for entity in visible_room_roots.iter().copied() {
                draw_scene_entity_tree(entity, 0, &mut y, &mut room_draw, &mut room_host, ecs);
            }
        }

        if visible_entities.as_ref().is_some_and(HashSet::is_empty)
            && area.is_fully_visible(y, ROW_HEIGHT)
        {
            ctx.draw_text(
                "No matching entities",
                rect.x + HIERARCHY_HORIZONTAL_PADDING,
                y + 16.0,
                HEADER_FONT_SIZE,
                with_theme(|t| t.text_muted),
            );
        }
        if visible_entities.is_none() {
            self.expanded = expanded_for_view;
        }

        area.draw_scrollbar(ctx, &self.scroll_state);
        draw_drag_ghost(ctx, ecs, &mut self.dragging);
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
        prune_dead_hierarchy_state(
            ecs,
            &mut self.expanded,
            &mut self.dragging,
            &mut self.drag_candidate,
        );

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

        let visible_entities = self
            .search
            .is_active()
            .then(|| filter_hierarchy(&prefab_entities, self.search.query(), ecs));
        let visible_prefab_roots = visible_roots(&prefab_entities, visible_entities.as_ref());
        let show_prefab_section = visible_entities.is_none() || !visible_prefab_roots.is_empty();
        let mut expanded_for_view = self.expanded.clone();
        if let Some(visible) = &visible_entities {
            expanded_for_view.extend(visible.iter().copied());
        }

        let mut layout_y = 0.0;
        if show_prefab_section {
            layout_y += HEADER_HEIGHT;
            for entity in &visible_prefab_roots {
                layout_entity_tree(
                    *entity,
                    &mut layout_y,
                    &expanded_for_view,
                    visible_entities.as_ref(),
                    ecs,
                );
            }
        }
        if visible_entities.as_ref().is_some_and(HashSet::is_empty) {
            layout_y += ROW_HEIGHT;
        }
        layout_y += BOTTOM_PADDING;

        let scroll_rect = hierarchy_scroll_rect(rect);
        let area = ScrollableArea::new(scroll_rect, layout_y)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);
        let mut y = scroll_rect.y + self.scroll_state.scroll_y;
        let mut prefab_host = PrefabHierarchyHost {
            prefab_editor,
            mode: editor.mode,
        };

        if show_prefab_section {
            if area.is_fully_visible(y, HEADER_HEIGHT) {
                ctx.draw_text(
                    "Prefab",
                    rect.x + HIERARCHY_HORIZONTAL_PADDING,
                    y + 14.,
                    HEADER_FONT_SIZE,
                    with_theme(|t| t.text_muted),
                );
            }
            y += HEADER_HEIGHT;

            let mut prefab_draw = SceneHierarchyFrame {
                ctx,
                panel_rect: rect,
                area: &area,
                blocked,
                expanded: &mut expanded_for_view,
                visible_entities: visible_entities.as_ref(),
                dragging: &mut self.dragging,
                drag_candidate: &mut self.drag_candidate,
            };
            for entity in visible_prefab_roots.iter().copied() {
                draw_scene_entity_tree(entity, 0, &mut y, &mut prefab_draw, &mut prefab_host, ecs);
            }
        }

        if visible_entities.as_ref().is_some_and(HashSet::is_empty)
            && area.is_fully_visible(y, ROW_HEIGHT)
        {
            ctx.draw_text(
                "No matching entities",
                rect.x + HIERARCHY_HORIZONTAL_PADDING,
                y + 16.0,
                HEADER_FONT_SIZE,
                with_theme(|t| t.text_muted),
            );
        }
        if visible_entities.is_none() {
            self.expanded = expanded_for_view;
        }

        area.draw_scrollbar(ctx, &self.scroll_state);
        draw_drag_ghost(ctx, ecs, &mut self.dragging);
    }
}

fn hierarchy_search_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x + HIERARCHY_HORIZONTAL_PADDING,
        rect.y + TOP_PADDING,
        (rect.w - HIERARCHY_HORIZONTAL_PADDING * 2.0).max(0.0),
        SEARCH_FIELD_HEIGHT,
    )
}

fn hierarchy_scroll_rect(rect: Rect) -> Rect {
    let y = hierarchy_search_rect(rect).bottom() + ROW_SPACING;
    Rect::new(rect.x, y, rect.w, (rect.bottom() - y).max(0.0))
}

pub(crate) fn layout_entity_tree(
    entity: Entity,
    y: &mut f32,
    expanded: &HashSet<Entity>,
    visible_entities: Option<&HashSet<Entity>>,
    ecs: &Ecs,
) {
    if visible_entities.is_some_and(|visible| !visible.contains(&entity)) {
        return;
    }

    *y += ROW_HEIGHT;
    if expanded.contains(&entity) && has_children(ecs, entity) {
        for child in get_children(ecs, entity) {
            layout_entity_tree(child, y, expanded, visible_entities, ecs);
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
    drag_candidate: &mut Option<(Entity, Vec2)>,
) {
    expanded.retain(|entity| entity_exists_in_hierarchy(ecs, *entity));

    if dragging.is_some_and(|entity| !entity_exists_in_hierarchy(ecs, entity)) {
        *dragging = None;
    }

    if drag_candidate.is_some_and(|(entity, _)| !entity_exists_in_hierarchy(ecs, entity)) {
        *drag_candidate = None;
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

const ENTITY_DRAG_START_THRESHOLD: f32 = 4.0;

pub(crate) fn advance_entity_drag_candidate(
    candidate: &mut Option<(Entity, Vec2)>,
    dragging: &mut Option<Entity>,
    mouse: Vec2,
    mouse_down: bool,
) {
    if dragging.is_some() {
        *candidate = None;
        return;
    }

    let Some((entity, start)) = *candidate else {
        return;
    };
    let delta = mouse - start;
    if delta.length_squared() >= ENTITY_DRAG_START_THRESHOLD * ENTITY_DRAG_START_THRESHOLD {
        *dragging = Some(entity);
        *candidate = None;
    } else if !mouse_down {
        *candidate = None;
    }
}

pub(crate) fn clear_drag_on_mouse_release(dragging: &mut Option<Entity>, mouse_released: bool) {
    if mouse_released {
        *dragging = None;
    }
}

fn draw_drag_ghost(ctx: &mut WgpuContext, ecs: &Ecs, dragging: &mut Option<Entity>) {
    if let Some(dragged) = *dragging {
        let (mx, my) = ctx.mouse_position();
        let name = get_entity_name(ecs, dragged);
        const DRAG_GHOST_HORIZONTAL_PADDING: f32 = 4.0;
        let name_width = measure_text(ctx, &name, ENTITY_NAME_FONT_SIZE).width;
        let ghost_width = name_width + DRAG_GHOST_HORIZONTAL_PADDING * 2.0;
        let ghost_x = mx - ghost_width * 0.5;
        let ghost_y = my - ROW_HEIGHT * 0.5;
        ctx.draw_rectangle(
            ghost_x,
            ghost_y,
            ghost_width,
            ROW_HEIGHT,
            Color::new(0.3, 0.5, 0.7, 1.0).with_alpha(0.5),
        );
        ctx.draw_text(
            &name,
            ghost_x + DRAG_GHOST_HORIZONTAL_PADDING,
            ghost_y + 16.0,
            ENTITY_NAME_FONT_SIZE,
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
