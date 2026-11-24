use super::state::{BindingScope, InputBindingsPromptState};
use super::InputBindingsPromptResult;
use crate::app::control::escape::modal_escape_requested;
use bishop::input::{GamepadAxis, GamepadButton, Input, KeyCode};
use bishop::prelude::{Color, Rect, WgpuContext};
use bishop::Text;
use engine_core::input::{
    AxisDirection, InputActionMap, InputBinding, InputSource,
};
use engine_core::storage::InputSettings;
use uuid::Uuid;
use widgets::{
    text_input_reset, Button, Dropdown, ScrollState, ScrollableArea, TextInput, Widget, WidgetId,
};

const ACTION_ROW_HEIGHT: f32 = 30.0;
const BINDING_ROW_HEIGHT: f32 = 30.0;
const AXIS_CAPTURE_THRESHOLD: f32 = 0.5;
const GAMEPAD_BUTTONS: [GamepadButton; 16] = [
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::North,
    GamepadButton::West,
    GamepadButton::LeftShoulder,
    GamepadButton::RightShoulder,
    GamepadButton::LeftTrigger,
    GamepadButton::RightTrigger,
    GamepadButton::Select,
    GamepadButton::Start,
    GamepadButton::LeftStick,
    GamepadButton::RightStick,
    GamepadButton::DPadUp,
    GamepadButton::DPadDown,
    GamepadButton::DPadLeft,
    GamepadButton::DPadRight,
];
const GAMEPAD_AXES: [GamepadAxis; 6] = [
    GamepadAxis::LeftX,
    GamepadAxis::LeftY,
    GamepadAxis::RightX,
    GamepadAxis::RightY,
    GamepadAxis::LeftTrigger,
    GamepadAxis::RightTrigger,
];

#[derive(Clone, Copy)]
enum CaptureMode {
    Key,
    GamepadButton,
    GamepadAxis,
}

pub(crate) struct InputBindingsPrompt {
    rect: Rect,
    state: InputBindingsPromptState,
    action_name: String,
    capture_mode: Option<CaptureMode>,
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
            capture_mode: None,
            scope_id: WidgetId::default(),
            action_name_id: WidgetId::default(),
            actions_scroll: ScrollState::new(),
            bindings_scroll: ScrollState::new(),
        }
    }

    pub(crate) fn draw(&mut self, ctx: &mut WgpuContext) -> Option<InputBindingsPromptResult> {
        let escape_pressed = modal_escape_requested();
        if let Some(mode) = self.capture_mode {
            if escape_pressed {
                self.capture_mode = None;
            } else if let Some(binding) = capture_binding(ctx, mode) {
                self.state.add_binding(binding);
                self.capture_mode = None;
            }
        } else if escape_pressed {
            return Some(InputBindingsPromptResult::Cancel);
        }

        let title_x = self.rect.x + 24.0;
        ctx.draw_text("Input Bindings", title_x, self.rect.y + 34.0, 20.0, Color::WHITE);
        self.draw_scope_and_action_controls(ctx);
        self.draw_action_list(ctx);
        self.draw_selected_action(ctx);
        self.draw_footer(ctx)
    }

    fn draw_scope_and_action_controls(&mut self, ctx: &mut WgpuContext) {
        let x = self.rect.x + 24.0;
        let y = self.rect.y + 52.0;
        let current_scope = self.state.scope().label().to_string();
        let scope_options = vec![
            BindingScope::GameDefaults.label().to_string(),
            BindingScope::Global.label().to_string(),
            BindingScope::PerGame.label().to_string(),
        ];
        let scope_rect = Rect::new(x, y, 190.0, 30.0);
        if let Some(selected) = Dropdown::new(
            self.scope_id,
            scope_rect,
            &current_scope,
            &scope_options,
            |value: &String| value.clone(),
        )
        .show(ctx)
        {
            if let Some(scope) = BindingScope::from_label(&selected) {
                self.state.set_scope(scope);
            }
        }

        let action_x = scope_rect.right() + 12.0;
        let name_rect = Rect::new(action_x, y, 240.0, 30.0);
        let (name, _) = TextInput::new(self.action_name_id, name_rect, &self.action_name).show(ctx);
        self.action_name = name;
        let add_rect = Rect::new(name_rect.right() + 8.0, y, 92.0, 30.0);
        if Button::new(add_rect, "Add Action")
            .blocked(self.action_name.trim().is_empty())
            .show(ctx)
        {
            if self.state.add_action(&self.action_name) {
                self.action_name.clear();
                text_input_reset(self.action_name_id);
            }
        }
    }

    fn draw_action_list(&mut self, ctx: &mut WgpuContext) {
        let list_rect = Rect::new(
            self.rect.x + 20.0,
            self.rect.y + 96.0,
            212.0,
            self.rect.h - 158.0,
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
                let row_rect = Rect::new(list_rect.x, y, list_rect.w - 12.0, ACTION_ROW_HEIGHT);
                if Button::new(row_rect, action_id.as_str())
                    .active(self.state.selected_action() == Some(&action_id))
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
        let x = self.rect.x + 252.0;
        let y = self.rect.y + 98.0;
        let width = self.rect.w - 276.0;
        let Some(action_id) = self.state.selected_action().cloned() else {
            ctx.draw_text("Add an action to begin", x, y + 18.0, 16.0, Color::WHITE);
            return;
        };

        ctx.draw_text(action_id.as_str(), x, y + 18.0, 16.0, Color::WHITE);
        let remove_action_rect = Rect::new(x + width - 112.0, y, 112.0, 26.0);
        if Button::new(remove_action_rect, "Remove Action").show(ctx) {
            self.state.remove_selected_action();
            self.bindings_scroll.scroll_y = 0.0;
            return;
        }
        ctx.draw_text(self.state.scope().label(), x, y + 42.0, 14.0, Color::WHITE);
        let effective = self.state.effective_bindings().unwrap_or(&[]);
        let is_override = self.state.scope() != BindingScope::GameDefaults;
        let explicit = self.state.has_explicit_override();
        let displayed = if is_override && !explicit {
            effective
        } else {
            self.state.bindings().unwrap_or(&[])
        };
        let list_label = if is_override && !explicit {
            "Inherited bindings"
        } else {
            "Bindings in this scope"
        };
        ctx.draw_text(list_label, x, y + 68.0, 14.0, Color::WHITE);

        let list_rect = Rect::new(x, y + 80.0, width, self.rect.h - 320.0);
        let content_height = displayed.len() as f32 * BINDING_ROW_HEIGHT;
        let area = ScrollableArea::new(list_rect, content_height.max(list_rect.h))
            .begin(ctx, &mut self.bindings_scroll);
        let mut binding_to_remove = None;
        let can_remove = !is_override || explicit;
        let mut row_y = list_rect.y + self.bindings_scroll.scroll_y;

        ctx.push_clip_rect(list_rect);
        if displayed.is_empty() && !is_override {
            ctx.draw_text("Unbound", x, row_y + 18.0, 14.0, Color::WHITE);
        } else if displayed.is_empty() && explicit {
            ctx.draw_text("Explicitly unbound", x, row_y + 18.0, 14.0, Color::WHITE);
        } else if displayed.is_empty() {
            ctx.draw_text("No bindings", x, row_y + 18.0, 14.0, Color::WHITE);
        } else {
            for (index, binding) in displayed.iter().enumerate() {
                if area.is_visible(row_y, BINDING_ROW_HEIGHT) {
                    let binding_label = format!("{:?}", binding.source);
                    ctx.draw_text(&binding_label, x, row_y + 18.0, 14.0, Color::WHITE);
                    let remove_rect = Rect::new(x + width - 72.0, row_y, 68.0, 26.0);
                    if Button::new(remove_rect, "Remove")
                        .blocked(!can_remove)
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

        let buttons_y = list_rect.bottom() + 8.0;
        let capture_buttons = [
            ("Capture Key", CaptureMode::Key),
            ("Capture Button", CaptureMode::GamepadButton),
            ("Capture Axis", CaptureMode::GamepadAxis),
        ];
        let button_width = (width - 16.0) / 3.0;
        for (index, (label, mode)) in capture_buttons.into_iter().enumerate() {
            let button_rect = Rect::new(
                x + index as f32 * (button_width + 8.0),
                buttons_y,
                button_width,
                28.0,
            );
            if Button::new(button_rect, label).show(ctx) {
                self.capture_mode = Some(mode);
            }
        }

        if is_override {
            let reset_rect = Rect::new(x + width - 112.0, y + 42.0, 112.0, 26.0);
            if Button::new(reset_rect, "Reset Override")
                .blocked(!explicit)
                .show(ctx)
            {
                self.state.reset_override();
            }
        }

        if let Some(mode) = self.capture_mode {
            let message = match mode {
                CaptureMode::Key => "Press a key to bind; Escape cancels capture",
                CaptureMode::GamepadButton => "Press a gamepad button; Escape cancels capture",
                CaptureMode::GamepadAxis => "Move a gamepad axis past 0.5; Escape cancels capture",
            };
            ctx.draw_text(message, x, buttons_y + 46.0, 13.0, Color::WHITE);
        }
    }

    fn draw_footer(&mut self, ctx: &mut WgpuContext) -> Option<InputBindingsPromptResult> {
        let y = self.rect.bottom() - 44.0;
        let save_rect = Rect::new(self.rect.right() - 204.0, y, 84.0, 28.0);
        let cancel_rect = Rect::new(self.rect.right() - 108.0, y, 84.0, 28.0);
        if Button::new(save_rect, "Save").show(ctx) {
            return self
                .state
                .finish(true)
                .map(InputBindingsPromptResult::Save);
        }
        if Button::new(cancel_rect, "Cancel").show(ctx) {
            return Some(InputBindingsPromptResult::Cancel);
        }
        None
    }
}

fn capture_binding(ctx: &WgpuContext, mode: CaptureMode) -> Option<InputBinding> {
    match mode {
        CaptureMode::Key => ctx.pressed_keys().into_iter().find_map(key_capture_binding),
        CaptureMode::GamepadButton => GAMEPAD_BUTTONS
            .into_iter()
            .find(|button| ctx.gamepad_state().button_pressed(*button))
            .map(gamepad_button_capture_binding),
        CaptureMode::GamepadAxis => GAMEPAD_AXES
            .into_iter()
            .filter_map(|axis| {
                let value = ctx.gamepad_state().axis_value(axis);
                axis_capture_binding(axis, value).map(|binding| (binding, value))
            })
            .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
            .map(|(binding, _)| binding),
    }
}

pub(crate) fn key_capture_binding(key: KeyCode) -> Option<InputBinding> {
    (key != KeyCode::Escape).then(|| InputBinding::keyboard(key))
}

pub(crate) fn gamepad_button_capture_binding(button: GamepadButton) -> InputBinding {
    InputBinding {
        source: InputSource::GamepadButton(button),
    }
}

pub(crate) fn axis_capture_binding(axis: GamepadAxis, value: f32) -> Option<InputBinding> {
    if !value.is_finite() || value.abs() < AXIS_CAPTURE_THRESHOLD {
        return None;
    }
    Some(InputBinding {
        source: InputSource::GamepadAxis {
            axis,
            direction: if value > 0.0 {
                AxisDirection::Positive
            } else {
                AxisDirection::Negative
            },
            threshold: AXIS_CAPTURE_THRESHOLD,
        },
    })
}
