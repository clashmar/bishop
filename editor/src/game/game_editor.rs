// editor/src/game/game_editor.rs
use crate::app::EditorCameraController;
use crate::app::SubEditor;
use crate::commands::game::*;
use crate::editor_assets::assets::*;
use crate::gui::gui_constants::*;
use crate::gui::menu_bar::*;
use crate::gui::modals::edit_world::EditWorldData;
use crate::gui::mode_selector::ModeInfo;
use crate::gui::mode_selector::ModeSelector;
use crate::push_command;
use crate::world::coord;
use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::theme::with_theme;
use once_cell::sync::Lazy;
use widgets::constants::layout;

use std::collections::HashMap;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Copy, Clone, PartialEq, EnumIter)]
pub enum GameEditorMode {
    Select,
    Edit,
    Move,
    Delete,
}

impl ModeInfo for GameEditorMode {
    fn label(&self) -> &'static str {
        match self {
            GameEditorMode::Select => "Select: S",
            GameEditorMode::Edit => "Edit: E",
            GameEditorMode::Move => "Move: M",
            GameEditorMode::Delete => "Delete: D",
        }
    }
    fn icon(&self) -> &'static Texture2D {
        match self {
            GameEditorMode::Select => select_icon(),
            GameEditorMode::Edit => edit_icon(),
            GameEditorMode::Move => move_icon(),
            GameEditorMode::Delete => delete_icon(),
        }
    }
    fn shortcut(self) -> Option<fn(&WgpuContext) -> bool> {
        match self {
            GameEditorMode::Select => Some(Controls::s),
            GameEditorMode::Edit => Some(Controls::e),
            GameEditorMode::Move => Some(Controls::m),
            GameEditorMode::Delete => Some(Controls::d),
        }
    }
}

pub struct GameEditor {
    mode: GameEditorMode,
    mode_selector: ModeSelector<GameEditorMode>,
    active_rects: Vec<Rect>,
    dragged_world: Option<WorldId>,
    dragging: bool,
    drag_offset: Vec2,
    drag_start_position: Option<Vec2>,
    world_widget_ids: HashMap<WorldId, WidgetId>,
    pub(crate) pending_edit_world: Option<EditWorldData>,
    pub(crate) pending_delete_world: Option<WorldId>,
    pending_camera_init: bool,
}

impl GameEditor {
    pub fn new() -> Self {
        let mode = GameEditorMode::Select;
        Self {
            mode,
            mode_selector: ModeSelector {
                current: mode,
                options: *ALL_MODES,
            },
            active_rects: Vec::new(),
            dragged_world: None,
            dragging: false,
            drag_offset: Vec2::ZERO,
            drag_start_position: None,
            world_widget_ids: HashMap::new(),
            pending_edit_world: None,
            pending_delete_world: None,
            pending_camera_init: false,
        }
    }

    pub fn update(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        game: &mut Game,
    ) -> Option<WorldId> {
        self.handle_mouse_cursor(ctx);

        match self.mode {
            GameEditorMode::Select => {
                // Select world
                if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx)
                {
                    let world_data: Vec<(WorldId, Vec2, Option<SpriteId>)> = game.worlds().iter()
                        .map(|w| (w.id, w.meta.position, w.meta.sprite_id))
                        .collect();
                    for (world_id, position, sprite_id) in world_data {
                        let texture = match sprite_id {
                            Some(id) => game.sprite_manager.get_texture_from_id(ctx, id),
                            None => circle_120px(),
                        };
                        let bounds = Rect::new(
                            position.x,
                            position.y,
                            texture.width(),
                            texture.height(),
                        );
                        let world_mouse = coord::mouse_world_pos(ctx, camera);
                        if bounds.contains(world_mouse) {
                            return Some(world_id);
                        }
                    }
                }
            }
            GameEditorMode::Edit => {
                if ctx.is_mouse_button_pressed(MouseButton::Left)
                    && !self.should_block_canvas(ctx)
                    && !is_modal_open()
                {
                    let world_data: Vec<(WorldId, Vec2, String, Option<SpriteId>)> = game.worlds().iter()
                        .map(|w| (w.id, w.meta.position, w.name.clone(), w.meta.sprite_id))
                        .collect();
                    for (world_id, position, name, sprite_id) in world_data {
                        let texture = match sprite_id {
                            Some(id) => game.sprite_manager.get_texture_from_id(ctx, id),
                            None => circle_120px(),
                        };
                        let bounds = Rect::new(
                            position.x,
                            position.y,
                            texture.width(),
                            texture.height(),
                        );
                        let world_mouse = coord::mouse_world_pos(ctx, camera);
                        if bounds.contains(world_mouse) {
                            let current_sprite = sprite_id.unwrap_or(SpriteId(0));
                            let widget_id = self.widget_id_for_world(world_id);

                            self.pending_edit_world = Some(EditWorldData {
                                world_id,
                                current_name: name,
                                current_sprite,
                                widget_id,
                            });
                            break;
                        }
                    }
                }
            }
            GameEditorMode::Move => {
                if !self.should_block_canvas(ctx) {
                    // Drag world
                    self.handle_drag(ctx, camera, game);
                }
            }
            GameEditorMode::Delete => {
                if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx)
                {
                    let world_data: Vec<(WorldId, Vec2, Option<SpriteId>)> = game.worlds().iter()
                        .map(|w| (w.id, w.meta.position, w.meta.sprite_id))
                        .collect();
                    for (world_id, position, sprite_id) in world_data {
                        let texture = match sprite_id {
                            Some(id) => game.sprite_manager.get_texture_from_id(ctx, id),
                            None => circle_120px(),
                        };
                        let bounds = Rect::new(
                            position.x,
                            position.y,
                            texture.width(),
                            texture.height(),
                        );
                        let world_mouse = coord::mouse_world_pos(ctx, camera);
                        if bounds.contains(world_mouse) {
                            self.pending_delete_world = Some(world_id);
                            break;
                        }
                    }
                }
            }
        }

        self.handle_shortcuts(ctx);

        None
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext, camera: &mut Camera2D, game: &mut Game) {
        if self.pending_camera_init && !game.worlds().is_empty() {
            GameEditor::init_camera(self, ctx, camera, game);
            self.pending_camera_init = false;
        }

        ctx.set_camera(camera);
        ctx.clear_background(Color::BLACK);

        self.draw_worlds(ctx, camera, game);
        self.draw_ui(ctx);
    }

    fn draw_worlds(&mut self, ctx: &mut WgpuContext, camera: &Camera2D, game: &mut Game) {
        // Collect world data first (immutable borrow of worlds only)
        let world_data: Vec<(Vec2, Option<SpriteId>, String)> = game.worlds().iter()
            .map(|w| (w.meta.position, w.meta.sprite_id, w.name.clone()))
            .collect();

        // Draw worlds (mutable borrow of sprite_manager in the loop)
        for (position, sprite_id, name) in world_data {
            let texture = match sprite_id {
                Some(id) => game.sprite_manager.get_texture_from_id(ctx, id),
                None => circle_120px(),
            };

            // Hover tint — inline the bounds check
            let world_mouse = coord::mouse_world_pos(ctx, camera);
            let bounds = Rect::new(
                position.x,
                position.y,
                texture.width(),
                texture.height(),
            );
            let is_hovered = bounds.contains(world_mouse)
                && !self.should_block_canvas(ctx)
                && self.dragged_world.is_none();

            let tint = if is_hovered {
                match self.mode {
                    GameEditorMode::Delete => with_theme(|t| t.danger),
                    _ => with_theme(|t| t.primary),
                }
            } else {
                Color::WHITE
            };

            // Default is a circle
            ctx.draw_texture(texture, position.x, position.y, tint);

            // Display name
            const NAME_HEIGHT: f32 = 24.0;
            let center = position.x + (texture.width() / 2.);
            let (x, width) = center_text_field(ctx, center, &name);

            let name_rect = Rect::new(
                x,
                position.y - SPACING - NAME_HEIGHT,
                width,
                NAME_HEIGHT,
            );

            draw_input_field_text(ctx, &name, name_rect);
        }
    }

    fn handle_drag(&mut self, ctx: &WgpuContext, camera: &Camera2D, game: &mut Game) {
        // Start dragging
        if !self.dragging && ctx.is_mouse_button_pressed(MouseButton::Left) {
            let world_data: Vec<(WorldId, Vec2, Option<SpriteId>)> = game.worlds().iter()
                .map(|w| (w.id, w.meta.position, w.meta.sprite_id))
                .collect();
            for (world_id, position, sprite_id) in world_data {
                let texture = match sprite_id {
                    Some(id) => game.sprite_manager.get_texture_from_id(ctx, id),
                    None => circle_120px(),
                };
                let bounds = Rect::new(
                    position.x,
                    position.y,
                    texture.width(),
                    texture.height(),
                );
                let world_mouse = coord::mouse_world_pos(ctx, camera);
                if bounds.contains(world_mouse) {
                    self.dragging = true;
                    self.dragged_world = Some(world_id);

                    let mouse_world = coord::mouse_world_pos(ctx, camera);
                    let world_pos = position;
                    self.drag_offset = world_pos - mouse_world;
                    self.drag_start_position = Some(world_pos);
                    break;
                }
            }
        }

        // While dragging
        if self.dragging {
            if let Some(id) = self.dragged_world {
                if ctx.is_mouse_button_down(MouseButton::Left) {
                    let mouse_world = coord::mouse_world_pos(ctx, camera);

                    if let Some(world) = game.get_world_mut(id) {
                        world.meta.position = mouse_world + self.drag_offset;
                    }
                }

                // Finish on release
                if ctx.is_mouse_button_released(MouseButton::Left) {
                    if let (Some(start_pos), Some(id)) =
                        (self.drag_start_position.take(), self.dragged_world.take())
                    {
                        if let Some(world) = game.get_world(id) {
                            let final_pos = world.meta.position;

                            // Only push command if world actually moved
                            if (final_pos - start_pos).length_squared() > 0.0 {
                                push_command(Box::new(MoveWorldCmd::new(id, start_pos, final_pos)));
                            }
                        }
                    }

                    self.dragging = false;
                }
            }
        }
    }

    fn draw_ui(&mut self, ctx: &mut WgpuContext) {
        ctx.set_default_camera();

        self.active_rects.clear();
        self.register_rect(draw_top_panel_full(ctx));

        if self.mode_selector.draw(ctx).1 {
            self.mode = self.mode_selector.current;
        }
        self.mode_selector.draw_tooltips(ctx);

        self.draw_menu_buttons(ctx);
    }

    fn draw_menu_buttons(&mut self, ctx: &mut WgpuContext) {
        const BTN_MARGIN: f32 = 10.0;

        let create_label = "New World";
        let txt_create = measure_text(ctx, create_label, layout::HEADER_FONT_SIZE_20);
        let create_btn = Rect::new(
            ctx.screen_width() - txt_create.width - BTN_MARGIN - PADDING,
            BTN_MARGIN,
            txt_create.width + PADDING,
            BTN_HEIGHT,
        );

        if menu_button(ctx, create_btn, create_label, false) {
            push_command(Box::new(CreateWorldCmd::new()));
            self.pending_camera_init = true;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &WgpuContext) {
        for mode in GameEditorMode::iter() {
            if let Some(shortcut) = mode.shortcut() {
                if shortcut(ctx)
                    && !input_is_focused()
                    && !is_modal_open()
                    && !is_context_menu_open()
                {
                    self.mode = mode;
                    self.mode_selector.current = mode;
                    break;
                }
            }
        }
    }

    fn widget_id_for_world(&mut self, world_id: WorldId) -> WidgetId {
        *self.world_widget_ids.entry(world_id).or_default()
    }

    #[inline]
    fn register_rect(&mut self, rect: Rect) -> Rect {
        self.active_rects.push(rect);
        rect
    }

    fn handle_mouse_cursor(&self, ctx: &mut WgpuContext) {
        if self.should_block_canvas(ctx) {
            ctx.set_cursor_icon(CursorIcon::Default);
        } else {
            match self.mode {
                GameEditorMode::Select => {
                    ctx.set_cursor_icon(CursorIcon::Pointer);
                }
                GameEditorMode::Edit => {
                    ctx.set_cursor_icon(CursorIcon::Crosshair);
                }
                GameEditorMode::Move => {
                    ctx.set_cursor_icon(CursorIcon::Move);
                }
                GameEditorMode::Delete => {
                    ctx.set_cursor_icon(CursorIcon::Crosshair);
                }
            }
        }
    }


    /// Sets the default camera for the game editor.
    pub fn init_camera(&self, ctx: &WgpuContext, camera: &mut Camera2D, game: &mut Game) {
        let (min, max) = self.world_bounds(ctx, game);
        let center = (min + max) * 0.5;
        let size = max - min;

        // Get the zoom for the whole area
        let zoom =
            EditorCameraController::zoom_for_size(ctx, size, 2.0, game.current_world().grid_size);

        // Apply the results
        camera.target = center;
        camera.zoom = zoom;
    }

    /// Returns the (min, max) world‑space corners that contain all worlds.
    fn world_bounds(&self, loader: &impl TextureLoader, game: &mut Game) -> (Vec2, Vec2) {
        if game.worlds().is_empty() {
            return (vec2(0.0, 0.0), vec2(1.0, 1.0));
        }

        let mut min = vec2(f32::INFINITY, f32::INFINITY);
        let mut max = vec2(f32::NEG_INFINITY, f32::NEG_INFINITY);

        // Collect world data first (immutable borrow of worlds only)
        let world_data: Vec<(Vec2, Option<SpriteId>)> = game.worlds().iter()
            .map(|w| (w.meta.position, w.meta.sprite_id))
            .collect();

        // Compute bounds (mutable borrow of sprite_manager in the loop)
        for (position, sprite_id) in world_data {
            let tex = match sprite_id {
                Some(id) => game.sprite_manager.get_texture_from_id(loader, id),
                None => circle_120px(),
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
}

/// A slice of all the modes.
static ALL_MODES: Lazy<&'static [GameEditorMode]> =
    Lazy::new(|| Box::leak(Box::new(GameEditorMode::iter().collect::<Vec<_>>())));
