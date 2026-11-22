use std::fmt::{self, Display, Formatter};

use bishop::text::FontId;
use engine_core::assets::{AssetKey, AssetRegistry};
use widgets::{Dropdown, WidgetId};

pub(crate) const DEFAULT_FONT_LABEL: &str = "Default";

#[derive(Clone, PartialEq)]
pub(crate) struct FontOption {
    pub(crate) id: Option<FontId>,
    pub(crate) label: String,
}

impl Display for FontOption {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub(crate) fn font_options(
    registry: &AssetRegistry,
    selected: Option<FontId>,
) -> (Vec<FontOption>, FontOption) {
    let mut options = vec![FontOption {
        id: None,
        label: DEFAULT_FONT_LABEL.to_owned(),
    }];
    for (key, record) in registry.records() {
        if let AssetKey::Font(id) = key {
            let name = record
                .path
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("Font {}", id.0));
            options.push(FontOption {
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
            let missing = FontOption {
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

pub(crate) fn draw_font_dropdown(
    ctx: &mut bishop::WgpuContext,
    widget_id: WidgetId,
    rect: bishop::Rect,
    registry: &AssetRegistry,
    selected: Option<FontId>,
    blocked: bool,
) -> Option<Option<FontId>> {
    let (options, current) = font_options(registry, selected);
    Dropdown::new(
        widget_id,
        rect,
        &current.label,
        &options,
        |option| option.label.clone(),
    )
    .suppressed(blocked)
    .fixed_width()
    .show(ctx)
    .map(|option| option.id)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use bishop::text::FontId;
    use engine_core::assets::AssetRegistry;
    use engine_core::constants::extensions;

    use super::{DEFAULT_FONT_LABEL, font_options};

    #[test]
    fn font_options_use_the_default_label_and_preserve_missing_ids() {
        let id = FontId(99);
        let (options, selected) = font_options(&AssetRegistry::default(), Some(id));

        assert_eq!(options[0].id, None);
        assert_eq!(options[0].label, DEFAULT_FONT_LABEL);
        assert_eq!(selected.id, Some(id));
        assert_eq!(selected.label, format!("Missing Font {}", id.0));
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

        let (options, selected) = font_options(&registry, Some(id));
        assert_eq!(selected.id, Some(id));
        assert_eq!(selected.label, format!("heading ({})", id.0));
        assert!(options.iter().any(|option| option.id == Some(id)));
        Ok(())
    }
}
