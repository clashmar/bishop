use crate::gui::gui_constants::inspector;
use crate::gui::inspector::action_strip::{self, ActionStripAction, ActionStripLayout};
use crate::gui::inspector::scrollable_pane;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::gui::properties::game::GameProperties;
use crate::gui::properties::room::RoomProperties;
use crate::gui::properties::tilemap::TilemapProperties;
use crate::gui::properties::world::WorldProperties;
use crate::shared::scene_ui::inspector::{EntityInspector, InspectorContent, InspectorContext, InspectorOutput};
use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use engine_core::ecs::*;
use engine_core::tiles::TileDefId;
use engine_core::storage::editor_config;
use widgets::{clear_all_input_focus, with_theme, ScrollState};

/// Shared inspector shell for all editor modes.
pub struct Inspector {
    pub rect: Rect,
    scroll_state: ScrollState,

    pub game: GameProperties,
    pub world: WorldProperties,
    pub room: RoomProperties,
    pub tilemap: TilemapProperties,
    pub entity: Option<EntityInspector>,
    cycle_entities: Vec<Entity>,

    active: ActivePane,
    interactive_rects: Vec<Rect>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActivePane {
    Empty,
    Game,
    World,
    Room,
    Tilemap,
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
            tilemap: TilemapProperties::new(),
            entity: None,
            cycle_entities: Vec::new(),
            active: ActivePane::Empty,
            interactive_rects: Vec::new(),
        }
    }

    /// Sets the on-screen rectangle for the inspector.
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Selects the room-properties pane for the next draw.
    pub fn select_room(&mut self) {
        self.active = ActivePane::Room;
    }

    /// Selects the tilemap pane for the next draw.
    pub fn select_tilemap(&mut self) {
        self.active = ActivePane::Tilemap;
    }

    /// Sets the entities available for inspector navigation.
    pub(crate) fn set_cycle_entities(&mut self, entities: Vec<Entity>) {
        self.cycle_entities = entities;
    }

    /// Selects the entity-inspector pane for the next draw.
    pub fn select_entity(&mut self, entity: Entity) {
        let needs_rebuild = self
            .entity
            .as_ref()
            .and_then(EntityInspector::target)
            != Some(entity);
        if needs_rebuild {
            self.entity = Some(EntityInspector::new(entity));
        }
        self.active = ActivePane::Entity;
    }

    /// Draws the game properties inspector.
    pub fn draw_game_pane(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        self.active = ActivePane::Game;
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

    /// Returns the active tile brush, if one is selected.
    pub fn selected_tile_brush(&self) -> Option<TileDefId> {
        self.tilemap.selected_brush_id()
    }

    /// Moves the inspector target to the next cycle entity and returns it.
    pub(crate) fn cycle_selected_entity(&mut self, forward: bool) -> Option<Entity> {
        let current = self.selected_entity()?;
        if self.cycle_entities.len() < 2 {
            return None;
        }
        let index = self.cycle_entities.iter().position(|entity| *entity == current)?;
        let next_index = if forward {
            (index + 1) % self.cycle_entities.len()
        } else {
            (index + self.cycle_entities.len() - 1) % self.cycle_entities.len()
        };
        let target = self.cycle_entities[next_index];
        self.select_entity(target);
        Some(target)
    }

    /// Returns the entity currently shown in the inspector.
    pub fn selected_entity(&self) -> Option<Entity> {
        match self.active {
            ActivePane::Entity => self.entity.as_ref().and_then(EntityInspector::target),
            _ => None,
        }
    }

    /// Toggles the inspector visibility globally.
    pub fn toggle_visible(&mut self) {
        let visible = !editor_config::get_inspector_visible();
        editor_config::set_inspector_visible(visible);
    }

    /// Returns whether the inspector is visible.
    pub fn is_visible(&self) -> bool {
        editor_config::get_inspector_visible()
    }

    /// Clears the current target without changing visibility.
    pub fn clear_target(&mut self) {
        self.active = ActivePane::Empty;
        self.entity = None;
    }

    #[cfg(test)]
    pub(crate) fn entity_inspector_addr(&self) -> Option<usize> {
        self.entity
            .as_ref()
            .map(|entity| entity as *const EntityInspector as usize)
    }

    /// Returns the on-screen rectangle for the toggle strip.
    pub(crate) fn strip_rect(&self) -> Rect {
        let content_top_offset = self.content_top_offset();
        Rect::new(
            self.rect.x + self.rect.w - inspector::EDGE_GAP - inspector::STRIP_WIDTH,
            self.rect.y + content_top_offset,
            inspector::STRIP_WIDTH,
            self.rect.h - content_top_offset - inspector::EDGE_GAP,
        )
    }

    fn content_top_offset(&self) -> f32 {
        inspector::CONTENT_TOP_OFFSET - inspector::HEADER_HEIGHT + self.header_height()
    }

    pub(super) fn content_frame_rect(&self) -> Rect {
        let content_top_offset = self.content_top_offset();
        Rect::new(
            self.rect.x,
            self.rect.y + content_top_offset,
            self.rect.w - inspector::EDGE_GAP - inspector::STRIP_WIDTH,
            self.rect.h - content_top_offset - inspector::EDGE_GAP,
        )
    }

    pub(super) fn action_strip_layout(&self, insp_ctx: &InspectorContext) -> ActionStripLayout {
        action_strip::layout(
            self.content_frame_rect(),
            matches!(self.active, ActivePane::Entity),
            insp_ctx.command_mode,
            self.cycle_entities.len(),
            self.active_has_collapsible_modules(),
        )
    }

    fn active_has_collapsible_modules(&self) -> bool {
        match self.active {
            ActivePane::Game => InspectorContent::has_collapsible_modules(&self.game),
            ActivePane::World => InspectorContent::has_collapsible_modules(&self.world),
            ActivePane::Room => InspectorContent::has_collapsible_modules(&self.room),
            ActivePane::Tilemap => InspectorContent::has_collapsible_modules(&self.tilemap),
            ActivePane::Entity => self
                .entity
                .as_ref()
                .is_some_and(InspectorContent::has_collapsible_modules),
            ActivePane::Empty => false,
        }
    }

    pub(crate) fn collapse_active_pane(&mut self) {
        clear_all_input_focus();
        match self.active {
            ActivePane::Game => InspectorContent::collapse_all(&mut self.game),
            ActivePane::World => InspectorContent::collapse_all(&mut self.world),
            ActivePane::Room => InspectorContent::collapse_all(&mut self.room),
            ActivePane::Tilemap => InspectorContent::collapse_all(&mut self.tilemap),
            ActivePane::Entity => {
                if let Some(entity) = &mut self.entity {
                    InspectorContent::collapse_all(entity);
                }
            }
            ActivePane::Empty => {}
        }
    }

    /// Returns the header height for the active pane.
    fn header_height(&self) -> f32 {
        match self.active {
            ActivePane::Game | ActivePane::World | ActivePane::Room => {
                inspector::HEADER_HEIGHT
            }
            ActivePane::Tilemap => 0.0,
            ActivePane::Entity => self.entity.as_ref().map_or(0.0, |e| e.header_height()),
            ActivePane::Empty => 0.0,
        }
    }

    /// Returns whether a given screen-space point should be treated as over the inspector.
    pub(crate) fn hit_test_point(&self, mouse: Vec2) -> bool {
        let strip_hit = self.strip_rect().contains(mouse);
        let header_hit = mouse.y >= self.rect.y
            && mouse.y <= self.rect.y + self.header_height()
            && mouse.x >= self.rect.x
            && mouse.x <= self.rect.x + self.rect.w;

        if !self.is_visible() {
            return strip_hit || header_hit;
        }

        self.interactive_rects.iter().any(|r| r.contains(mouse))
            || self.rect.contains(mouse)
            || strip_hit
    }

    /// Returns whether the mouse is over the inspector.
    pub fn is_mouse_over(&self, ctx: &WgpuContext) -> bool {
        self.hit_test_point(ctx.mouse_position().into())
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        let strip = self.strip_rect();
        let mouse: Vec2 = ctx.mouse_position().into();
        let strip_hovered = strip.contains(mouse);

        let strip_color = with_theme(|theme| {
            if strip_hovered {
                theme.hover
            } else {
                theme.panel
            }
        });
        ctx.draw_rectangle(strip.x, strip.y, strip.w, strip.h, strip_color);
        ctx.draw_rectangle_lines(strip.x, strip.y, strip.w, strip.h, 2.0, Color::WHITE);

        if !is_mouse_over_panel(ctx)
            && ctx.is_mouse_button_pressed(MouseButton::Left)
            && strip_hovered
        {
            self.toggle_visible();
        }

        let visible = self.is_visible();
        let blocked = is_mouse_over_panel(ctx);
        self.interactive_rects.clear();
        let action_strip = self.action_strip_layout(insp_ctx);
        let mut strip_interactive_rects = Vec::new();
        let mut cycle_target = None;
        if visible {
            strip_interactive_rects = action_strip.interactive_rects();
            match action_strip::draw(ctx, action_strip, blocked) {
                Some(ActionStripAction::CollapseAll) => self.collapse_active_pane(),
                Some(ActionStripAction::CyclePrevious) => {
                    cycle_target = self.cycle_selected_entity(false);
                }
                Some(ActionStripAction::CycleNext) => {
                    cycle_target = self.cycle_selected_entity(true);
                }
                None => {}
            }
        }
        let header_rect = Rect::new(self.rect.x, self.rect.y, self.rect.w, self.header_height());
        let content_frame_rect = self.content_frame_rect();
        let body_rect = Rect::new(
            content_frame_rect.x,
            content_frame_rect.y + inspector::ACTION_STRIP_HEIGHT,
            content_frame_rect.w,
            (content_frame_rect.h - inspector::ACTION_STRIP_HEIGHT).max(0.0),
        );

        if let ActivePane::Entity = self.active {
            if let Some(entity) = &mut self.entity {
                entity.clear_interactive_rects();
            }
        }

        let (mut output, mut interactive_rects) = match self.active {
            ActivePane::Game => {
                let mut output = self
                    .game
                    .draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx);
                let body_output = if visible {
                    scrollable_pane::draw(
                        &mut self.game,
                        ctx,
                        blocked,
                        game_ctx,
                        insp_ctx,
                        body_rect,
                        &mut self.scroll_state,
                    )
                } else {
                    InspectorOutput::default()
                };
                output.merge(body_output);
                (output, self.game.interactive_rects())
            }
            ActivePane::World => {
                let mut output = self
                    .world
                    .draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx);
                let body_output = if visible {
                    scrollable_pane::draw(
                        &mut self.world,
                        ctx,
                        blocked,
                        game_ctx,
                        insp_ctx,
                        body_rect,
                        &mut self.scroll_state,
                    )
                } else {
                    InspectorOutput::default()
                };
                output.merge(body_output);
                (output, self.world.interactive_rects())
            }
            ActivePane::Room => {
                let mut output = self
                    .room
                    .draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx);
                let body_output = if visible {
                    scrollable_pane::draw(
                        &mut self.room,
                        ctx,
                        blocked,
                        game_ctx,
                        insp_ctx,
                        body_rect,
                        &mut self.scroll_state,
                    )
                } else {
                    InspectorOutput::default()
                };
                output.merge(body_output);
                (output, self.room.interactive_rects())
            }
            ActivePane::Tilemap => {
                let mut output = self
                    .tilemap
                    .draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx);
                let body_output = if visible {
                    scrollable_pane::draw(
                        &mut self.tilemap,
                        ctx,
                        blocked,
                        game_ctx,
                        insp_ctx,
                        body_rect,
                        &mut self.scroll_state,
                    )
                } else {
                    InspectorOutput::default()
                };
                output.merge(body_output);
                (output, self.tilemap.interactive_rects())
            }
            ActivePane::Entity => {
                let output = if let Some(entity) = &mut self.entity {
                    let mut output =
                        entity.draw_header(ctx, header_rect, blocked, game_ctx, insp_ctx);
                    let body_output = if visible {
                        scrollable_pane::draw(
                            entity,
                            ctx,
                            blocked,
                            game_ctx,
                            insp_ctx,
                            body_rect,
                            &mut self.scroll_state,
                        )
                    } else {
                        InspectorOutput::default()
                    };
                    output.merge(body_output);
                    output
                } else {
                    InspectorOutput::default()
                };
                let rects = self
                    .entity
                    .as_mut()
                    .map_or_else(Vec::new, |entity| entity.interactive_rects());
                (output, rects)
            }
            ActivePane::Empty => (InspectorOutput::default(), vec![]),
        };

        if let Some(cycle_target) = cycle_target {
            output.merge(InspectorOutput {
                cycle_target: Some(cycle_target),
                ..InspectorOutput::default()
            });
        }
        interactive_rects.extend(strip_interactive_rects);
        self.interactive_rects = interactive_rects;
        if visible {
            ctx.draw_rectangle_lines(
                content_frame_rect.x,
                content_frame_rect.y,
                content_frame_rect.w,
                content_frame_rect.h,
                2.0,
                Color::WHITE,
            );
        }
        output
    }
}
