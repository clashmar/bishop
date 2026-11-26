use super::state::InputBindingsPromptState;
use engine_core::input::InputActionId;
use bishop::prelude::{Rect, WgpuContext};
use bishop::Text;
use engine_core::theme::with_theme;
use widgets::{text_input_reset, Button, TextInput, Widget, WidgetId};

pub(super) struct ActionRenamer {
    name: String,
    is_open: bool,
    error: bool,
    input_id: WidgetId,
}

impl ActionRenamer {
    pub(super) fn new() -> Self {
        Self {
            name: String::new(),
            is_open: false,
            error: false,
            input_id: WidgetId::default(),
        }
    }

    pub(super) fn is_open(&self) -> bool {
        self.is_open
    }

    pub(super) fn begin(&mut self, action_id: &InputActionId) {
        self.name = action_id.as_str().to_string();
        self.error = false;
        self.is_open = true;
        text_input_reset(self.input_id);
    }

    pub(super) fn cancel_if_open(&mut self) -> bool {
        if !self.is_open {
            return false;
        }
        self.close();
        true
    }

    pub(super) fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        panel_rect: Rect,
        content_x: f32,
        state: &mut InputBindingsPromptState,
    ) {
        let name_width = panel_rect.w - 188.0;
        let name_rect = Rect::new(content_x, panel_rect.y + 8.0, name_width, 28.0);
        let (name, _) = TextInput::new(self.input_id, name_rect, &self.name).show(ctx);
        if name != self.name {
            self.error = false;
        }
        self.name = name;

        let apply_rect = Rect::new(name_rect.right() + 8.0, name_rect.y, 72.0, 28.0);
        if Button::new(apply_rect, "Apply")
            .blocked(self.name.trim().is_empty())
            .show(ctx)
        {
            if state.rename_selected_action(&self.name) {
                self.close();
            } else {
                self.error = true;
            }
        }
        let cancel_rect = Rect::new(apply_rect.right() + 8.0, name_rect.y, 72.0, 28.0);
        if Button::new(cancel_rect, "Cancel").show(ctx) {
            self.close();
        }

        let (message, color) = if self.error {
            ("Action ID must be unique", with_theme(|theme| theme.danger))
        } else {
            (
                "Save regenerates input_actions.lua; fix stale Lua refs",
                with_theme(|theme| theme.text_muted),
            )
        };
        ctx.draw_text(message, content_x, panel_rect.y + 48.0, 12.0, color);
    }

    fn close(&mut self) {
        self.name.clear();
        self.error = false;
        self.is_open = false;
        text_input_reset(self.input_id);
    }
}
