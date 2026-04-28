use crate::editor_assets::assets::entity_icon;
use crate::gui::gui_constants::BTN_HEIGHT;
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::room::prefab_preview::{build_prefab_preview, PrefabPreview, PrefabPreviewVisual};
use crate::room::room_editor::RoomEditorMode;
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::Path;

const PANEL_W: f32 = 250.0;
const PANEL_H: f32 = 750.0;
const PANEL_LEFT_MARGIN: f32 = 20.0;
const PANEL_TOP: f32 = 60.0;
const PANEL_PADDING: f32 = 10.0;
const CONTROL_GAP: f32 = 10.0;
const CARD_GAP: f32 = 12.0;
const RECENT_CARD_COLUMNS: usize = 2;
const NAME_H: f32 = 32.0;
const PREVIEW_PADDING: f32 = 10.0;
const SCROLLBAR_CARD_GAP: f32 = 16.0;

/// Panel title for the room prefab palette.
pub const PREFAB_PALETTE_PANEL: &str = "Prefab Palette";

#[derive(Clone, PartialEq)]
struct PrefabChoice {
    prefab_id: PrefabId,
    label: String,
}

impl Display for PrefabChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.label.fmt(f)
    }
}

/// Room-only prefab palette panel with prefab selection and recent previews.
pub struct PrefabPalettePanel {
    dropdown_id: WidgetId,
    file_button_id: WidgetId,
    open_prefab_editor_button_id: WidgetId,
    recent_scroll: ScrollState,
}

struct RecentPrefabGridLayout {
    content_rect: Rect,
    content_height: f32,
}

impl PrefabPalettePanel {
    /// Creates a new prefab palette panel.
    pub fn new() -> Self {
        Self {
            dropdown_id: WidgetId::default(),
            file_button_id: WidgetId::default(),
            open_prefab_editor_button_id: WidgetId::default(),
            recent_scroll: ScrollState::new(),
        }
    }
}

impl PanelDefinition for PrefabPalettePanel {
    fn title(&self) -> &'static str {
        PREFAB_PALETTE_PANEL
    }

    fn default_rect(&self, ctx: &WgpuContext) -> Rect {
        prefab_palette_default_rect(ctx.screen_width())
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        if !matches!(editor.mode, crate::app::EditorMode::Room(_)) {
            return;
        }
        let blocked = blocked || editor.room_editor.mode == RoomEditorMode::Tilemap;

        let mut y = rect.y + PANEL_PADDING;
        let content_w = rect.w - PANEL_PADDING * 2.0;
        let dropdown_rect = Rect::new(rect.x + PANEL_PADDING, y, content_w, BTN_HEIGHT);

        let active_prefab = active_prefab_choice(
            &editor.game.prefab_manager,
            editor.room_editor.active_prefab_id,
        );
        let prefab_choices = prefab_choices(&editor.game.prefab_manager);
        let selected_label = active_prefab
            .as_ref()
            .map(|choice| choice.label.clone())
            .unwrap_or_else(|| "Select Prefab".to_string());

        y += BTN_HEIGHT + CONTROL_GAP;

        let action_row_rect = Rect::new(rect.x + PANEL_PADDING, y, content_w, BTN_HEIGHT);
        let (file_rect, edit_rect) = prefab_palette_action_row_rects(action_row_rect);
        let pick_file = Button::new(file_rect, "File")
            .interaction_id(self.file_button_id)
            .suppressed(blocked)
            .show(ctx);
        if pick_file {
            pick_prefab_from_disk(editor);
        }

        let open_prefab_editor = Button::new(edit_rect, "Edit")
            .interaction_id(self.open_prefab_editor_button_id)
            .suppressed(blocked)
            .blocked(active_prefab.is_none())
            .show(ctx);
        if open_prefab_editor {
            if let Some(active_prefab) = active_prefab.as_ref() {
                editor.open_prefab_editor_for_id(active_prefab.prefab_id);
            }
        }

        y += BTN_HEIGHT + CONTROL_GAP;

        let cards_rect = Rect::new(
            rect.x + PANEL_PADDING,
            y,
            content_w,
            rect.bottom() - y - PANEL_PADDING,
        );
        let recent_ids = editor.room_editor.recent_prefab_ids.to_vec();
        let grid_layout = resolve_recent_prefab_grid_layout(cards_rect, recent_ids.len());
        let area = ScrollableArea::new(cards_rect, grid_layout.content_height.max(cards_rect.h))
            .blocked(prefab_palette_cards_blocked(
                blocked,
                is_mouse_over_dropdown_list(ctx),
            ))
            .begin(ctx, &mut self.recent_scroll);
        let cards_content = grid_layout.content_rect;
        let card_mouse =
            prefab_palette_card_mouse_position(cards_rect, ctx.mouse_position().into());

        ctx.push_clip_rect(cards_rect);
        for (index, prefab_id) in recent_ids.into_iter().enumerate() {
            let card_rect =
                recent_prefab_card_rect(cards_content, index, self.recent_scroll.scroll_y);
            if area.is_visible(card_rect.y, card_rect.h) {
                if let Some(prefab) = editor.game.prefab_manager.prefabs.get(&prefab_id).cloned() {
                    draw_recent_prefab_card(ctx, editor, blocked, card_rect, &prefab, card_mouse);
                }
            }
        }
        if editor.room_editor.recent_prefab_ids.is_empty() {
            ctx.draw_text(
                "No recent prefabs",
                cards_content.x + 4.0,
                cards_content.y + DEFAULT_FONT_SIZE_16,
                DEFAULT_FONT_SIZE_16,
                Color::GREY,
            );
        }

        area.draw_scrollbar(ctx, &self.recent_scroll);
        ctx.pop_clip_rect();

        if let Some(choice) = Dropdown::new(
            self.dropdown_id,
            dropdown_rect,
            &selected_label,
            &prefab_choices,
            |choice| choice.to_string(),
        )
        .filterable()
        .list_width(dropdown_rect.w)
        .truncate_trigger_text()
        .suppressed(blocked)
        .show(ctx)
        {
            let _ = editor.activate_prefab(choice.prefab_id);
        }
    }
}

fn prefab_choices(prefab_manager: &PrefabManager) -> Vec<PrefabChoice> {
    let mut choices = prefab_manager
        .prefabs
        .values()
        .map(|prefab| PrefabChoice {
            prefab_id: prefab.id,
            label: prefab.name.clone(),
        })
        .collect::<Vec<_>>();

    choices.sort_by(|left, right| {
        let label_cmp = left.label.to_lowercase().cmp(&right.label.to_lowercase());
        if label_cmp == Ordering::Equal {
            left.prefab_id.0.cmp(&right.prefab_id.0)
        } else {
            label_cmp
        }
    });
    choices
}

fn active_prefab_choice(
    prefab_manager: &PrefabManager,
    active_prefab_id: Option<PrefabId>,
) -> Option<PrefabChoice> {
    active_prefab_id.and_then(|prefab_id| {
        prefab_manager
            .prefabs
            .get(&prefab_id)
            .map(|prefab| PrefabChoice {
                prefab_id,
                label: prefab.name.clone(),
            })
    })
}

fn prefab_palette_action_row_rects(row_rect: Rect) -> (Rect, Rect) {
    let button_w = ((row_rect.w - CONTROL_GAP).max(0.0)) * 0.5;
    let file_rect = Rect::new(row_rect.x, row_rect.y, button_w, row_rect.h);
    let edit_rect = Rect::new(
        row_rect.x + button_w + CONTROL_GAP,
        row_rect.y,
        button_w,
        row_rect.h,
    );
    (file_rect, edit_rect)
}

fn prefab_palette_cards_blocked(blocked: bool, mouse_over_dropdown: bool) -> bool {
    blocked || mouse_over_dropdown
}

fn prefab_palette_card_mouse_position(cards_rect: Rect, mouse: Vec2) -> Vec2 {
    if cards_rect.contains(mouse) {
        mouse
    } else {
        Vec2::new(-1.0, -1.0)
    }
}

fn pick_prefab_from_disk(editor: &mut Editor) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Prefab RON", &["ron"])
        .set_directory(prefabs_folder())
        .pick_file()
    else {
        return;
    };

    if !path.starts_with(prefabs_folder()) {
        editor.toast = Some(Toast::new(
            "Selected prefab must be inside this project's prefabs folder.",
            3.0,
        ));
        return;
    }

    match load_prefab_asset_from_path(&path) {
        Ok(prefab) => {
            editor
                .game
                .prefab_manager
                .prefabs
                .insert(prefab.id, prefab.clone());
            let _ = editor.activate_prefab(prefab.id);
        }
        Err(error) => {
            editor.toast = Some(Toast::new(format!("Could not load prefab: {error}"), 3.0));
        }
    }
}

fn load_prefab_asset_from_path(path: &Path) -> io::Result<PrefabAsset> {
    let ron = fs::read_to_string(path)?;
    let prefab = ron::from_str(&ron).map_err(|error| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Could not parse prefab '{}': {error}", path.display()),
        )
    })?;
    validate_prefab(&prefab)?;
    Ok(prefab)
}

fn draw_recent_prefab_card(
    ctx: &mut WgpuContext,
    editor: &mut Editor,
    blocked: bool,
    rect: Rect,
    prefab: &PrefabAsset,
    mouse_position: Vec2,
) {
    let active = editor.room_editor.active_prefab_id == Some(prefab.id);
    let border = if active { Color::YELLOW } else { Color::WHITE };
    ctx.draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.18, 0.18, 0.20, 1.0),
    );
    ctx.draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, border);

    let footer_rect = Rect::new(
        rect.x + PREVIEW_PADDING,
        rect.bottom() - NAME_H - PREVIEW_PADDING,
        rect.w - PREVIEW_PADDING * 2.0,
        NAME_H,
    );
    let preview_rect = Rect::new(
        rect.x + PREVIEW_PADDING,
        rect.y + PREVIEW_PADDING,
        rect.w - PREVIEW_PADDING * 2.0,
        footer_rect.y - rect.y - PREVIEW_PADDING,
    );
    let preview = build_prefab_preview(
        ctx,
        prefab,
        &mut editor.game.asset_registry,
        &mut editor.game.sprite_manager,
    );
    if preview.has_drawable_visual {
        draw_prefab_preview(ctx, &mut editor.game.sprite_manager, preview_rect, &preview);
    } else {
        draw_prefab_fallback(ctx, preview_rect);
    }

    ctx.draw_rectangle(
        footer_rect.x,
        footer_rect.y,
        footer_rect.w,
        footer_rect.h,
        Color::new(0.10, 0.10, 0.12, 0.95),
    );

    let name = sanitise_name(&prefab.name);
    let truncated_name = truncate_to_width(
        ctx,
        &name,
        footer_rect.w - WIDGET_PADDING,
        DEFAULT_FONT_SIZE_16,
    );
    let text_dims = measure_text(ctx, &truncated_name, DEFAULT_FONT_SIZE_16);
    let text_x = footer_rect.x + (footer_rect.w - text_dims.width).max(0.0) * 0.5;
    let text_y = footer_rect.y + footer_rect.h - 10.0;
    ctx.draw_text(
        &truncated_name,
        text_x,
        text_y,
        DEFAULT_FONT_SIZE_16,
        Color::WHITE,
    );

    let clicked = Button::new(rect, "")
        .mouse_position(mouse_position)
        .plain()
        .suppressed(blocked)
        .show(ctx);
    if clicked {
        let _ = editor.activate_prefab(prefab.id);
    }
}

fn draw_prefab_fallback(ctx: &mut WgpuContext, rect: Rect) {
    let size = rect.w.min(rect.h).min(72.0);
    let x = rect.x + (rect.w - size) * 0.5;
    let y = rect.y + (rect.h - size) * 0.5;
    ctx.draw_texture_ex(
        entity_icon(),
        x,
        y,
        Color::WHITE,
        DrawTextureParams {
            dest_size: Some(Vec2::splat(size)),
            ..Default::default()
        },
    );
}

fn draw_prefab_preview(
    ctx: &mut WgpuContext,
    sprite_manager: &mut SpriteManager,
    rect: Rect,
    preview: &PrefabPreview,
) {
    let width = preview.stamp_bounds.w.max(1.0);
    let height = preview.stamp_bounds.h.max(1.0);
    let scale = ((rect.w - PREVIEW_PADDING * 2.0) / width)
        .min((rect.h - PREVIEW_PADDING * 2.0) / height)
        .max(0.01);
    let draw_w = width * scale;
    let draw_h = height * scale;
    let origin = Vec2::new(
        rect.x + (rect.w - draw_w) * 0.5,
        rect.y + (rect.h - draw_h) * 0.5,
    );
    let bounds_origin = Vec2::new(preview.stamp_bounds.x, preview.stamp_bounds.y);

    for item in &preview.items {
        let draw_pos = origin + (item.stamp_position - bounds_origin) * scale;
        match item.visual {
            PrefabPreviewVisual::Sprite { sprite_id } => {
                let texture = sprite_manager.get_texture_from_id(ctx, sprite_id);
                ctx.draw_texture_ex(
                    texture,
                    draw_pos.x,
                    draw_pos.y,
                    Color::WHITE,
                    DrawTextureParams {
                        dest_size: Some(item.size * scale),
                        ..Default::default()
                    },
                );
            }
            PrefabPreviewVisual::CurrentFrame {
                sprite_id,
                source,
                flip_x,
            } => {
                let texture = sprite_manager.get_texture_from_id(ctx, sprite_id);
                ctx.draw_texture_ex(
                    texture,
                    draw_pos.x,
                    draw_pos.y,
                    Color::WHITE,
                    DrawTextureParams {
                        dest_size: Some(item.size * scale),
                        source: Some(source),
                        flip_x,
                        ..Default::default()
                    },
                );
            }
            PrefabPreviewVisual::Placeholder => {
                ctx.draw_rectangle(
                    draw_pos.x,
                    draw_pos.y,
                    item.size.x * scale,
                    item.size.y * scale,
                    Color::GREEN,
                );
            }
        }
    }
}

/// Returns the default prefab palette rect for the given viewport.
pub(crate) fn prefab_palette_default_rect(_screen_width: f32) -> Rect {
    Rect::new(PANEL_LEFT_MARGIN, PANEL_TOP, PANEL_W, PANEL_H)
}

/// Returns the side length for a recent prefab card.
pub(crate) fn prefab_palette_recent_card_size(cards_rect: Rect) -> f32 {
    ((cards_rect.w - CARD_GAP).max(0.0)) / RECENT_CARD_COLUMNS as f32
}

fn resolve_recent_prefab_grid_layout(cards_rect: Rect, count: usize) -> RecentPrefabGridLayout {
    let full_width_content_height = recent_prefab_cards_content_height(cards_rect, count);
    let reserve_scrollbar = full_width_content_height > cards_rect.h;
    let content_rect = if reserve_scrollbar {
        Rect::new(
            cards_rect.x,
            cards_rect.y,
            (cards_rect.w - SCROLLBAR_CARD_GAP).max(0.0),
            cards_rect.h,
        )
    } else {
        cards_rect
    };
    let content_height = recent_prefab_cards_content_height(content_rect, count);

    RecentPrefabGridLayout {
        content_rect,
        content_height,
    }
}

/// Returns the rect for a recent prefab card in the two-column grid.
pub(crate) fn recent_prefab_card_rect(cards_rect: Rect, index: usize, scroll_y: f32) -> Rect {
    let card_size = prefab_palette_recent_card_size(cards_rect);
    let column = (index % RECENT_CARD_COLUMNS) as f32;
    let row = (index / RECENT_CARD_COLUMNS) as f32;
    let x = cards_rect.x + column * (card_size + CARD_GAP);
    let y = cards_rect.y + scroll_y + row * (card_size + CARD_GAP);
    Rect::new(x, y, card_size, card_size)
}

/// Returns the scrollable content height for the recent prefab grid.
pub(crate) fn recent_prefab_cards_content_height(cards_rect: Rect, count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }

    let card_size = prefab_palette_recent_card_size(cards_rect);
    let rows = count.div_ceil(RECENT_CARD_COLUMNS);
    rows as f32 * card_size + (rows.saturating_sub(1) as f32 * CARD_GAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefab_palette_recent_card_layout_is_two_column_and_square() {
        let cards_rect = Rect::new(10.0, 100.0, 280.0, 400.0);

        let first = recent_prefab_card_rect(cards_rect, 0, 0.0);
        let second = recent_prefab_card_rect(cards_rect, 1, 0.0);
        let third = recent_prefab_card_rect(cards_rect, 2, 0.0);

        assert_eq!(first.w, first.h);
        assert_eq!(second.w, second.h);
        assert_eq!(third.w, third.h);
        assert_eq!(first.w, 134.0);
        assert_eq!(second.x, first.x + first.w + CARD_GAP);
        assert_eq!(second.y, first.y);
        assert_eq!(third.x, first.x);
        assert_eq!(third.y, first.y + first.h + CARD_GAP);
    }

    #[test]
    fn prefab_palette_recent_cards_content_height_matches_rows() {
        let cards_rect = Rect::new(10.0, 100.0, 280.0, 400.0);

        assert_eq!(recent_prefab_cards_content_height(cards_rect, 0), 0.0);
        assert_eq!(recent_prefab_cards_content_height(cards_rect, 1), 134.0);
        assert_eq!(recent_prefab_cards_content_height(cards_rect, 2), 134.0);
        assert_eq!(recent_prefab_cards_content_height(cards_rect, 3), 280.0);
    }

    #[test]
    fn prefab_palette_cards_block_when_dropdown_is_hovered() {
        assert!(!prefab_palette_cards_blocked(false, false));
        assert!(prefab_palette_cards_blocked(false, true));
        assert!(prefab_palette_cards_blocked(true, false));
    }

    #[test]
    fn prefab_palette_cards_only_hit_test_inside_viewport() {
        let cards_rect = Rect::new(10.0, 100.0, 280.0, 400.0);
        let inside_mouse = Vec2::new(50.0, 150.0);
        let outside_mouse = Vec2::new(50.0, 520.0);

        assert_eq!(
            prefab_palette_card_mouse_position(cards_rect, inside_mouse),
            inside_mouse,
        );
        assert_eq!(
            prefab_palette_card_mouse_position(cards_rect, outside_mouse),
            Vec2::new(-1.0, -1.0),
        );
    }

    #[test]
    fn active_prefab_choice_returns_active_prefab_when_present() {
        let prefab_id = PrefabId(7);
        let mut prefab_manager = PrefabManager::default();
        prefab_manager
            .prefabs
            .insert(prefab_id, create_prefab(prefab_id, "Crate".to_string()));

        let choice = active_prefab_choice(&prefab_manager, Some(prefab_id))
            .expect("active prefab should resolve");

        assert_eq!(choice.prefab_id, prefab_id);
        assert_eq!(choice.label, "Crate");
    }

    #[test]
    fn active_prefab_choice_is_none_when_open_prefab_should_be_semantically_blocked() {
        let prefab_id = PrefabId(7);
        let mut prefab_manager = PrefabManager::default();
        prefab_manager
            .prefabs
            .insert(prefab_id, create_prefab(prefab_id, "Crate".to_string()));

        assert!(active_prefab_choice(&prefab_manager, None).is_none());
        assert!(active_prefab_choice(&prefab_manager, Some(PrefabId(999))).is_none());
        assert!(active_prefab_choice(&prefab_manager, Some(prefab_id)).is_some());
    }
}
