// editor/src/menu_editor/menu_properties_panel/nav_helpers.rs
use super::{common_properties::row_visible, FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::{MenuEditor, NavWidgetIds};
use bishop::prelude::*;
use engine_core::prelude::*;

pub struct NavMeta<T> {
    pub(crate) label: &'static str,
    pub(crate) id: WidgetId,
    pub(crate) get: fn(&T) -> Option<usize>,
    pub(crate) set: fn(&mut T, Option<usize>),
}

#[derive(Clone, Copy)]
pub(crate) struct NavSectionStyle<'a> {
    pub(crate) x: f32,
    pub(crate) w: f32,
    pub(crate) blocked: bool,
    pub(crate) clip: &'a Rect,
}

impl MenuEditor {
    pub(crate) fn draw_nav_section<T>(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        style: NavSectionStyle<'_>,
        nav_ids: &NavWidgetIds,
    ) where
        T: Clone + Navigable,
    {
        for &dir in &[
            NavDirection::Up,
            NavDirection::Down,
            NavDirection::Left,
            NavDirection::Right,
        ] {
            let meta = self.nav_meta(dir, nav_ids);

            let current = self
                .selected_element()
                .and_then(T::from_element)
                .and_then(|e| (meta.get)(e));

            let set_fn = meta.set;
            self.draw_nav_dropdown(
                ctx,
                y,
                style,
                current,
                meta,
                |element, value| {
                    if let Some(mut cloned_t) = T::from_element(element).cloned() {
                        set_fn(&mut cloned_t, value);
                        element.kind = cloned_t.wrap_into_element();
                    }
                },
            );
        }
    }

    pub(crate) fn draw_nav_dropdown<T>(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        style: NavSectionStyle<'_>,
        current: Option<usize>,
        meta: NavMeta<T>,
        apply: impl FnOnce(&mut MenuElement, Option<usize>),
    ) {
        let NavSectionStyle {
            x,
            w,
            blocked,
            clip,
        } = style;
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text(meta.label, x, *y + 16.0, 12.0, Color::WHITE);

            let options = self.get_focusable_element_names();

            let current_label = current
                .and_then(|idx| options.iter().find(|(i, _)| *i == idx))
                .map(|(_, name)| name.as_str())
                .unwrap_or("None");

            let mut nav_options = vec!["None".to_string()];
            nav_options.extend(options.iter().map(|(_, name)| name.clone()));

            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);

            if let Some(selected) =
                Dropdown::new(meta.id, dropdown_rect, current_label, &nav_options, |s| {
                    s.clone()
                })
                .blocked(blocked)
                .fixed_width()
                .show(ctx)
            {
                let new_nav = if selected == "None" {
                    None
                } else {
                    options
                        .iter()
                        .find(|(_, name)| name == &selected)
                        .map(|(idx, _)| *idx)
                };

                self.push_element_update(|element| {
                    apply(element, new_nav);
                });
            }
        }

        *y += ROW_HEIGHT;
    }

    /// Generic nav meta builder for any element type `T`.
    fn nav_meta<T: Navigable>(&self, dir: NavDirection, nav_ids: &NavWidgetIds) -> NavMeta<T> {
        match dir {
            NavDirection::Up => NavMeta {
                label: "Nav Up:",
                id: nav_ids.up,
                get: |element| element.nav_targets().up,
                set: |element, v| element.nav_targets_mut().up = v,
            },
            NavDirection::Down => NavMeta {
                label: "Nav Down:",
                id: nav_ids.down,
                get: |element| element.nav_targets().down,
                set: |el, v| el.nav_targets_mut().down = v,
            },
            NavDirection::Left => NavMeta {
                label: "Nav Left:",
                id: nav_ids.left,
                get: |element| element.nav_targets().left,
                set: |element, v| element.nav_targets_mut().left = v,
            },
            NavDirection::Right => NavMeta {
                label: "Nav Right:",
                id: nav_ids.right,
                get: |element| element.nav_targets().right,
                set: |element, v| element.nav_targets_mut().right = v,
            },
        }
    }

    pub(crate) fn get_focusable_element_names(&self) -> Vec<(usize, String)> {
        let Some(template) = self.current_template() else {
            return Vec::new();
        };

        let selected = self.primary_selected_index();
        template
            .elements
            .iter()
            .enumerate()
            .filter(|(idx, _)| selected != Some(*idx))
            .filter_map(|(idx, element)| {
                let name = if !element.name.is_empty() {
                    element.name.clone()
                } else {
                    match &element.kind {
                        MenuElementKind::Button(button) => button.text_key.clone(),
                        MenuElementKind::LayoutGroup(group) => {
                            let button_count = group
                                .children
                                .iter()
                                .filter(|c| matches!(c.element.kind, MenuElementKind::Button(_)))
                                .count();
                            format!("Layout Group ({} buttons)", button_count)
                        }
                        _ => return None,
                    }
                };
                match &element.kind {
                    MenuElementKind::Button(_) | MenuElementKind::LayoutGroup(_) => {
                        Some((idx, format!("{}: {}", idx, name)))
                    }
                    _ => None,
                }
            })
            .collect()
    }
}
