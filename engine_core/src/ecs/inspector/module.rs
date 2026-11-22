use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::inspector::collapsible_header::CollapsibleHeader;
use crate::ecs::inspector::layout::InspectorBodyLayout;
use crate::game::GameCtxMut;
use crate::storage::editor_config::*;
use crate::ui::widgets::*;
use bishop::prelude::*;
use widgets::constants::layout;

/// Every inspector sub‑module implements this trait.
pub trait InspectorModule {
    /// Return true when the module should be shown for the given entity.
    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool;

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

    /// Returns the current expansion state for collapsible modules.
    fn expansion_state(&self) -> Option<bool> {
        None
    }

    /// Sets the expansion state for collapsible modules.
    fn set_expanded(&mut self, _expanded: bool) {}

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
    /// Default returns `false`. `CollapsibleComponentModule` overrides this.
    fn take_remove_request(&mut self) -> bool {
        false
    }

    /// Return true if any input widget inside this module was actively being edited
    /// (focused or committed) during the most recent draw.
    fn was_input_active(&self) -> bool {
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
pub struct CollapsibleComponentModule<T: InspectorModule> {
    inner: T,
    expanded: bool,
    /// Optional custom title. If `None`, ask the inner module for its
    /// `title()` implementation.
    custom_title: Option<String>,
    /// Set when the user clicks the "x" remove button; polled by the inspector panel.
    remove_requested: bool,
}

impl<T: InspectorModule> CollapsibleComponentModule<T> {
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

    fn set_expanded_state(&mut self, expanded: bool) {
        if self.expanded != expanded {
            self.expanded = expanded;
            self.persist_saved_state();
        }
    }

    fn collapse_button_rect(rect: Rect) -> Rect {
        Rect::new(rect.x + 4.0, rect.y + 4.0, 16.0, 16.0)
    }
}

impl<T: InspectorModule> InspectorModule for CollapsibleComponentModule<T> {
    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        self.inner.visible(ecs, entity)
    }

    fn undo_component_type(&self) -> Option<&'static str> {
        self.inner.undo_component_type()
    }

    fn take_remove_request(&mut self) -> bool {
        std::mem::take(&mut self.remove_requested)
    }

    fn was_input_active(&self) -> bool {
        self.inner.was_input_active()
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        if draw_component_header(ctx, self.title(), rect, blocked, self.inner.removable()) {
            self.remove_requested = true;
            return;
        }

        // Toggle button (‑ when open, ＋ when closed)
        let symbol = if self.expanded { "-" } else { "+" };
        if Button::new(Self::collapse_button_rect(rect), symbol)
            .text_offset(CollapsibleHeader::BUTTON_TEXT_OFFSET)
            .suppressed(blocked)
            .show(ctx)
        {
            self.set_expanded_state(!self.expanded);
        }

        // Body, when expanded
        if self.expanded {
            // Give the inner module a little padding inside the panel
            let body_rect = Rect::new(
                rect.x + 4.0,
                rect.y + CollapsibleHeader::HEADER_HEIGHT + 4.0,
                rect.w - 8.0,
                rect.h - CollapsibleHeader::HEADER_HEIGHT - 8.0,
            );
            self.inner.draw(ctx, blocked, body_rect, game_ctx, entity);
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        self.inner.body_layout()
    }

    fn height(&self) -> f32 {
        if self.expanded {
            CollapsibleHeader::HEADER_HEIGHT + self.inner.body_layout().height()
        } else {
            CollapsibleHeader::HEADER_HEIGHT
        }
    }

    fn expansion_state(&self) -> Option<bool> {
        Some(self.expanded)
    }

    fn set_expanded(&mut self, expanded: bool) {
        self.set_expanded_state(expanded);
    }

    fn title(&self) -> &str {
        self.title()
    }
}

/// Displays a marker component without an expandable body.
pub struct MarkerComponentModule<T: InspectorModule> {
    inner: T,
    custom_title: Option<String>,
    remove_requested: bool,
}

impl<T: InspectorModule> MarkerComponentModule<T> {
    /// Creates a marker module around an inspector module.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            custom_title: None,
            remove_requested: false,
        }
    }

    /// Sets the title shown in the marker row.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.custom_title = Some(title.into());
        self
    }

    fn title(&self) -> &str {
        if let Some(ref title) = self.custom_title {
            title
        } else {
            self.inner.title()
        }
    }
}

impl<T: InspectorModule> InspectorModule for MarkerComponentModule<T> {
    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        self.inner.visible(ecs, entity)
    }

    fn undo_component_type(&self) -> Option<&'static str> {
        self.inner.undo_component_type()
    }

    fn take_remove_request(&mut self) -> bool {
        std::mem::take(&mut self.remove_requested)
    }

    fn was_input_active(&self) -> bool {
        self.inner.was_input_active()
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        _game_ctx: &mut GameCtxMut,
        _entity: Entity,
    ) {
        if draw_component_header(ctx, self.title(), rect, blocked, self.inner.removable()) {
            self.remove_requested = true;
            return;
        }

        let placeholder_rect = CollapsibleComponentModule::<T>::collapse_button_rect(rect);
        ctx.draw_rectangle(
            placeholder_rect.x,
            placeholder_rect.y,
            placeholder_rect.w,
            placeholder_rect.h,
            Color::WHITE,
        );
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new().padding(0.0, 0.0)
    }

    fn height(&self) -> f32 {
        CollapsibleHeader::HEADER_HEIGHT
    }

    fn title(&self) -> &str {
        self.title()
    }

    fn removable(&self) -> bool {
        self.inner.removable()
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        self.inner.remove(game_ctx, entity);
    }
}

fn remove_button_rect(rect: Rect) -> Rect {
    const BUTTON_WIDTH: f32 = 20.0;
    const BUTTON_HEIGHT: f32 = 20.0;
    Rect::new(
        rect.x + rect.w - BUTTON_WIDTH - 4.0,
        rect.y + (CollapsibleHeader::HEADER_HEIGHT - BUTTON_HEIGHT) / 2.0,
        BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn draw_component_header(
    ctx: &mut WgpuContext,
    title: &str,
    rect: Rect,
    blocked: bool,
    removable: bool,
) -> bool {
    ctx.draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        CollapsibleHeader::HEADER_HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.4),
    );
    ctx.draw_text(
        title,
        rect.x + 28.0,
        rect.y + 18.0,
        layout::DEFAULT_FONT_SIZE_16,
        Color::WHITE,
    );
    removable
        && Button::new(remove_button_rect(rect), "x")
            .text_offset(CollapsibleHeader::BUTTON_TEXT_OFFSET)
            .suppressed(blocked)
            .show(ctx)
}
