// game/src/diagnostics/overlay.rs
//! In-game diagnostics overlay toggled with F3/F4.

use crate::engine::game_instance::GameInstance;
use engine_core::prelude::*;
use std::collections::{HashMap, HashSet};

/// Detail level for the diagnostics overlay.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayDetailLevel {
    /// Overlay is hidden.
    #[default]
    Off,
    /// Show basic metrics (FPS only).
    Basic,
    /// Show detailed metrics.
    Detailed,
}

impl OverlayDetailLevel {
    /// Cycle to the next detail level.
    pub fn cycle(self) -> Self {
        match self {
            OverlayDetailLevel::Off => OverlayDetailLevel::Basic,
            OverlayDetailLevel::Basic => OverlayDetailLevel::Detailed,
            OverlayDetailLevel::Detailed => OverlayDetailLevel::Off,
        }
    }
}

/// Runtime diagnostics overlay for the game.
pub struct DiagnosticsOverlay {
    /// Current detail level.
    pub detail_level: OverlayDetailLevel,
    /// Metrics collector.
    collector: DiagnosticsCollector,
    /// Cached metrics for display.
    cached_fps: f32,
    cached_frame_time: f32,
    cached_render_time: f32,
    cached_entity_count: usize,
    cached_texture_count: usize,
    cached_script_instances: usize,
    cached_listener_count: usize,
    cached_script_id_count: usize,
    cached_sprite_id_count: usize,
    cached_audio_working_set_resident: usize,
    cached_audio_working_set_total: usize,
    cached_audio_count: usize,
    cached_audio_pinned_count: usize,
    cached_audio_matching_refs: usize,
    cached_audio_checked_refs: usize,
    cached_audio_rows: Vec<AudioDiagnosticsRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AudioDiagnosticsRow {
    id: String,
    cached: bool,
    pinned: bool,
    ref_count: usize,
    ecs_count: usize,
}

impl AudioDiagnosticsRow {
    fn is_attention(&self) -> bool {
        self.ecs_count != self.ref_count || (self.ecs_count > 0 && !self.cached)
    }

    fn display_line(&self) -> String {
        let mut line = format!("Audio {} rc={} ecs={}", self.id, self.ref_count, self.ecs_count);
        if self.cached {
            line.push_str(" cached");
        }
        if self.pinned {
            line.push_str(" pinned");
        }
        line
    }
}

impl Default for DiagnosticsOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsOverlay {
    pub fn new() -> Self {
        Self {
            detail_level: OverlayDetailLevel::Off,
            collector: DiagnosticsCollector::new(),
            cached_fps: 0.0,
            cached_frame_time: 0.0,
            cached_render_time: 0.0,
            cached_entity_count: 0,
            cached_texture_count: 0,
            cached_script_instances: 0,
            cached_listener_count: 0,
            cached_script_id_count: 0,
            cached_sprite_id_count: 0,
            cached_audio_working_set_resident: 0,
            cached_audio_working_set_total: 0,
            cached_audio_count: 0,
            cached_audio_pinned_count: 0,
            cached_audio_matching_refs: 0,
            cached_audio_checked_refs: 0,
            cached_audio_rows: Vec::new(),
        }
    }

    /// Toggle the overlay on/off.
    pub fn toggle(&mut self) {
        self.detail_level = if self.detail_level == OverlayDetailLevel::Off {
            OverlayDetailLevel::Basic
        } else {
            OverlayDetailLevel::Off
        };
    }

    /// Cycle through detail levels.
    pub fn cycle_detail(&mut self) {
        self.detail_level = self.detail_level.cycle();
    }

    /// Update frame timing metrics.
    pub fn update(&mut self, dt: f32) {
        self.collector.record_frame(dt);
        self.cached_fps = self.collector.frame_metrics.fps;
        self.cached_frame_time = self.collector.frame_metrics.avg_frame_time_ms;
    }

    /// Pulls current metrics from the game instance and render system.
    pub fn update_from_game(
        &mut self,
        game_instance: &GameInstance,
        render_time_ms: f32,
        audio_manager: &AudioManager,
    ) {
        let game = &game_instance.game;
        let audio_snapshot = audio_manager.diagnostics_snapshot();
        self.cached_entity_count = game.ecs.get_store::<Transform>().data.len();
        self.cached_texture_count = game.asset_manager.texture_count();
        self.cached_script_instances = game.script_manager.instances.len();
        self.cached_listener_count = game.script_manager.event_bus.listener_count();
        self.cached_script_id_count = game.script_manager.script_id_to_path.len();
        self.cached_sprite_id_count = game.asset_manager.sprite_id_to_path.len();
        self.cached_render_time = render_time_ms;

        let audio_sources = AudioSource::store(&game.ecs);
        let expected_audio_refs = expected_audio_ref_counts(audio_sources.data.values());
        let audio_rows = all_audio_diagnostics_rows(&expected_audio_refs, &audio_snapshot);

        self.cached_audio_working_set_resident = audio_rows
            .iter()
            .filter(|row| row.ecs_count > 0 && row.cached)
            .count();
        self.cached_audio_working_set_total = expected_audio_refs.len();
        self.cached_audio_count = audio_snapshot.cached_sound_count;
        self.cached_audio_pinned_count = audio_snapshot.pinned_sound_count;
        let (matching_refs, checked_refs) = audio_ref_summary(&audio_rows);
        self.cached_audio_matching_refs = matching_refs;
        self.cached_audio_checked_refs = checked_refs;
        self.cached_audio_rows = audio_diagnostics_rows(&expected_audio_refs, &audio_snapshot);
    }

    /// Handle input for toggling the overlay.
    pub fn handle_input(
        &mut self, 
        ctx: &mut impl BishopContext,
    ) {
        if ctx.is_key_pressed(KeyCode::F3) {
            self.toggle();
        }
        if ctx.is_key_pressed(KeyCode::F4) {
            self.cycle_detail();
        }
    }

    /// Draw the overlay.
    pub fn draw<C: BishopContext>(
        &self,
        ctx: &mut C,
    ) {
        if self.detail_level == OverlayDetailLevel::Off {
            return;
        }

        const PADDING: f32 = 10.0;
        const LINE_HEIGHT: f32 = 18.0;
        const FONT_SIZE: f32 = 14.0;
        const BG_ALPHA: f32 = 0.7;

        let mut lines: Vec<String> = Vec::new();

        // FPS line
        let fps_str = format!("FPS: {:.1}", self.cached_fps);
        lines.push(fps_str);

        if self.detail_level == OverlayDetailLevel::Detailed {
            lines.push(format!("Frame: {:.2} ms", self.cached_frame_time));
            lines.push(format!("Render: {:.2} ms", self.cached_render_time));
            lines.push(format!("Entities: {}", self.cached_entity_count));
            lines.push(format!("Textures: {}", self.cached_texture_count));
            lines.push(format!("Sprite IDs: {}", self.cached_sprite_id_count));
            lines.push(format!("Script IDs: {}", self.cached_script_id_count));
            lines.push(format!("Script Instances: {}", self.cached_script_instances));
            lines.push(format!("Listeners: {}", self.cached_listener_count));
            lines.push(format!(
                "Audio Working Set: {}/{}",
                self.cached_audio_working_set_resident,
                self.cached_audio_working_set_total
            ));
            lines.push(format!(
                "Audio Cache: {} cached, {} pinned",
                self.cached_audio_count,
                self.cached_audio_pinned_count
            ));
            lines.push(format!(
                "Audio Refs: {}/{} IDs match ECS",
                self.cached_audio_matching_refs,
                self.cached_audio_checked_refs
            ));
            lines.extend(self.cached_audio_rows.iter().map(AudioDiagnosticsRow::display_line));
        }

        // Calculate background size
        let max_width = lines
            .iter()
            .map(|s| ctx.measure_text(s, FONT_SIZE).width)
            .fold(0.0_f32, f32::max);

        let bg_width = max_width + PADDING * 2.0;
        let bg_height = lines.len() as f32 * LINE_HEIGHT + PADDING * 2.0;

        // Draw background
        ctx.draw_rectangle(
            PADDING,
            PADDING,
            bg_width,
            bg_height,
            Color::new(0.0, 0.0, 0.0, BG_ALPHA),
        );

        // Draw text
        let fps_color = Self::fps_color(self.cached_fps);

        for (i, line) in lines.iter().enumerate() {
            let color = if i == 0 { fps_color } else { Color::WHITE };
            let y = PADDING * 2.0 + LINE_HEIGHT * i as f32;
            ctx.draw_text(line, PADDING * 2.0, y + FONT_SIZE, FONT_SIZE, color);
        }
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

fn expected_audio_ref_counts<'a>(
    sources: impl IntoIterator<Item = &'a AudioSource>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();

    for source in sources {
        for id in source.all_sound_ids() {
            *counts.entry(id).or_insert(0) += 1;
        }
    }

    counts
}

fn audio_diagnostics_rows(
    ecs_counts: &HashMap<String, usize>,
    snapshot: &AudioDiagnosticsSnapshot,
) -> Vec<AudioDiagnosticsRow> {
    let mut rows = all_audio_diagnostics_rows(ecs_counts, snapshot);
    rows.truncate(6);
    rows
}

fn all_audio_diagnostics_rows(
    ecs_counts: &HashMap<String, usize>,
    snapshot: &AudioDiagnosticsSnapshot,
) -> Vec<AudioDiagnosticsRow> {
    let mut snapshot_entries = snapshot
        .entries
        .iter()
        .map(|entry| (entry.id.clone(), entry))
        .collect::<HashMap<_, _>>();
    let mut ids = ecs_counts.keys().cloned().collect::<HashSet<_>>();
    ids.extend(snapshot_entries.keys().cloned());

    let mut rows = ids
        .into_iter()
        .map(|id| {
            let snapshot_entry = snapshot_entries.remove(&id);

            AudioDiagnosticsRow {
                cached: snapshot_entry.is_some_and(|entry| entry.cached),
                pinned: snapshot_entry.is_some_and(|entry| entry.pinned),
                ref_count: snapshot_entry.map(|entry| entry.ref_count).unwrap_or(0),
                ecs_count: ecs_counts.get(&id).copied().unwrap_or(0),
                id,
            }
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        right
            .is_attention()
            .cmp(&left.is_attention())
            .then_with(|| left.id.cmp(&right.id))
    });

    rows
}

fn audio_ref_summary(rows: &[AudioDiagnosticsRow]) -> (usize, usize) {
    let relevant_rows = rows
        .iter()
        .filter(|row| row.ecs_count > 0 || row.ref_count > 0)
        .collect::<Vec<_>>();
    let matching = relevant_rows
        .iter()
        .filter(|row| row.ecs_count == row.ref_count)
        .count();

    (matching, relevant_rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn audio_source(sound_groups: &[(&str, &[&str])]) -> AudioSource {
        let mut source = AudioSource::default();

        for (group_name, sounds) in sound_groups {
            source.groups.insert(
                SoundGroupId::Custom((*group_name).to_string()),
                AudioGroup {
                    sounds: sounds.iter().map(|sound| (*sound).to_string()).collect(),
                    ..Default::default()
                },
            );
        }

        source
    }

    #[test]
    fn expected_audio_ref_counts_uses_each_source_sound_ids() {
        let first = audio_source(&[("One", &["shared", "shared", "first"])]);
        let second = audio_source(&[("Two", &["shared", "second"])]);

        let counts = expected_audio_ref_counts([&first, &second]);

        assert_eq!(counts.get("first"), Some(&1));
        assert_eq!(counts.get("second"), Some(&1));
        assert_eq!(counts.get("shared"), Some(&2));
    }

    #[test]
    fn audio_diagnostics_rows_prioritize_attention_before_alphabetical() {
        let ecs_counts = HashMap::from([
            ("alpha".to_string(), 2),
            ("beta".to_string(), 0),
            ("gamma".to_string(), 1),
            ("zeta".to_string(), 1),
        ]);
        let snapshot = AudioDiagnosticsSnapshot {
            cached_sound_count: 3,
            pinned_sound_count: 1,
            ref_count_entry_count: 4,
            entries: vec![
                AudioDiagnosticsEntry {
                    id: "alpha".to_string(),
                    cached: true,
                    pinned: false,
                    ref_count: 2,
                },
                AudioDiagnosticsEntry {
                    id: "beta".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 1,
                },
                AudioDiagnosticsEntry {
                    id: "gamma".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 1,
                },
                AudioDiagnosticsEntry {
                    id: "zeta".to_string(),
                    cached: true,
                    pinned: false,
                    ref_count: 0,
                },
            ],
        };

        let rows = audio_diagnostics_rows(&ecs_counts, &snapshot);

        assert_eq!(
            rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            vec!["beta", "gamma", "zeta", "alpha"]
        );
        assert!(rows[0].is_attention());
        assert!(rows[1].is_attention());
        assert!(rows[2].is_attention());
        assert!(!rows[3].is_attention());
    }

    #[test]
    fn audio_diagnostics_rows_are_capped_to_six_entries() {
        let ecs_counts = HashMap::new();
        let snapshot = AudioDiagnosticsSnapshot {
            cached_sound_count: 8,
            pinned_sound_count: 0,
            ref_count_entry_count: 8,
            entries: vec![
                AudioDiagnosticsEntry {
                    id: "alpha".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 0,
                },
                AudioDiagnosticsEntry {
                    id: "beta".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 0,
                },
                AudioDiagnosticsEntry {
                    id: "charlie".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 0,
                },
                AudioDiagnosticsEntry {
                    id: "delta".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 0,
                },
                AudioDiagnosticsEntry {
                    id: "echo".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 0,
                },
                AudioDiagnosticsEntry {
                    id: "foxtrot".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 0,
                },
                AudioDiagnosticsEntry {
                    id: "golf".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 0,
                },
                AudioDiagnosticsEntry {
                    id: "hotel".to_string(),
                    cached: false,
                    pinned: false,
                    ref_count: 0,
                },
            ],
        };

        let rows = audio_diagnostics_rows(&ecs_counts, &snapshot);

        assert_eq!(rows.len(), 6);
        assert_eq!(
            rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "beta", "charlie", "delta", "echo", "foxtrot"]
        );
    }

    #[test]
    fn audio_ref_summary_ignores_cache_only_entries() {
        let rows = vec![
            AudioDiagnosticsRow {
                id: "cache-only".to_string(),
                cached: true,
                pinned: false,
                ref_count: 0,
                ecs_count: 0,
            },
            AudioDiagnosticsRow {
                id: "matching".to_string(),
                cached: true,
                pinned: false,
                ref_count: 1,
                ecs_count: 1,
            },
            AudioDiagnosticsRow {
                id: "stale".to_string(),
                cached: true,
                pinned: false,
                ref_count: 2,
                ecs_count: 0,
            },
        ];

        assert_eq!(audio_ref_summary(&rows), (1, 2));
    }
}
