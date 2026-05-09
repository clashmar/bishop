use crate::app::EditorMode;
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::cmp::Ordering;
use widgets::constants::layout;

pub(crate) const PREFAB_BROWSER_PANEL: &str = "Prefab Browser";
const CONTENT_PADDING: f32 = 8.0;
const ROW_HEIGHT: f32 = 28.0;
const ROW_GAP: f32 = 6.0;

/// Prefab-only browser panel for opening existing prefabs.
pub struct PrefabBrowserPanel {
    scroll_state: ScrollState,
}

impl PrefabBrowserPanel {
    /// Creates a new prefab browser panel.
    pub fn new() -> Self {
        Self {
            scroll_state: ScrollState::new(),
        }
    }
}

impl PanelDefinition for PrefabBrowserPanel {
    fn title(&self) -> &'static str {
        PREFAB_BROWSER_PANEL
    }

    fn default_rect(&self, ctx: &WgpuContext) -> Rect {
        Rect::new(ctx.screen_width() - 290.0, 60.0, 270.0, 400.0)
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        if !matches!(editor.mode, EditorMode::Prefab(_)) {
            return;
        }

        let entries = prefab_browser_entries(&editor.game.prefab_manager);
        let content_height = if entries.is_empty() {
            CONTENT_PADDING * 2.0 + ROW_HEIGHT
        } else {
            CONTENT_PADDING * 2.0
                + entries.len() as f32 * ROW_HEIGHT
                + entries.len().saturating_sub(1) as f32 * ROW_GAP
        };
        let area = ScrollableArea::new(rect, content_height)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);

        let mut y = rect.y + CONTENT_PADDING + self.scroll_state.scroll_y;
        let row_width = area.usable_width() - CONTENT_PADDING;

        ctx.push_clip_rect(rect);

        if entries.is_empty() {
            if area.is_visible(y, ROW_HEIGHT) {
                ctx.draw_text(
                    "No prefabs available",
                    rect.x + CONTENT_PADDING,
                    y + ROW_HEIGHT * 0.75,
                    layout::DEFAULT_FONT_SIZE_16,
                    Color::GREY,
                );
            }
        } else {
            for (prefab_id, prefab_name) in entries {
                if area.is_visible(y, ROW_HEIGHT) {
                    let row_rect = Rect::new(rect.x + CONTENT_PADDING, y, row_width, ROW_HEIGHT);
                    if Button::new(row_rect, prefab_name.as_str())
                        .suppressed(blocked)
                        .show(ctx)
                        && !blocked
                    {
                        editor.enter_prefab_transition(ctx, prefab_id);
                    }
                }

                y += ROW_HEIGHT + ROW_GAP;
            }
        }

        ctx.pop_clip_rect();
        area.draw_scrollbar(ctx, &self.scroll_state);
    }
}

/// Returns prefabs sorted alphabetically by name, tie-breaking by prefab id.
pub(crate) fn prefab_browser_entries(prefab_manager: &PrefabManager) -> Vec<(PrefabId, String)> {
    let mut entries = prefab_manager
        .prefabs
        .values()
        .map(|prefab| (prefab.id, prefab.name.clone()))
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        let label_cmp = left.1.to_lowercase().cmp(&right.1.to_lowercase());
        if label_cmp == Ordering::Equal {
            left.0 .0.cmp(&right.0 .0)
        } else {
            label_cmp
        }
    });

    entries
}
