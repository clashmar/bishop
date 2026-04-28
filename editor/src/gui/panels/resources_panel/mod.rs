pub mod breadcrumb;
pub mod context_menu;
pub mod icon_mapper;
pub mod navigation;
pub mod path_filter;
mod selection;

use crate::editor_assets::assets::{
    audio_icon, entity_icon, file_icon, folder_icon, image_icon, lua_icon, text_icon,
};
use crate::gui::gui_constants::HIGHLIGHT_GREEN;
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::shared::selection::{draw_selection_box, rect_from_two_points, rects_intersect};
use crate::Editor;
use bishop::prelude::*;
#[cfg(test)]
use context_menu::ResourceMenuAction;
use context_menu::{
    draw_context_menu, handle_pending_action, open_resource, pending_action_for,
    pending_action_for_background, ActiveMenu, EntryKind, PendingResourceAction,
    ResourceOpenResult,
};
use engine_core::prelude::*;
use icon_mapper::{IconMapper, IconType};
use navigation::Navigation;
use path_filter::PathFilter;
use std::collections::BTreeSet;
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

const SELECTION_BG: Color = Color::new(0.706, 0.824, 1.0, 0.25);

#[derive(Default)]
struct MarqueeSelectionState {
    active: bool,
    additive: bool,
    start_content_pos: Option<Vec2>,
    selection_snapshot: BTreeSet<usize>,
}

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
        matches!(
            self.kind,
            EntryKind::Parent | EntryKind::Directory | EntryKind::SystemDirectory
        )
    }

    fn is_registered(&self) -> bool {
        self.kind == EntryKind::RegisteredFile
    }

    #[cfg(test)]
    fn context_menu_actions(&self) -> &'static [ResourceMenuAction] {
        context_menu::context_menu_actions_for(self.kind)
    }
}

pub struct ResourcesPanel {
    navigation: Navigation,
    scroll_state: ScrollState,
    entries: Vec<Entry>,
    active_menu: Option<ActiveMenu>,
    pending_action: Option<PendingResourceAction>,
    context_menu_id: WidgetId,
    selected_indices: BTreeSet<usize>,
    marquee_selection: MarqueeSelectionState,
}

impl ResourcesPanel {
    pub fn new() -> Self {
        Self {
            navigation: Navigation::new(),
            scroll_state: ScrollState::new(),
            entries: Vec::new(),
            active_menu: None,
            pending_action: None,
            context_menu_id: WidgetId(0xC07E_0001),
            selected_indices: BTreeSet::new(),
            marquee_selection: MarqueeSelectionState::default(),
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
                    if is_protected_path(&full_path, &resources_folder_current()) {
                        EntryKind::SystemDirectory
                    } else {
                        EntryKind::Directory
                    }
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

fn content_space_mouse_position(mouse: Vec2, content_rect: Rect, scroll_y: f32) -> Vec2 {
    Vec2::new(
        mouse.x - content_rect.x,
        mouse.y - content_rect.y - scroll_y,
    )
}

fn content_space_to_screen(content_pos: Vec2, content_rect: Rect, scroll_y: f32) -> Vec2 {
    Vec2::new(
        content_rect.x + content_pos.x,
        content_rect.y + content_pos.y + scroll_y,
    )
}

fn cell_content_rect(index: usize, cols: usize) -> Rect {
    let col = index % cols;
    let row = index / cols;
    let x = GRID_PADDING + col as f32 * (CELL_SIZE + GRID_PADDING);
    let y = GRID_PADDING + row as f32 * (CELL_SIZE + GRID_PADDING);
    Rect::new(x, y, CELL_SIZE, CELL_SIZE)
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

        let left_clicked = ctx.is_mouse_button_pressed(MouseButton::Left);
        let right_clicked = ctx.is_mouse_button_pressed(MouseButton::Right);
        if right_clicked && !blocked && widgets::is_context_menu_open() {
            widgets::close_open_context_menus();
            self.active_menu = None;
            self.clear_selection();
        }

        let interaction_blocked = blocked || widgets::is_context_menu_open();

        let mouse: Vec2 = ctx.mouse_position().into();
        let mut top_y = rect.y + TOP_BAR_PADDING;

        let breadcrumb_y = top_y;
        if let Some(target_depth) = breadcrumb::draw_breadcrumb(
            ctx,
            rect.x + GRID_PADDING,
            breadcrumb_y,
            &self.navigation,
            interaction_blocked,
        ) {
            self.clear_selection();
            self.navigation.truncate_to(target_depth);
            widgets::consume_click();
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

        let mut clicked_empty_space = left_clicked
            && !interaction_blocked
            && !blocked
            && content_rect.contains(mouse)
            && !widgets::is_click_consumed();
        let mut right_clicked_entry = false;
        let shift_held =
            ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
        let mut content_mouse =
            content_space_mouse_position(mouse, content_rect, self.scroll_state.scroll_y);

        if self.marquee_selection.active && !interaction_blocked && !blocked {
            if area.apply_drag_edge_autoscroll(ctx, &mut self.scroll_state, true) {
                content_mouse =
                    content_space_mouse_position(mouse, content_rect, self.scroll_state.scroll_y);
            }
        }

        let live_marquee_rect = if self.marquee_selection.active {
            self.marquee_selection
                .start_content_pos
                .map(|start| rect_from_two_points(start, content_mouse))
        } else {
            None
        };

        for i in 0..self.entries.len() {
            let (display_name, icon_type, is_registered, is_dir_like) = {
                let entry = &self.entries[i];
                (
                    entry.display_name.clone(),
                    entry.icon_type,
                    entry.is_registered(),
                    entry.is_dir_like(),
                )
            };
            let cell_content = cell_content_rect(i, cols);
            let x = content_rect.x + cell_content.x;
            let cell_y = content_rect.y + cell_content.y + self.scroll_state.scroll_y;

            if !area.is_visible(cell_y, CELL_SIZE) {
                continue;
            }

            let in_marquee = live_marquee_rect
                .as_ref()
                .is_some_and(|rect| rects_intersect(*rect, cell_content_rect(i, cols)))
                && !self.entries[i].is_parent();
            let is_selected =
                if in_marquee && self.marquee_selection.active && self.marquee_selection.additive {
                    !self.marquee_selection.selection_snapshot.contains(&i)
                } else if in_marquee && self.marquee_selection.active {
                    true
                } else {
                    self.selected_indices.contains(&i)
                };
            if is_selected {
                let size = CELL_SIZE * 0.9 + 4.0;
                let offset = (CELL_SIZE - size) / 2.0;
                ctx.draw_rectangle(x + offset, cell_y, size, size, SELECTION_BG);
            }

            let icon_x = x + (CELL_SIZE - ICON_SIZE) / 2.0;
            let icon_y = cell_y;

            let texture = self.icon_texture(icon_type);
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

            if is_registered {
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
            let label = truncate_label(ctx, &display_name, CELL_SIZE, LABEL_FONT_SIZE);
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

            if !interaction_blocked
                && !blocked
                && cell_rect.contains(mouse)
                && !self.marquee_selection.active
            {
                clicked_empty_space = false;

                if ctx.is_mouse_button_double_clicked(MouseButton::Left) {
                    if let Some(path) = self.handle_primary_click_on_entry(i, false, true) {
                        if let ResourceOpenResult::PrefabTransition(prefab_id) =
                            open_resource(&path, editor)
                        {
                            editor.enter_prefab_transition(ctx, prefab_id);
                        }
                    } else if is_dir_like {
                        widgets::consume_click();
                    }
                    break;
                }

                if left_clicked {
                    self.handle_primary_click_on_entry(i, shift_held, false);
                }

                if right_clicked {
                    right_clicked_entry = true;
                    self.handle_secondary_click_on_entry(i, mouse);
                }
            }
        }

        if self.marquee_selection.active {
            if let Some(start) = self.marquee_selection.start_content_pos {
                let start_screen =
                    content_space_to_screen(start, content_rect, self.scroll_state.scroll_y);
                let current_screen = content_space_to_screen(
                    content_mouse,
                    content_rect,
                    self.scroll_state.scroll_y,
                );
                draw_selection_box(ctx, start_screen, current_screen);
            }
        }

        ctx.pop_clip_rect();
        area.draw_scrollbar(ctx, self.scroll_state.scroll_y);

        if clicked_empty_space {
            self.begin_marquee_selection(content_mouse, shift_held);
        }

        if self.marquee_selection.active && ctx.is_mouse_button_released(MouseButton::Left) {
            if let Some(start) = self.marquee_selection.start_content_pos {
                let marquee_rect = rect_from_two_points(start, content_mouse);
                let matched: BTreeSet<usize> = self
                    .entries
                    .iter()
                    .enumerate()
                    .filter_map(|(index, entry)| {
                        if entry.is_parent() {
                            return None;
                        }
                        rects_intersect(marquee_rect, cell_content_rect(index, cols))
                            .then_some(index)
                    })
                    .collect();
                self.commit_marquee_selection(matched);
            } else {
                self.reset_marquee_selection();
            }
        }

        if !interaction_blocked
            && !blocked
            && right_clicked
            && content_rect.contains(mouse)
            && !right_clicked_entry
        {
            self.handle_secondary_click_on_background(mouse);
        }

        if let Some(ref menu) = self.active_menu {
            if let Some(selected) = draw_context_menu(self.context_menu_id, menu, ctx, blocked) {
                match menu {
                    ActiveMenu::Entry(ref target) => {
                        if let Some(entry) = self.entries.get(target.entry_index) {
                            self.pending_action =
                                pending_action_for(entry, selected, &editor.game.asset_registry);
                        }
                    }
                    ActiveMenu::Background(_) => {
                        let current_dir = self.navigation.current();
                        self.pending_action = Some(pending_action_for_background(&current_dir));
                    }
                }
                self.active_menu = None;
                self.clear_selection();
            } else {
                let state = widgets::context_menu_state::get(self.context_menu_id);
                if !state.open {
                    self.active_menu = None;
                    self.clear_selection();
                }
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
