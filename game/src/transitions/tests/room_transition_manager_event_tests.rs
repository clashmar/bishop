use super::support::setup_tagged_game;
use crate::engine::game_instance::GameInstance;
use crate::transitions::room_transition_manager::RoomTransitionManager;
use std::collections::HashMap;

#[test]
fn room_entered_event_carries_tags_when_room_has_tags() {
    let (lua, game, player, received) = setup_tagged_game(vec!["autosave".into()]);
    let mut game_instance = GameInstance {
        game,
        prev_positions: HashMap::new(), traversal_residency_diagnostics: None,
    };
    RoomTransitionManager::handle_transitions(&lua, &mut game_instance);
    assert_eq!(game_instance.game.ecs.get_player_entity(), Some(player));
    assert_eq!(received.lock().unwrap().as_slice(), ["2", "autosave"]);
}

#[test]
fn room_entered_event_omits_extra_args_when_room_has_no_tags() {
    let (lua, game, _player, received) = setup_tagged_game(vec![]);
    let mut game_instance = GameInstance {
        game,
        prev_positions: HashMap::new(), traversal_residency_diagnostics: None,
    };
    RoomTransitionManager::handle_transitions(&lua, &mut game_instance);
    assert_eq!(received.lock().unwrap().as_slice(), ["2"]);
}
