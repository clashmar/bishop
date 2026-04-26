pub mod icon_mapper;
pub mod navigation;
pub mod path_filter;

use crate::editor_assets::assets::{
    audio_icon, entity_icon, file_icon, folder_icon, image_icon, lua_icon, text_icon,
};
use crate::gui::gui_constants::HIGHLIGHT_GREEN;
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;
use icon_mapper::{IconMapper, IconType};
use navigation::Navigation;
use path_filter::PathFilter;
use std::path::{Path, PathBuf};

pub const RESOURCES_PANEL: &str = "Resources";

const CELL_SIZE: f32 = 72.0;
const GRID_PADDING: f32 = 12.0;
const ICON_SIZE: f32 = 42.0;
const LABEL_FONT_SIZE: f32 = 12.0;
const REGISTRATION_BADGE_SIZE: f32 = 8.0;
const BREADCRUMB_HEIGHT: f32 = 20.0;
const TOP_BAR_PADDING: f32 = 8.0;

/// An entry in the Resources browser.
pub struct Entry {
    pub name: String,
    pub display_name: String,
    pub is_dir: bool,
    pub is_parent: bool,
    pub path: PathBuf,
    pub icon_type: IconType,
    pub is_registered: bool,
}

pub struct ResourcesPanel {
    navigation: Navigation,
    scroll_state: ScrollState,
    entries: Vec<Entry>,
}

impl ResourcesPanel {
    pub fn new() -> Self {
        Self {
            navigation: Navigation::new(),
            scroll_state: ScrollState::new(),
            entries: Vec::new(),
        }
    }

    fn scan_current_dir(&mut self, registry: &AssetRegistry) {
        let root = resources_folder_current();
        let current = self.navigation.current();
        let Ok(entries) = std::fs::read_dir(&current) else {
            self.entries.clear();
            return;
        };
        let mut visible: Vec<Entry> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let file_name = e.file_name();
                let name = file_name.to_string_lossy().to_string();
                let is_dir = e.file_type().is_ok_and(|ft| ft.is_dir());

                if is_dir {
                    if !PathFilter::dir_visible(&name) {
                        return None;
                    }
                } else if !PathFilter::file_visible(&name) {
                    return None;
                }

                let icon_type = if is_dir {
                    IconMapper::dir_icon()
                } else {
                    IconMapper::file_icon(&name)
                };

                let display_name = if is_dir {
                    name.clone()
                } else {
                    Path::new(&name)
                        .file_stem()
                        .and_then(|s: &std::ffi::OsStr| s.to_str())
                        .unwrap_or(&name)
                        .to_string()
                };

                let full_path = e.path();
                let relative_path = full_path
                    .strip_prefix(&root)
                    .unwrap_or(&full_path)
                    .to_path_buf();
                let is_registered = registry.key_for_path(&relative_path).is_some();

                Some(Entry {
                    name,
                    display_name,
                    is_dir,
                    is_parent: false,
                    path: full_path,
                    icon_type,
                    is_registered,
                })
            })
            .collect();

        visible.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        if !self.navigation.is_at_root() {
            let parent_path = {
                let mut p = self.navigation.current();
                p.pop();
                p
            };
            visible.insert(
                0,
                Entry {
                    name: "..".to_string(),
                    display_name: "..".to_string(),
                    is_dir: true,
                    is_parent: true,
                    path: parent_path,
                    icon_type: IconMapper::dir_icon(),
                    is_registered: false,
                },
            );
        }

        self.entries = visible;
    }

    fn icon_texture(&self, icon_type: IconType) -> &'static Texture2D {
        match icon_type {
            IconType::Folder => folder_icon(),
            IconType::LuaScript => lua_icon(),
            IconType::Image => image_icon(),
            IconType::Audio => audio_icon(),
            IconType::Text => text_icon(),
            IconType::Prefab => entity_icon(),
            IconType::File => file_icon(),
        }
    }
}

impl PanelDefinition for ResourcesPanel {
    fn title(&self) -> &'static str {
        RESOURCES_PANEL
    }

    fn default_rect(&self, ctx: &WgpuContext) -> Rect {
        Rect::new(
            0.0,
            ctx.screen_height() - 300.0,
            ctx.screen_width() * 0.5,
            300.0,
        )
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        self.scan_current_dir(&editor.game.asset_registry);

        let mut top_y = rect.y + TOP_BAR_PADDING;

        let breadcrumb_y = top_y;
        let current = self.navigation.current();
        let breadcrumb = current
            .strip_prefix(resources_folder_current())
            .unwrap_or(&current)
            .to_string_lossy()
            .to_string();
        let breadcrumb_text = if breadcrumb.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", breadcrumb.replace(std::path::MAIN_SEPARATOR, "/"))
        };
        ctx.draw_text(
            &breadcrumb_text,
            rect.x + GRID_PADDING,
            breadcrumb_y + BREADCRUMB_HEIGHT * 0.7,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );
        top_y += BREADCRUMB_HEIGHT + GRID_PADDING;

        let content_rect = Rect::new(rect.x, top_y, rect.w, rect.y + rect.h - top_y);

        let cols = if content_rect.w > CELL_SIZE + GRID_PADDING {
            ((content_rect.w - GRID_PADDING) / (CELL_SIZE + GRID_PADDING)) as usize
        } else {
            1
        };

        let content_height = if self.entries.is_empty() {
            CELL_SIZE
        } else {
            let rows = self.entries.len().div_ceil(cols);
            rows as f32 * (CELL_SIZE + GRID_PADDING) + GRID_PADDING
        };

        let area = ScrollableArea::new(content_rect, content_height)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);

        ctx.push_clip_rect(content_rect);

        for (i, entry) in self.entries.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = content_rect.x + GRID_PADDING + col as f32 * (CELL_SIZE + GRID_PADDING);
            let cell_y = content_rect.y
                + GRID_PADDING
                + row as f32 * (CELL_SIZE + GRID_PADDING)
                + self.scroll_state.scroll_y;

            if !area.is_visible(cell_y, CELL_SIZE) {
                continue;
            }

            let icon_x = x + (CELL_SIZE - ICON_SIZE) / 2.0;
            let icon_y = cell_y;

            let texture = self.icon_texture(entry.icon_type);
            ctx.draw_texture_ex(
                texture,
                icon_x,
                icon_y,
                Color::WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(ICON_SIZE, ICON_SIZE)),
                    ..Default::default()
                },
            );

            if entry.is_registered {
                let badge_x = icon_x + ICON_SIZE - REGISTRATION_BADGE_SIZE;
                let badge_y = icon_y + ICON_SIZE - REGISTRATION_BADGE_SIZE;
                ctx.draw_circle(
                    badge_x + REGISTRATION_BADGE_SIZE / 2.0,
                    badge_y + REGISTRATION_BADGE_SIZE / 2.0,
                    REGISTRATION_BADGE_SIZE / 2.0,
                    HIGHLIGHT_GREEN,
                );
            }

            let text_y = cell_y + ICON_SIZE + 4.0;
            let label = truncate_label(ctx, &entry.display_name, CELL_SIZE, LABEL_FONT_SIZE);
            let label_width = measure_text(ctx, &label, LABEL_FONT_SIZE).width;
            let text_x = x + (CELL_SIZE - label_width) / 2.0;
            ctx.draw_text(
                &label,
                text_x,
                text_y + LABEL_FONT_SIZE,
                LABEL_FONT_SIZE,
                Color::WHITE,
            );

            if entry.is_dir && !blocked {
                let cell_rect = Rect::new(x, cell_y, CELL_SIZE, CELL_SIZE);
                let mouse: Vec2 = ctx.mouse_position().into();
                if cell_rect.contains(mouse) && ctx.is_mouse_button_pressed(MouseButton::Left) {
                    if entry.is_parent {
                        self.navigation.pop();
                    } else {
                        self.navigation.push(&entry.name);
                    }
                    self.scan_current_dir(&editor.game.asset_registry);
                    break;
                }
            }
        }

        ctx.pop_clip_rect();
        area.draw_scrollbar(ctx, self.scroll_state.scroll_y);

        ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, Color::WHITE);
    }
}

fn truncate_label(ctx: &WgpuContext, text: &str, max_width: f32, font_size: f32) -> String {
    if measure_text(ctx, text, font_size).width <= max_width {
        return text.to_string();
    }
    let ellipsis = "…";
    let ellipsis_w = measure_text(ctx, ellipsis, font_size).width;
    let target = max_width - ellipsis_w;
    let mut end_byte = 0;
    for (byte_idx, ch) in text.char_indices() {
        let w = measure_text(ctx, &text[..byte_idx + ch.len_utf8()], font_size).width;
        if w > target {
            break;
        }
        end_byte = byte_idx + ch.len_utf8();
    }
    format!("{}{}", &text[..end_byte], ellipsis)
}

#[cfg(test)]
mod tests;
