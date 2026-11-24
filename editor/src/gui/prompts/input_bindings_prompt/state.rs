use super::InputBindingsChanges;
use engine_core::input::{InputActionId, InputActionMap, InputBinding};
use engine_core::storage::InputSettings;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BindingScope {
    GameDefaults,
    Global,
    PerGame,
}

impl BindingScope {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::GameDefaults => "Game defaults",
            Self::Global => "Global override",
            Self::PerGame => "This game override",
        }
    }

    pub(super) fn from_label(label: &str) -> Option<Self> {
        [Self::GameDefaults, Self::Global, Self::PerGame]
            .into_iter()
            .find(|scope| scope.label() == label)
    }
}

/// Staged input action and remapping edits for the prompt.
pub(crate) struct InputBindingsPromptState {
    game_id: Uuid,
    initial_actions: InputActionMap,
    actions: InputActionMap,
    initial_settings: InputSettings,
    settings: InputSettings,
    selected_action: Option<InputActionId>,
    scope: BindingScope,
}

impl InputBindingsPromptState {
    pub(crate) fn new(
        actions: InputActionMap,
        settings: InputSettings,
        game_id: Uuid,
    ) -> Self {
        let mut action_ids = actions.actions.keys().cloned().collect::<Vec<_>>();
        action_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        let selected_action = action_ids.into_iter().next();
        Self {
            game_id,
            initial_actions: actions.clone(),
            actions,
            initial_settings: settings.clone(),
            settings,
            selected_action,
            scope: BindingScope::GameDefaults,
        }
    }

    pub(crate) fn scope(&self) -> BindingScope {
        self.scope
    }

    pub(crate) fn set_scope(&mut self, scope: BindingScope) {
        self.scope = scope;
    }

    pub(crate) fn selected_action(&self) -> Option<&InputActionId> {
        self.selected_action.as_ref()
    }

    pub(crate) fn select_action(&mut self, action_id: &InputActionId) -> bool {
        if self.actions.actions.contains_key(action_id) {
            self.selected_action = Some(action_id.clone());
            true
        } else {
            false
        }
    }

    pub(crate) fn action_ids(&self) -> Vec<InputActionId> {
        let mut action_ids = self.actions.actions.keys().cloned().collect::<Vec<_>>();
        action_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        action_ids
    }

    pub(crate) fn add_action(&mut self, action_id: &str) -> bool {
        let action_id = action_id.trim();
        if action_id.is_empty() {
            return false;
        }

        let action_id = InputActionId::new(action_id);
        if self.actions.actions.contains_key(&action_id) {
            return false;
        }

        self.actions.actions.insert(action_id.clone(), Vec::new());
        self.selected_action = Some(action_id);
        true
    }

    pub(crate) fn remove_selected_action(&mut self) -> bool {
        let Some(action_id) = self.selected_action.take() else {
            return false;
        };
        if self.actions.actions.remove(&action_id).is_none() {
            return false;
        }

        let game_key = self.game_id.to_string();
        if let Some(overrides) = self.settings.per_game_overrides.get_mut(&game_key) {
            overrides.remove(&action_id);
            if overrides.is_empty() {
                self.settings.per_game_overrides.remove(&game_key);
            }
        }

        self.selected_action = self.action_ids().into_iter().next();
        true
    }

    pub(crate) fn bindings(&self) -> Option<&[InputBinding]> {
        let action_id = self.selected_action.as_ref()?;
        match self.scope {
            BindingScope::GameDefaults => self
                .actions
                .actions
                .get(action_id)
                .map(Vec::as_slice),
            BindingScope::Global => Some(
                self.settings
                    .global_overrides
                    .get(action_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            ),
            BindingScope::PerGame => Some(
                self.settings
                    .per_game_overrides
                    .get(&self.game_id.to_string())
                    .and_then(|overrides| overrides.get(action_id))
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            ),
        }
    }

    pub(crate) fn effective_bindings(&self) -> Option<&[InputBinding]> {
        let action_id = self.selected_action.as_ref()?;
        Some(
            self.settings
                .bindings_for(self.game_id, action_id, &self.actions),
        )
    }

    pub(crate) fn has_explicit_override(&self) -> bool {
        let Some(action_id) = self.selected_action.as_ref() else {
            return false;
        };
        match self.scope {
            BindingScope::GameDefaults => false,
            BindingScope::Global => self.settings.global_overrides.contains_key(action_id),
            BindingScope::PerGame => self
                .settings
                .per_game_overrides
                .get(&self.game_id.to_string())
                .is_some_and(|overrides| overrides.contains_key(action_id)),
        }
    }

    pub(crate) fn add_binding(&mut self, binding: InputBinding) {
        let Some(action_id) = self.selected_action.clone() else {
            return;
        };
        let bindings = self.bindings_mut(&action_id);
        if !bindings.contains(&binding) {
            bindings.push(binding);
        }
    }

    pub(crate) fn remove_binding(&mut self, index: usize) -> bool {
        let Some(action_id) = self.selected_action.clone() else {
            return false;
        };
        let Some(bindings) = self.bindings_mut_optional(&action_id) else {
            return false;
        };
        if index >= bindings.len() {
            return false;
        }
        bindings.remove(index);
        true
    }

    pub(crate) fn reset_override(&mut self) {
        let Some(action_id) = self.selected_action.as_ref() else {
            return;
        };
        match self.scope {
            BindingScope::GameDefaults => {}
            BindingScope::Global => {
                self.settings.global_overrides.remove(action_id);
            }
            BindingScope::PerGame => {
                let game_key = self.game_id.to_string();
                if let Some(overrides) = self.settings.per_game_overrides.get_mut(&game_key) {
                    overrides.remove(action_id);
                    if overrides.is_empty() {
                        self.settings.per_game_overrides.remove(&game_key);
                    }
                }
            }
        }
    }

    pub(crate) fn finish(&self, confirmed: bool) -> Option<InputBindingsChanges> {
        if !confirmed {
            return None;
        }
        Some(InputBindingsChanges {
            input_actions: (self.actions != self.initial_actions)
                .then(|| self.actions.clone()),
            input_settings: (self.settings != self.initial_settings)
                .then(|| self.settings.clone()),
        })
    }

    fn bindings_mut(&mut self, action_id: &InputActionId) -> &mut Vec<InputBinding> {
        match self.scope {
            BindingScope::GameDefaults => self.actions.actions.entry(action_id.clone()).or_default(),
            BindingScope::Global => self
                .settings
                .global_overrides
                .entry(action_id.clone())
                .or_default(),
            BindingScope::PerGame => self
                .settings
                .per_game_overrides
                .entry(self.game_id.to_string())
                .or_default()
                .entry(action_id.clone())
                .or_default(),
        }
    }

    fn bindings_mut_optional(&mut self, action_id: &InputActionId) -> Option<&mut Vec<InputBinding>> {
        match self.scope {
            BindingScope::GameDefaults => self.actions.actions.get_mut(action_id),
            BindingScope::Global => self.settings.global_overrides.get_mut(action_id),
            BindingScope::PerGame => self
                .settings
                .per_game_overrides
                .get_mut(&self.game_id.to_string())
                .and_then(|overrides| overrides.get_mut(action_id)),
        }
    }
}
