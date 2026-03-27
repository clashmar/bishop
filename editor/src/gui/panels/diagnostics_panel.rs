// editor/src/gui/panels/diagnostics_panel.rs
use crate::editor_global::with_command_manager;
use crate::gui::panels::generic_panel::PanelDefinition;
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;

const ROW_HEIGHT: f32 = 16.0;
const SECTION_SPACING: f32 = 8.0;
const TOP_PADDING: f32 = 8.0;
const LEFT_PADDING: f32 = 8.0;
const FONT_SIZE: f32 = 13.0;
const HEADER_FONT_SIZE: f32 = 14.0;
const UPDATE_INTERVAL: f32 = 0.5; // Update every 500ms

pub struct DiagnosticsPanel {
    scroll_state: ScrollState,
    collector: DiagnosticsCollector,
    last_snapshot: Option<DiagnosticsSnapshot>,
    time_since_update: f32,
}

impl DiagnosticsPanel {
    pub fn new() -> Self {
        Self {
            scroll_state: ScrollState::new(),
            collector: DiagnosticsCollector::new(),
            last_snapshot: None,
            time_since_update: UPDATE_INTERVAL, // Force immediate update
        }
    }

    fn collect_metrics(&self, editor: &Editor) -> DiagnosticsSnapshot {
        let game = &editor.game;

        // Asset metrics
        let asset_metrics = AssetMetrics {
            texture_count: game.asset_manager.texture_count(),
            tile_def_count: game.asset_manager.tile_def_count(),
            sprite_id_count: game.asset_manager.sprite_id_to_path.len(),
            script_id_count: game.script_manager.script_id_to_path.len(),
        };

        // Script metrics
        let script_metrics = ScriptMetrics {
            loaded_count: game.script_manager.table_defs.len(),
            instance_count: game.script_manager.instances.len(),
            event_listener_count: game.script_manager.event_bus.listener_count(),
            ref_counts: game
                .script_manager
                .script_id_to_path
                .keys()
                .map(|id| (id.0, game.script_manager.get_ref_count(*id)))
                .collect(),
        };

        // ECS metrics
        let ecs = &game.ecs;
        let entity_count = ecs.get_store::<Transform>().data.len();
        let ecs_metrics = EcsMetrics {
            entity_count,
            component_store_count: ecs.stores.len(),
            components_by_type: std::collections::HashMap::new(),
        };

        // Command metrics
        let command_metrics = with_command_manager(|cm| CommandMetrics {
            undo_stack_size: cm.undo_stack_len(),
            redo_stack_size: cm.redo_stack_len(),
            pending_size: cm.pending_len(),
        });

        DiagnosticsSnapshot {
            frame: self.collector.frame_metrics.clone(),
            assets: asset_metrics,
            scripts: script_metrics,
            ecs: ecs_metrics,
            commands: command_metrics,
        }
    }

    fn draw_section_header(
        ctx: &mut WgpuContext,
        area: &ActiveScrollArea,
        label: &str,
        y: f32,
        rect: &Rect,
    ) -> f32 {
        if area.is_fully_visible(y, ROW_HEIGHT) {
            ctx.draw_text(
                label,
                rect.x + LEFT_PADDING,
                y + HEADER_FONT_SIZE,
                HEADER_FONT_SIZE,
                Color::YELLOW,
            );
        }
        y + ROW_HEIGHT + 4.0
    }

    fn draw_row(
        ctx: &mut WgpuContext,
        area: &ActiveScrollArea,
        label: &str,
        value: &str,
        y: f32,
        rect: &Rect,
        color: Color,
    ) -> f32 {
        if area.is_fully_visible(y, ROW_HEIGHT) {
            ctx.draw_text(
                label,
                rect.x + LEFT_PADDING + 8.0,
                y + FONT_SIZE,
                FONT_SIZE,
                Color::GREY,
            );
            let value_x = rect.x + rect.w * 0.55;
            ctx.draw_text(value, value_x, y + FONT_SIZE, FONT_SIZE, color);
        }
        y + ROW_HEIGHT
    }

    fn fps_color(fps: f32) -> Color {
        if fps >= 55.0 {
            Color::GREEN
        } else if fps >= 30.0 {
            Color::YELLOW
        } else {
            Color::RED
        }
    }
}

pub const DIAGNOSTICS_PANEL: &str = "Diagnostics";

impl PanelDefinition for DiagnosticsPanel {
    fn title(&self) -> &'static str {
        DIAGNOSTICS_PANEL
    }

    fn default_rect(&self, ctx: &WgpuContext) -> Rect {
        Rect::new(ctx.screen_width() - 280., 60., 260., 360.)
    }

    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool) {
        let dt = ctx.get_frame_time();

        // Update frame timing continuously
        self.collector.record_frame(dt);

        // Update full snapshot at intervals to reduce overhead
        self.time_since_update += dt;
        if self.time_since_update >= UPDATE_INTERVAL {
            self.time_since_update = 0.0;
            self.last_snapshot = Some(self.collect_metrics(editor));
        }

        // Draw content
        let snapshot = match &self.last_snapshot {
            Some(s) => s.clone(),
            None => return,
        };

        // Calculate content height
        let mut content_height = TOP_PADDING;
        content_height += ROW_HEIGHT + SECTION_SPACING; // Warnings header
        let warnings = self.collector.generate_warnings(&snapshot);
        content_height += warnings.len().max(1) as f32 * ROW_HEIGHT;
        content_height += SECTION_SPACING;

        content_height += ROW_HEIGHT + SECTION_SPACING; // Performance
        content_height += 5.0 * ROW_HEIGHT; // FPS, avg, min, max, render
        content_height += SECTION_SPACING;

        content_height += ROW_HEIGHT + SECTION_SPACING; // Assets
        content_height += 3.0 * ROW_HEIGHT; // Textures, Tile Defs, Sprite IDs
        content_height += SECTION_SPACING;

        content_height += ROW_HEIGHT + SECTION_SPACING; // Scripts
        content_height += 4.0 * ROW_HEIGHT; // Script IDs, Loaded, Instances, Listeners
        content_height += SECTION_SPACING;

        content_height += ROW_HEIGHT + SECTION_SPACING; // ECS
        content_height += 2.0 * ROW_HEIGHT;
        content_height += SECTION_SPACING;

        content_height += ROW_HEIGHT + SECTION_SPACING; // Undo/Redo
        content_height += 3.0 * ROW_HEIGHT;
        content_height += TOP_PADDING;

        let area = ScrollableArea::new(rect, content_height)
            .blocked(blocked)
            .begin(ctx, &mut self.scroll_state);

        let mut y = rect.y + self.scroll_state.scroll_y + TOP_PADDING;

        // Warnings section
        y = Self::draw_section_header(ctx, &area, "Warnings", y, &rect);
        if warnings.is_empty() {
            y = Self::draw_row(ctx, &area, "Status", "OK", y, &rect, Color::GREEN);
        } else {
            for warning in &warnings {
                y = Self::draw_row(
                    ctx,
                    &area,
                    "!",
                    &warning.description(),
                    y,
                    &rect,
                    Color::RED,
                );
            }
        }
        y += SECTION_SPACING;

        // Performance section
        y = Self::draw_section_header(ctx, &area, "Performance", y, &rect);
        let fps = snapshot.frame.fps;
        y = Self::draw_row(
            ctx,
            &area,
            "FPS",
            &format!("{:.1}", fps),
            y,
            &rect,
            Self::fps_color(fps),
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Avg",
            &format!("{:.2} ms", snapshot.frame.avg_frame_time_ms),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Min",
            &format!("{:.2} ms", snapshot.frame.min_frame_time_ms),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Max",
            &format!("{:.2} ms", snapshot.frame.max_frame_time_ms),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Render",
            &format!("{:.2} ms", editor.render_system.render_time_ms),
            y,
            &rect,
            Color::WHITE,
        );
        y += SECTION_SPACING;

        // Assets section
        y = Self::draw_section_header(ctx, &area, "Assets", y, &rect);
        y = Self::draw_row(
            ctx,
            &area,
            "Textures",
            &snapshot.assets.texture_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Tile Defs",
            &snapshot.assets.tile_def_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Sprite IDs",
            &snapshot.assets.sprite_id_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y += SECTION_SPACING;

        // Scripts section
        y = Self::draw_section_header(ctx, &area, "Scripts", y, &rect);
        y = Self::draw_row(
            ctx,
            &area,
            "Script IDs",
            &snapshot.assets.script_id_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Loaded",
            &snapshot.scripts.loaded_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Instances",
            &snapshot.scripts.instance_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Listeners",
            &snapshot.scripts.event_listener_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y += SECTION_SPACING;

        // ECS section
        y = Self::draw_section_header(ctx, &area, "ECS", y, &rect);
        y = Self::draw_row(
            ctx,
            &area,
            "Entities",
            &snapshot.ecs.entity_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Component Stores",
            &snapshot.ecs.component_store_count.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y += SECTION_SPACING;

        // Undo/Redo section
        y = Self::draw_section_header(ctx, &area, "Undo/Redo", y, &rect);
        y = Self::draw_row(
            ctx,
            &area,
            "Undo Stack",
            &snapshot.commands.undo_stack_size.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        y = Self::draw_row(
            ctx,
            &area,
            "Redo Stack",
            &snapshot.commands.redo_stack_size.to_string(),
            y,
            &rect,
            Color::WHITE,
        );
        let _ = Self::draw_row(
            ctx,
            &area,
            "Pending",
            &snapshot.commands.pending_size.to_string(),
            y,
            &rect,
            Color::WHITE,
        );

        area.draw_scrollbar(ctx, self.scroll_state.scroll_y);
    }
}
