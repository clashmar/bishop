// engine_core/src/ecs/inspector_module.rs
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::inspector_layout::InspectorBodyLayout;
use crate::game::GameCtxMut;
use crate::storage::editor_config::{get_inspector_module_expanded, set_inspector_module_expanded};
use crate::ui::widgets::*;
use bishop::prelude::*;

/// Every inspector sub‑module implements this trait.
pub trait InspectorModule {
    /// Return true when the module should be shown for the given entity.
    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool;

    // TODO: Make this async
    /// Draw the UI for the module inside the supplied rectangle.
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    );

    /// Layout that describes the expanded body height for this module.
    fn body_layout(&self) -> InspectorBodyLayout;

    /// Height in screen pixels that the module occupies when expanded.
    fn height(&self) -> f32 {
        self.body_layout().height()
    }

    /// Title that appears in the collapsible header. Uses the
    /// Rust type name by default and can be overriden.
    fn title(&self) -> &str {
        std::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .unwrap_or("Module")
    }

    /// The component type name this module edits, used for undo/redo snapshot tracking.
    /// Return `Some(T::TYPE_NAME)` for single-component modules.
    /// Return `None` for modules that manage multiple components or don't support undo.
    fn undo_component_type(&self) -> Option<&'static str>;

    /// Returns `true` and clears the internal flag if a remove was requested this draw.
    /// Default returns `false`. `CollapsibleModule` overrides this.
    fn take_remove_request(&mut self) -> bool {
        false
    }

    /// Return true if the module should get a “Remove” button in the header.
    /// Default is false.
    fn removable(&self) -> bool {
        false
    }

    /// Called when the user clicks the remove component button.
    /// Default implementation does nothing.
    fn remove(&mut self, _game_ctx: &mut GameCtxMut, _entity: Entity) {}
}

/// Generic wrapper that adds a collapsible header around any concrete
/// `InspectorModule`.  It stores the `expanded` flag, draws the “‑/＋” button,
/// and forwards the actual drawing to the inner module when expanded.
pub struct CollapsibleModule<T: InspectorModule> {
    inner: T,
    expanded: bool,
    /// Optional custom title. If `None`, ask the inner module for its
    /// `title()` implementation.
    custom_title: Option<String>,
    /// Set when the user clicks the "x" remove button; polled by the inspector panel.
    remove_requested: bool,
}

impl<T: InspectorModule> CollapsibleModule<T> {
    pub fn new(inner: T) -> Self {
        let mut module = Self {
            inner,
            expanded: true, // start opened
            custom_title: None,
            remove_requested: false,
        };

        module.sync_saved_state();
        module
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.custom_title = Some(title.into());
        self.sync_saved_state();
        self
    }

    fn title(&self) -> &str {
        if let Some(ref t) = self.custom_title {
            t
        } else {
            self.inner.title()
        }
    }

    fn sync_saved_state(&mut self) {
        if let Some(expanded) = get_inspector_module_expanded(self.title()) {
            self.expanded = expanded;
        }
    }

    fn persist_saved_state(&self) {
        set_inspector_module_expanded(self.title(), self.expanded);
    }

    /// The clickable area that contains the “‑/＋” button, title and remove button.
    const HEADER_HEIGHT: f32 = 24.0;
}

impl<T: InspectorModule> InspectorModule for CollapsibleModule<T> {
    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        self.inner.visible(ecs, entity)
    }

    fn undo_component_type(&self) -> Option<&'static str> {
        self.inner.undo_component_type()
    }

    fn take_remove_request(&mut self) -> bool {
        std::mem::take(&mut self.remove_requested)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        // Background for the header
        ctx.draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            Self::HEADER_HEIGHT,
            Color::new(0., 0., 0., 0.4),
        );

        ctx.draw_text(
            self.title(),
            rect.x + 28.0,
            rect.y + 18.0,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        // Toggle button (‑ when open, ＋ when closed)
        let btn = Rect::new(rect.x + 4.0, rect.y + 4.0, 16.0, 16.0);
        let symbol = if self.expanded { "-" } else { "+" };
        if Button::new(btn, symbol)
            .text_offset(Vec2::new(-0.3, 1.5))
            .blocked(blocked)
            .show(ctx)
        {
            self.expanded = !self.expanded;
            self.persist_saved_state();
        }

        // Remove component
        if self.inner.removable() {
            const BTN_W: f32 = 20.0;
            const BTN_H: f32 = 20.0;
            // Right‑aligned, vertically centred in the header
            let btn_rect = Rect::new(
                rect.x + rect.w - BTN_W - 4.0,
                rect.y + (Self::HEADER_HEIGHT - BTN_H) / 2.0,
                BTN_W,
                BTN_H,
            );
            if Button::new(btn_rect, "x").blocked(blocked).show(ctx) {
                self.remove_requested = true;
                return; // Don't draw the rest of the module
            }
        }

        // Body, when expanded
        if self.expanded {
            // Give the inner module a little padding inside the panel
            let body_rect = Rect::new(
                rect.x + 4.0,
                rect.y + Self::HEADER_HEIGHT + 4.0,
                rect.w - 8.0,
                rect.h - Self::HEADER_HEIGHT - 8.0,
            );
            self.inner.draw(ctx, blocked, body_rect, game_ctx, entity);
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.inner.body_layout()
    }

    fn height(&self) -> f32 {
        if self.expanded {
            Self::HEADER_HEIGHT + self.inner.body_layout().height()
        } else {
            Self::HEADER_HEIGHT
        }
    }

    fn title(&self) -> &str {
        self.title()
    }
}
