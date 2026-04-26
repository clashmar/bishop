pub mod breadcrumb;
pub mod context_menu;
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
use context_menu::{
    context_target_for_entry, draw_context_menu, handle_pending_action, pending_action_for,
    ContextTarget, EntryKind, PendingResourceAction, ResourceMenuAction,
};
use engine_core::prelude::*;
use icon_mapper::{IconMapper, IconType};
use navigation::Navigation;
use path_filter::PathFilter;
use std::path::{Path, PathBuf};
use widgets::WidgetId;

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
    kind: EntryKind,
    pub path: PathBuf,
    pub icon_type: IconType,
}

impl Entry {
    fn is_parent(&self) -> bool {
        self.kind == EntryKind::Parent
    }

    fn is_dir_like(&self) -> bool {
        matches!(self.kind, EntryKind::Parent | EntryKind::Directory)
    }

    fn is_registered(&self) -> bool {
        self.kind == EntryKind::RegisteredFile
    }

    fn context_menu_actions(&self) -> &'static [ResourceMenuAction] {
        context_menu::context_menu_actions_for(self.kind)
    }
}

pub struct ResourcesPanel {
    navigation: Navigation,
    scroll_state: ScrollState,
    entries: Vec<Entry>,
    context_target: Option<ContextTarget>,
    pending_action: Option<PendingResourceAction>,
    context_menu_id: WidgetId,
}

impl ResourcesPanel {
    pub fn new() -> Self {
        Self {
            navigation: Navigation::new(),
            scroll_state: ScrollState::new(),
            entries: Vec::new(),
            context_target: None,
            pending_action: None,
            context_menu_id: WidgetId(0xC07E_0001),
        }
    }

    fn scan_current_dir(&mut self, registry: &AssetRegistry) {
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
                let kind = if is_dir {
                    EntryKind::Directory
                } else if registry.key_for_full_path(&full_path).is_some() {
                    EntryKind::RegisteredFile
                } else {
                    EntryKind::UnregisteredFile
                };

                Some(Entry {
                    name,
                    display_name,
                    kind,
                    path: full_path,
                    icon_type,
                })
            })
            .collect();

        visible.sort_by(|a, b| match (a.is_dir_like(), b.is_dir_like()) {
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
            let parent_entry = Entry {
                name: "..".to_string(),
                display_name: "..".to_string(),
                kind: EntryKind::Parent,
                path: parent_path,
                icon_type: IconMapper::dir_icon(),
            };
            visible.insert(0, parent_entry);
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

        let interaction_blocked = blocked || widgets::is_context_menu_open();

        let mut top_y = rect.y + TOP_BAR_PADDING;

        let breadcrumb_y = top_y;
        if let Some(target_depth) = breadcrumb::draw_breadcrumb(
            ctx,
            rect.x + GRID_PADDING,
            breadcrumb_y,
            &self.navigation,
            interaction_blocked,
        ) {
            self.navigation.truncate_to(target_depth);
            self.scan_current_dir(&editor.game.asset_registry);
        }
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
            .blocked(interaction_blocked)
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

            if entry.is_registered() {
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

            let cell_rect = Rect::new(x, cell_y, CELL_SIZE, CELL_SIZE);
            let mouse: Vec2 = ctx.mouse_position().into();

            if !interaction_blocked {
                if entry.is_dir_like()
                    && cell_rect.contains(mouse)
                    && ctx.is_mouse_button_pressed(MouseButton::Left)
                {
                    if entry.is_parent() {
                        self.navigation.pop();
                    } else {
                        self.navigation.push(&entry.name);
                    }
                    self.scan_current_dir(&editor.game.asset_registry);
                    break;
                }

                if cell_rect.contains(mouse) && ctx.is_mouse_button_pressed(MouseButton::Right) {
                    if let Some(target) = context_target_for_entry(i, entry, mouse) {
                        self.context_target = Some(target);
                    }
                }
            }
        }

        ctx.pop_clip_rect();
        area.draw_scrollbar(ctx, self.scroll_state.scroll_y);

        if let Some(ref target) = self.context_target {
            if let Some(selected) =
                draw_context_menu(self.context_menu_id, target, ctx, interaction_blocked)
            {
                let entry = &self.entries[target.entry_index];
                self.pending_action = pending_action_for(entry, selected);
                self.context_target = None;
            }
        }

        self.pending_action = handle_pending_action(self.pending_action.take(), editor, ctx);

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
