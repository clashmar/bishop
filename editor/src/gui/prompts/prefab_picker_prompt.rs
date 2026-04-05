use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::confirm_cancel_rects;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefabPickerResult {
    Existing(PrefabId),
    New,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PrefabChoice {
    prefab_id: PrefabId,
    label: String,
}

impl Display for PrefabChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.label.fmt(f)
    }
}

pub struct PrefabPickerPrompt {
    dropdown_id: WidgetId,
    rect: Rect,
    prefabs: Vec<PrefabChoice>,
    selected: Option<PrefabId>,
}

impl PrefabPickerPrompt {
    pub fn new(modal_rect: Rect, prefabs: Vec<PrefabAsset>) -> Self {
        const TOP_PADDING: f32 = 12.0;
        const LABEL_H: f32 = 20.0;
        const GAP: f32 = 12.0;
        const ACTION_GAP: f32 = 14.0;
        const BOTTOM_PADDING: f32 = 16.0;
        let inner_w = modal_rect.w * 0.82;
        let inner_x = modal_rect.x + (modal_rect.w - inner_w) / 2.0;
        let total_h = TOP_PADDING
            + LABEL_H
            + GAP
            + FIELD_H
            + GAP
            + BUTTON_H
            + ACTION_GAP
            + BUTTON_H
            + BOTTOM_PADDING;
        let inner_y = modal_rect.y + (modal_rect.h - total_h) / 2.0;

        Self {
            dropdown_id: WidgetId::default(),
            rect: Rect::new(inner_x, inner_y, inner_w, total_h),
            prefabs: prefabs
                .into_iter()
                .map(|prefab| PrefabChoice {
                    prefab_id: prefab.id,
                    label: format!("{} ({})", prefab.name, prefab.id),
                })
                .collect(),
            selected: None,
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<PrefabPickerResult> {
        const TOP_PADDING: f32 = 12.0;
        const LABEL_H: f32 = 20.0;
        const GAP: f32 = 12.0;
        const ACTION_GAP: f32 = 14.0;

        ctx.draw_text(
            "Open prefab:",
            self.rect.x,
            self.rect.y + TOP_PADDING,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        let dropdown_rect = Rect::new(
            self.rect.x,
            self.rect.y + TOP_PADDING + LABEL_H + GAP,
            self.rect.w,
            FIELD_H,
        );
        let selected_label = self
            .selected
            .and_then(|prefab_id| {
                self.prefabs
                    .iter()
                    .find(|choice| choice.prefab_id == prefab_id)
                    .map(|choice| choice.label.clone())
            })
            .unwrap_or_else(|| "Select prefab".to_string());

        if let Some(choice) = Dropdown::new(
            self.dropdown_id,
            dropdown_rect,
            &selected_label,
            &self.prefabs,
            |choice| choice.to_string(),
        )
        .filterable()
        .menu_style()
        .show(ctx)
        {
            self.selected = Some(choice.prefab_id);
        }

        let new_rect = Rect::new(
            self.rect.x,
            dropdown_rect.y + dropdown_rect.h + GAP,
            self.rect.w,
            BUTTON_H,
        );
        let btn_y = new_rect.y + new_rect.h + ACTION_GAP;
        let (open_rect, cancel_rect) = confirm_cancel_rects(self.rect, btn_y);

        let open_clicked = Button::new(open_rect, "Open")
            .blocked(self.selected.is_none())
            .show(ctx);
        let new_clicked = Button::new(new_rect, "New Prefab").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        if new_clicked {
            return Some(PrefabPickerResult::New);
        }

        if open_clicked {
            return self.selected.map(PrefabPickerResult::Existing);
        }

        if cancel_clicked || Controls::escape(ctx) {
            return Some(PrefabPickerResult::Cancelled);
        }

        None
    }
}
