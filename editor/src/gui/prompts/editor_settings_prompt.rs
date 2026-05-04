use crate::app::escape::modal_escape_requested;
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::theme::preset::all_presets;
use engine_core::theme::{set_theme, with_theme, Theme};
use widgets::constants::layout;

pub struct EditorSettingsResult {
    pub preset_name: Option<String>,
    pub snapshot_theme: Theme,
    pub confirmed: bool,
}

pub struct EditorSettingsPrompt {
    rect: Rect,
    dropdown_id: WidgetId,
    selected_index: usize,
    selected_preset_name: Option<String>,
    snapshot_theme: Theme,
}

impl EditorSettingsPrompt {
    pub fn new(modal_rect: Rect, dropdown_id: WidgetId) -> Self {
        let presets = all_presets();
        let current_theme = with_theme(|t| *t);
        let (selected_index, selected_preset_name) = presets
            .iter()
            .position(|p| (p.build)() == current_theme)
            .map(|i| (i, Some(presets[i].name.to_string())))
            .unwrap_or((0, None));

        let total_h = PROMPT_TOP_PADDING
            + layout::DEFAULT_FONT_SIZE_16
            + PROMPT_TEXT_GAP
            + FIELD_H
            + PROMPT_SECTION_GAP
            + BUTTON_H
            + PROMPT_BOTTOM_PADDING;
        let rect = prompt_content_rect(modal_rect, total_h);

        Self {
            rect,
            dropdown_id,
            selected_index,
            selected_preset_name,
            snapshot_theme: current_theme,
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<EditorSettingsResult> {
        let presets = all_presets();
        let preset_names: Vec<String> = presets.iter().map(|p| p.name.to_string()).collect();

        let mut y = self.rect.y + PROMPT_TOP_PADDING;

        let label_dims = draw_prompt_label(ctx, "Theme:", self.rect.x, y);
        y += label_dims.height + PROMPT_TEXT_GAP;

        let dropdown_rect = Rect::new(self.rect.x, y, self.rect.w, FIELD_H);
        if let Some(selected_name) = Dropdown::new(
            self.dropdown_id,
            dropdown_rect,
            &preset_names[self.selected_index],
            &preset_names,
            |s: &String| s.clone(),
        )
        .show(ctx)
        {
            if let Some(idx) = preset_names.iter().position(|n| n == &selected_name) {
                self.selected_index = idx;
                self.selected_preset_name = Some(selected_name.clone());
                set_theme((presets[idx].build)());
            }
        }

        y += dropdown_rect.h + PROMPT_SECTION_GAP;

        let (confirm_rect, cancel_rect) = confirm_cancel_rects(self.rect, y);
        let confirm_clicked = Button::new(confirm_rect, "OK").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        if confirm_clicked || Controls::enter(ctx) {
            return Some(EditorSettingsResult {
                preset_name: self.selected_preset_name.clone(),
                snapshot_theme: self.snapshot_theme,
                confirmed: true,
            });
        }

        if cancel_clicked || modal_escape_requested() {
            return Some(EditorSettingsResult {
                preset_name: None,
                snapshot_theme: self.snapshot_theme,
                confirmed: false,
            });
        }

        None
    }
}
