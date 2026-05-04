use crate::app::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::theme::with_theme;
use std::cell::RefCell;
use std::thread::LocalKey;

pub mod confirm;
pub mod delete_prefab;
pub mod delete_resource;
pub mod delete_world;
pub mod dirty_prefab_exit;
pub mod edit_world;
pub mod editor_settings;
pub mod empty_prefab_exit;
pub mod empty_prefab_save;
pub mod export_overwrite;
pub mod new_game;
pub mod new_resource_folder;
pub mod prefab_picker;
pub mod rename;
pub mod rename_resource;
pub mod rename_resource_folder;
pub mod save_as;
pub mod unsaved_exit;
pub mod world_settings;

#[derive(Default)]
pub struct Modal {
    pub rect: Rect,
    pub open: bool,
    widgets: BoxedWidgets,
    just_opened: bool,
}

pub use widgets::is_modal_open;

pub type BoxedWidget =
    Box<dyn FnMut(&mut WgpuContext, &mut AssetRegistry, &mut SpriteManager) + 'static>;
type BoxedWidgets = Vec<BoxedWidget>;

#[derive(Clone, PartialEq)]
pub enum ModalResult {
    String(String),
    ClickedOutside,
}

impl Modal {
    pub fn new(ctx: &WgpuContext, width: f32, height: f32) -> Self {
        let rect = Rect::new(
            (ctx.screen_width() - width) / 2.0,
            (ctx.screen_height() - height) / 2.0,
            width,
            height,
        );

        Self {
            rect,
            open: false,
            widgets: Vec::new(),
            just_opened: false,
        }
    }

    pub fn open(&mut self, callbacks: Vec<BoxedWidget>) {
        close_open_dropdowns();
        close_open_context_menus();
        self.open = true;
        self.widgets = callbacks;
        self.just_opened = true;
        widgets::set_modal_open(true);
    }

    pub fn close(&mut self) {
        close_open_dropdowns();
        close_open_context_menus();
        self.open = false;
        self.widgets = Vec::new();
        widgets::set_modal_open(false);
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        asset_registry: &mut AssetRegistry,
        sprite_manager: &mut SpriteManager,
    ) -> bool {
        if !self.open {
            return false;
        }

        if self.just_opened {
            self.just_opened = false;
            return false;
        }

        ctx.draw_rectangle(
            0.0,
            0.0,
            ctx.screen_width(),
            ctx.screen_height(),
            with_theme(|t| t.overlay.with_alpha(0.6)),
        );

        ctx.draw_rectangle(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            with_theme(|t| t.panel),
        );

        ctx.draw_rectangle_lines(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            2.0,
            with_theme(|t| t.border),
        );

        for widget in self.widgets.iter_mut() {
            widget.as_mut()(ctx, asset_registry, sprite_manager);
        }

        if ctx.is_mouse_button_pressed(MouseButton::Left) {
            let mouse = ctx.mouse_position().into();
            if !modal_hit_region_contains(self.rect, mouse) {
                return true;
            }
        }

        false
    }
}

fn modal_hit_region_contains(modal_rect: Rect, point: Vec2) -> bool {
    modal_rect.contains(point)
        || dropdown_state::STATE.with(|state| {
            state
                .borrow()
                .values()
                .any(|dropdown| dropdown.open && dropdown.rect.contains(point))
        })
        || context_menu_state::STATE.with(|state| {
            state
                .borrow()
                .values()
                .any(|cm| cm.open && cm.rect.contains(point))
        })
}

fn close_open_dropdowns() {
    dropdown_state::STATE.with(|state| {
        for dropdown in state.borrow_mut().values_mut() {
            dropdown.open = false;
        }
    });
    update_global_dropdown_flag();
}

pub fn open_modal_with_prompt<T: 'static>(
    modal: &mut Modal,
    mut prompt: impl FnMut(&mut WgpuContext) -> Option<T> + 'static,
    result_store: &'static LocalKey<RefCell<Option<T>>>,
) {
    let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _, _| {
        if let Some(result) = prompt(ctx) {
            result_store.with(|c| *c.borrow_mut() = Some(result));
        }
    })];
    modal.open(widgets);
}

pub fn take_modal_result<T>(store: &'static LocalKey<RefCell<Option<T>>>) -> Option<T> {
    store.with(|c| c.borrow_mut().take())
}

pub trait ModalHandler {
    type Result: 'static;

    fn result_store(&self) -> &'static LocalKey<RefCell<Option<Self::Result>>>;

    /// Open the modal, configuring editor.modal with prompt widgets.
    fn open(&mut self, editor: &mut Editor, ctx: &WgpuContext);

    /// Process a result from the thread-local store.
    /// Return Some(ModalResult) if the orchestrator should propagate it.
    fn handle(
        &mut self,
        editor: &mut Editor,
        ctx: &mut WgpuContext,
        result: Self::Result,
    ) -> Option<ModalResult>;

    /// Called when the user clicks outside the modal.
    fn on_outside_click(&mut self, _editor: &mut Editor) {
        take_modal_result(self.result_store());
    }
}

pub(crate) trait ErasedHandler {
    fn try_handle(&mut self, editor: &mut Editor, ctx: &mut WgpuContext) -> Option<ModalResult>;
    fn on_outside_click(&mut self, editor: &mut Editor);
}

pub(crate) struct HandlerWrapper<H: ModalHandler> {
    inner: H,
}

impl<H: ModalHandler> ErasedHandler for HandlerWrapper<H> {
    fn try_handle(&mut self, editor: &mut Editor, ctx: &mut WgpuContext) -> Option<ModalResult> {
        let result = take_modal_result(self.inner.result_store())?;
        self.inner.handle(editor, ctx, result)
    }

    fn on_outside_click(&mut self, editor: &mut Editor) {
        self.inner.on_outside_click(editor);
    }
}

pub struct ModalRegistry {
    handlers: Vec<Box<dyn ErasedHandler>>,
}

impl ModalRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn try_handle_all(
        &mut self,
        editor: &mut Editor,
        ctx: &mut WgpuContext,
    ) -> Option<ModalResult> {
        for handler in self.handlers.iter_mut() {
            if let Some(result) = handler.try_handle(editor, ctx) {
                return Some(result);
            }
        }
        None
    }

    pub fn handle_outside_click(&mut self, editor: &mut Editor) {
        for handler in self.handlers.iter_mut() {
            handler.on_outside_click(editor);
        }
    }

    pub fn init_from_inventory(&mut self) {
        for entry in inventory::iter::<ModalEntry> {
            self.handlers.push((entry.construct)());
        }
    }
}

pub struct ModalEntry {
    pub construct: fn() -> Box<dyn ErasedHandler>,
}
inventory::collect!(ModalEntry);

#[macro_export]
macro_rules! register_modal {
    ($modal:ident) => {
        inventory::submit! {
            $crate::gui::modals::ModalEntry {
                construct: || Box::new($crate::gui::modals::HandlerWrapper { inner: $modal }),
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_modal_clears_open_dropdown_state() {
        let dropdown_id = WidgetId::default();
        dropdown_state::set(
            dropdown_id,
            dropdown_state::DropState {
                open: true,
                rect: Rect::new(120.0, 240.0, 160.0, 100.0),
                scroll_offset: 0.0,
            },
        );
        update_global_dropdown_flag();

        assert!(is_dropdown_open());

        let mut modal = Modal {
            open: true,
            ..Default::default()
        };
        modal.close();

        assert!(!is_dropdown_open());

        dropdown_state::set(dropdown_id, dropdown_state::DropState::default());
        update_global_dropdown_flag();
    }

    #[test]
    fn modal_hit_region_includes_open_dropdown_lists() {
        let modal_rect = Rect::new(100.0, 100.0, 200.0, 120.0);
        let dropdown_rect = Rect::new(120.0, 240.0, 160.0, 100.0);
        let dropdown_id = WidgetId::default();

        dropdown_state::set(
            dropdown_id,
            dropdown_state::DropState {
                open: true,
                rect: dropdown_rect,
                scroll_offset: 0.0,
            },
        );

        assert!(modal_hit_region_contains(
            modal_rect,
            Vec2::new(140.0, 260.0)
        ));
        assert!(!modal_hit_region_contains(
            modal_rect,
            Vec2::new(40.0, 40.0)
        ));

        dropdown_state::set(dropdown_id, dropdown_state::DropState::default());
        update_global_dropdown_flag();
    }
}
