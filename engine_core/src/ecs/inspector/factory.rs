#[cfg(feature = "editor")]
use crate::ecs::component::Component;
#[cfg(feature = "editor")]
use crate::ecs::ecs::Ecs;
#[cfg(feature = "editor")]
use crate::ecs::entity::Entity;
#[cfg(feature = "editor")]
use crate::ecs::inspector::generic_module::GenericModule;
#[cfg(feature = "editor")]
use crate::ecs::inspector::module::*;
#[cfg(feature = "editor")]
use crate::ecs::reflect_field::Reflect;
#[cfg(feature = "editor")]
use once_cell::sync::Lazy;

/// Human‑readable names of all components that have been registered with `inspector_module!`.
#[cfg(feature = "editor")]
pub static MODULES: Lazy<Vec<&'static ModuleFactoryEntry>> =
    Lazy::new(|| inventory::iter::<ModuleFactoryEntry>.into_iter().collect());

#[cfg(feature = "editor")]
pub trait InspectorModuleFactory {
    /// Human‑readable name that will be shown as the collapsible title.
    fn title(&self) -> &'static str;
    /// Builds the concrete module.
    fn make(&self) -> Box<dyn InspectorModule>;
}

#[cfg(feature = "editor")]
pub struct ModuleFactoryEntry {
    pub type_name: &'static str,
    pub title: &'static str,
    /// Factory that builds the concrete UI module.
    pub factory: fn() -> Box<dyn InspectorModule>,
    /// Optional predicate; when `Some`, the component is excluded from Add Component for entities that return `false`.
    pub allowed_for: Option<fn(Entity, &Ecs) -> bool>,
}

#[cfg(feature = "editor")]
inventory::collect!(ModuleFactoryEntry);

#[cfg(feature = "editor")]
pub fn module_title(type_name: &str) -> &str {
    MODULES
        .iter()
        .find(|entry| entry.type_name == type_name)
        .map(|entry| entry.title)
        .unwrap_or(type_name)
}

#[cfg(feature = "editor")]
pub fn make_module<T>(title: &str, removable: bool) -> Box<dyn InspectorModule>
where
    T: Component + Reflect + Default + 'static,
{
    Box::new(CollapsibleComponentModule::new(GenericModule::<T>::new(removable)).with_title(title))
}

/// Creates a non-collapsible inspector module for a marker component.
#[cfg(feature = "editor")]
pub fn make_marker_module<T>(title: &str, removable: bool) -> Box<dyn InspectorModule>
where
    T: Component + Reflect + Default + 'static,
{
    Box::new(MarkerComponentModule::new(GenericModule::<T>::new(removable)).with_title(title))
}

/// Public macro for each component that appears in the inspector.
#[cfg(feature = "editor")]
#[macro_export]
macro_rules! inspector_module {
    ($ty:ty, marker = true, removable = $removable:expr, title = $title:expr) => {
        inventory::submit! {
            $crate::ecs::inspector::factory::ModuleFactoryEntry {
                type_name: <$ty>::TYPE_NAME,
                title: $title,
                factory: || $crate::ecs::inspector::factory::make_marker_module::<$ty>(
                    $title,
                    $removable,
                ),
                allowed_for: None,
            }
        }
    };

    ($ty:ty) => {
        inspector_module!($ty, removable = true);
    };

    ($ty:ty, removable = $removable:expr) => {
        inspector_module!($ty, removable = $removable, title = <$ty>::TYPE_NAME);
    };

    ($ty:ty, removable = $removable:expr, title = $title:expr) => {
        inventory::submit! {
            $crate::ecs::inspector::factory::ModuleFactoryEntry {
                type_name: <$ty>::TYPE_NAME,
                title: $title,
                factory: || $crate::ecs::inspector::factory::make_module::<$ty>($title, $removable),
                allowed_for: None,
            }
        }
    };
}

/// No-op outside editor builds so component definitions compile in the game crate.
#[cfg(not(feature = "editor"))]
#[macro_export]
macro_rules! inspector_module {
    ($ty:ty, marker = true, removable = $removable:expr, title = $title:expr) => {};
    ($ty:ty) => {};
    ($ty:ty, removable = $removable:expr) => {};
    ($ty:ty, removable = $removable:expr, title = $title:expr) => {};
}

#[cfg(all(test, feature = "editor"))]
mod marker_component_tests {
    use super::MODULES;
    use crate::ecs::components::{Active, MotionBody, PhysicsBody, Sensor};
    use crate::ecs::inspector::layout::InspectorBodyLayout;
    use crate::ecs::inspector::module::InspectorModule;

    fn registered_module(type_name: &str) -> Option<Box<dyn InspectorModule>> {
        MODULES
            .iter()
            .find(|entry| entry.type_name == type_name)
            .map(|entry| (entry.factory)())
    }

    #[test]
    fn marker_component_factories_use_compact_rows() {
        let empty_body_height = InspectorBodyLayout::new().padding(0.0, 0.0).height();

        for type_name in [
            Sensor::TYPE_NAME,
            PhysicsBody::TYPE_NAME,
            MotionBody::TYPE_NAME,
        ] {
            let Some(module) = registered_module(type_name) else {
                panic!("missing inspector module for {type_name}");
            };
            assert_eq!(module.body_layout().height(), empty_body_height);
        }

        let Some(active) = registered_module(Active::TYPE_NAME) else {
            panic!("missing inspector module for {}", Active::TYPE_NAME);
        };
        assert!(active.body_layout().height() > empty_body_height);
    }
}
