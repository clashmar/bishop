pub mod breadcrumb;
pub mod context_menu;
pub mod icon_mapper;
pub mod navigation;
pub mod path_filter;
mod selection;

use crate::commands::asset::{BatchMoveCmd, MoveTarget};
use crate::editor_assets::assets::{
    audio_icon, entity_icon, file_icon, folder_icon, image_icon, lua_icon, system_folder_icon,
    text_icon,
};
use crate::editor_global::push_command;
use crate::gui::gui_constants::HIGHLIGHT_GREEN;
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::shared::selection::{draw_selection_box, rect_from_two_points, rects_intersect};
use crate::Editor;
use bishop::prelude::*;
use context_menu::{
    draw_context_menu, handle_pending_action, open_resource, pending_action_for,
    pending_action_for_background, ActiveMenu, EntryKind, PendingResourceAction,
    ResourceMenuAction, ResourceOpenResult,
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

const SELECTION_BG: Color = Color::new(0.706, 0.824, 1.0, 0.25);
const DRAG_ACTIVATION_THRESHOLD: f32 = 4.0;
const GHOST_OFFSET: f32 = 10.0;
const DROP_TARGET_OUTLINE: Color = SELECTION_BG;

#[derive(Default)]
struct MarqueeSelectionState {
    active: bool,
    additive: bool,
    start_content_pos: Option<Vec2>,
    selection_snapshot: BTreeSet<usize>,
}

#[derive(Default)]
struct DragState {
    active: bool,
    start_screen_pos: Vec2,
    payload: Vec<DragPayload>,
}

pub(crate) struct DragPayload {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) icon_type: IconType,
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
    drag_state: DragState,
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
            drag_state: DragState::default(),
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

                let icon_type = if is_dir {
                    IconMapper::dir_icon_for(&full_path, &resources_folder_current())
                } else {
                    IconMapper::file_icon(&name)
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
            IconType::SystemFolder => system_folder_icon(),
            IconType::LuaScript => lua_icon(),
            IconType::Image => image_icon(),
            IconType::Audio => audio_icon(),
            IconType::Text => text_icon(),
            IconType::Prefab => entity_icon(),
            IconType::File => file_icon(),
        }
    }

    pub(crate) fn drop_target_index(
        &self,
        mouse: Vec2,
        content_rect: Rect,
        cols: usize,
    ) -> Option<usize> {
        self.entries.iter().enumerate().find_map(|(i, entry)| {
            if !entry.is_dir_like() {
                return None;
            }
            if self.is_dragged_item(&entry.path) {
                return None;
            }
            let cell = cell_content_rect(i, cols);
            let screen_rect = Rect::new(
                content_rect.x + cell.x,
                content_rect.y + cell.y + self.scroll_state.scroll_y,
                cell.w,
                cell.h,
            );
            if screen_rect.contains(mouse) {
                Some(i)
            } else {
                None
            }
        })
    }

    pub(crate) fn is_dragged_item(&self, path: &Path) -> bool {
        self.drag_state.payload.iter().any(|p| p.path == path)
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

    fn draw_custom_title(&mut self, ctx: &mut WgpuContext, title_bar: Rect, blocked: bool) -> bool {
        let start_x = title_bar.x + 30.0;
        let max_width = title_bar.right() - 30.0 - start_x;
        let style = breadcrumb::BreadcrumbStyle {
            x: start_x,
            y: title_bar.y,
            max_width,
            height: title_bar.h,
            root_label: RESOURCES_PANEL,
        };
        if let Some(target_depth) =
            breadcrumb::draw_breadcrumb(ctx, &style, &self.navigation, blocked)
        {
            self.clear_selection();
            self.navigation.truncate_to(target_depth);
            widgets::consume_click();
        }
        true
    }

    fn on_defocus(&mut self) {
        self.clear_selection();
        self.reset_marquee_selection();
        self.active_menu = None;
        self.drag_state = DragState::default();
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        self.scan_current_dir(&editor.game.asset_registry);

        let left_clicked = ctx.is_mouse_button_pressed(MouseButton::Left);
        let left_held = ctx.is_mouse_button_down(MouseButton::Left);
        let right_clicked = ctx.is_mouse_button_pressed(MouseButton::Right);
        if right_clicked && !blocked && widgets::is_context_menu_open() {
            widgets::close_open_context_menus();
            self.active_menu = None;
            self.clear_selection();
        }

        let interaction_blocked = blocked || widgets::is_context_menu_open();

        let mouse: Vec2 = ctx.mouse_position().into();
        let content_rect = rect;

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
            && !widgets::is_click_consumed()
            && !self.drag_state.active;
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

        let drop_target = if self.drag_state.active {
            self.drop_target_index(mouse, content_rect, cols)
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

            if self.drag_state.active {
                if let Some(target_i) = drop_target {
                    if target_i == i {
                        ctx.draw_rectangle_lines(
                            x,
                            cell_y,
                            CELL_SIZE,
                            CELL_SIZE,
                            2.0,
                            DROP_TARGET_OUTLINE,
                        );
                    }
                }
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

                if self.drag_state.active {
                    // skip normal click handling during drag
                } else if ctx.is_mouse_button_double_clicked(MouseButton::Left) {
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
                } else if left_clicked {
                    if !self.selected_indices.contains(&i) {
                        self.handle_primary_click_on_entry(i, shift_held, false);
                    }
                    if self.is_draggable(i) {
                        self.drag_state.start_screen_pos = mouse;
                        self.drag_state.payload = self.build_drag_payload(i);
                    }
                }

                if !self.drag_state.active && right_clicked {
                    right_clicked_entry = true;
                    self.handle_secondary_click_on_entry(i, mouse);
                }
            }
        }

        if !self.drag_state.payload.is_empty()
            && left_held
            && !self.drag_state.active
            && mouse.distance(self.drag_state.start_screen_pos) > DRAG_ACTIVATION_THRESHOLD
        {
            self.drag_state.active = true;
            widgets::consume_click();
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

        if self.drag_state.active {
            let ghost_x = mouse.x - GHOST_OFFSET;
            let ghost_y = mouse.y - GHOST_OFFSET;
            let ghost_alpha = Color::new(1.0, 1.0, 1.0, 0.6);

            if self.drag_state.payload.len() == 1 {
                let payload = &self.drag_state.payload[0];
                let texture = self.icon_texture(payload.icon_type);
                ctx.draw_texture_ex(
                    texture,
                    ghost_x,
                    ghost_y,
                    ghost_alpha,
                    DrawTextureParams {
                        dest_size: Some(vec2(ICON_SIZE, ICON_SIZE)),
                        ..Default::default()
                    },
                );
                let label = truncate_label(ctx, &payload.name, ICON_SIZE, LABEL_FONT_SIZE);
                let label_width = measure_text(ctx, &label, LABEL_FONT_SIZE).width;
                ctx.draw_text(
                    &label,
                    ghost_x + (ICON_SIZE - label_width) / 2.0,
                    ghost_y + ICON_SIZE + 4.0 + LABEL_FONT_SIZE,
                    LABEL_FONT_SIZE,
                    ghost_alpha,
                );
            } else {
                let payload = &self.drag_state.payload[0];
                let texture = self.icon_texture(payload.icon_type);
                ctx.draw_texture_ex(
                    texture,
                    ghost_x,
                    ghost_y,
                    ghost_alpha,
                    DrawTextureParams {
                        dest_size: Some(vec2(ICON_SIZE, ICON_SIZE)),
                        ..Default::default()
                    },
                );
                let count = self.drag_state.payload.len() - 1;
                let badge_text = format!("+{}", count);
                let badge_font_size = 14.0;
                let text_width = measure_text(ctx, &badge_text, badge_font_size).width;
                let badge_padding = 4.0;
                let badge_w = text_width + badge_padding * 2.0;
                let badge_h = badge_font_size + badge_padding;
                let badge_x = ghost_x + ICON_SIZE - badge_w + 4.0;
                let badge_y = ghost_y + ICON_SIZE - badge_h + 4.0;
                ctx.draw_rectangle(
                    badge_x,
                    badge_y,
                    badge_w,
                    badge_h,
                    Color::new(0.2, 0.2, 0.2, 0.8),
                );
                ctx.draw_text(
                    &badge_text,
                    badge_x + badge_padding,
                    badge_y + badge_font_size,
                    badge_font_size,
                    Color::WHITE,
                );
            }
        }

        area.draw_scrollbar(ctx, &self.scroll_state);

        if clicked_empty_space {
            self.begin_marquee_selection(content_mouse, shift_held);
        }

        if ctx.is_mouse_button_released(MouseButton::Left) {
            if self.drag_state.active {
                let mut dropped = false;
                if let Some(target_index) = self.drop_target_index(mouse, content_rect, cols) {
                    let dest_dir = if self.entries[target_index].is_parent() {
                        self.navigation
                            .current()
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_default()
                    } else {
                        self.entries[target_index].path.clone()
                    };

                    let targets: Vec<MoveTarget> = self
                        .drag_state
                        .payload
                        .iter()
                        .filter_map(|p| {
                            if p.path.parent() == Some(dest_dir.as_path()) {
                                return None;
                            }
                            if p.path.is_dir() && dest_dir.starts_with(&p.path) {
                                return None;
                            }
                            if p.path.is_dir() {
                                Some(MoveTarget::Directory {
                                    old_path: p.path.clone().into(),
                                    new_path: dest_dir.join(&p.name),
                                })
                            } else {
                                Some(MoveTarget::File {
                                    old_path: p.path.clone(),
                                    new_path: dest_dir.join(&p.name),
                                    key: editor.game.asset_registry.key_for_full_path(&p.path),
                                })
                            }
                        })
                        .collect();

                    if !targets.is_empty() {
                        push_command(Box::new(BatchMoveCmd::new(targets)));
                        dropped = true;
                    }
                }
                self.drag_state = DragState::default();
                if dropped {
                    self.clear_selection();
                }
            } else if !self.drag_state.payload.is_empty() {
                self.drag_state = DragState::default();
            } else if self.marquee_selection.active {
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
                    ActiveMenu::MultiSelection(_) => {
                        if selected == ResourceMenuAction::Delete {
                            self.pending_action =
                                self.pending_delete_for_selection(&editor.game.asset_registry);
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

        if !interaction_blocked
            && !blocked
            && widgets::focused_panel() == Some(RESOURCES_PANEL)
            && !widgets::input_is_focused()
            && (ctx.is_key_pressed(KeyCode::Delete) || ctx.is_key_pressed(KeyCode::Backspace))
        {
            if let Some(action) = self.pending_delete_for_selection(&editor.game.asset_registry) {
                self.pending_action = Some(action);
                self.clear_selection();
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
