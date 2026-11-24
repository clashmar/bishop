use super::{compare_click_candidate_priority, topmost_entity_from_click_candidates};
use bishop::prelude::Vec2;
use engine_core::ecs::{Ecs, Entity};
use engine_core::worlds::{RoomId, RoomLayer};

#[derive(Default)]
pub(crate) struct ClickCycle {
    last_position: Option<Vec2>,
    last_room: Option<RoomId>,
    last_layer: Option<RoomLayer>,
    ordered_candidates: Vec<Entity>,
    selected_index: usize,
}

impl ClickCycle {
    pub(crate) fn next_candidate(
        &mut self,
        ecs: &Ecs,
        candidates: &[(Entity, i32, bool)],
        position: Vec2,
        room_id: RoomId,
        layer: RoomLayer,
    ) -> Option<Entity> {
        let Some(first) = topmost_entity_from_click_candidates(ecs, candidates) else {
            self.reset();
            return None;
        };
        let mut ordered = vec![first];
        let mut remaining: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|(entity, _, _)| *entity != first)
            .collect();
        remaining.sort_by(|a, b| compare_click_candidate_priority(*b, *a));
        ordered.extend(remaining.into_iter().map(|(entity, _, _)| entity));

        let same_cycle = self.last_position == Some(position)
            && self.last_room == Some(room_id)
            && self.last_layer == Some(layer)
            && self.ordered_candidates == ordered;
        let selected_index = if same_cycle {
            (self.selected_index + 1) % ordered.len()
        } else {
            0
        };

        self.last_position = Some(position);
        self.last_room = Some(room_id);
        self.last_layer = Some(layer);
        self.ordered_candidates = ordered;
        self.selected_index = selected_index;
        self.ordered_candidates.get(selected_index).copied()
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}
