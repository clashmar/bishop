use super::{
    axis_capture_binding, gamepad_button_capture_binding, key_capture_binding, BindingScope,
    InputBindingsPromptState,
};
use bishop::input::{GamepadAxis, GamepadButton, KeyCode};
use engine_core::input::{InputActionId, InputActionMap, InputBinding, InputSource};
use engine_core::storage::InputSettings;
use std::collections::HashMap;
use uuid::Uuid;

const JUMP_ACTION_ID: &str = "jump";
const NEW_ACTION_ID: &str = "dash";

fn action_map() -> InputActionMap {
    action_map_with_bindings(vec![InputBinding::keyboard(KeyCode::Space)])
}

fn action_map_with_bindings(bindings: Vec<InputBinding>) -> InputActionMap {
    InputActionMap {
        actions: HashMap::from([(InputActionId::new(JUMP_ACTION_ID), bindings)]),
    }
}

fn prompt_state(game_id: Uuid) -> InputBindingsPromptState {
    InputBindingsPromptState::new(action_map(), InputSettings::default(), game_id)
}

#[test]
fn binding_scope_labels_round_trip() {
    for scope in [BindingScope::GameDefaults, BindingScope::PerGame] {
        assert_eq!(BindingScope::from_label(scope.label()), Some(scope));
    }
}

#[test]
fn selection_and_scope_start_on_authored_defaults() {
    let state = prompt_state(Uuid::new_v4());

    assert_eq!(state.scope(), BindingScope::GameDefaults);
    assert_eq!(state.selected_action(), Some(&InputActionId::new(JUMP_ACTION_ID)));
}

#[test]
fn per_game_edits_do_not_change_another_games_bindings() {
    let game_id = Uuid::new_v4();
    let action_id = InputActionId::new(JUMP_ACTION_ID);
    let mut state = prompt_state(game_id);
    state.set_scope(BindingScope::PerGame);
    state.add_binding(InputBinding::keyboard(KeyCode::Enter));

    let changes = state.finish(true).unwrap();
    let settings = changes.input_settings.unwrap();
    assert_eq!(
        settings.per_game_overrides[&game_id.to_string()][&action_id],
        vec![InputBinding::keyboard(KeyCode::Enter)]
    );
    let other_game = InputBindingsPromptState::new(
        action_map(),
        settings,
        Uuid::new_v4(),
    );
    assert_eq!(
        other_game.effective_bindings(),
        Some(&[InputBinding::keyboard(KeyCode::Space)][..])
    );
}

#[test]
fn renaming_an_action_moves_its_per_game_override() {
    let game_id = Uuid::new_v4();
    let action_id = InputActionId::new(JUMP_ACTION_ID);
    let game_overrides = HashMap::from([(
        action_id.clone(),
        vec![InputBinding::keyboard(KeyCode::Enter)],
    )]);
    let settings = InputSettings {
        per_game_overrides: HashMap::from([(game_id.to_string(), game_overrides)]),
    };
    let mut state = InputBindingsPromptState::new(
        action_map_with_bindings(vec![
            InputBinding::keyboard(KeyCode::Space),
            InputBinding::keyboard(KeyCode::Tab),
        ]),
        settings,
        game_id,
    );
    state.set_scope(BindingScope::PerGame);

    assert!(state.rename_selected_action("confirm"));

    let renamed_id = InputActionId::new("confirm");
    assert_eq!(state.selected_action(), Some(&renamed_id));
    assert_eq!(
        state.action_ids(),
        vec![renamed_id.clone()]
    );
    assert_eq!(
        state.bindings(),
        Some(&[InputBinding::keyboard(KeyCode::Enter)][..])
    );

    let changes = state.finish(true).unwrap();
    let actions = changes.input_actions.unwrap();
    assert_eq!(
        actions.actions[&renamed_id],
        vec![
            InputBinding::keyboard(KeyCode::Space),
            InputBinding::keyboard(KeyCode::Tab),
        ]
    );
    let settings = changes.input_settings.unwrap();
    assert_eq!(
        settings.per_game_overrides[&game_id.to_string()][&renamed_id],
        vec![InputBinding::keyboard(KeyCode::Enter)]
    );
}

#[test]
fn renaming_an_action_rejects_empty_or_duplicate_ids() {
    let mut state = prompt_state(Uuid::new_v4());
    assert!(state.add_action("dash"));

    assert!(!state.rename_selected_action("  "));
    assert!(!state.rename_selected_action(JUMP_ACTION_ID));
    assert_eq!(state.selected_action(), Some(&InputActionId::new(NEW_ACTION_ID)));
    assert_eq!(state.action_ids().len(), 2);
}

#[test]
fn renaming_an_action_rejects_a_per_game_override_conflict() {
    let game_id = Uuid::new_v4();
    let settings = InputSettings {
        per_game_overrides: HashMap::from([(
            game_id.to_string(),
            HashMap::from([(InputActionId::new("confirm"), Vec::new())]),
        )]),
    };
    let mut state = InputBindingsPromptState::new(action_map(), settings, game_id);
    state.set_scope(BindingScope::PerGame);

    assert!(!state.rename_selected_action("confirm"));
    assert_eq!(state.selected_action(), Some(&InputActionId::new(JUMP_ACTION_ID)));
    assert!(state.action_ids().contains(&InputActionId::new(JUMP_ACTION_ID)));
}

#[test]
fn replacing_a_binding_updates_only_the_selected_binding() {
    let game_id = Uuid::new_v4();
    let actions = action_map_with_bindings(vec![
        InputBinding::keyboard(KeyCode::Space),
        InputBinding::keyboard(KeyCode::Tab),
    ]);
    let mut state = InputBindingsPromptState::new(actions, InputSettings::default(), game_id);

    assert!(state.replace_binding(0, InputBinding::keyboard(KeyCode::Enter)));

    assert_eq!(
        state.bindings(),
        Some(
            &[
                InputBinding::keyboard(KeyCode::Enter),
                InputBinding::keyboard(KeyCode::Tab),
            ][..]
        )
    );
}

#[test]
fn replacing_an_inherited_binding_creates_an_override_with_other_bindings_preserved() {
    let game_id = Uuid::new_v4();
    let actions = action_map_with_bindings(vec![
        InputBinding::keyboard(KeyCode::Space),
        InputBinding::keyboard(KeyCode::Tab),
    ]);
    let mut state = InputBindingsPromptState::new(actions, InputSettings::default(), game_id);
    state.set_scope(BindingScope::PerGame);

    assert!(state.replace_binding(0, InputBinding::keyboard(KeyCode::Enter)));

    assert!(state.has_explicit_override());
    assert_eq!(
        state.bindings(),
        Some(
            &[
                InputBinding::keyboard(KeyCode::Enter),
                InputBinding::keyboard(KeyCode::Tab),
            ][..]
        )
    );
}

#[test]
fn removing_the_last_override_binding_keeps_an_explicit_empty_override() {
    let mut state = prompt_state(Uuid::new_v4());
    state.set_scope(BindingScope::PerGame);
    state.add_binding(InputBinding::keyboard(KeyCode::Enter));

    state.remove_binding(0);

    assert!(state.has_explicit_override());
    assert_eq!(state.bindings(), Some(&[][..]));
}

#[test]
fn reset_override_restores_inheritance_and_cancel_discards_staged_edits() {
    let game_id = Uuid::new_v4();
    let mut state = prompt_state(game_id);
    state.set_scope(BindingScope::PerGame);
    state.add_binding(InputBinding::keyboard(KeyCode::Enter));
    state.reset_override();

    assert!(!state.has_explicit_override());
    assert_eq!(state.bindings(), Some(&[][..]));
    assert!(state.finish(false).is_none());
}

#[test]
fn adding_an_action_selects_it_and_cancel_returns_no_changes() {
    let mut state = prompt_state(Uuid::new_v4());
    let new_action = InputActionId::new(NEW_ACTION_ID);

    assert!(state.add_action(NEW_ACTION_ID));
    assert!(!state.add_action(NEW_ACTION_ID));
    assert_eq!(state.selected_action(), Some(&new_action));
    assert!(state.finish(false).is_none());
}

#[test]
fn removing_an_authored_action_clears_its_per_game_override() {
    let game_id = Uuid::new_v4();
    let action_id = InputActionId::new(NEW_ACTION_ID);
    let actions = InputActionMap {
        actions: HashMap::from([
            (
                InputActionId::new(JUMP_ACTION_ID),
                vec![InputBinding::keyboard(KeyCode::Space)],
            ),
            (
                action_id.clone(),
                vec![InputBinding::keyboard(KeyCode::LeftShift)],
            ),
        ]),
    };
    let settings = InputSettings {
        per_game_overrides: HashMap::from([(
            game_id.to_string(),
            HashMap::from([(action_id.clone(), vec![InputBinding::keyboard(KeyCode::Tab)])]),
        )]),
    };
    let mut state = InputBindingsPromptState::new(actions, settings, game_id);
    assert!(state.select_action(&action_id));

    assert!(state.remove_selected_action());

    let changes = state.finish(true).unwrap();
    assert!(!changes.input_actions.unwrap().actions.contains_key(&action_id));
    let settings = changes.input_settings.unwrap();
    assert!(settings
        .per_game_overrides
        .get(&game_id.to_string())
        .map_or(true, |overrides| !overrides.contains_key(&action_id)));
}

#[test]
fn confirmed_changes_only_include_changed_persistence_layers() {
    let state = prompt_state(Uuid::new_v4());
    let changes = state.finish(true).unwrap();

    assert!(changes.input_actions.is_none());
    assert!(changes.input_settings.is_none());
}

#[test]
fn key_and_button_capture_produce_physical_bindings() {
    assert_eq!(
        key_capture_binding(KeyCode::Enter),
        Some(InputBinding::keyboard(KeyCode::Enter))
    );
    assert!(key_capture_binding(KeyCode::Escape).is_none());
    assert_eq!(
        gamepad_button_capture_binding(GamepadButton::South).source,
        InputSource::GamepadButton(GamepadButton::South)
    );
}

#[test]
fn axis_capture_records_sign_and_activation_threshold() {
    let binding = axis_capture_binding(GamepadAxis::LeftX, -0.8).unwrap();

    assert_eq!(
        binding.source,
        InputSource::GamepadAxis {
            axis: GamepadAxis::LeftX,
            direction: engine_core::input::AxisDirection::Negative,
            threshold: 0.5,
        }
    );
    assert!(axis_capture_binding(GamepadAxis::RightY, 0.2).is_none());
    assert!(axis_capture_binding(GamepadAxis::RightY, f32::INFINITY).is_none());
    assert!(axis_capture_binding(GamepadAxis::RightY, f32::NAN).is_none());
}
