use super::capture::{
    binding_label, capture_binding, capture_mode_for, CaptureMode, CaptureRequest,
};
use super::rename::ActionRenamer;
use super::state::{BindingScope, InputBindingsPromptState};
use super::InputBindingsPromptResult;
use crate::app::control::escape::modal_escape_requested;
use bishop::prelude::{Color, Rect, WgpuContext};
use bishop::Draw;
use bishop::Text;
use engine_core::input::InputActionMap;
use engine_core::storage::InputSettings;
use engine_core::theme::with_theme;
use uuid::Uuid;
use widgets::{
    text_input_reset, truncate_to_width, Button, Dropdown, ScrollState, ScrollableArea, TextInput,
    Widget, WidgetId,
};

const ACTION_ROW_HEIGHT: f32 = 30.0;
const BINDING_ROW_HEIGHT: f32 = 30.0;
pub(crate) struct InputBindingsPrompt {
    rect: Rect,
    state: InputBindingsPromptState,
    action_name: String,
    action_renamer: ActionRenamer,
    capture_request: Option<CaptureRequest>,
    scope_id: WidgetId,
    action_name_id: WidgetId,
    actions_scroll: ScrollState,
    bindings_scroll: ScrollState,
}

impl InputBindingsPrompt {
    pub(crate) fn new(
        rect: Rect,
        actions: InputActionMap,
        settings: InputSettings,
        game_id: Uuid,
    ) -> Self {
        Self {
            rect,
            state: InputBindingsPromptState::new(actions, settings, game_id),
            action_name: String::new(),
            action_renamer: ActionRenamer::new(),
            capture_request: None,
            scope_id: WidgetId::default(),
            action_name_id: WidgetId::default(),
            actions_scroll: ScrollState::new(),
            bindings_scroll: ScrollState::new(),
        }
    }

    pub(crate) fn draw(&mut self, ctx: &mut WgpuContext) -> Option<InputBindingsPromptResult> {
        let escape_pressed = modal_escape_requested();
        let escape_cancelled_rename =
            escape_pressed && self.action_renamer.cancel_if_open();
        if !escape_cancelled_rename {
            if let Some(request) = self.capture_request {
                if escape_pressed {
                    self.capture_request = None;
                } else if let Some(binding) = capture_binding(ctx, request.mode) {
                    if let Some(index) = request.replacement_index {
                        self.state.replace_binding(index, binding);
                    } else {
                        self.state.add_binding(binding);
                    }
                    self.capture_request = None;
                }
            } else if escape_pressed {
                return Some(InputBindingsPromptResult::Cancel);
            }
        }

        let title_x = self.rect.x + 24.0;
        ctx.draw_text(
            "Input Bindings",
            title_x,
            self.rect.y + 36.0,
            20.0,
            with_theme(|theme| theme.text),
        );
        self.draw_actions_panel(ctx);
        self.draw_selected_action(ctx);
        self.draw_footer(ctx)
    }

    fn draw_actions_panel(&mut self, ctx: &mut WgpuContext) {
        let panel_rect = Rect::new(
            self.rect.x + 20.0,
            self.rect.y + 62.0,
            212.0,
            self.rect.h - 126.0,
        );
        draw_panel(ctx, panel_rect, with_theme(|theme| theme.background));
        ctx.draw_text(
            "New action ID",
            panel_rect.x + 8.0,
            panel_rect.y + 22.0,
            13.0,
            with_theme(|theme| theme.text_muted),
        );

        let name_width = panel_rect.w - 16.0 - 80.0 - 8.0;
        let name_rect = Rect::new(panel_rect.x + 8.0, panel_rect.y + 28.0, name_width, 30.0);
        let (name, _) = TextInput::new(self.action_name_id, name_rect, &self.action_name).show(ctx);
        self.action_name = name;
        let add_rect = Rect::new(name_rect.right() + 8.0, name_rect.y, 80.0, 30.0);
        if Button::new(add_rect, "Add")
            .blocked(self.action_name.trim().is_empty())
            .suppressed(self.capture_request.is_some() || self.action_renamer.is_open())
            .show(ctx)
        {
            if self.state.add_action(&self.action_name) {
                self.action_name.clear();
                text_input_reset(self.action_name_id);
            }
        }

        ctx.draw_text(
            "Actions",
            panel_rect.x + 8.0,
            panel_rect.y + 80.0,
            14.0,
            with_theme(|theme| theme.text_muted),
        );
        let list_rect = Rect::new(
            panel_rect.x + 8.0,
            panel_rect.y + 88.0,
            panel_rect.w - 16.0,
            panel_rect.h - 100.0,
        );
        let action_ids = self.state.action_ids();
        let content_height = action_ids.len() as f32 * ACTION_ROW_HEIGHT;
        let area = ScrollableArea::new(list_rect, content_height.max(list_rect.h))
            .begin(ctx, &mut self.actions_scroll);
        let mut y = list_rect.y + self.actions_scroll.scroll_y;
        let mut selected = None;

        ctx.push_clip_rect(list_rect);
        for action_id in action_ids {
            if area.is_visible(y, ACTION_ROW_HEIGHT) {
                let row_rect = Rect::new(list_rect.x, y, list_rect.w, ACTION_ROW_HEIGHT);
                if Button::new(row_rect, action_id.as_str())
                    .plain()
                    .active(self.state.selected_action() == Some(&action_id))
                    .suppressed(self.capture_request.is_some() || self.action_renamer.is_open())
                    .show(ctx)
                {
                    selected = Some(action_id);
                }
            }
            y += ACTION_ROW_HEIGHT;
        }
        ctx.pop_clip_rect();
        area.draw_scrollbar(ctx, &self.actions_scroll);

        if let Some(action_id) = selected {
            self.state.select_action(&action_id);
        }
    }

    fn draw_selected_action(&mut self, ctx: &mut WgpuContext) {
        let panel_rect = Rect::new(
            self.rect.x + 252.0,
            self.rect.y + 62.0,
            self.rect.w - 272.0,
            self.rect.h - 126.0,
        );
        draw_panel(ctx, panel_rect, with_theme(|theme| theme.background));
        let content_x = panel_rect.x + 12.0;
        let Some(action_id) = self.state.selected_action().cloned() else {
            ctx.draw_text(
                "No actions yet",
                content_x,
                panel_rect.y + 30.0,
                16.0,
                with_theme(|theme| theme.text),
            );
            return;
        };

        if self.action_renamer.is_open() {
            self.action_renamer
                .draw(ctx, panel_rect, content_x, &mut self.state);
        } else {
            let action_label = truncate_to_width(
                ctx,
                action_id.as_str(),
                panel_rect.w - 254.0,
                18.0,
            );
            ctx.draw_text(
                &action_label,
                content_x,
                panel_rect.y + 28.0,
                18.0,
                with_theme(|theme| theme.text),
            );
            let rename_action_rect =
                Rect::new(panel_rect.right() - 242.0, panel_rect.y + 8.0, 72.0, 28.0);
            if Button::new(rename_action_rect, "Rename")
                .suppressed(self.capture_request.is_some())
                .show(ctx)
            {
                self.action_renamer.begin(&action_id);
            }
            let remove_action_rect = Rect::new(
                panel_rect.right() - 162.0,
                panel_rect.y + 8.0,
                150.0,
                28.0,
            );
            if Button::new(remove_action_rect, "Remove Action")
                .suppressed(self.capture_request.is_some())
                .show(ctx)
            {
                self.state.remove_selected_action();
                self.bindings_scroll.scroll_y = 0.0;
                return;
            }
        }

        ctx.draw_text(
            "Binding scope",
            content_x,
            panel_rect.y + 65.0,
            13.0,
            with_theme(|theme| theme.text_muted),
        );
        let current_scope = self.state.scope().label().to_string();
        let scope_options = vec![
            BindingScope::GameDefaults.label().to_string(),
            BindingScope::PerGame.label().to_string(),
        ];
        let scope_rect = Rect::new(content_x, panel_rect.y + 71.0, 200.0, 28.0);
        if let Some(selected) = Dropdown::new(
            self.scope_id,
            scope_rect,
            &current_scope,
            &scope_options,
            |value: &String| value.clone(),
        )
        .suppressed(self.capture_request.is_some() || self.action_renamer.is_open())
        .show(ctx)
        {
            if let Some(scope) = BindingScope::from_label(&selected) {
                self.state.set_scope(scope);
            }
        }

        let is_override = self.state.scope() != BindingScope::GameDefaults;
        let explicit = self.state.has_explicit_override();
        if is_override {
            let reset_rect = Rect::new(
                panel_rect.right() - 148.0,
                panel_rect.y + 71.0,
                136.0,
                28.0,
            );
            if Button::new(reset_rect, "Reset Override")
                .blocked(!explicit)
                .suppressed(self.capture_request.is_some() || self.action_renamer.is_open())
                .show(ctx)
            {
                self.state.reset_override();
            }
        }

        ctx.draw_text(
            "Bindings",
            content_x,
            panel_rect.y + 121.0,
            14.0,
            with_theme(|theme| theme.text),
        );
        let status = if is_override && !explicit {
            "Inherited"
        } else if is_override {
            "Custom override"
        } else {
            "Game default"
        };
        ctx.draw_text(
            status,
            panel_rect.right() - 116.0,
            panel_rect.y + 121.0,
            12.0,
            with_theme(|theme| theme.text_muted),
        );

        let effective = self.state.effective_bindings().unwrap_or(&[]);
        let displayed = if is_override && !explicit {
            effective
        } else {
            self.state.bindings().unwrap_or(&[])
        };
        let list_rect = Rect::new(
            panel_rect.x + 8.0,
            panel_rect.y + 129.0,
            panel_rect.w - 16.0,
            124.0,
        );
        let content_height = displayed.len() as f32 * BINDING_ROW_HEIGHT;
        let area = ScrollableArea::new(list_rect, content_height.max(list_rect.h))
            .begin(ctx, &mut self.bindings_scroll);
        let mut binding_to_remove = None;
        let can_remove = !is_override || explicit;
        let mut row_y = list_rect.y + self.bindings_scroll.scroll_y;

        ctx.push_clip_rect(list_rect);
        if displayed.is_empty() && !is_override {
            ctx.draw_text(
                "No default bindings",
                list_rect.x + 8.0,
                row_y + 18.0,
                14.0,
                with_theme(|theme| theme.text),
            );
        } else if displayed.is_empty() && explicit {
            ctx.draw_text(
                "Explicitly unbound",
                list_rect.x + 8.0,
                row_y + 18.0,
                14.0,
                with_theme(|theme| theme.text),
            );
        } else if displayed.is_empty() {
            ctx.draw_text(
                "No bindings",
                list_rect.x + 8.0,
                row_y + 18.0,
                14.0,
                with_theme(|theme| theme.text),
            );
        } else {
            for (index, binding) in displayed.iter().enumerate() {
                if area.is_visible(row_y, BINDING_ROW_HEIGHT) {
                    let label = binding_label(binding.source);
                    ctx.draw_text(
                        &label,
                        list_rect.x + 8.0,
                        row_y + 19.0,
                        14.0,
                        with_theme(|theme| theme.text),
                    );
                    let rebind_rect = Rect::new(list_rect.right() - 152.0, row_y + 2.0, 72.0, 26.0);
                    if Button::new(rebind_rect, "Rebind")
                        .plain()
                        .suppressed(self.capture_request.is_some() || self.action_renamer.is_open())
                        .show(ctx)
                    {
                        self.capture_request = Some(CaptureRequest {
                            mode: capture_mode_for(binding.source),
                            replacement_index: Some(index),
                        });
                    }
                    let remove_rect = Rect::new(list_rect.right() - 72.0, row_y + 2.0, 64.0, 26.0);
                    if Button::new(remove_rect, "Remove")
                        .plain()
                        .blocked(!can_remove)
                        .suppressed(self.capture_request.is_some() || self.action_renamer.is_open())
                        .show(ctx)
                    {
                        binding_to_remove = Some(index);
                    }
                }
                row_y += BINDING_ROW_HEIGHT;
            }
        }
        ctx.pop_clip_rect();
        area.draw_scrollbar(ctx, &self.bindings_scroll);

        if let Some(index) = binding_to_remove {
            self.state.remove_binding(index);
        }

        let capture_buttons = [
            ("Add Key", CaptureMode::Key),
            ("Add Button", CaptureMode::GamepadButton),
            ("Add Axis", CaptureMode::GamepadAxis),
        ];
        let buttons_y = panel_rect.y + 277.0;
        let button_width = (panel_rect.w - 40.0) / 3.0;
        for (index, (label, mode)) in capture_buttons.into_iter().enumerate() {
            let button_rect = Rect::new(
                content_x + index as f32 * (button_width + 8.0),
                buttons_y,
                button_width,
                28.0,
            );
            if Button::new(button_rect, label)
                .suppressed(self.capture_request.is_some() || self.action_renamer.is_open())
                .show(ctx)
            {
                self.capture_request = Some(CaptureRequest {
                    mode,
                    replacement_index: None,
                });
            }
        }

        if let Some(request) = self.capture_request {
            let message = match request.mode {
                CaptureMode::Key => "Press a key to bind; Escape cancels",
                CaptureMode::GamepadButton => "Press a gamepad button; Escape cancels",
                CaptureMode::GamepadAxis => "Move a gamepad axis past 0.5; Escape cancels",
            };
            ctx.draw_text(
                message,
                content_x,
                panel_rect.y + 267.0,
                12.0,
                with_theme(|theme| theme.text_muted),
            );
        }
    }

    fn draw_footer(&mut self, ctx: &mut WgpuContext) -> Option<InputBindingsPromptResult> {
        let y = self.rect.bottom() - 44.0;
        let save_rect = Rect::new(self.rect.right() - 204.0, y, 84.0, 28.0);
        let cancel_rect = Rect::new(self.rect.right() - 108.0, y, 84.0, 28.0);
        if Button::new(save_rect, "Save")
            .suppressed(self.action_renamer.is_open())
            .show(ctx)
        {
            return self
                .state
                .finish(true)
                .map(InputBindingsPromptResult::Save);
        }
        if Button::new(cancel_rect, "Cancel")
            .suppressed(self.action_renamer.is_open())
            .show(ctx)
        {
            return Some(InputBindingsPromptResult::Cancel);
        }
        None
    }
}

fn draw_panel(ctx: &mut WgpuContext, rect: Rect, fill: Color) {
    ctx.draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    ctx.draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        with_theme(|theme| theme.border),
    );
}
