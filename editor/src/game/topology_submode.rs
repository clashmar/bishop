use super::drawing::rect_edge_point;
use bishop::prelude::*;
use engine_core::game::Game;
use engine_core::theme::with_theme;
use engine_core::worlds::topology::{extract_topology, RoomEdgeKind, TraversalTopology};
use engine_core::worlds::{RoomId, World, WorldId};
use std::collections::{BTreeMap, BTreeSet};
use widgets::constants::layout;

const ROOM_LABEL_PREFIX: &str = "Room ";
const WORLD_LABEL_PREFIX: &str = "World ";
const TOPOLOGY_ARROW_HEAD_LENGTH: f32 = 10.0;
const TOPOLOGY_ARROW_HEAD_HALF_WIDTH: f32 = 4.0;
const STUB_OFFSET: f32 = 56.0;
const STUB_SPACING: f32 = 18.0;
const LABEL_ROW_HEIGHT: f32 = 24.0;
const CROSS_WORLD_DIRECTION: Vec2 = Vec2::new(1.0, 1.0);
const LEGEND_MARGIN: f32 = 16.0;
const LEGEND_LINE: f32 = 20.0;
const LABEL_OFFSET_X: f32 = 8.0;
const LABEL_OFFSET_Y: f32 = 18.0;
const PORTAL_COLOR: Color = Color::ORANGE;
const SCRIPTED_COLOR: Color = Color::LIME;

type RoomEdgeList = Vec<TopologyRoomEdge>;

/// A room node positioned for the topology canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct TopologyRoomNode {
    /// Stable room identifier.
    pub id: RoomId,
    /// Owning world identifier.
    pub world_id: WorldId,
    /// Display label for the room.
    pub label: String,
    /// Top-left room position on the canvas.
    pub position: Vec2,
    /// Room size on the canvas.
    pub size: Vec2,
}

/// A same-world room link drawn in the topology canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopologyRoomEdge {
    /// Source room identifier.
    pub from: RoomId,
    /// Destination room identifier.
    pub to: RoomId,
}

/// A cross-world edge stub projected from a source room edge.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossWorldStub {
    /// Source world identifier.
    pub from_world: WorldId,
    /// Source room identifier.
    pub from_room: RoomId,
    /// Destination world identifier.
    pub to_world: WorldId,
    /// Source edge kind.
    pub kind: RoomEdgeKind,
    /// Stub start point on the source room edge.
    pub from: Vec2,
    /// Stub arrow tip.
    pub tip: Vec2,
    /// Stub label anchor point.
    pub anchor: Vec2,
    /// Display label for the destination world.
    pub label: String,
}

/// The projected view of one world's room topology.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TopologyView {
    /// Room nodes for the selected world.
    pub rooms: Vec<TopologyRoomNode>,
    /// The hovered room (if any).
    pub current: BTreeSet<RoomId>,
    /// One-hop neighbors of the hovered room.
    pub warm: BTreeSet<RoomId>,
    /// Same-world adjacency links.
    pub same_world_adjacency: RoomEdgeList,
    /// Same-world directed room-exit links.
    pub same_world_room_exits: RoomEdgeList,
    /// Same-world world-exit links.
    pub same_world_world_exits: RoomEdgeList,
    /// Same-world scripted traversal links.
    pub same_world_scripted: RoomEdgeList,
    /// Cross-world edge stubs from this world.
    pub cross_world_stubs: Vec<CrossWorldStub>,
}

/// Owns the topology canvas state.
#[derive(Debug, Clone, Default)]
pub struct TopologySubmode {
    /// Screen-space pan applied to the room field.
    pub pan: Vec2,
}

impl TopologySubmode {
    /// Builds a world-scoped topology view with optional hover preview.
    pub fn build_view(
        game: &Game,
        topology: &TraversalTopology,
        world_id: WorldId,
        hovered_room: Option<RoomId>,
    ) -> TopologyView {
        let mut rooms = Vec::new();
        let mut same_world_adjacency = Vec::new();
        let mut same_world_room_exits = Vec::new();
        let mut same_world_world_exits = Vec::new();
        let mut same_world_scripted = Vec::new();
        let mut cross_world_stubs = Vec::new();
        let selected_world = game.get_world(world_id);

        for room_id in topology.rooms_in_world(world_id) {
            rooms.push(TopologyRoomNode {
                id: room_id,
                world_id,
                label: selected_world
                    .and_then(|world| world.get_room(room_id))
                    .map(|room| display_room_label(room.id, &room.name))
                    .unwrap_or_else(|| format!("{ROOM_LABEL_PREFIX}{}", room_id.0)),
                position: Vec2::ZERO,
                size: Vec2::ZERO,
            });

            for edge in topology.room_graph.edges_from(room_id) {
                let Some(target_world) = topology.world_for_room(edge.to) else {
                    continue;
                };
                let entry = TopologyRoomEdge {
                    from: room_id,
                    to: edge.to,
                };
                if target_world == world_id {
                    match edge.kind {
                        RoomEdgeKind::Adjacency => same_world_adjacency.push(entry),
                        RoomEdgeKind::RoomExit => same_world_room_exits.push(entry),
                        RoomEdgeKind::WorldExit => same_world_world_exits.push(entry),
                        RoomEdgeKind::ScriptedTraversal => same_world_scripted.push(entry),
                    }
                    continue;
                }

                cross_world_stubs.push(CrossWorldStub {
                    from_world: world_id,
                    from_room: room_id,
                    to_world: target_world,
                    kind: edge.kind,
                    from: Vec2::ZERO,
                    tip: Vec2::ZERO,
                    anchor: Vec2::ZERO,
                    label: game
                        .get_world(target_world)
                        .map(|world| display_world_label(world.id, &world.name))
                        .unwrap_or_else(|| format!("{WORLD_LABEL_PREFIX}{}", target_world.0)),
                });
            }
        }

        let (current, warm) = match hovered_room.filter(|room_id| topology.world_for_room(*room_id) == Some(world_id)) {
            Some(room_id) => {
                let mut current = BTreeSet::new();
                current.insert(room_id);
                let warm = topology
                    .room_graph
                    .neighbors(room_id)
                    .into_iter()
                    .filter(|neighbor| *neighbor != room_id)
                    .filter(|neighbor| topology.world_for_room(*neighbor) == Some(world_id))
                    .collect();
                (current, warm)
            }
            None => (BTreeSet::new(), BTreeSet::new()),
        };

        cross_world_stubs.sort_by_key(|stub| (stub.to_world, stub.from_room, stub.kind));

        TopologyView {
            rooms,
            current,
            warm,
            same_world_adjacency,
            same_world_room_exits,
            same_world_world_exits,
            same_world_scripted,
            cross_world_stubs,
        }
    }

    /// Centers the canvas on the selected world's room field.
    pub fn center_on_world(&mut self, ctx: &WgpuContext, world: &World) {
        let Some(bounds) = room_field_bounds(world) else {
            self.pan = Vec2::ZERO;
            return;
        };

        let screen_center = vec2(ctx.screen_width() * 0.5, ctx.screen_height() * 0.5);
        self.pan = screen_center - bounds.center();
    }

    /// Updates the topology canvas panning interaction.
    pub fn update(&mut self, ctx: &WgpuContext) {
        if ctx.is_mouse_button_down(MouseButton::Middle) || ctx.is_key_down(KeyCode::Space) {
            let delta = ctx.mouse_delta_position();
            self.pan += vec2(delta.0, delta.1);
        }
    }

    /// Draws the selected world's room topology.
    pub fn draw(&mut self, ctx: &mut WgpuContext, game: &Game, world_id: WorldId) {
        let topology = extract_topology(game);
        let Some(world) = game.get_world(world_id) else {
            return;
        };
        let hovered_room = hovered_room_id(ctx, world, self.pan);
        let mut view = Self::build_view(game, &topology, world.id, hovered_room);
        apply_room_geometry(world, &mut view);
        apply_stub_geometry(game, &mut view);
        apply_pan(self.pan, &mut view);

        ctx.set_default_camera();
        draw_links(ctx, &view);
        draw_nodes(ctx, &view);
        draw_cross_world_stubs(ctx, &view);
        draw_stub_legend(ctx, world);
    }
}

/// Draws same-world links.
pub fn draw_links(ctx: &mut WgpuContext, view: &TopologyView) {
    for edge in &view.same_world_adjacency {
        if let Some((from, to)) = edge_centers(view, *edge) {
            ctx.draw_line(
                from.x,
                from.y,
                to.x,
                to.y,
                2.0,
                with_theme(|theme| theme.border),
            );
        }
    }

    for edge in &view.same_world_room_exits {
        if let Some((from, to)) = edge_centers(view, *edge) {
            draw_arrow(
                ctx,
                from,
                to,
                with_theme(|theme| theme.accent),
                TOPOLOGY_ARROW_HEAD_LENGTH,
                TOPOLOGY_ARROW_HEAD_HALF_WIDTH,
            );
        }
    }

    for edge in &view.same_world_world_exits {
        if let Some((from, to)) = edge_centers(view, *edge) {
            draw_arrow(
                ctx,
                from,
                to,
                PORTAL_COLOR,
                TOPOLOGY_ARROW_HEAD_LENGTH,
                TOPOLOGY_ARROW_HEAD_HALF_WIDTH,
            );
        }
    }

    for edge in &view.same_world_scripted {
        if let Some((from, to)) = edge_centers(view, *edge) {
            draw_arrow(
                ctx,
                from,
                to,
                SCRIPTED_COLOR,
                TOPOLOGY_ARROW_HEAD_LENGTH,
                TOPOLOGY_ARROW_HEAD_HALF_WIDTH,
            );
        }
    }
}

/// Draws cross-world stubs above the room nodes.
pub fn draw_cross_world_stubs(ctx: &mut WgpuContext, view: &TopologyView) {
    for stub in &view.cross_world_stubs {
        draw_arrow(
            ctx,
            stub.from,
            stub.tip,
            with_theme(|theme| theme.danger),
            TOPOLOGY_ARROW_HEAD_LENGTH,
            TOPOLOGY_ARROW_HEAD_HALF_WIDTH,
        );
        ctx.draw_text(
            &stub.label,
            stub.anchor.x + LABEL_OFFSET_X,
            stub.anchor.y,
            layout::DEFAULT_FONT_SIZE_16,
            with_theme(|theme| theme.danger),
        );
    }
}

/// Draws the selected world's room nodes.
pub fn draw_nodes(ctx: &mut WgpuContext, view: &TopologyView) {
    for room in &view.rooms {
        let border = if view.current.contains(&room.id) {
            with_theme(|theme| theme.primary)
        } else if view.warm.contains(&room.id) {
            with_theme(|theme| theme.accent)
        } else {
            with_theme(|theme| theme.border)
        };
        let fill = if view.current.contains(&room.id) {
            border.with_alpha(0.22)
        } else if view.warm.contains(&room.id) {
            border.with_alpha(0.14)
        } else {
            with_theme(|theme| theme.surface).with_alpha(0.55)
        };

        ctx.draw_rectangle(room.position.x, room.position.y, room.size.x, room.size.y, fill);
        ctx.draw_rectangle_lines(room.position.x, room.position.y, room.size.x, room.size.y, 2.0, border);
        ctx.draw_text(
            &room.label,
            room.position.x + LABEL_OFFSET_X,
            room.position.y + LABEL_OFFSET_Y,
            layout::DEFAULT_FONT_SIZE_16,
            with_theme(|theme| theme.text),
        );
    }
}

/// Draws the topology legend and selected-world label.
pub fn draw_stub_legend(ctx: &mut WgpuContext, world: &World) {
    let base_y = ctx.screen_height() - LEGEND_MARGIN - LEGEND_LINE * 3.0;
    let title = if world.name.is_empty() {
        format!("{WORLD_LABEL_PREFIX}{}", world.id.0)
    } else {
        world.name.clone()
    };

    ctx.draw_text(
        &title,
        LEGEND_MARGIN,
        base_y - LEGEND_LINE,
        layout::HEADER_FONT_SIZE_20,
        with_theme(|theme| theme.text),
    );

    draw_arrow(
        ctx,
        vec2(LEGEND_MARGIN, base_y),
        vec2(LEGEND_MARGIN + 24.0, base_y),
        with_theme(|theme| theme.accent),
        TOPOLOGY_ARROW_HEAD_LENGTH,
        TOPOLOGY_ARROW_HEAD_HALF_WIDTH,
    );
    ctx.draw_text(
        "Exit",
        LEGEND_MARGIN + 32.0,
        base_y + 4.0,
        layout::DEFAULT_FONT_SIZE_16,
        with_theme(|theme| theme.accent),
    );

    draw_arrow(
        ctx,
        vec2(LEGEND_MARGIN, base_y + LEGEND_LINE),
        vec2(LEGEND_MARGIN + 24.0, base_y + LEGEND_LINE),
        PORTAL_COLOR,
        TOPOLOGY_ARROW_HEAD_LENGTH,
        TOPOLOGY_ARROW_HEAD_HALF_WIDTH,
    );
    ctx.draw_text(
        "Portal",
        LEGEND_MARGIN + 32.0,
        base_y + LEGEND_LINE + 4.0,
        layout::DEFAULT_FONT_SIZE_16,
        PORTAL_COLOR,
    );

    draw_arrow(
        ctx,
        vec2(LEGEND_MARGIN, base_y + LEGEND_LINE * 2.0),
        vec2(LEGEND_MARGIN + 24.0, base_y + LEGEND_LINE * 2.0),
        with_theme(|theme| theme.danger),
        TOPOLOGY_ARROW_HEAD_LENGTH,
        TOPOLOGY_ARROW_HEAD_HALF_WIDTH,
    );
    ctx.draw_text(
        "Cross-world",
        LEGEND_MARGIN + 32.0,
        base_y + LEGEND_LINE * 2.0 + 4.0,
        layout::DEFAULT_FONT_SIZE_16,
        with_theme(|theme| theme.danger),
    );
}

fn edge_centers(view: &TopologyView, edge: TopologyRoomEdge) -> Option<(Vec2, Vec2)> {
    let from = view.rooms.iter().find(|room| room.id == edge.from)?;
    let to = view.rooms.iter().find(|room| room.id == edge.to)?;
    Some((room_center(from), room_center(to)))
}

fn room_center(room: &TopologyRoomNode) -> Vec2 {
    room.position + room.size * 0.5
}

fn room_field_bounds(world: &World) -> Option<Rect> {
    let mut min = vec2(f32::INFINITY, f32::INFINITY);
    let mut max = vec2(f32::NEG_INFINITY, f32::NEG_INFINITY);

    for room in world.rooms() {
        let rect = room.world_rect(world.grid_size);
        min.x = min.x.min(rect.x);
        min.y = min.y.min(rect.y);
        max.x = max.x.max(rect.x + rect.w);
        max.y = max.y.max(rect.y + rect.h);
    }

    if !min.x.is_finite() {
        None
    } else {
        Some(Rect::new(min.x, min.y, max.x - min.x, max.y - min.y))
    }
}

fn hovered_room_id(ctx: &WgpuContext, world: &World, pan: Vec2) -> Option<RoomId> {
    let mouse: Vec2 = ctx.mouse_position().into();
    world.rooms().iter().find_map(|room| {
        let rect = room.world_rect(world.grid_size);
        let shifted = Rect::new(rect.x + pan.x, rect.y + pan.y, rect.w, rect.h);
        shifted.contains(mouse).then_some(room.id)
    })
}

fn apply_room_geometry(world: &World, view: &mut TopologyView) {
    for node in &mut view.rooms {
        if let Some(room) = world.get_room(node.id) {
            let rect = room.world_rect(world.grid_size);
            node.position = rect.top_left();
            node.size = vec2(rect.w, rect.h);
        }
    }
}

fn apply_stub_geometry(game: &Game, view: &mut TopologyView) {
    let Some(source_world) = game.get_world(view.rooms.first().map(|room| room.world_id).unwrap_or_default()) else {
        return;
    };
    let mut group_sizes = BTreeMap::new();
    for stub in &view.cross_world_stubs {
        *group_sizes.entry((stub.from_room, stub.to_world)).or_insert(0usize) += 1;
    }
    let mut group_indices = BTreeMap::new();

    for stub in &mut view.cross_world_stubs {
        let Some(source_node) = view.rooms.iter().find(|room| room.id == stub.from_room) else {
            continue;
        };
        let room_rect = Rect::new(
            source_node.position.x,
            source_node.position.y,
            source_node.size.x,
            source_node.size.y,
        );
        let room_center = room_rect.center();
        let target_pos = game
            .get_world(stub.to_world)
            .map(|world| world.meta.position)
            .unwrap_or(source_world.meta.position + vec2(1.0, 0.0));
        let mut edge_direction = target_pos - source_world.meta.position;
        if edge_direction.length_squared() == 0.0 {
            edge_direction = CROSS_WORLD_DIRECTION;
        }
        let edge_direction = edge_direction.normalize();
        let from = rect_edge_point(room_rect, room_center, edge_direction);
        let offset_index = group_indices
            .entry((stub.from_room, stub.to_world))
            .or_insert(0usize);
        let offset = grouped_stub_offset(
            *offset_index,
            group_sizes[&(stub.from_room, stub.to_world)],
        );
        *offset_index += 1;
        let normal = vec2(-edge_direction.y, edge_direction.x) * offset;
        stub.from = from + normal;
        stub.tip = stub.from + CROSS_WORLD_DIRECTION * STUB_OFFSET;
        stub.anchor = stub.tip;
    }

    resolve_stub_label_rows(view);
    sync_stub_tip_and_label_positions(view);
}

fn grouped_stub_offset(index: usize, total: usize) -> f32 {
    (index as f32 - (total.saturating_sub(1)) as f32 * 0.5) * STUB_SPACING
}

fn resolve_stub_label_rows(view: &mut TopologyView) {
    let mut indices: Vec<usize> = (0..view.cross_world_stubs.len()).collect();
    indices.sort_by(|&left, &right| {
        let left_stub = &view.cross_world_stubs[left];
        let right_stub = &view.cross_world_stubs[right];
        left_stub
            .anchor
            .y
            .total_cmp(&right_stub.anchor.y)
            .then(left_stub.anchor.x.total_cmp(&right_stub.anchor.x))
    });

    let mut next_row_y = f32::NEG_INFINITY;
    for index in indices {
        let stub = &mut view.cross_world_stubs[index];
        if stub.anchor.y < next_row_y {
            stub.anchor.y = next_row_y;
        }
        next_row_y = stub.anchor.y + LABEL_ROW_HEIGHT;
    }
}

fn sync_stub_tip_and_label_positions(view: &mut TopologyView) {
    let direction_y = CROSS_WORLD_DIRECTION.y;
    if direction_y.abs() <= f32::EPSILON {
        return;
    }

    for stub in &mut view.cross_world_stubs {
        let scale = ((stub.anchor.y - stub.from.y) / direction_y).max(STUB_OFFSET);
        stub.tip = stub.from + CROSS_WORLD_DIRECTION * scale;
        stub.anchor.x = stub.tip.x;
        stub.anchor.y = stub.tip.y;
    }
}

fn apply_pan(pan: Vec2, view: &mut TopologyView) {
    for room in &mut view.rooms {
        room.position += pan;
    }
    for stub in &mut view.cross_world_stubs {
        stub.from += pan;
        stub.tip += pan;
        stub.anchor += pan;
    }
}

fn display_room_label(room_id: RoomId, name: &str) -> String {
    if name.is_empty() {
        format!("{ROOM_LABEL_PREFIX}{}", room_id.0)
    } else {
        name.to_string()
    }
}

fn display_world_label(world_id: WorldId, name: &str) -> String {
    if name.is_empty() {
        format!("{WORLD_LABEL_PREFIX}{}", world_id.0)
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::{WorldEntry, WorldExit};
    use engine_core::worlds::topology::{RoomGraph, WorldGraph};
    use engine_core::worlds::world::WorldExitTrigger;
    use engine_core::worlds::{ExitDestination, Room, World};
    use std::collections::HashMap;

    fn sample_game() -> Game {
        let mut main_world = World::new(WorldId(2), "Main World".to_string(), 16.0);
        main_world.add_room(Room {
            id: RoomId(9),
            name: "Left Room".to_string(),
            ..Default::default()
        });
        main_world.add_room(Room {
            id: RoomId(10),
            name: "Right Room".to_string(),
            adjacent_rooms: vec![RoomId(9)],
            ..Default::default()
        });

        let mut other_world = World::new(WorldId(3), "Other World".to_string(), 16.0);
        other_world.add_room(Room {
            id: RoomId(20),
            name: "Foreign Room".to_string(),
            ..Default::default()
        });

        let mut game = Game::default();
        game.add_world(main_world);
        game.add_world(other_world);
        game.current_world_id = Some(WorldId(2));

        game.ecs
            .create_entity()
            .with(WorldEntry {
                name: "Portal".to_string(),
                ..Default::default()
            })
            .with_current_room(RoomId(10))
            .finish();

        game.ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(2))),
                entry: Some("Portal".to_string()),
                trigger: WorldExitTrigger::OnInteract,
            })
            .with_current_room(RoomId(9))
            .finish();

        game.ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(3))),
                entry: None,
                trigger: WorldExitTrigger::OnInteract,
            })
            .with_current_room(RoomId(9))
            .finish();

        game
    }

    #[test]
    fn selected_world_projection_only_contains_rooms_for_that_world() {
        let game = sample_game();
        let topology = extract_topology(&game);
        let world_view = TopologySubmode::build_view(&game, &topology, WorldId(2), None);

        assert!(world_view.rooms.iter().all(|room| room.world_id == WorldId(2)));
        assert_eq!(world_view.rooms[0].label, "Left Room");
        assert_eq!(world_view.rooms[1].label, "Right Room");
        assert!(world_view.current.is_empty());
        assert!(world_view.warm.is_empty());
    }

    #[test]
    fn hovering_room_previews_one_hop_reachable_rooms() {
        let game = sample_game();
        let topology = extract_topology(&game);
        let world_view = TopologySubmode::build_view(&game, &topology, WorldId(2), Some(RoomId(9)));

        assert!(world_view.current.contains(&RoomId(9)));
        assert!(world_view.warm.contains(&RoomId(10)));
        assert!(!world_view.warm.contains(&RoomId(9)));
    }

    #[test]
    fn cross_world_exit_builds_edge_stub_instead_of_foreign_room_node() {
        let game = sample_game();
        let topology = extract_topology(&game);
        let world_view = TopologySubmode::build_view(&game, &topology, WorldId(2), Some(RoomId(9)));

        assert_eq!(world_view.cross_world_stubs.len(), 1);
        assert_eq!(world_view.cross_world_stubs[0].label, "Other World");
        assert!(world_view.rooms.iter().all(|room| room.world_id == WorldId(2)));
    }

    #[test]
    fn duplicate_cross_world_exits_build_multiple_stubs() {
        let mut game = sample_game();
        game.ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(3))),
                entry: None,
                trigger: WorldExitTrigger::OnInteract,
            })
            .with_current_room(RoomId(10))
            .finish();

        let topology = extract_topology(&game);
        let world_view = TopologySubmode::build_view(&game, &topology, WorldId(2), None);

        assert_eq!(world_view.cross_world_stubs.len(), 2);
        assert!(world_view
            .cross_world_stubs
            .iter()
            .all(|stub| stub.label == "Other World"));
        assert_eq!(world_view.cross_world_stubs[0].from_room, RoomId(9));
        assert_eq!(world_view.cross_world_stubs[1].from_room, RoomId(10));
    }

    #[test]
    fn cross_world_stub_labels_use_distinct_rows_across_the_world() {
        let mut main_world = World::new(WorldId(2), "Main World".to_string(), 16.0);
        main_world.add_room(Room {
            id: RoomId(9),
            name: "Left Room".to_string(),
            position: Vec2::ZERO,
            size: vec2(4.0, 4.0),
            ..Default::default()
        });
        main_world.add_room(Room {
            id: RoomId(10),
            name: "Right Room".to_string(),
            position: vec2(64.0, 0.0),
            size: vec2(4.0, 4.0),
            ..Default::default()
        });

        let mut world_a = World::new(WorldId(3), "World A".to_string(), 16.0);
        world_a.meta.position = vec2(300.0, 0.0);
        world_a.add_room(Room {
            id: RoomId(20),
            ..Default::default()
        });

        let mut world_b = World::new(WorldId(4), "World B".to_string(), 16.0);
        world_b.meta.position = vec2(320.0, 0.0);
        world_b.add_room(Room {
            id: RoomId(21),
            ..Default::default()
        });

        let mut game = Game::default();
        game.add_world(main_world.clone());
        game.add_world(world_a);
        game.add_world(world_b);
        game.current_world_id = Some(WorldId(2));

        game.ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(3))),
                entry: None,
                trigger: WorldExitTrigger::OnInteract,
            })
            .with_current_room(RoomId(9))
            .finish();

        game.ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(4))),
                entry: None,
                trigger: WorldExitTrigger::OnInteract,
            })
            .with_current_room(RoomId(10))
            .finish();

        let topology = extract_topology(&game);
        let mut world_view = TopologySubmode::build_view(&game, &topology, WorldId(2), None);
        apply_room_geometry(&main_world, &mut world_view);
        apply_stub_geometry(&game, &mut world_view);

        assert_eq!(world_view.cross_world_stubs.len(), 2);
        let a = world_view.cross_world_stubs[0].anchor.y;
        let b = world_view.cross_world_stubs[1].anchor.y;
        assert!((a - b).abs() >= LABEL_ROW_HEIGHT);
        assert!(world_view
            .cross_world_stubs
            .iter()
            .all(|stub| stub.tip.x > stub.from.x));
        assert!(world_view
            .cross_world_stubs
            .iter()
            .all(|stub| stub.tip.y > stub.from.y));
        assert!(world_view
            .cross_world_stubs
            .iter()
            .all(|stub| (stub.tip.y - stub.anchor.y).abs() < f32::EPSILON));
        assert!(world_view
            .cross_world_stubs
            .iter()
            .all(|stub| (stub.tip.x - stub.anchor.x).abs() < f32::EPSILON));
    }

    fn manual_topology(edges: &[(RoomId, RoomId, RoomEdgeKind)]) -> TraversalTopology {
        let mut room_graph = RoomGraph::default();
        let world_graph = WorldGraph::default();
        let room_worlds = HashMap::from([
            (RoomId(9), WorldId(2)),
            (RoomId(10), WorldId(2)),
            (RoomId(20), WorldId(3)),
        ]);

        for &(from, to, kind) in edges {
            room_graph.insert_edge(from, to, kind);
        }

        TraversalTopology::from_parts(room_graph, world_graph, room_worlds)
    }

    #[test]
    fn same_world_portal_builds_distinct_portal_link() {
        let game = sample_game();
        let topology = extract_topology(&game);
        let world_view = TopologySubmode::build_view(&game, &topology, WorldId(2), Some(RoomId(9)));

        assert_eq!(world_view.same_world_world_exits, vec![TopologyRoomEdge {
            from: RoomId(9),
            to: RoomId(10),
        }]);
        assert!(world_view.same_world_room_exits.is_empty());
    }

    #[test]
    fn same_world_scripted_traversal_builds_distinct_scripted_link() {
        let game = sample_game();
        let topology = manual_topology(&[(RoomId(9), RoomId(10), RoomEdgeKind::ScriptedTraversal)]);
        let world_view = TopologySubmode::build_view(&game, &topology, WorldId(2), Some(RoomId(9)));

        assert_eq!(world_view.same_world_scripted, vec![TopologyRoomEdge {
            from: RoomId(9),
            to: RoomId(10),
        }]);
        assert!(world_view.same_world_world_exits.is_empty());
        assert!(world_view.cross_world_stubs.is_empty());
    }

    #[test]
    fn cross_world_scripted_traversal_builds_cross_world_stub_from_topology_edge() {
        let game = sample_game();
        let topology = manual_topology(&[(RoomId(9), RoomId(20), RoomEdgeKind::ScriptedTraversal)]);
        let world_view = TopologySubmode::build_view(&game, &topology, WorldId(2), Some(RoomId(9)));

        assert_eq!(world_view.cross_world_stubs.len(), 1);
        assert_eq!(world_view.cross_world_stubs[0].from_room, RoomId(9));
        assert_eq!(world_view.cross_world_stubs[0].to_world, WorldId(3));
        assert_eq!(world_view.cross_world_stubs[0].kind, RoomEdgeKind::ScriptedTraversal);
        assert_eq!(world_view.cross_world_stubs[0].label, "Other World");
    }
}
