use crate::menu::menu_canvas::drawing::MenuCanvasFrame;
use crate::menu::menu_properties_panel::common_properties::row_visible;
use crate::menu::menu_properties_panel::{FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::MenuEditor;
use bishop::prelude::*;
use bishop::text::FontId;
use engine_core::assets::{AssetKey, AssetRegistry};
use engine_core::menu::{HorizontalAlign, MenuElement, MenuElementKind};
use std::fmt::{self, Display, Formatter};
use widgets::*;

impl MenuEditor {
    pub(crate) fn draw_label(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        element: &MenuElement,
        element_rect: Rect,
        is_selected: bool,
    ) {
        let MenuElementKind::Label(label) = &element.kind else {
            return;
        };
        if !frame.preview {
            let outline_color = if is_selected {
                with_theme(|theme| theme.highlight)
            } else {
                Color::new(0., 0., 0., 0.)
            };
            let thickness = if is_selected { 2.0 } else { 1.0 };
            frame.ctx.draw_rectangle_lines(
                element_rect.x,
                element_rect.y,
                element_rect.w,
                element_rect.h,
                thickness,
                outline_color,
            );
        }
        let font = label.font.and_then(|id| {
            frame.font_manager.ensure_font(
                frame.ctx,
                frame.asset_registry,
                frame.resource_source,
                id,
            )
        });
        let label_align = match label.alignment {
            HorizontalAlign::Left => LabelAlign::Left,
            HorizontalAlign::Center => LabelAlign::Center,
            HorizontalAlign::Right => LabelAlign::Right,
        };
        Label::new(element_rect, &label.text_key)
            .font_size(label.font_size)
            .font(font)
            .alignment(label_align)
            .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
            .show(frame.ctx);
    }

    pub(crate) fn draw_label_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
        asset_registry: &AssetRegistry,
    ) {
        let (current_text_key, current_font_size, current_font, current_alignment) = {
            let Some(element) = self.selected_element() else {
                return;
            };
            let MenuElementKind::Label(label) = &element.kind else {
                return;
            };
            (
                label.text_key.clone(),
                label.font_size,
                label.font,
                label.alignment,
            )
        };

        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Text Key:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_text_key, commit) = TextInput::new(
                self.properties_panel.widget_ids.text_id,
                field_rect,
                &current_text_key,
            )
            .blocked(blocked)
            .show(ctx);
            self.push_input_update(commit, |el| {
                if let MenuElementKind::Label(label) = &mut el.kind {
                    label.text_key = new_text_key;
                }
            });
        }
        *y += ROW_HEIGHT;

        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Font Size:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
            let (new_font_size, commit) = NumberInput::new(
                self.properties_panel.widget_ids.font_size_id,
                field_rect,
                current_font_size,
            )
            .blocked(blocked)
            .min(8.0)
            .max(72.0)
            .show(ctx);
            self.push_input_update(commit, |el| {
                if let MenuElementKind::Label(label) = &mut el.kind {
                    label.font_size = new_font_size;
                }
            });
        }
        *y += ROW_HEIGHT;

        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Font:", x, *y + 16.0, 12.0, Color::WHITE);
            let (font_options, current_font_option) =
                label_font_options(asset_registry, current_font);
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.label_font_id,
                dropdown_rect,
                &current_font_option.label,
                &font_options,
                |option| option.label.clone(),
            )
            .suppressed(blocked)
            .fixed_width()
            .show(ctx)
            {
                self.push_element_update(|element| {
                    if let MenuElementKind::Label(label) = &mut element.kind {
                        label.font = selected.id;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Align:", x, *y + 16.0, 12.0, Color::WHITE);
            let h_options = ["Left", "Center", "Right"];
            let current_h = match current_alignment {
                HorizontalAlign::Left => "Left",
                HorizontalAlign::Center => "Center",
                HorizontalAlign::Right => "Right",
            };
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.label_h_align_id,
                dropdown_rect,
                current_h,
                &h_options,
                |s| s.to_string(),
            )
            .suppressed(blocked)
            .fixed_width()
            .show(ctx)
            {
                let new_align = match selected {
                    "Left" => HorizontalAlign::Left,
                    "Center" => HorizontalAlign::Center,
                    "Right" => HorizontalAlign::Right,
                    _ => current_alignment,
                };
                self.push_element_update(|el| {
                    if let MenuElementKind::Label(label) = &mut el.kind {
                        label.alignment = new_align;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;
    }
}

#[derive(Clone, PartialEq)]
struct LabelFontOption {
    id: Option<FontId>,
    label: String,
}

impl Display for LabelFontOption {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

fn label_font_options(
    registry: &AssetRegistry,
    selected: Option<FontId>,
) -> (Vec<LabelFontOption>, LabelFontOption) {
    let mut options = vec![LabelFontOption {
        id: None,
        label: "Engine Default".to_owned(),
    }];
    for (key, record) in registry.records() {
        if let AssetKey::Font(id) = key {
            let name = record
                .path
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("Font {}", id.0));
            options.push(LabelFontOption {
                id: Some(*id),
                label: format!("{name} ({})", id.0),
            });
        }
    }
    options[1..].sort_by(|left, right| left.label.cmp(&right.label));

    let mut current = selected
        .and_then(|id| options.iter().find(|option| option.id == Some(id)).cloned());
    if current.is_none() {
        if let Some(id) = selected {
            let missing = LabelFontOption {
                id: Some(id),
                label: format!("Missing Font {}", id.0),
            };
            options.push(missing.clone());
            current = Some(missing);
        }
    }
    let current = current.unwrap_or_else(|| options[0].clone());
    (options, current)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use bishop::text::FontId;
    use engine_core::assets::AssetRegistry;
    use engine_core::constants::extensions;

    use super::label_font_options;

    #[test]
    fn font_options_keep_the_engine_default_and_preserve_missing_font_ids() -> Result<(), String> {
        let id = FontId(99);
        let (options, selected) = label_font_options(&AssetRegistry::default(), Some(id));

        assert_eq!(options[0].id, None);
        assert_eq!(options[0].label, "Engine Default");
        assert_eq!(selected.id, Some(id));
        assert_eq!(selected.label, "Missing Font 99");
        Ok(())
    }

    #[test]
    fn font_options_use_registered_asset_names_and_ids() -> Result<(), String> {
        let id = FontId(13);
        let mut registry = AssetRegistry::default();
        registry
            .register_asset_relative_path(
                id,
                PathBuf::from(format!("heading.{}", extensions::TTF)),
            )
            .map_err(|error| error.to_string())?;

        let (options, selected) = label_font_options(&registry, Some(id));
        assert_eq!(selected.id, Some(id));
        assert_eq!(selected.label, "heading (13)");
        assert!(options.iter().any(|option| option.id == Some(id)));
        Ok(())
    }
}
