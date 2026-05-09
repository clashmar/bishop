use crate::app::escape::modal_escape_requested;
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::{draw_prompt_label, prompt_content_rect, three_button_rects};
use crate::prefab::prefab_editor::actions::pick_initial_prefab_save_path;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::PathBuf;
use widgets::constants::layout;

#[derive(Clone, Debug, PartialEq)]
pub enum PrefabPickerResult {
    Existing(PrefabAsset),
    New(PathBuf),
    File(PathBuf),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq)]
struct PrefabChoice {
    prefab: PrefabAsset,
    label: String,
}

impl Display for PrefabChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.label.fmt(f)
    }
}

pub struct PrefabPickerPrompt {
    dropdown_id: WidgetId,
    file_button_id: WidgetId,
    new_button_id: WidgetId,
    rect: Rect,
    prefabs: Vec<PrefabChoice>,
    selected: Option<PrefabId>,
    exit_instead_of_cancel: bool,
}

impl PrefabPickerPrompt {
    pub fn new(
        modal_rect: Rect,
        prefabs: Vec<PrefabAsset>,
        excluded_prefab_id: Option<PrefabId>,
        exit_instead_of_cancel: bool,
    ) -> Self {
        let total_h = PROMPT_TOP_PADDING
            + layout::DEFAULT_FONT_SIZE_16
            + PROMPT_TEXT_GAP
            + FIELD_H
            + PROMPT_SECTION_GAP
            + BUTTON_H
            + PROMPT_ACTION_GAP
            + BUTTON_H
            + PROMPT_BOTTOM_PADDING;

        Self {
            dropdown_id: WidgetId::default(),
            file_button_id: WidgetId::default(),
            new_button_id: WidgetId::default(),
            rect: prompt_content_rect(modal_rect, total_h),
            prefabs: prefabs
                .into_iter()
                .filter(|prefab| Some(prefab.id) != excluded_prefab_id)
                .map(|prefab| PrefabChoice {
                    label: prefab.name.clone(),
                    prefab,
                })
                .collect(),
            selected: None,
            exit_instead_of_cancel,
        }
    }

    fn secondary_action_label(&self) -> &'static str {
        if self.exit_instead_of_cancel {
            "Exit"
        } else {
            "Cancel"
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<PrefabPickerResult> {
        draw_prompt_label(
            ctx,
            "Open prefab:",
            self.rect.x,
            self.rect.y + PROMPT_TOP_PADDING,
        );

        let dropdown_rect = Rect::new(
            self.rect.x,
            self.rect.y + PROMPT_TOP_PADDING + layout::DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
            self.rect.w,
            FIELD_H,
        );
        let selected_label = self
            .selected
            .and_then(|prefab_id| {
                self.prefabs
                    .iter()
                    .find(|choice| choice.prefab.id == prefab_id)
                    .map(|choice| choice.label.clone())
            })
            .unwrap_or_else(|| "Select Prefab".to_string());

        let new_rect = Rect::new(
            self.rect.x,
            dropdown_rect.y + dropdown_rect.h + PROMPT_SECTION_GAP,
            self.rect.w,
            BUTTON_H,
        );
        let btn_y = new_rect.y + new_rect.h + PROMPT_ACTION_GAP;
        let (open_rect, file_rect, cancel_rect) = three_button_rects(self.rect, btn_y);

        let open_clicked = Button::new(open_rect, "Open")
            .blocked(self.selected.is_none())
            .show(ctx);
        let file_clicked = Button::new(file_rect, "File")
            .interaction_id(self.file_button_id)
            .show_native_dialog(ctx);
        let new_clicked = Button::new(new_rect, "New Prefab")
            .interaction_id(self.new_button_id)
            .show_native_dialog(ctx);
        let cancel_clicked = Button::new(cancel_rect, self.secondary_action_label()).show(ctx);

        if let Some(choice) = Dropdown::new(
            self.dropdown_id,
            dropdown_rect,
            &selected_label,
            &self.prefabs,
            |choice| choice.to_string(),
        )
        .filterable()
        .list_width(dropdown_rect.w)
        .truncate_trigger_text()
        .show(ctx)
        {
            self.selected = Some(choice.prefab.id);
        }

        if new_clicked {
            if let Some(path) = pick_initial_prefab_save_path("Prefab") {
                return Some(PrefabPickerResult::New(path));
            }
        }

        if open_clicked {
            return self.selected.and_then(|prefab_id| {
                self.prefabs
                    .iter()
                    .find(|choice| choice.prefab.id == prefab_id)
                    .map(|choice| PrefabPickerResult::Existing(choice.prefab.clone()))
            });
        }

        if file_clicked {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Prefab RON", &["ron"])
                .set_directory(prefabs_folder())
                .pick_file()
            {
                return Some(PrefabPickerResult::File(path));
            }
        }

        if cancel_clicked || (!self.exit_instead_of_cancel && modal_escape_requested()) {
            return Some(PrefabPickerResult::Cancelled);
        }

        None
    }
}
