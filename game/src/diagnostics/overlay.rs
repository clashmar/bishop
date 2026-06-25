//! In-game diagnostics overlay toggled with F3/F4.

use bishop::prelude::*;
use crate::diagnostics::timing_trace::{TimingTraceLogger, TimingTraceSample};
use crate::engine::game_instance::GameInstance;
use engine_core::assets::*;
use engine_core::audio::{AudioDiagnosticsSnapshot, AudioManager};
use engine_core::diagnostics::{
    DiagnosticsCollector, PinnedEntitySnapshot, ResourceResidencySnapshot,
    RuntimeResidencySnapshot, TraversalClassCount, TraversalOutcomeSnapshot,
    TraversalResidencySnapshot, TraversalThrashSnapshot, WarmRoomSnapshot, WarmWorldSnapshot,
};
use engine_core::ecs::*;
use engine_core::hydration::{CoordinatorSnapshot, HydrationScope, ResourceClaim};
use std::collections::{HashMap, HashSet};
use std::path::Path;

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
    cached_raw_dt_ms: f32,
    cached_sim_dt_ms: f32,
    cached_redraw_interval_ms: f32,
    cached_acquire_wait_ms: f32,
    cached_present_wait_ms: f32,
    cached_render_time: f32,
    cached_entity_count: usize,
    cached_texture_count: usize,
    cached_listener_count: usize,
    cached_audio_working_set_resident: usize,
    cached_audio_working_set_total: usize,
    cached_audio_matching_refs: usize,
    cached_audio_checked_refs: usize,
    cached_audio_rows: Vec<AudioDiagnosticsRow>,
    cached_residency: RuntimeResidencySnapshot,
    cached_coordinator: CoordinatorSnapshot,
    cached_traversal: TraversalResidencySnapshot,
    cached_room_frontier: usize,
    cached_world_frontier: usize,
    cached_pinned_entities: usize,
    timing_trace: TimingTraceLogger,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AudioDiagnosticsRow {
    id: String,
    cached: bool,
    loading: bool,
    ref_count: usize,
    ecs_count: usize,
}

impl AudioDiagnosticsRow {
    fn is_attention(&self) -> bool {
        self.ecs_count != self.ref_count || (self.ecs_count > 0 && !self.cached && !self.loading)
    }

    fn display_line(&self) -> String {
        let mut line = format!(
            "Audio {} rc={} ecs={}",
            self.id, self.ref_count, self.ecs_count
        );
        if self.cached {
            line.push_str(" cached");
        }
        if self.loading {
            line.push_str(" loading");
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
            cached_raw_dt_ms: 0.0,
            cached_sim_dt_ms: 0.0,
            cached_redraw_interval_ms: 0.0,
            cached_acquire_wait_ms: 0.0,
            cached_present_wait_ms: 0.0,
            cached_render_time: 0.0,
            cached_entity_count: 0,
            cached_texture_count: 0,
            cached_listener_count: 0,
            cached_audio_working_set_resident: 0,
            cached_audio_working_set_total: 0,
            cached_audio_matching_refs: 0,
            cached_audio_checked_refs: 0,
            cached_audio_rows: Vec::new(),
            cached_residency: RuntimeResidencySnapshot::default(),
            cached_coordinator: CoordinatorSnapshot::default(),
            cached_traversal: TraversalResidencySnapshot::default(),
            cached_room_frontier: 0,
            cached_world_frontier: 0,
            cached_pinned_entities: 0,
            timing_trace: TimingTraceLogger::from_env(),
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
    pub fn update(&mut self, sample: TimingTraceSample) {
        let dt = sample.raw_dt;
        self.collector.record_frame(dt);
        self.cached_fps = self.collector.frame_metrics.fps;
        self.cached_frame_time = self.collector.frame_metrics.avg_frame_time_ms;
        self.cached_raw_dt_ms = dt * 1000.0;
        self.cached_sim_dt_ms = sample.sim_dt * 1000.0;
        self.cached_redraw_interval_ms = sample.redraw_interval * 1000.0;
        self.cached_acquire_wait_ms = sample.acquire_wait * 1000.0;
        self.cached_present_wait_ms = sample.present_wait * 1000.0;
    }

    pub fn record_timing_trace(&mut self, sample: TimingTraceSample) {
        self.timing_trace.record(sample);
    }

    pub fn timing_trace_path(&self) -> &Path {
        self.timing_trace.path()
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
        self.cached_texture_count = game.sprite_manager.texture_count();
        self.cached_listener_count = game.script_manager.event_listener_count();
        self.cached_render_time = render_time_ms;

        let audio_sources = AudioSource::store(&game.ecs);
        let expected_audio_refs =
            expected_audio_ref_counts(&game.asset_registry, audio_sources.data.values());
        let audio_rows = all_audio_diagnostics_rows(&expected_audio_refs, &audio_snapshot);

        self.cached_audio_working_set_resident = audio_rows
            .iter()
            .filter(|row| row.ecs_count > 0 && (row.cached || row.loading))
            .count();
        self.cached_audio_working_set_total = expected_audio_refs.len();
        let (matching_refs, checked_refs) = audio_ref_summary(&audio_rows);
        self.cached_audio_matching_refs = matching_refs;
        self.cached_audio_checked_refs = checked_refs;
        self.cached_audio_rows = audio_diagnostics_rows(&expected_audio_refs, &audio_snapshot);
        self.cached_residency = RuntimeResidencySnapshot::from_sources(
            &game.sprite_manager,
            &game.script_manager,
            &audio_snapshot,
            expected_audio_refs.len(),
            audio_rows.iter().filter(|row| row.ecs_count > 0).count(),
        );
        self.cached_coordinator = game.hydration_coordinator.snapshot();
        self.cached_traversal = game_instance
            .traversal_residency_diagnostics
            .as_ref()
            .map(|d| d.snapshot.clone())
            .unwrap_or_default();
        self.cached_room_frontier = self.cached_traversal.rooms.len();
        self.cached_world_frontier = self.cached_traversal.worlds.len();
        self.cached_pinned_entities = self.cached_traversal.pinned_entities.len();
    }

    /// Handle input for toggling the overlay.
    pub fn handle_input(&mut self, ctx: &mut impl BishopContext) {
        if ctx.is_key_pressed(KeyCode::F3) {
            self.toggle();
        }
        if ctx.is_key_pressed(KeyCode::F4) {
            self.cycle_detail();
        }
    }

    /// Draw the overlay.
    pub fn draw<C: BishopContext>(&self, ctx: &mut C) {
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
            lines.push(format!("Raw dt: {:.2} ms", self.cached_raw_dt_ms));
            lines.push(format!("Sim dt: {:.2} ms", self.cached_sim_dt_ms));
            lines.push(format!("Redraw: {:.2} ms", self.cached_redraw_interval_ms));
            lines.push(format!("Acquire: {:.2} ms", self.cached_acquire_wait_ms));
            lines.push(format!("Present: {:.2} ms", self.cached_present_wait_ms));
            lines.push(format!("DT trace: {}", self.timing_trace_path().display()));
            lines.push(format!("Render: {:.2} ms", self.cached_render_time));
            lines.push(format!("Entities: {}", self.cached_entity_count));
            lines.push(format!("Textures: {}", self.cached_texture_count));
            lines.push(format!("Listeners: {}", self.cached_listener_count));
            lines.push(format!(
                "Audio Working Set: {}/{}",
                self.cached_audio_working_set_resident, self.cached_audio_working_set_total
            ));
            lines.push(format!(
                "Audio Refs: {}/{} IDs match ECS",
                self.cached_audio_matching_refs, self.cached_audio_checked_refs
            ));
            lines.extend(residency_summary_lines(&self.cached_residency));
            lines.extend(coordinator_summary_lines(&self.cached_coordinator));
            lines.push(format!(
                "Room frontier: {} rooms",
                self.cached_room_frontier
            ));
            lines.push(format!(
                "World frontier: {} worlds",
                self.cached_world_frontier
            ));
            lines.push(format!(
                "Pinned entities: {}",
                self.cached_pinned_entities
            ));
            lines.extend(traversal_room_lines(&self.cached_traversal));
            lines.extend(traversal_world_lines(&self.cached_traversal));
            lines.extend(traversal_pinned_lines(&self.cached_traversal));
            lines.extend(traversal_global_lines(&self.cached_traversal));
            lines.extend(traversal_outcome_lines(&self.cached_traversal));
            lines.extend(traversal_thrash_lines(&self.cached_traversal));
            lines.extend(
                self.cached_audio_rows
                    .iter()
                    .map(AudioDiagnosticsRow::display_line),
            );
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

fn residency_summary_line(resource: &ResourceResidencySnapshot) -> String {
    format!(
        "{} known={} resident={} pending={} pinned={} active={} cold={}",
        resource.label,
        resource.counts.known,
        resource.counts.resident,
        resource.counts.pending,
        resource.counts.pinned,
        resource.counts.active,
        resource.counts.cold(),
    )
}

fn residency_summary_lines(snapshot: &RuntimeResidencySnapshot) -> Vec<String> {
    vec![
        residency_summary_line(&snapshot.textures),
        residency_summary_line(&snapshot.scripts),
        residency_summary_line(&snapshot.audio),
    ]
}

fn coordinator_summary_line(scope: &HydrationScope, claims: &[&ResourceClaim]) -> String {
    let mut parts = vec![format!("{:?}", scope)];
    for claim in claims {
        parts.push(format!(
            "{}={}",
            claim.class.display_abbreviation(),
            claim.assets.len()
        ));
    }
    parts.join(" ")
}

fn coordinator_summary_lines(snapshot: &CoordinatorSnapshot) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for scope in &snapshot.active_scopes {
        let scope_claims: Vec<&ResourceClaim> = snapshot
            .claims
            .iter()
            .filter(|c| c.scope == *scope)
            .collect();
        lines.push(coordinator_summary_line(scope, &scope_claims));
    }
    if lines.is_empty() {
        lines.push("Coordinator: no active scopes".to_string());
    }
    lines
}

fn class_counts_line(counts: &[engine_core::diagnostics::TraversalClassCount]) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }

    counts
        .iter()
        .map(|count| format!("{}={}", count.class.display_abbreviation(), count.count))
        .collect::<Vec<_>>()
        .join(" ")
}

fn traversal_room_line(room: &WarmRoomSnapshot) -> String {
    format!(
        "Warm room {} why={} claims {}",
        room.room_id.0,
        room.reasons.join(", "),
        class_counts_line(&room.claims)
    )
}

fn traversal_room_lines(snapshot: &TraversalResidencySnapshot) -> Vec<String> {
    snapshot.rooms.iter().map(traversal_room_line).collect()
}

fn traversal_world_line(world: &WarmWorldSnapshot) -> String {
    format!(
        "Warm world {} why={} claims {}",
        world.world_id.0,
        world.reasons.join(", "),
        class_counts_line(&world.claims)
    )
}

fn traversal_world_lines(snapshot: &TraversalResidencySnapshot) -> Vec<String> {
    snapshot.worlds.iter().map(traversal_world_line).collect()
}

fn traversal_pinned_line(entity: &PinnedEntitySnapshot) -> String {
    let room = entity
        .room_id
        .map(|room_id| format!("Room({})", room_id.0))
        .unwrap_or_else(|| "roomless".to_string());
    format!(
        "Pinned entity {} {} why={} claims {}",
        entity.entity.0,
        room,
        entity.reasons.join(", "),
        class_counts_line(&entity.claims)
    )
}

fn traversal_pinned_lines(snapshot: &TraversalResidencySnapshot) -> Vec<String> {
    snapshot
        .pinned_entities
        .iter()
        .map(traversal_pinned_line)
        .collect()
}

fn traversal_global_line(counts: &[TraversalClassCount]) -> String {
    format!("Global claims {}", class_counts_line(counts))
}

fn traversal_global_lines(snapshot: &TraversalResidencySnapshot) -> Vec<String> {
    if snapshot.global_claims.is_empty() {
        return Vec::new();
    }
    vec![traversal_global_line(&snapshot.global_claims)]
}

fn traversal_outcome_line(outcome: &TraversalOutcomeSnapshot) -> String {
    format!(
        "Outcome {} claim {} hydrate {} evict {} failures={}",
        outcome.label,
        class_counts_line(&outcome.claimed),
        class_counts_line(&outcome.hydrated),
        class_counts_line(&outcome.evicted),
        outcome.failures
    )
}

fn traversal_outcome_lines(snapshot: &TraversalResidencySnapshot) -> Vec<String> {
    snapshot.outcomes.iter().map(traversal_outcome_line).collect()
}

fn traversal_thrash_line(thrash: &TraversalThrashSnapshot) -> String {
    format!(
        "Thrash {:?} events={} hydrates={} evicts={}",
        thrash.asset,
        thrash.events,
        thrash.hydrates,
        thrash.evictions
    )
}

fn traversal_thrash_lines(snapshot: &TraversalResidencySnapshot) -> Vec<String> {
    snapshot.thrash.iter().map(traversal_thrash_line).collect()
}

fn expected_audio_ref_counts<'a>(
    asset_registry: &AssetRegistry,
    sources: impl IntoIterator<Item = &'a AudioSource>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();

    for source in sources {
        for id in sound_command_ids(asset_registry, source.all_sound_ids()) {
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
                loading: snapshot_entry.is_some_and(|entry| entry.loading),
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
    use engine_core::assets::AssetKey;
    use engine_core::diagnostics::{
        AUDIO_RESIDENCY_LABEL, ResidencyCounts, SCRIPTS_RESIDENCY_LABEL,
        TEXTURES_RESIDENCY_LABEL,
    };
    use engine_core::ecs::SpriteId;
    use engine_core::hydration::ResourceClass;
    use engine_core::worlds::RoomId;

    #[test]
    fn residency_summary_lines_include_known_pending_and_active_counts() {
        let snapshot = RuntimeResidencySnapshot {
            textures: ResourceResidencySnapshot::new(
                TEXTURES_RESIDENCY_LABEL,
                ResidencyCounts {
                    known: 8,
                    resident: 5,
                    pending: 1,
                    pinned: 0,
                    active: 0,
                },
            ),
            scripts: ResourceResidencySnapshot::new(
                SCRIPTS_RESIDENCY_LABEL,
                ResidencyCounts {
                    known: 4,
                    resident: 2,
                    pending: 1,
                    pinned: 0,
                    active: 3,
                },
            ),
            audio: ResourceResidencySnapshot::new(
                AUDIO_RESIDENCY_LABEL,
                ResidencyCounts {
                    known: 6,
                    resident: 4,
                    pending: 1,
                    pinned: 2,
                    active: 5,
                },
            ),
        };

        let lines = residency_summary_lines(&snapshot);

        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines[0],
            format!(
                "{} known=8 resident=5 pending=1 pinned=0 active=0 cold=2",
                TEXTURES_RESIDENCY_LABEL,
            )
        );
        assert_eq!(
            lines[1],
            format!(
                "{} known=4 resident=2 pending=1 pinned=0 active=3 cold=1",
                SCRIPTS_RESIDENCY_LABEL,
            )
        );
        assert_eq!(
            lines[2],
            format!(
                "{} known=6 resident=4 pending=1 pinned=2 active=5 cold=1",
                AUDIO_RESIDENCY_LABEL,
            )
        );
    }

    #[test]
    fn coordinator_summary_line_formats_scope_and_claim_counts_from_assets() {
        let snapshot = CoordinatorSnapshot {
            active_scopes: vec![HydrationScope::Room(RoomId(3))],
            claims: vec![ResourceClaim {
                scope: HydrationScope::Room(RoomId(3)),
                class: ResourceClass::Texture,
                assets: vec![
                    AssetKey::Sprite(SpriteId(1)),
                    AssetKey::Sprite(SpriteId(2)),
                ],
            }],
        };

        let lines = coordinator_summary_lines(&snapshot);

        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains(&format!(
            "{}={}",
            ResourceClass::Texture.display_abbreviation(),
            2
        )));
    }

    #[test]
    fn update_from_game_caches_real_coordinator_snapshot() {
        use bishop::audio::AudioBackend;
        use engine_core::assets::AssetKey;
        use engine_core::ecs::SpriteId;
        use engine_core::game::Game;
        use engine_core::hydration::HydrationScope;
        use std::collections::HashMap;

        struct TestBackend;
        impl AudioBackend for TestBackend {
            fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(_render_fn: F) -> Self {
                Self
            }
        }

        let mut game = Game::default();
        game.hydration_coordinator
            .activate_scope(HydrationScope::Boot);
        game.hydration_coordinator
            .claim_asset(HydrationScope::Boot, AssetKey::Sprite(SpriteId(1)));

        let instance = GameInstance {
            game,
            prev_positions: HashMap::new(), traversal_residency_diagnostics: None,
        };
        let audio_manager = AudioManager::new::<TestBackend>();
        let mut overlay = DiagnosticsOverlay::new();

        overlay.update_from_game(&instance, 0.0, &audio_manager);

        assert_eq!(
            overlay.cached_coordinator.active_scopes,
            vec![HydrationScope::Boot]
        );
        assert_eq!(overlay.cached_coordinator.claims.len(), 1);
        assert_eq!(overlay.cached_coordinator.claims[0].assets.len(), 1);
    }

    #[test]
    fn traversal_room_line_includes_reasons_and_claim_counts() {
        let line = traversal_room_line(&WarmRoomSnapshot {
            room_id: RoomId(4),
            reasons: vec!["current room".to_string(), "exit from Room(2)".to_string()],
            claims: vec![engine_core::diagnostics::TraversalClassCount {
                class: ResourceClass::Texture,
                count: 3,
            }],
        });

        assert!(line.contains("Warm room 4"));
        assert!(line.contains("current room"));
        assert!(line.contains("T=3"));
    }
}
