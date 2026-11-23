use crate::app::{Editor, EditorMode};
use crate::game::GameEditorSubmode;
use crate::gui::panels::generic_panel::{PanelDefinition, PanelResizeLimits};
use bishop::prelude::{MouseButton, Rect, WgpuContext};
use bishop::{Input, Text};
use engine_core::theme::with_theme;
use engine_core::worlds::{World, WorldId};
use widgets::constants::layout;
use widgets::{
    matches_query_normalized,
    Button,
    ScrollState,
    ScrollableArea,
    TextInput,
    Widget,
    WidgetId,
};

const CONTENT_PADDING: f32 = 8.0;
const ROW_HEIGHT: f32 = 28.0;
const ROW_GAP: f32 = 4.0;
const SEARCH_FIELD_HEIGHT: f32 = 26.0;
const SEARCH_GAP: f32 = 6.0;
const SEARCH_WIDGET_ID: WidgetId = WidgetId(0x574f_524c);

pub(crate) const WORLD_BROWSER_PANEL: &str = "Worlds";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorldBrowserEmptyState {
    NoWorlds,
    NoMatches,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorldBrowserRowAction {
    Select(WorldId),
    Enter(WorldId),
}

pub struct WorldBrowserPanel {
    query: String,
    scroll_state: ScrollState,
}

impl WorldBrowserPanel {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            scroll_state: ScrollState::new(),
        }
    }
}

impl PanelDefinition for WorldBrowserPanel {
    fn title(&self) -> &'static str {
        WORLD_BROWSER_PANEL
    }

    fn default_rect(&self, _ctx: &WgpuContext) -> Rect {
        Rect::new(20.0, 60.0, 260.0, 440.0)
    }

    fn resize_limits(&self) -> Option<PanelResizeLimits> {
        Some(PanelResizeLimits {
            min_width: 180.0,
            min_height: 150.0,
        })
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        if !matches!(editor.mode, EditorMode::Game(GameEditorSubmode::Worlds)) {
            return;
        }

        let search_rect = Rect::new(
            rect.x + CONTENT_PADDING,
            rect.y + CONTENT_PADDING,
            (rect.w - CONTENT_PADDING * 2.0).max(0.0),
            SEARCH_FIELD_HEIGHT,
        );
        let (query, _) = TextInput::new(SEARCH_WIDGET_ID, search_rect, &self.query)
            .clear_button()
            .blocked(blocked)
            .show(ctx);
        self.query = query;

        let world_count = editor.game.worlds().len();
        let matching_count = filter_worlds(editor.game.worlds(), &self.query).count();
        let row_count = matching_count.max(1);
        let content_height = CONTENT_PADDING * 2.0
            + row_count as f32 * ROW_HEIGHT
            + row_count.saturating_sub(1) as f32 * ROW_GAP;
        let scroll_top = search_rect.bottom() + SEARCH_GAP;
        let scroll_rect = Rect::new(
            rect.x,
            scroll_top,
            rect.w,
            (rect.bottom() - scroll_top).max(0.0),
        );
        let area = ScrollableArea::new(scroll_rect, content_height)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);
        let mut y = scroll_rect.y + CONTENT_PADDING + self.scroll_state.scroll_y;
        let row_width = (area.usable_width() - CONTENT_PADDING).max(0.0);
        let mouse = ctx.mouse_position().into();

        let mut row_action = None;
        ctx.push_clip_rect(scroll_rect);
        if matching_count == 0 {
            let empty_state = world_browser_empty_state(world_count);
            ctx.draw_text(
                empty_state.label(),
                rect.x + CONTENT_PADDING,
                y + ROW_HEIGHT * 0.7,
                layout::DEFAULT_FONT_SIZE_16,
                with_theme(|theme| theme.text_muted),
            );
        } else {
            for world in filter_worlds(editor.game.worlds(), &self.query) {
                if area.is_visible(y, ROW_HEIGHT) {
                    let row_rect = Rect::new(rect.x + CONTENT_PADDING, y, row_width, ROW_HEIGHT);
                    let clicked = Button::new(row_rect, &world.name)
                        .active(editor.game_editor.is_world_selected(world.id))
                        .suppressed(blocked)
                        .show(ctx);

                    if !blocked
                        && ctx.is_mouse_button_pressed(MouseButton::Left)
                        && row_rect.contains(mouse)
                        && ctx.is_mouse_button_double_clicked(MouseButton::Left)
                    {
                        row_action = Some(world_row_action(world.id, true));
                    } else if !blocked && clicked {
                        row_action = Some(world_row_action(world.id, false));
                    }
                }
                y += ROW_HEIGHT + ROW_GAP;
            }
        }
        ctx.pop_clip_rect();
        area.draw_scrollbar(ctx, &self.scroll_state);

        if let Some(action) = row_action {
            let world_id = match action {
                WorldBrowserRowAction::Select(world_id)
                | WorldBrowserRowAction::Enter(world_id) => world_id,
            };
            editor.game_editor.select_world_from_browser(
                ctx,
                &mut editor.camera,
                &mut editor.game,
                world_id,
            );
            if matches!(action, WorldBrowserRowAction::Enter(_)) {
                editor.game_editor.request_world_entry_from_browser(world_id);
            }
        }
    }
}

fn filter_worlds<'a>(worlds: &'a [World], query: &str) -> impl Iterator<Item = &'a World> + 'a {
    let normalized_query = query.to_lowercase();
    let has_query = !normalized_query.is_empty();
    worlds
        .iter()
        .filter(move |world| !has_query || matches_query_normalized(&world.name, &normalized_query))
}

fn world_browser_empty_state(world_count: usize) -> WorldBrowserEmptyState {
    if world_count == 0 {
        WorldBrowserEmptyState::NoWorlds
    } else {
        WorldBrowserEmptyState::NoMatches
    }
}

impl WorldBrowserEmptyState {
    fn label(self) -> &'static str {
        match self {
            Self::NoWorlds => "No worlds yet",
            Self::NoMatches => "No matching worlds",
        }
    }
}

fn world_row_action(world_id: WorldId, double_clicked: bool) -> WorldBrowserRowAction {
    if double_clicked {
        WorldBrowserRowAction::Enter(world_id)
    } else {
        WorldBrowserRowAction::Select(world_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        filter_worlds,
        world_browser_empty_state,
        world_row_action,
        WorldBrowserEmptyState,
        WorldBrowserRowAction,
    };
    use engine_core::worlds::{World, WorldId};

    fn worlds() -> [World; 3] {
        [
            World::new(WorldId(1), "Cavern".to_owned(), 16.0),
            World::new(WorldId(2), "Overworld".to_owned(), 16.0),
            World::new(WorldId(3), "Cavern Exit".to_owned(), 16.0),
        ]
    }

    #[test]
    fn world_browser_search_filters_names_case_insensitively_and_preserves_game_order() {
        let worlds = worlds();

        let matches = filter_worlds(&worlds, "CAVERN")
            .map(|world| world.id)
            .collect::<Vec<_>>();

        assert_eq!(matches, vec![WorldId(1), WorldId(3)]);
    }

    #[test]
    fn world_browser_empty_search_returns_all_worlds_in_game_order() {
        let worlds = worlds();

        let matches = filter_worlds(&worlds, "")
            .map(|world| world.id)
            .collect::<Vec<_>>();

        assert_eq!(matches, vec![WorldId(1), WorldId(2), WorldId(3)]);
    }

    #[test]
    fn world_browser_empty_state_distinguishes_empty_game_and_no_matches() {
        assert_eq!(
            world_browser_empty_state(0),
            WorldBrowserEmptyState::NoWorlds
        );
        assert_eq!(
            world_browser_empty_state(3),
            WorldBrowserEmptyState::NoMatches
        );
    }

    #[test]
    fn world_browser_single_click_selects_without_entering() {
        assert_eq!(
            world_row_action(WorldId(2), false),
            WorldBrowserRowAction::Select(WorldId(2))
        );
    }

    #[test]
    fn world_browser_double_click_requests_entry() {
        assert_eq!(
            world_row_action(WorldId(2), true),
            WorldBrowserRowAction::Enter(WorldId(2))
        );
    }
}
