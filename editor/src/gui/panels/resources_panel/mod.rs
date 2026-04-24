pub mod icon_mapper;
pub mod navigation;
pub mod path_filter;

use crate::editor_assets::assets::{
    audio_icon, file_icon, folder_icon, image_icon, lua_icon, text_icon,
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

const CELL_SIZE: f32 = 80.0;
const GRID_PADDING: f32 = 8.0;
const ICON_SIZE: f32 = 48.0;
const REGISTRATION_BADGE_SIZE: f32 = 8.0;
const BACK_BUTTON_HEIGHT: f32 = 28.0;
const BREADCRUMB_HEIGHT: f32 = 20.0;
const TOP_BAR_PADDING: f32 = 8.0;

/// An entry in the Resources browser.
pub struct Entry {
    pub name: String,
    pub display_name: String,
    pub is_dir: bool,
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
        let root = resources_folder_current();
        Self {
            navigation: Navigation::new(root),
            scroll_state: ScrollState::new(),
            entries: Vec::new(),
        }
    }

    fn scan_current_dir(&mut self, registry: &AssetRegistry) {
        let current = self.navigation.current().to_path_buf();
        let root = resources_folder_current();

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

        self.entries = visible;
    }

    fn icon_texture(&self, icon_type: IconType) -> &'static Texture2D {
        match icon_type {
            IconType::Folder => folder_icon(),
            IconType::LuaScript => lua_icon(),
            IconType::Image => image_icon(),
            IconType::Audio => audio_icon(),
            IconType::Text => text_icon(),
            IconType::Prefab => file_icon(),
            IconType::File => file_icon(),
        }
    }
}

impl PanelDefinition for ResourcesPanel {
    fn title(&self) -> &'static str {
        RESOURCES_PANEL
    }

    fn default_rect(&self, ctx: &WgpuContext) -> Rect {
        Rect::new(0.0, ctx.screen_height() - 300.0, ctx.screen_width(), 300.0)
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        self.scan_current_dir(&editor.game.asset_registry);

        let mut top_y = rect.y + TOP_BAR_PADDING;

        if !self.navigation.is_at_root() {
            let back_rect = Rect::new(rect.x + GRID_PADDING, top_y, 60.0, BACK_BUTTON_HEIGHT);
            if Button::new(back_rect, "< Back")
                .suppressed(blocked)
                .show(ctx)
                && !blocked
            {
                self.navigation.pop();
                self.scan_current_dir(&editor.game.asset_registry);
            }
            top_y += BACK_BUTTON_HEIGHT + GRID_PADDING;
        }

        let breadcrumb_y = top_y;
        let current = self.navigation.current();
        let breadcrumb = current
            .strip_prefix(resources_folder_current())
            .unwrap_or(current)
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
            let rows = (self.entries.len() + cols - 1) / cols;
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

            let text_y = cell_y + ICON_SIZE + 2.0;
            ctx.draw_text(
                &entry.display_name,
                x,
                text_y + DEFAULT_FONT_SIZE_16 * 0.7,
                DEFAULT_FONT_SIZE_16,
                Color::WHITE,
            );

            if entry.is_dir && !blocked {
                let cell_rect = Rect::new(x, cell_y, CELL_SIZE, CELL_SIZE);
                let mouse: Vec2 = ctx.mouse_position().into();
                if cell_rect.contains(mouse) && ctx.is_mouse_button_pressed(MouseButton::Left) {
                    self.navigation.push(&entry.name);
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

#[cfg(test)]
mod tests;
