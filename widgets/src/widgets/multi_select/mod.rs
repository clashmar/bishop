use crate::constants::layout::{DEFAULT_FONT_SIZE_16, WIDGET_SPACING};
use crate::widgets::dropdown::CreateNewFn;
use crate::*;
use std::fmt::Display;

/// Result of a MultiSelect interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiSelectDelta<T> {
    /// Items that were added this frame.
    pub added: Vec<T>,
    /// Items that were removed this frame.
    pub removed: Vec<T>,
}

/// A multi-select widget that wraps Dropdown for picking items and shows
/// selected items as removable chip buttons below.
pub struct MultiSelect<'a, T> {
    id: WidgetId,
    rect: Rect,
    label: &'a str,
    options: &'a [T],
    selected: &'a mut Vec<T>,
    to_string: Box<dyn Fn(&T) -> String + 'a>,
    filterable: bool,
    chip_height: f32,
    chip_padding: f32,
    create_new: Option<CreateNewFn<'a, T>>,
    base: WidgetBase,
}

impl<'a, T: Clone + PartialEq + Display + 'static> MultiSelect<'a, T> {
    /// Creates a new multi-select widget.
    pub fn new(
        id: WidgetId,
        rect: impl Into<Rect>,
        label: &'a str,
        options: &'a [T],
        selected: &'a mut Vec<T>,
        to_string: impl Fn(&T) -> String + 'a,
    ) -> Self {
        Self {
            id,
            rect: rect.into(),
            label,
            options,
            selected,
            to_string: Box::new(to_string),
            filterable: true,
            chip_height: 20.0,
            chip_padding: 4.0,
            create_new: None,
            base: WidgetBase::default(),
        }
    }

    /// Shows the multi-select widget and returns any changes.
    pub fn show<C: BishopContext>(self, ctx: &mut C) -> Option<MultiSelectDelta<T>> {
        let mut added = Vec::new();
        let mut removed = Vec::new();

        let available: Vec<T> = self
            .options
            .iter()
            .filter(|option| !self.selected.contains(option))
            .cloned()
            .collect();

        let dropdown_rect = Rect::new(self.rect.x, self.rect.y, self.rect.w, self.rect.h);
        let create_new = self.create_new;
        let mut d = Dropdown::new(
            self.id,
            dropdown_rect,
            self.label,
            &available,
            &*self.to_string,
        )
        .filterable()
        .overrides(self.base.overrides)
        .blocked(self.base.blocked);
        if let Some(f) = create_new {
            d = d.create_new(f);
        }
        if let Some(picked) = d.show(ctx)
            && !self.selected.contains(&picked)
        {
            self.selected.push(picked.clone());
            added.push(picked);
        }

        // Chip area
        if !self.selected.is_empty() {
            let chip_pad_x = 8.0;
            let chip_area_h = WIDGET_SPACING + self.chip_height;
            let mut chip_x = self.rect.x;
            let mut chip_y = self.rect.y + self.rect.h + WIDGET_SPACING;
            let mut col = 0;
            let max_row_w = self.rect.w;

            let mut to_remove: Vec<usize> = Vec::new();
            for (i, item) in self.selected.iter().enumerate() {
                let label = (self.to_string)(item);
                let text_w = measure_text_ui(ctx, &label, DEFAULT_FONT_SIZE_16).width;
                let chip_w = text_w + chip_pad_x * 2.0;
                // Wrap to next row if this chip would overflow
                if col > 0 && chip_x + chip_w > self.rect.x + max_row_w {
                    col = 0;
                    chip_x = self.rect.x;
                    chip_y += chip_area_h;
                }
                let chip_rect = Rect::new(chip_x, chip_y, chip_w, self.chip_height);
                if Button::new(chip_rect, &label)
                    .overrides(self.base.overrides)
                    .show(ctx)
                {
                    to_remove.push(i);
                }
                col += 1;
                chip_x += chip_w + self.chip_padding;
            }

            for &i in to_remove.iter().rev() {
                removed.push(self.selected.remove(i));
            }
        }

        if added.is_empty() && removed.is_empty() {
            None
        } else {
            Some(MultiSelectDelta { added, removed })
        }
    }

    /// Enables or disables the filterable TextInput on the dropdown.
    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    /// Sets a callback to create new items from filter text.
    pub fn create_new(mut self, f: impl Fn(&str) -> T + 'a) -> Self {
        self.create_new = Some(Box::new(f));
        self
    }

    /// Sets the theme overrides for this widget.
    pub fn overrides(mut self, overrides: WidgetTheme) -> Self {
        self.base.overrides = overrides;
        self
    }
}

impl<T> Widget for MultiSelect<'_, T> {
    fn widget_type() -> WidgetType {
        WidgetType::Dropdown
    }
    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
}

#[cfg(test)]
mod tests;
