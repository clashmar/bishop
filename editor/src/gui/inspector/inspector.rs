use crate::gui::gui_constants::INSPECTOR_CONTENT_TOP_OFFSET;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::gui::properties::room::RoomProperties;
use crate::shared::scene_ui::inspector::{EntityInspector, InspectorContent, SceneInspectorContext, SceneInspectorOutput};
use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use engine_core::prelude::*;

/// Shell that handles scroll, clipping, and background. Delegates
/// all domain rendering to a Box<dyn InspectorContent>.
pub struct Inspector {
    pub rect: Rect,
    scroll_state: ScrollState,
    content: Box<dyn InspectorContent>,
    content_kind: InspectorContentKind,
    interactive_rects: Vec<Rect>,
    hidden: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorContentKind {
    Empty,
    RoomProperties,
    Entity(Entity),
}

impl Inspector {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            scroll_state: ScrollState::new(),
            content: Box::new(EmptyContent),
            content_kind: InspectorContentKind::Empty,
            interactive_rects: Vec::new(),
            hidden: false,
        }
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Set the content to display. Resets scroll state.
    pub fn set_content(&mut self, content: Box<dyn InspectorContent>) {
        self.content = content;
        self.content_kind = InspectorContentKind::Empty;
        self.scroll_state = ScrollState::new();
        self.hidden = false;
    }

    /// Show the shared entity inspector for the given target.
    pub fn show_entity(&mut self, entity: Entity) {
        if !self.hidden && self.content_kind == InspectorContentKind::Entity(entity) {
            return;
        }
        self.set_content(Box::new(EntityInspector::new(entity)));
        self.content_kind = InspectorContentKind::Entity(entity);
    }

    /// Show room properties when no entity is selected.
    pub fn show_room_properties(&mut self) {
        if !self.hidden && self.content_kind == InspectorContentKind::RoomProperties {
            return;
        }
        self.set_content(Box::new(RoomProperties::new()));
        self.content_kind = InspectorContentKind::RoomProperties;
    }

    /// Hide the inspector entirely (no content rendered).
    pub fn hide(&mut self) {
        if self.hidden {
            return;
        }
        self.hidden = true;
        self.content = Box::new(EmptyContent);
        self.content_kind = InspectorContentKind::Empty;
    }

    /// Whether the current content has an active target (entity selected).
    pub fn has_target(&self) -> bool {
        self.content.target().is_some()
    }

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

    /// Draw the inspector. Shell handles background, clipping, and scrolling.
    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut,
        scene_ctx: &SceneInspectorContext,
    ) -> SceneInspectorOutput {
        if self.hidden {
            return SceneInspectorOutput::default();
        }

        self.interactive_rects.clear();
        let blocked = is_mouse_over_panel(ctx);

        // Background plate
        let top_offset = INSPECTOR_CONTENT_TOP_OFFSET;
        let inner = Rect::new(
            self.rect.x,
            self.rect.y + top_offset,
            self.rect.w - 20.0,
            self.rect.h - top_offset - 20.0,
        );
        ctx.draw_rectangle(
            inner.x,
            inner.y,
            inner.w,
            inner.h,
            Color::new(0., 0., 0., 0.6),
        );

        // Header
        let header_rect = Rect::new(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.content.header_height(),
        );
        let mut output = self
            .content
            .draw_header(ctx, header_rect, blocked, game_ctx, scene_ctx);

        // Scrollable modules
        let total = self
            .content
            .total_content_height(game_ctx, scene_ctx);
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

        ctx.push_clip_rect(inner);
        let module_output = self
            .content
            .draw_modules(ctx, scrolled_content_rect, blocked, game_ctx, scene_ctx);
        ctx.pop_clip_rect();

        area.draw_scrollbar(ctx, &self.scroll_state);
        ctx.draw_rectangle_lines(inner.x, inner.y, inner.w, inner.h, 2., Color::WHITE);

        let _ = self.content.was_input_active();
        self.interactive_rects = self.content.interactive_rects();
        output.merge(module_output);
        output
    }
}

const SCROLL_SPEED: f32 = 5.0;

/// Default empty content used before any content is set.
struct EmptyContent;

impl InspectorContent for EmptyContent {
    fn draw_modules(
        &mut self,
        _ctx: &mut WgpuContext,
        _rect: Rect,
        _blocked: bool,
        _game_ctx: &mut GameCtxMut,
        _scene_ctx: &SceneInspectorContext,
    ) -> SceneInspectorOutput {
        SceneInspectorOutput::default()
    }

    fn total_content_height(
        &self,
        _game_ctx: &mut GameCtxMut,
        _scene_ctx: &SceneInspectorContext,
    ) -> f32 {
        0.0
    }
}
