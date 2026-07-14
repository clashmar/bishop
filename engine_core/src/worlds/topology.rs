use crate::ecs::{CurrentRoom, WorldEntry, WorldExit};
use crate::game::Game;
use crate::logging::omni_error;
use crate::worlds::scripted_traversal::collect_scripted_traversal_edges;
use crate::worlds::{ExitDestination, RoomId, WorldId};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Classifies a directed room-to-room edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoomEdgeKind {
    /// Links rooms that touch in the same world.
    Adjacency,
    /// Links a room to an authored room-exit target.
    RoomExit,
    /// Links a room to a resolved `WorldExit` destination room.
    WorldExit,
    /// Links a room to a literal scripted `move_to_entry` destination room.
    ScriptedTraversal,
}

/// Stores a directed room neighbor and its edge class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoomEdge {
    /// Destination room id.
    pub to: RoomId,
    /// Edge classification for this destination.
    pub kind: RoomEdgeKind,
}

/// Stores directed room reachability edges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoomGraph {
    outgoing: BTreeMap<RoomId, BTreeSet<RoomEdge>>,
}

impl RoomGraph {
    /// Ensures a room exists in the graph.
    pub fn ensure_room(&mut self, room: RoomId) {
        self.outgoing.entry(room).or_default();
    }

    /// Inserts a directed room edge.
    pub fn insert_edge(&mut self, from: RoomId, to: RoomId, kind: RoomEdgeKind) {
        self.ensure_room(from);
        self.ensure_room(to);
        self.outgoing.entry(from).or_default().insert(RoomEdge { to, kind });
    }

    /// Returns unique outgoing room neighbors.
    pub fn neighbors(&self, room: RoomId) -> BTreeSet<RoomId> {
        self.outgoing
            .get(&room)
            .map(|edges| edges.iter().map(|edge| edge.to).collect())
            .unwrap_or_default()
    }

    /// Returns all typed outgoing room edges.
    pub fn edges_from(&self, room: RoomId) -> Vec<RoomEdge> {
        self.outgoing
            .get(&room)
            .map(|edges| edges.iter().copied().collect())
            .unwrap_or_default()
    }
}

/// Classifies a directed world-to-world edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldEdgeKind {
    /// Comes from a WorldExit component.
    WorldExit,
}

/// Stores a directed world neighbor and its edge class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldEdge {
    /// Destination world id.
    pub to: WorldId,
    /// Edge classification for this destination.
    pub kind: WorldEdgeKind,
}

/// Stores directed world reachability edges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldGraph {
    outgoing: BTreeMap<WorldId, BTreeSet<WorldEdge>>,
}

impl WorldGraph {
    /// Ensures a world exists in the graph.
    pub fn ensure_world(&mut self, world: WorldId) {
        self.outgoing.entry(world).or_default();
    }

    /// Inserts a directed world edge.
    pub fn insert_edge(&mut self, from: WorldId, to: WorldId, kind: WorldEdgeKind) {
        self.ensure_world(from);
        self.ensure_world(to);
        self.outgoing.entry(from).or_default().insert(WorldEdge { to, kind });
    }

    /// Returns unique outgoing world neighbors.
    pub fn neighbors(&self, world: WorldId) -> BTreeSet<WorldId> {
        self.outgoing
            .get(&world)
            .map(|edges| edges.iter().map(|edge| edge.to).collect())
            .unwrap_or_default()
    }

    /// Returns all typed outgoing world edges.
    pub fn edges_from(&self, world: WorldId) -> Vec<WorldEdge> {
        self.outgoing
            .get(&world)
            .map(|edges| edges.iter().copied().collect())
            .unwrap_or_default()
    }
}

/// Bundles extracted room/world traversal reachability.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraversalTopology {
    /// Directed room reachability graph.
    pub room_graph: RoomGraph,
    /// Directed world reachability graph.
    pub world_graph: WorldGraph,
    room_worlds: HashMap<RoomId, WorldId>,
}

impl TraversalTopology {
    /// Builds a topology from pre-constructed graphs and room-world mapping.
    #[cfg(any(test, feature = "editor"))]
    pub fn from_parts(
        room_graph: RoomGraph,
        world_graph: WorldGraph,
        room_worlds: HashMap<RoomId, WorldId>,
    ) -> Self {
        Self {
            room_graph,
            world_graph,
            room_worlds,
        }
    }

    /// Returns the current room plus its one-hop warm frontier.
    pub fn room_frontier(&self, current_room: RoomId) -> BTreeSet<RoomId> {
        let mut frontier = self.room_graph.neighbors(current_room);
        frontier.insert(current_room);
        frontier
    }

    /// Returns the current world plus its one-hop world frontier.
    pub fn world_frontier(&self, current_world: WorldId) -> BTreeSet<WorldId> {
        let mut frontier = self.world_graph.neighbors(current_world);
        frontier.insert(current_world);
        frontier
    }

    /// Returns the world that owns a room.
    pub fn world_for_room(&self, room: RoomId) -> Option<WorldId> {
        self.room_worlds.get(&room).copied()
    }

    /// Returns all rooms belonging to a world.
    pub fn rooms_in_world(&self, world_id: WorldId) -> Vec<RoomId> {
        let mut rooms: Vec<RoomId> = self
            .room_worlds
            .iter()
            .filter_map(|(&room, &owner)| (owner == world_id).then_some(room))
            .collect();
        rooms.sort();
        rooms
    }
}

/// Extracts traversal graphs from authored worlds, world exits, and literal scripted traversal.
pub fn extract_topology(game: &Game) -> TraversalTopology {
    let room_worlds = game.room_world_map();
    let mut room_graph = RoomGraph::default();
    let mut world_graph = WorldGraph::default();

    for world in game.worlds() {
        world_graph.ensure_world(world.id);
        for room in world.rooms() {
            room_graph.ensure_room(room.id);

            for &adjacent in &room.adjacent_rooms {
                room_graph.insert_edge(room.id, adjacent, RoomEdgeKind::Adjacency);
            }

            for exit in &room.exits {
                let Some(target_room) = exit.target_room_id else {
                    continue;
                };
                let Some(source_world) = room_worlds.get(&room.id).copied() else {
                    omni_error!(
                        "Skipping room exit from Room({}): source world missing",
                        room.id.0
                    );
                    continue;
                };
                let Some(target_world) = room_worlds.get(&target_room).copied() else {
                    omni_error!(
                        "Skipping room exit from Room({}) to Room({}): target room missing",
                        room.id.0,
                        target_room.0
                    );
                    continue;
                };
                if source_world != target_world {
                    omni_error!(
                        "Skipping room exit from Room({}) to Room({}): cross-world room exits are unsupported",
                        room.id.0,
                        target_room.0
                    );
                    continue;
                }
                room_graph.insert_edge(room.id, target_room, RoomEdgeKind::RoomExit);
            }
        }
    }

    for (&entity, exit) in &game.ecs.get_store::<WorldExit>().data {
        let Some(CurrentRoom(source_room)) = game.ecs.get::<CurrentRoom>(entity).copied() else {
            omni_error!("Skipping WorldExit entity {:?}: missing CurrentRoom", entity);
            continue;
        };
        let Some(source_world) = room_worlds.get(&source_room).copied() else {
            omni_error!(
                "Skipping WorldExit entity {:?}: source Room({}) has no owning world",
                entity,
                source_room.0
            );
            continue;
        };
        let Some(ExitDestination::World(target_world)) = exit.destination.as_ref() else {
            continue;
        };
        if game.get_world(*target_world).is_none() {
            omni_error!(
                "Skipping WorldExit entity {:?}: target World({}) is missing",
                entity,
                target_world.0
            );
            continue;
        }

        match resolve_world_exit_target_room(game, *target_world, exit.entry.as_deref()) {
            Some(target_room) if target_room != source_room => {
                room_graph.insert_edge(source_room, target_room, RoomEdgeKind::WorldExit);
            }
            Some(_) => {}
            None => {
                omni_error!(
                    "Skipping WorldExit entity {:?}: entry {:?} in World({}) could not be resolved",
                    entity,
                    exit.entry,
                    target_world.0
                );
            }
        }

        if source_world != *target_world {
            world_graph.insert_edge(source_world, *target_world, WorldEdgeKind::WorldExit);
        }
    }

    for edge in collect_scripted_traversal_edges(game) {
        room_graph.insert_edge(edge.from, edge.to, RoomEdgeKind::ScriptedTraversal);
    }

    TraversalTopology {
        room_graph,
        world_graph,
        room_worlds,
    }
}

fn resolve_world_exit_target_room(
    game: &Game,
    target_world: WorldId,
    entry_name: Option<&str>,
) -> Option<RoomId> {
    let target_world_ref = game.get_world(target_world)?;

    if entry_name.is_none() || entry_name == Some(WorldEntry::START_NAME) {
        for (&entity, entry) in &game.ecs.get_store::<WorldEntry>().data {
            if !entry.is_start {
                continue;
            }
            let Some(CurrentRoom(room_id)) = game.ecs.get::<CurrentRoom>(entity).copied() else {
                continue;
            };
            if target_world_ref.get_room(room_id).is_some() {
                return Some(room_id);
            }
        }
        return target_world_ref.rooms().first().map(|room| room.id);
    }

    let sought_name = entry_name.unwrap();
    for (&entity, entry) in &game.ecs.get_store::<WorldEntry>().data {
        if entry.name != sought_name {
            continue;
        }
        let Some(CurrentRoom(room_id)) = game.ecs.get::<CurrentRoom>(entity).copied() else {
            continue;
        };
        if target_world_ref.get_room(room_id).is_some() {
            return Some(room_id);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{Active, Script, WorldExit};
    use crate::engine_global::set_game_name;
    use crate::game::Game;
    use crate::storage::path_utils::scripts_folder;
    use crate::storage::test_utils::{TestGameFolder, game_fs_test_lock};
    use crate::worlds::{Exit, ExitDestination, Room, World, WorldExitTrigger};
    use std::fs;

    fn topology_test_game() -> Game {
        let mut world_a = World::new(WorldId(1), "World A".to_string(), 16.0);
        world_a.current_room_id = Some(RoomId(1));
        world_a.add_room(Room {
            id: RoomId(1),
            adjacent_rooms: vec![RoomId(2)],
            exits: vec![Exit {
                target_room_id: Some(RoomId(3)),
                ..Default::default()
            }],
            ..Default::default()
        });
        world_a.add_room(Room {
            id: RoomId(2),
            ..Default::default()
        });
        world_a.add_room(Room {
            id: RoomId(3),
            ..Default::default()
        });

        let mut world_b = World::new(WorldId(2), "World B".to_string(), 16.0);
        world_b.add_room(Room {
            id: RoomId(9),
            adjacent_rooms: vec![RoomId(10)],
            ..Default::default()
        });
        world_b.add_room(Room {
            id: RoomId(10),
            ..Default::default()
        });

        let mut game = Game::default();
        game.add_world(world_a);
        game.add_world(world_b);
        game.current_world_id = Some(WorldId(1));

        game.ecs
            .create_entity()
            .with(WorldEntry {
                name: "Portal".to_string(),
                ..Default::default()
            })
            .with_current_room(RoomId(3))
            .finish();

        game.ecs
            .create_entity()
            .with(WorldEntry {
                name: WorldEntry::START_NAME.to_string(),
                is_start: true,
            })
            .with_current_room(RoomId(9))
            .finish();

        game.ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(1))),
                entry: Some("Portal".to_string()),
                trigger: WorldExitTrigger::OnInteract,
                ..Default::default()
            })
            .with_current_room(RoomId(2))
            .finish();

        game.ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(2))),
                trigger: WorldExitTrigger::OnInteract,
                ..Default::default()
            })
            .with_current_room(RoomId(2))
            .finish();

        game
    }

    #[test]
    fn room_graph_includes_adjacency_room_exits_and_world_exits() {
        let game = topology_test_game();
        let topology = extract_topology(&game);

        assert!(topology.room_graph.edges_from(RoomId(1)).contains(&RoomEdge {
            to: RoomId(2),
            kind: RoomEdgeKind::Adjacency,
        }));
        assert!(topology.room_graph.edges_from(RoomId(1)).contains(&RoomEdge {
            to: RoomId(3),
            kind: RoomEdgeKind::RoomExit,
        }));
        assert!(topology.room_graph.edges_from(RoomId(2)).contains(&RoomEdge {
            to: RoomId(3),
            kind: RoomEdgeKind::WorldExit,
        }));
    }

    #[test]
    fn room_frontier_is_one_hop_and_outbound_only() {
        let topology = extract_topology(&topology_test_game());
        let frontier = topology.room_frontier(RoomId(1));

        assert!(frontier.contains(&RoomId(1)));
        assert!(frontier.contains(&RoomId(2)));
        assert!(frontier.contains(&RoomId(3)));
        assert!(!frontier.contains(&RoomId(9)));
        assert!(!frontier.contains(&RoomId(10)));
    }

    #[test]
    fn cross_world_room_exit_targets_do_not_create_room_edges() {
        let mut world_a = World::new(WorldId(1), "World A".to_string(), 16.0);
        world_a.add_room(Room {
            id: RoomId(1),
            exits: vec![Exit {
                target_room_id: Some(RoomId(9)),
                ..Default::default()
            }],
            ..Default::default()
        });

        let mut world_b = World::new(WorldId(2), "World B".to_string(), 16.0);
        world_b.add_room(Room {
            id: RoomId(9),
            ..Default::default()
        });

        let mut game = Game::default();
        game.add_world(world_a);
        game.add_world(world_b);

        let topology = extract_topology(&game);

        assert!(!topology.room_graph.neighbors(RoomId(1)).contains(&RoomId(9)));
    }

    #[test]
    fn extract_topology_when_cross_world_world_exit_resolves_entry_adds_room_edge_and_world_summary() {
        let topology = extract_topology(&topology_test_game());

        assert!(topology.room_graph.edges_from(RoomId(2)).contains(&RoomEdge {
            to: RoomId(9),
            kind: RoomEdgeKind::WorldExit,
        }));
        assert!(topology.world_graph.edges_from(WorldId(1)).contains(&WorldEdge {
            to: WorldId(2),
            kind: WorldEdgeKind::WorldExit,
        }));
    }

    #[test]
    fn extract_topology_when_script_moves_to_literal_entry_adds_scripted_room_edge() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let folder = TestGameFolder::new("topology_scripted_traversal");
        set_game_name(folder.name());
        fs::create_dir_all(scripts_folder()).unwrap();
        fs::write(
            scripts_folder().join("scripted.lua"),
            "return { init = function(self) self.entity:move_to_entry(Entries.SecondWorld.Start) end }",
        )
        .unwrap();

        let mut game = Game::with_name(folder.name());
        let mut source_world = World::new(WorldId(1), "Main World".to_string(), 16.0);
        source_world.current_room_id = Some(RoomId(1));
        source_world.add_room(Room {
            id: RoomId(1),
            ..Default::default()
        });
        let mut target_world = World::new(WorldId(2), "Second World".to_string(), 16.0);
        target_world.add_room(Room {
            id: RoomId(9),
            ..Default::default()
        });
        game.add_world(source_world);
        game.add_world(target_world);
        game.current_world_id = Some(WorldId(1));

        game.ecs
            .create_entity()
            .with(WorldEntry {
                name: WorldEntry::START_NAME.to_string(),
                is_start: true,
            })
            .with_current_room(RoomId(9))
            .finish();

        let script_id = game
            .script_manager
            .get_or_load(&mut game.asset_registry, "scripted.lua")
            .expect("script should register");

        game.ecs
            .create_entity()
            .with(Active::default())
            .with(Script {
                script_id,
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();

        let topology = extract_topology(&game);

        assert!(topology.room_graph.edges_from(RoomId(1)).contains(&RoomEdge {
            to: RoomId(9),
            kind: RoomEdgeKind::ScriptedTraversal,
        }));
    }

    #[test]
    fn resolve_world_exit_target_room_finds_is_start_when_no_entry_name() {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Test".to_string(), 16.0);
        world.add_room(Room { id: RoomId(5), ..Default::default() });
        game.add_world(world);

        game.ecs
            .create_entity()
            .with(WorldEntry { name: "Main".to_string(), is_start: true })
            .with_current_room(RoomId(5))
            .finish();

        game.ecs
            .create_entity()
            .with(WorldEntry { name: "Side".to_string(), is_start: false })
            .with_current_room(RoomId(5))
            .finish();

        let result = resolve_world_exit_target_room(&game, WorldId(1), None);
        assert_eq!(result, Some(RoomId(5)));
    }
}
