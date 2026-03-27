// engine_core/src/ecs/module_factory.rs
#[cfg(feature = "editor")]
use crate::ecs::component::Component;
#[cfg(feature = "editor")]
use crate::ecs::generic_module::GenericModule;
#[cfg(feature = "editor")]
use crate::ecs::inspector_module::*;
#[cfg(feature = "editor")]
use crate::ecs::reflect_field::Reflect;
#[cfg(feature = "editor")]
use once_cell::sync::Lazy;

/// Human‑readable names of all components that have been registered with `inspector_module!`.
#[cfg(feature = "editor")]
pub static MODULES: Lazy<Vec<&'static ModuleFactoryEntry>> = Lazy::new(|| {
    inventory::iter::<ModuleFactoryEntry>.into_iter().collect()
});

#[cfg(feature = "editor")]
pub trait InspectorModuleFactory {
    /// Human‑readable name that will be shown as the collapsible title.
    fn title(&self) -> &'static str;
    /// Builds the concrete module.
    fn make(&self) -> Box<dyn InspectorModule>;
}

#[cfg(feature = "editor")]
pub struct ModuleFactoryEntry {
    pub title: &'static str,
    /// Factory that builds the concrete UI module.
    pub factory: fn() -> Box<dyn InspectorModule>,
}

#[cfg(feature = "editor")]
inventory::collect!(ModuleFactoryEntry);

#[cfg(feature = "editor")]
pub fn make_module<T>(title: &str, removable: bool) -> Box<dyn InspectorModule>
where
    T: Component + Reflect + Default + 'static,
{
    Box::new(
        CollapsibleModule::new(GenericModule::<T>::new(removable))
            .with_title(title),
    )
}

/// Public macro for each component that appears in the inspector.
#[cfg(feature = "editor")]
#[macro_export]
macro_rules! inspector_module {
    ($ty:ty) => {
        inspector_module!($ty, removable = true);
    };

    ($ty:ty, removable = $removable:expr) => {
        inventory::submit! {
            $crate::ecs::module_factory::ModuleFactoryEntry {
                title: <$ty>::TYPE_NAME,
                factory: || $crate::ecs::module_factory::make_module::<$ty>(<$ty>::TYPE_NAME, $removable),
            }
        }
    };
}

/// No-op outside editor builds so component definitions compile in the game crate.
#[cfg(not(feature = "editor"))]
#[macro_export]
macro_rules! inspector_module {
    ($ty:ty) => {};
    ($ty:ty, removable = $removable:expr) => {};
}
