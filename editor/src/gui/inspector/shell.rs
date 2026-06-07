use crate::gui::gui_constants::inspector;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::gui::properties::game::GameProperties;
use crate::gui::properties::room::RoomProperties;
use crate::gui::properties::world::WorldProperties;
use crate::shared::scene_ui::inspector::{EntityInspector, InspectorContent, InspectorContext, InspectorOutput};
use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use engine_core::ecs::*;
use engine_core::ui::*;

/// Shared inspector shell for all editor modes.
pub struct Inspector {
    pub rect: Rect,
    scroll_state: ScrollState,

    game: GameProperties,
    world: WorldProperties,
    room: RoomProperties,
    entity: Option<EntityInspector>,

    active: ActivePane,
    interactive_rects: Vec<Rect>,
    hidden: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActivePane {
    Empty,
    Game,
    World,
    Room,
    Entity,
}

impl Inspector {
    /// Creates a new inspector with pre-built panes.
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            scroll_state: ScrollState::new(),
            game: GameProperties::new(),
            world: WorldProperties::new(),
            room: RoomProperties::new(),
            entity: None,
            active: ActivePane::Empty,
            interactive_rects: Vec::new(),
            hidden: false,
        }
    }

    /// Sets the on-screen rectangle for the inspector.
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Hides the inspector.
    pub fn hide(&mut self) {
        self.active = ActivePane::Empty;
        self.hidden = true;
    }

    /// Selects the room-properties pane for the next draw.
    pub fn select_room(&mut self) {
        self.active = ActivePane::Room;
        self.hidden = false;
    }

    /// Selects the entity-inspector pane for the next draw.
    pub fn select_entity(&mut self, entity: Entity) {
        self.entity = Some(EntityInspector::new(entity));
        self.active = ActivePane::Entity;
        self.hidden = false;
    }

    /// Draws the game properties inspector.
    pub fn draw_game_pane(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        self.active = ActivePane::Game;
        self.hidden = false;
        self.draw(ctx, game_ctx, insp_ctx)
    }

    /// Draws the world properties inspector.
    pub fn draw_world_pane(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        self.active = ActivePane::World;
        self.hidden = false;
        self.draw(ctx, game_ctx, insp_ctx)
    }

    /// Draws whichever pane was last selected via select_* or draw_*_pane.
    pub fn draw_active_pane(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        self.draw(ctx, game_ctx, insp_ctx)
    }

    /// Returns whether the active pane is targeting an entity.
    pub fn has_target(&self) -> bool {
        matches!(self.active, ActivePane::Entity)
    }

    /// Returns whether the mouse is over the inspector.
    pub fn is_mouse_over(&self, ctx: &WgpuContext) -> bool {
        if self.hidden {
            return false;
        }
        let mouse: Vec2 = ctx.mouse_position().into();
        self.interactive_rects
            .iter()
            .any(|r| r.contains(mouse))
            || self.rect.contains(mouse)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        if self.hidden {
            return InspectorOutput::default();
        }

        self.interactive_rects.clear();
        let blocked = is_mouse_over_panel(ctx);

        // Clear content interactive rects before module/header passes
        match self.active {
            ActivePane::Entity => {
                if let Some(e) = &mut self.entity {
                    e.clear_interactive_rects();
                }
            }
            _ => {}
        }

        let top_offset = inspector::CONTENT_TOP_OFFSET;
        let inner = Rect::new(
            self.rect.x,
            self.rect.y + top_offset,
            self.rect.w - 20.0,
            self.rect.h - top_offset - 20.0,
        );
        ctx.draw_rectangle(inner.x, inner.y, inner.w, inner.h, Color::new(0., 0., 0., 0.6));

        let total = match self.active {
            ActivePane::Game => self.game.total_content_height(game_ctx, insp_ctx),
            ActivePane::World => self.world.total_content_height(game_ctx, insp_ctx),
            ActivePane::Room => self.room.total_content_height(game_ctx, insp_ctx),
            ActivePane::Entity => self
                .entity
                .as_ref()
                .map_or(0.0, |e| e.total_content_height(game_ctx, insp_ctx)),
            ActivePane::Empty => 0.0,
        };
        let area = ScrollableArea::new(inner, total)
            .scroll_speed(SCROLL_SPEED)
            .blocked(is_mouse_over_dropdown_list(ctx))
            .begin(ctx, &mut self.scroll_state);
        let content_rect = area.content_rect();
        let scrolled_content_rect = Rect::new(
            content_rect.x,
            content_rect.y + self.scroll_state.scroll_y,
            content_rect.w,
            content_rect.h,
        );

        // Modules draw first so header dropdown lists render on top.
        ctx.push_clip_rect(inner);
        let module_output = match self.active {
            ActivePane::Game => self.game.draw_modules(ctx, scrolled_content_rect, blocked, game_ctx, insp_ctx),
            ActivePane::World => self.world.draw_modules(ctx, scrolled_content_rect, blocked, game_ctx, insp_ctx),
            ActivePane::Room => self.room.draw_modules(ctx, scrolled_content_rect, blocked, game_ctx, insp_ctx),
            ActivePane::Entity => {
                if let Some(e) = &mut self.entity {
                    e.draw_modules(ctx, scrolled_content_rect, blocked, game_ctx, insp_ctx)
                } else {
                    InspectorOutput::default()
                }
            }
            ActivePane::Empty => InspectorOutput::default(),
        };
        ctx.pop_clip_rect();

        area.draw_scrollbar(ctx, &self.scroll_state);
        ctx.draw_rectangle_lines(inner.x, inner.y, inner.w, inner.h, 2., Color::WHITE);

        // Header draws last so dropdown lists render on top of modules
        let header_height = match self.active {
            ActivePane::Game | ActivePane::World | ActivePane::Room => {
                inspector::HEADER_HEIGHT
            }
            ActivePane::Entity => self.entity.as_ref().map_or(0.0, |e| e.header_height()),
            ActivePane::Empty => 0.0,
        };
        let header_rect = Rect::new(self.rect.x, self.rect.y, self.rect.w, header_height);
        let mut output = match self.active {
            ActivePane::Game => self.game.draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx),
            ActivePane::World => self.world.draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx),
            ActivePane::Room => self.room.draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx),
            ActivePane::Entity => {
                if let Some(e) = &mut self.entity {
                    e.draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx)
                } else {
                    InspectorOutput::default()
                }
            }
            ActivePane::Empty => InspectorOutput::default(),
        };

        output.merge(module_output);
        self.interactive_rects = match self.active {
            ActivePane::Game => self.game.interactive_rects(),
            ActivePane::World => self.world.interactive_rects(),
            ActivePane::Room => self.room.interactive_rects(),
            ActivePane::Entity => self.entity.as_mut().map_or(vec![], |e| e.interactive_rects()),
            ActivePane::Empty => vec![],
        };
        output
    }
}

const SCROLL_SPEED: f32 = 5.0;
