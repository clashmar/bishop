use crate::app::EditorCameraController;
use crate::app::EditorMode;
use crate::app::SubEditor;
use crate::commands::game::*;
use crate::editor_assets::assets::*;
use crate::gui::gui_constants::{self, *};
use crate::gui::inspector::shell::Inspector;
use crate::gui::menu_bar::*;
use crate::game::topology_submode::TopologySubmode;
use crate::game::world_selection::WorldSelection;
use crate::game::worlds_submode::{
    center_camera_on_world_bounds,
    handle_pointer_down,
    world_bounds,
    world_entry_request,
    PointerDownAction,
    WorldsSubmode,
};
use crate::push_command;
use crate::shared::input::canvas_blocked_by_global_ui;
use crate::shared::input::shortcuts_blocked;
use crate::shared::scene_ui::inspector::{InspectorHostAction, InspectorContext};
use crate::world::coord;
use crate::world::resolve_world_name;
use bishop::prelude::*;
use engine_core::controls::Controls;
use engine_core::game::Game;
use engine_core::ui::measure_text;
use engine_core::worlds::*;
use std::collections::HashSet;
use ::widgets::constants::layout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameEditorSubmode {
    Worlds,
    Topology(WorldId),
}

pub struct GameEditor {
    requested_submode: Option<GameEditorSubmode>,
    topology_submode: TopologySubmode,
    pub(super) world_selection: WorldSelection,
    pub(super) worlds_submode: WorldsSubmode,
    pub(crate) show_world_arrows: bool,
    active_rects: Vec<Rect>,
    inspector: Inspector,
    pending_camera_init: bool,
    requested_world_entry: Option<WorldId>,
}

impl GameEditor {
    pub fn new() -> Self {
        Self {
            requested_submode: None,
            topology_submode: TopologySubmode::default(),
            world_selection: WorldSelection::default(),
            worlds_submode: WorldsSubmode::default(),
            show_world_arrows: false,
            active_rects: Vec::new(),
            inspector: Inspector::new(),
            pending_camera_init: false,
            requested_world_entry: None,
        }
    }

    pub fn update_worlds(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        game: &mut Game,
    ) -> Option<WorldId> {
        self.handle_mouse_cursor(ctx, false);

        if self.worlds_submode.is_marquee_active() {
            let world_mouse = coord::mouse_world_pos(ctx, camera);
            let mouse_screen = ctx.mouse_position().into();
            self.worlds_submode.update_marquee(world_mouse);
            if ctx.is_mouse_button_released(MouseButton::Left) {
                let bounds = world_bounds(ctx, game);
                if self.world_selection.primary().is_some() {
                    let existing_world_ids: HashSet<WorldId> =
                        bounds.iter().map(|(world_id, _)| *world_id).collect();
                    self.world_selection.retain_existing_ids(&existing_world_ids);
                }
                self.worlds_submode.finish_marquee(
                    &mut self.world_selection,
                    world_mouse,
                    mouse_screen,
                    &bounds,
                );
            }
        }

        if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx) {
            let world_mouse = coord::mouse_world_pos(ctx, camera);
            let mouse_screen = ctx.mouse_position().into();
            let bounds = world_bounds(ctx, game);
            if self.world_selection.primary().is_some() {
                let existing_world_ids: HashSet<WorldId> =
                    bounds.iter().map(|(world_id, _)| *world_id).collect();
                self.world_selection.retain_existing_ids(&existing_world_ids);
            }
            let hit_world = bounds
                .iter()
                .find(|(_, rect)| rect.contains(world_mouse))
                .map(|(world_id, _)| *world_id);
            let shift_held = ctx.is_key_down(KeyCode::LeftShift)
                || ctx.is_key_down(KeyCode::RightShift);
            let action = handle_pointer_down(&mut self.world_selection, hit_world, shift_held);

            if let Some(world_id) = world_entry_request(
                action,
                ctx.is_mouse_button_double_clicked(MouseButton::Left),
            ) {
                return Some(world_id);
            }

            match action {
                PointerDownAction::StartMarquee => {
                    self.worlds_submode.begin_marquee(world_mouse, mouse_screen);
                }
                PointerDownAction::StartDrag(_) => {
                    let selected_positions = game
                        .worlds()
                        .iter()
                        .filter(|world| self.world_selection.contains(world.id))
                        .map(|world| (world.id, world.meta.position))
                        .collect();
                    self.worlds_submode
                        .begin_world_drag(world_mouse, selected_positions);
                }
                PointerDownAction::None => {}
            }
        }

        self.update_world_drag(ctx, camera, game);
        self.handle_shortcuts(ctx, game);

        None
    }

    pub fn update_topology(&mut self, ctx: &mut WgpuContext) {
        self.handle_mouse_cursor(ctx, true);
        self.topology_submode.update(ctx);
    }

    pub fn draw_worlds_view(&mut self, ctx: &mut WgpuContext, camera: &mut Camera2D, game: &mut Game) {
        if self.pending_camera_init && !game.worlds().is_empty() {
            GameEditor::init_camera(self, ctx, camera, game);
            self.pending_camera_init = false;
        }

        ctx.set_camera(camera);
        ctx.clear_background(Color::BLACK);
        self.draw_worlds(ctx, camera, game);
        self.worlds_submode.draw_marquee(ctx, game.current_world().grid_size);
        self.draw_ui(ctx, game, GameEditorSubmode::Worlds);
    }

    pub fn draw_topology_view(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &mut Camera2D,
        game: &mut Game,
        world_id: WorldId,
    ) {
        if self.pending_camera_init && !game.worlds().is_empty() {
            GameEditor::init_camera(self, ctx, camera, game);
            self.pending_camera_init = false;
        }

        ctx.set_camera(camera);
        ctx.clear_background(Color::BLACK);
        self.topology_submode.draw(ctx, game, world_id);
        self.draw_ui(ctx, game, GameEditorSubmode::Topology(world_id));
    }

    pub(crate) fn take_requested_submode(&mut self) -> Option<GameEditorSubmode> {
        self.requested_submode.take()
    }

    pub(crate) fn take_requested_world_entry(&mut self) -> Option<WorldId> {
        self.requested_world_entry.take()
    }

    pub(crate) fn request_world_entry_from_browser(&mut self, world_id: WorldId) {
        self.requested_world_entry = Some(world_id);
    }

    pub(crate) fn is_world_selected(&self, world_id: WorldId) -> bool {
        self.world_selection.contains(world_id)
    }

    pub(crate) fn select_world_from_browser(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &mut Camera2D,
        game: &mut Game,
        world_id: WorldId,
    ) {
        let Some((_, bounds)) = world_bounds(ctx, game)
            .into_iter()
            .find(|(candidate_id, _)| *candidate_id == world_id)
        else {
            return;
        };

        self.world_selection.select_only(world_id);
        center_camera_on_world_bounds(camera, bounds);
    }

    pub(crate) fn toggle_world_arrows(&mut self) {
        self.show_world_arrows = !self.show_world_arrows;
    }

    /// Sets the default camera for the game editor.
    pub fn init_camera(&self, ctx: &WgpuContext, camera: &mut Camera2D, game: &mut Game) {
        let (min, max) = self.world_bounds(ctx, game);
        let center = (min + max) * 0.5;
        let size = max - min;

        let zoom =
            EditorCameraController::zoom_for_size(ctx, size, 2.0, game.current_world().grid_size);

        camera.target = center;
        camera.zoom = zoom;
    }

    pub(crate) fn center_topology_on_world(&mut self, ctx: &WgpuContext, world: &World) {
        self.topology_submode.center_on_world(ctx, world);
    }

    fn update_world_drag(&mut self, ctx: &WgpuContext, camera: &Camera2D, game: &mut Game) {
        if self.worlds_submode.world_drag_start_positions().is_empty() {
            return;
        }

        if ctx.is_mouse_button_down(MouseButton::Left)
            || ctx.is_mouse_button_released(MouseButton::Left)
        {
            let mouse_world = coord::mouse_world_pos(ctx, camera);
            if let Some(delta) = self.worlds_submode.world_drag_delta(mouse_world) {
                for &(world_id, start_position) in
                    self.worlds_submode.world_drag_start_positions()
                {
                    if let Some(world) = game.get_world_mut(world_id) {
                        world.meta.position = start_position + delta;
                    }
                }
            }
        }

        if ctx.is_mouse_button_released(MouseButton::Left) {
            let start_positions = self.worlds_submode.finish_world_drag();
            let moves = start_positions
                .into_iter()
                .filter_map(|(world_id, start_position)| {
                    let final_position = game.get_world(world_id)?.meta.position;
                    ((final_position - start_position).length_squared() > 0.0)
                        .then_some((world_id, start_position, final_position))
                })
                .collect::<Vec<_>>();
            if !moves.is_empty() {
                push_command(Box::new(MoveWorldsCmd::new(moves)));
            }
        }
    }

    fn draw_ui(&mut self, ctx: &mut WgpuContext, game: &mut Game, submode: GameEditorSubmode) {
        ctx.set_default_camera();

        self.active_rects.clear();
        self.register_rect(draw_top_panel_full(ctx));

        self.draw_menu_buttons(ctx, game, submode);

        let inspector_rect = Rect::new(
            ctx.screen_width() - gui_constants::inspector::WIDTH,
            0.0,
            gui_constants::inspector::WIDTH,
            ctx.screen_height(),
        );
        self.inspector.set_rect(inspector_rect);

        let game_name = game.name.clone();
        let start_world_id = game.start_world_id;
        let game_default_font = game.text_settings.default_font;
        let inspector_context = InspectorContext {
            command_mode: EditorMode::Game(submode),
            game_name: Some(game_name),
            start_world_id,
            game_default_font,
            show_linked_prefab_metadata: false,
            hide_room_only_components: true,
            selected_create_parent: None,
            event_tags: Vec::new(),
            room_zone_tool_active: false,
            room_zones_visible: false,
        };
        let selected_world_id = if matches!(submode, GameEditorSubmode::Worlds) {
            self.world_selection.primary()
        } else {
            None
        }
        .filter(|world_id| game.get_world(*world_id).is_some());
        let inspector_output = if let Some(world_id) = selected_world_id {
            let mut game_ctx = game.ctx_mut_for_world(world_id);
            self.inspector
                .draw_world_overview_pane(ctx, &mut game_ctx, &inspector_context)
        } else {
            let mut game_ctx = game.ctx_mut();
            self.inspector.draw_game_pane(ctx, &mut game_ctx, &inspector_context)
        };

        match inspector_output.host_action {
            Some(InspectorHostAction::RenameWorld(name)) => {
                if let Some(world_id) = selected_world_id {
                    let unique = resolve_world_name(game, world_id, &name);
                    push_command(Box::new(EditWorldCmd::new(world_id, Some(unique), None)));
                }
            }
            Some(InspectorHostAction::RenameGame(name)) => {
                push_command(Box::new(RenameGameCmd::new(name, game.name.clone())));
            }
            Some(InspectorHostAction::SetStartWorld(id)) => {
                push_command(Box::new(SetStartWorldCmd::new(id, game.start_world_id)));
            }
            Some(InspectorHostAction::SetGameDefaultFont(font)) => {
                push_command(Box::new(SetGameDefaultFontCmd::new(
                    font,
                    game.text_settings.default_font,
                )));
            }
            _ => {}
        }
    }

    fn draw_menu_buttons(&mut self, ctx: &mut WgpuContext, game: &mut Game, submode: GameEditorSubmode) {
        const BTN_MARGIN: f32 = 10.0;

        let mut next_right = ctx.screen_width() - BTN_MARGIN;

        if matches!(submode, GameEditorSubmode::Topology(_)) {
            return;
        }

        let create_label = "New World";
        let create_size = measure_text(ctx, create_label, layout::HEADER_FONT_SIZE_20);
        let create_btn = Rect::new(
            next_right - create_size.width - PADDING,
            BTN_MARGIN,
            create_size.width + PADDING,
            BTN_HEIGHT,
        );
        next_right = create_btn.x - BTN_MARGIN;

        let topology_label = "Topology";
        let topology_size = measure_text(ctx, topology_label, layout::HEADER_FONT_SIZE_20);
        let topology_btn = Rect::new(
            next_right - topology_size.width - PADDING,
            BTN_MARGIN,
            topology_size.width + PADDING,
            BTN_HEIGHT,
        );

        if menu_button(ctx, topology_btn, topology_label, false, false) {
            self.request_topology_submode(ctx, game);
        }
        if menu_button(ctx, create_btn, create_label, false, false) {
            push_command(Box::new(CreateWorldCmd::new()));
            self.pending_camera_init = true;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &WgpuContext, game: &mut Game) {
        if shortcuts_blocked() {
            return;
        }

        if Controls::delete(ctx) {
            let selected_world_ids = game
                .worlds()
                .iter()
                .filter(|world| self.world_selection.contains(world.id))
                .map(|world| world.id)
                .collect::<Vec<_>>();
            if !selected_world_ids.is_empty() {
                push_command(Box::new(DeleteWorldsCmd::new(game, selected_world_ids)));
                self.world_selection.clear();
            }
            return;
        }

        if Controls::t(ctx) {
            self.request_topology_submode(ctx, game);
        }
    }

    #[inline]
    fn register_rect(&mut self, rect: Rect) -> Rect {
        self.active_rects.push(rect);
        rect
    }

    fn handle_mouse_cursor(&self, ctx: &mut WgpuContext, topology_active: bool) {
        if topology_active {
            ctx.set_cursor_icon(CursorIcon::Default);
            return;
        }

        if self.should_block_canvas(ctx) {
            ctx.set_cursor_icon(CursorIcon::Default);
        } else {
            ctx.set_cursor_icon(CursorIcon::Pointer);
        }
    }


    fn request_topology_submode(&mut self, ctx: &WgpuContext, game: &mut Game) {
        let Some(world_id) = resolve_topology_world_id(
            self.world_selection.primary(),
            game.current_world_id,
            game.worlds(),
        ) else {
            return;
        };
        game.current_world_id = Some(world_id);
        if let Some(world) = game.get_world_mut(world_id) {
            self.center_topology_on_world(ctx, world);
            self.requested_submode = Some(GameEditorSubmode::Topology(world_id));
        }
    }

    /// Returns the (min, max) world‑space corners that contain all worlds.
    fn world_bounds(&self, loader: &impl TextureLoader, game: &mut Game) -> (Vec2, Vec2) {
        if game.worlds().is_empty() {
            return (vec2(0.0, 0.0), vec2(1.0, 1.0));
        }

        let mut min = vec2(f32::INFINITY, f32::INFINITY);
        let mut max = vec2(f32::NEG_INFINITY, f32::NEG_INFINITY);

        // Collect world data first (immutable borrow of worlds only)
        let world_data: Vec<(Vec2, WorldVisual)> = game.worlds().iter()
            .map(|w| (w.meta.position, w.meta.visual))
            .collect();

        // Compute bounds (mutable borrow of sprite_manager in the loop)
        for (position, visual) in world_data {
            let tex = match visual {
                WorldVisual::Image(id) => game.sprite_manager.get_texture_from_id(loader, id),
                WorldVisual::Color(_) => circle_120px(),
            };
            let w = tex.width();
            let h = tex.height();

            let right = position.x + w;
            let bottom = position.y + h;

            if position.x < min.x {
                min.x = position.x;
            }
            if position.y < min.y {
                min.y = position.y;
            }
            if right > max.x {
                max.x = right;
            }
            if bottom > max.y {
                max.y = bottom;
            }
        }

        (min, max)
    }
}

impl SubEditor for GameEditor {
    fn active_rects(&self) -> &[Rect] {
        &self.active_rects
    }

    fn should_block_canvas(&self, ctx: &WgpuContext) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects.iter().any(|r| r.contains(mouse_screen))
            || self.inspector.is_mouse_over(ctx)
            || canvas_blocked_by_global_ui(ctx)
    }
}

fn resolve_topology_world_id(
    selected_world_id: Option<WorldId>,
    current_world_id: Option<WorldId>,
    worlds: &[World],
) -> Option<WorldId> {
    selected_world_id
        .filter(|id| worlds.iter().any(|world| world.id == *id))
        .or_else(|| current_world_id.filter(|id| worlds.iter().any(|world| world.id == *id)))
        .or_else(|| worlds.first().map(|world| world.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topology_test_worlds() -> [World; 2] {
        [
            World::new(WorldId(1), "first".to_owned(), 16.0),
            World::new(WorldId(2), "second".to_owned(), 16.0),
        ]
    }

    #[test]
    fn topology_target_prefers_primary_selection_over_current_world() {
        let worlds = topology_test_worlds();

        assert_eq!(
            resolve_topology_world_id(Some(WorldId(2)), Some(WorldId(1)), &worlds),
            Some(WorldId(2))
        );
    }

    #[test]
    fn topology_target_uses_current_world_when_selection_is_missing_or_stale() {
        let worlds = topology_test_worlds();

        assert_eq!(
            resolve_topology_world_id(None, Some(WorldId(2)), &worlds),
            Some(WorldId(2))
        );
        assert_eq!(
            resolve_topology_world_id(Some(WorldId(3)), Some(WorldId(1)), &worlds),
            Some(WorldId(1))
        );
    }

    #[test]
    fn topology_target_falls_back_to_first_world_when_current_world_is_invalid() {
        let worlds = topology_test_worlds();

        assert_eq!(
            resolve_topology_world_id(None, Some(WorldId(3)), &worlds),
            Some(WorldId(1))
        );
        assert_eq!(resolve_topology_world_id(None, None, &[]), None);
    }

    #[test]
    fn world_browser_entry_request_is_consumed_once() {
        let mut editor = GameEditor::new();
        let world_id = WorldId(2);

        editor.request_world_entry_from_browser(world_id);

        assert_eq!(editor.take_requested_world_entry(), Some(world_id));
        assert_eq!(editor.take_requested_world_entry(), None);
    }

    #[test]
    fn toggle_world_arrows_flips_visibility() {
        let mut editor = GameEditor::new();

        assert!(!editor.show_world_arrows);
        editor.toggle_world_arrows();
        assert!(editor.show_world_arrows);
        editor.toggle_world_arrows();
        assert!(!editor.show_world_arrows);
    }

}
