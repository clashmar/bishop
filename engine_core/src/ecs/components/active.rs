use crate::ecs::reflect_field::{FieldInfo, Reflect, ReflectField};
use crate::inspector_module;
use ecs_component::ecs_component;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Marks an entity as active for simulation systems.
#[ecs_component]
#[derive(Debug, Clone, Copy)]
pub struct Active {
    /// Authored activation state.
    pub value: bool,
    /// Runtime pin count preventing deactivation.
    pub pin_count: u16,
}

impl Active {
    /// Creates an `Active` component with the given authored state.
    pub const fn new(value: bool) -> Self {
        Self { value, pin_count: 0 }
    }

    /// Increments the runtime pin count.
    pub fn pin(&mut self) {
        self.pin_count = self.pin_count.saturating_add(1);
    }

    /// Decrements the runtime pin count.
    pub fn unpin(&mut self) {
        debug_assert!(self.pin_count > 0, "unpin without matching pin");
        self.pin_count = self.pin_count.saturating_sub(1);
    }

    /// Returns true when authored active or runtime-pinned.
    pub fn is_enabled(&self) -> bool {
        self.value || self.pin_count > 0
    }
}

impl Default for Active {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Serialize for Active {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Active {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ActiveRepr {
            Bool(bool),
            Tuple((bool,)),
        }

        ActiveRepr::deserialize(deserializer).map(|repr| match repr {
            ActiveRepr::Bool(value) => Self::new(value),
            ActiveRepr::Tuple((value,)) => Self::new(value),
        })
    }
}

impl Reflect for Active {
    fn fields(&mut self) -> Vec<FieldInfo<'_>> {
        vec![bool::field_info(&mut self.value, "value")]
    }
}

inspector_module!(Active);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_pin_count_is_runtime_only_and_not_serialized() {
        let mut active = Active::new(false);
        active.pin();
        let ron = ron::to_string(&active).unwrap();

        assert!(!ron.contains("pin_count"));
    }

    #[test]
    fn pinning_keeps_entity_effectively_active_until_unpinned() {
        let mut active = Active::new(false);
        assert!(!active.is_enabled());

        active.pin();
        assert!(active.is_enabled());

        active.unpin();
        assert!(!active.is_enabled());
    }

    #[test]
    fn active_deserializes_legacy_tuple_bool_shape() {
        let active: Active = ron::from_str("(true)").unwrap();

        assert!(active.value);
        assert_eq!(active.pin_count, 0);
    }
}
