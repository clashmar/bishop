// editor/src/gui/inspector/modal.rs
use crate::gui::prompts::confirm_prompt::*;
use bishop::prelude::*;
use engine_core::assets::asset_manager::AssetManager;
use std::{cell::RefCell, thread::LocalKey};

#[derive(Default)]
pub struct Modal {
    /// Position & size of the modal window.
    pub rect: Rect,
    pub open: bool,
    widgets: BoxedWidgets,
    just_opened: bool,
}

thread_local! {
    pub static MODAL_OPEN: RefCell<bool> = const { RefCell::new(false) };
}

/// Global flag that tells the rest of the editor whether a modal
/// is currently open.
pub fn is_modal_open() -> bool {
    MODAL_OPEN.with(|f| *f.borrow())
}

pub type BoxedWidget = Box<dyn FnMut(&mut WgpuContext, &mut AssetManager) + 'static>;
type BoxedWidgets = Vec<BoxedWidget>;

/// Used by callers of a a modal to decide what should happen if
/// the user clicks outside the modal.
#[derive(Clone, PartialEq)]
pub enum ModalResult {
    String(String),
    ClickedOutside,
}

impl Modal {
    /// Creates a new modal of the given size. It is automatically centered.
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

    /// Open the modal and set draw callbacks.
    pub fn open(&mut self, callbacks: Vec<BoxedWidget>) {
        self.open = true;
        self.widgets = callbacks;
        self.just_opened = true;

        // Let the editor know a modal is open
        MODAL_OPEN.with(|r| {
            *r.borrow_mut() = true;
        });
    }

    /// Close the modal.
    pub fn close(&mut self) {
        self.open = false;
        self.widgets = Vec::new();

        // Let the editor know the modal is close
        MODAL_OPEN.with(|r| {
            *r.borrow_mut() = false;
        });
    }

    /// Returns `true` if the modal is currently open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Render the modal. Returns `true`` when the user clicked outside the window.
    /// Needs asset manager for widgets that need to access assets.
    pub fn draw(&mut self, ctx: &mut WgpuContext, asset_manager: &mut AssetManager) -> bool {
        if !self.open {
            return false;
        }

        // Prevent any interaction on first click
        if self.just_opened {
            self.just_opened = false;
            return false;
        }

        // Dim the whole screen
        ctx.draw_rectangle(
            0.0,
            0.0,
            ctx.screen_width(),
            ctx.screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.6),
        );

        // Window background & outline
        ctx.draw_rectangle(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            Color::new(0.08, 0.08, 0.10, 0.95),
        );

        ctx.draw_rectangle_lines(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            2.0,
            Color::WHITE,
        );

        // Run all widgets
        for widget in self.widgets.iter_mut() {
            widget.as_mut()(ctx, asset_manager);
        }

        // Detect a click outside the window
        if ctx.is_mouse_button_pressed(MouseButton::Left) {
            let mouse = ctx.mouse_position().into();
            if !self.rect.contains(mouse) {
                return true;
            }
        }

        false
    }

    /// Opens a model with a confirm prompt widget.
    /// The caller must pass in a static reference to the result store.
    pub fn open_confirm_modal(
        ctx: &WgpuContext,
        result_store: &'static LocalKey<RefCell<Option<ConfirmPromptResult>>>,
    ) -> Modal {
        Self::open_confirm_modal_with_message(ctx, result_store, "Are You Sure?")
    }

    /// Opens a modal with a confirm prompt widget and custom message.
    pub fn open_confirm_modal_with_message(
        ctx: &WgpuContext,
        result_store: &'static LocalKey<RefCell<Option<ConfirmPromptResult>>>,
        prompt_message: impl Into<String>,
    ) -> Modal {
        let mut modal = Modal::new(ctx, 300.0, 120.0);
        let mut prompt = ConfirmPrompt::new(modal.rect, prompt_message);

        let widgets: Vec<BoxedWidget> = vec![Box::new(move |ctx, _| {
            if let Some(result) = prompt.draw(ctx) {
                // Write the result to the static thread local
                result_store.with(|c| *c.borrow_mut() = Some(result));
            }
        })];

        modal.open(widgets);
        modal
    }
}
