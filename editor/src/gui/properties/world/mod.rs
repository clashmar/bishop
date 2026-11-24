pub mod world_audio_source_module;
pub mod world_name_module;
pub mod world_navigation_module;
pub mod world_overview_properties;
pub mod world_physics_module;
pub mod world_script_module;
pub mod world_settings_module;
pub mod world_tags_module;
pub mod world_visual_module;

use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use engine_core::worlds::{world::World, WorldVisual};
use ::widgets::constants::layout;

use super::collapsible::CollapsiblePropertyModule;
use super::PropertyModule;
use crate::commands::world::EditWorldTagsCmd;
use crate::editor_global::push_command;
use crate::gui::gui_constants;
use crate::shared::scene_ui::inspector::InspectorContent;
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorOutput};
use world_name_module::WorldNameModule;
use world_navigation_module::WorldNavigationModule;

/// Editable properties for the current world in the World Editor.
pub struct WorldProperties {
    name: WorldNameModule,
    modules: Vec<Box<dyn PropertyModule<World>>>,
    /// Navigation entries/exits module.
    pub navigation: CollapsiblePropertyModule<World, WorldNavigationModule>,
}

impl WorldProperties {
    /// Creates a new world properties pane.
    pub fn new() -> Self {
        Self {
            name: WorldNameModule::new(),
            modules: vec![
                Box::new(CollapsiblePropertyModule::new(
                    world_settings_module::WorldSettingsModule::new(),
                )),
                Box::new(CollapsiblePropertyModule::new(
                    world_physics_module::WorldPhysicsModule::new(),
                )),
                Box::new(CollapsiblePropertyModule::new(
                    world_script_module::WorldScriptModule::new(),
                )),
                Box::new(CollapsiblePropertyModule::new(
                    world_audio_source_module::WorldAudioSourceModule::new(),
                )),
                Box::new(CollapsiblePropertyModule::new(
                    world_tags_module::WorldTagsModule::for_world(),
                )),
            ],
            navigation: CollapsiblePropertyModule::new(WorldNavigationModule::new()),
        }
    }

    /// Whether navigation icons should be shown in the world editor.
    pub fn show_navigation_icons(&self) -> bool {
        self.navigation.inner().show_icons
    }

    /// Collapses every collapsible world property module.
    pub fn collapse_all(&mut self) {
        collapse_world_modules(&mut self.modules);
        self.navigation.set_expanded(false);
    }
}

impl Default for WorldProperties {
    fn default() -> Self {
        Self::new()
    }
}

impl InspectorContent for WorldProperties {
    fn header_height(&self) -> f32 {
        gui_constants::inspector::HEADER_HEIGHT
    }

    fn has_collapsible_modules(&self) -> bool {
        self.modules
            .iter()
            .any(|module| module.expansion_state().is_some())
            || self.navigation.expansion_state().is_some()
    }

    fn collapse_all(&mut self) {
        WorldProperties::collapse_all(self);
    }

    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _blocked: bool,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        let mut output = draw_world_modules(
            &mut self.name,
            &mut self.modules,
            ctx,
            rect,
            game_ctx,
            insp_ctx,
        );
        let Some(world) = game_ctx.world.as_deref() else {
            return output;
        };
        let mut world_clone = world.clone();
        let y = rect.y + 10.0 + world_module_content_height(&self.name, &self.modules, game_ctx);
        let content_w = rect.w - 20.0;
        let nav_rect = Rect::new(rect.x + 10.0, y, content_w, self.navigation.height());
        let previous_navigation_expansion = self.navigation.expansion_state();
        self.navigation
            .draw(ctx, nav_rect, &mut world_clone, game_ctx, insp_ctx);
        if output.expanded_module_rect.is_none() {
            output.expanded_module_rect = self
                .navigation
                .expansion_reveal_rect(previous_navigation_expansion, nav_rect);
        }
        if output.host_action.is_none() {
            output.host_action = self.navigation.take_host_action();
        }
        output
    }

    fn total_content_height(
        &self,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> f32 {
        if game_ctx.world.is_none() {
            return 0.0;
        }
        world_module_content_height(&self.name, &self.modules, game_ctx)
            + self.navigation.height()
            + layout::WIDGET_SPACING
            + 20.0
    }
}

fn draw_world_modules(
    name: &mut WorldNameModule,
    modules: &mut [Box<dyn PropertyModule<World>>],
    ctx: &mut WgpuContext,
    rect: Rect,
    game_ctx: &mut GameCtxMut,
    insp_ctx: &InspectorContext,
) -> InspectorOutput {
    let mut output = InspectorOutput::default();
    let Some(world) = game_ctx.world.as_deref() else {
        return output;
    };
    let original_tags = world.tags.clone();
    let mut world_clone = world.clone();
    let original_visual = world_clone.meta.visual;
    let content_w = rect.w - 20.0;
    let mut y = rect.y + 10.0;

    let name_rect = Rect::new(rect.x + 10.0, y, content_w, name.height());
    name.draw(ctx, name_rect, &mut world_clone, game_ctx, insp_ctx);
    output.host_action = name.take_host_action();
    y += name.height() + layout::WIDGET_SPACING;

    for module in modules {
        if module.visible(&world_clone, game_ctx) {
            let module_rect = Rect::new(rect.x + 10.0, y, content_w, module.height());
            let previous_expansion = module.expansion_state();
            module.draw(ctx, module_rect, &mut world_clone, game_ctx, insp_ctx);
            if output.expanded_module_rect.is_none() {
                output.expanded_module_rect =
                    module.expansion_reveal_rect(previous_expansion, module_rect);
            }
            if output.host_action.is_none() {
                output.host_action = module.take_host_action();
            }
            y += module.height() + layout::WIDGET_SPACING;
        }
    }

    if let Some(live_world) = game_ctx.world.as_deref_mut() {
        copy_world_visual_preview(live_world, &world_clone, original_visual);
    }

    if original_tags != world_clone.tags {
        push_command(Box::new(EditWorldTagsCmd::new(
            world_clone.id,
            original_tags,
            world_clone.tags,
        )));
    }

    output
}

fn world_module_content_height(
    name: &WorldNameModule,
    modules: &[Box<dyn PropertyModule<World>>],
    game_ctx: &mut GameCtxMut,
) -> f32 {
    let Some(world) = game_ctx.world.as_deref() else {
        return 0.0;
    };
    let mut height = name.height() + layout::WIDGET_SPACING;

    for module in modules {
        if module.visible(world, game_ctx) {
            height += module.height() + layout::WIDGET_SPACING;
        }
    }
    height
}

fn copy_world_visual_preview(
    live_world: &mut World,
    edited_world: &World,
    original_visual: WorldVisual,
) {
    if live_world.id == edited_world.id && edited_world.meta.visual != original_visual {
        live_world.meta.visual = edited_world.meta.visual;
    }
}

fn collapse_world_modules(modules: &mut [Box<dyn PropertyModule<World>>]) {
    for module in modules {
        module.set_expanded(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module_titles(modules: &[Box<dyn PropertyModule<World>>]) -> Vec<String> {
        modules.iter().map(|module| module.title().to_owned()).collect()
    }

    #[test]
    fn world_inspector_composition_uses_host_specific_modules() {
        let overview = world_overview_properties::WorldOverviewProperties::new();
        let world_editor = WorldProperties::new();

        assert_eq!(overview.name.title(), "World Name");
        assert_eq!(module_titles(&overview.modules), ["World Visual"]);
        assert_eq!(world_editor.name.title(), "World Name");
        let world_editor_titles = module_titles(&world_editor.modules);
        assert!(world_editor_titles.contains(&"General Settings".to_owned()));
        assert!(world_editor_titles.contains(&"Physics".to_owned()));
        assert!(world_editor_titles.contains(&"Script".to_owned()));
        assert!(world_editor_titles.contains(&"Audio".to_owned()));
        assert_eq!(world_editor.modules.len(), 5);
        assert!(!world_editor_titles.contains(&"World Visual".to_owned()));
        assert_eq!(world_editor.navigation.title(), "Navigation");
    }

    #[test]
    fn world_visual_preview_copies_color_and_opacity_to_live_world() {
        let mut live_world = World::default();
        let initial = WorldVisual::Color(Color::new(0.2, 0.3, 0.4, 0.25));
        live_world.meta.visual = initial;
        let mut edited_world = live_world.clone();
        let preview = WorldVisual::Color(Color::new(0.7, 0.5, 0.1, 0.6));
        edited_world.meta.visual = preview;

        copy_world_visual_preview(&mut live_world, &edited_world, initial);

        assert_eq!(live_world.meta.visual, preview);
    }

    #[test]
    fn world_visual_preview_does_not_copy_unchanged_clone_over_live_change() {
        let initial = WorldVisual::Color(Color::new(0.2, 0.3, 0.4, 0.25));
        let mut live_world = World::default();
        live_world.meta.visual = WorldVisual::Color(Color::new(0.8, 0.1, 0.6, 0.9));
        let mut unchanged_clone = World::default();
        unchanged_clone.id = live_world.id;
        unchanged_clone.meta.visual = initial;

        copy_world_visual_preview(&mut live_world, &unchanged_clone, initial);

        assert_eq!(live_world.meta.visual, WorldVisual::Color(Color::new(0.8, 0.1, 0.6, 0.9)));
    }

    #[test]
    fn collapse_all_collapses_world_modules_and_navigation() {
        let mut properties = WorldProperties::new();
        assert!(!properties.modules.is_empty());

        properties.collapse_all();

        assert!(properties
            .modules
            .iter()
            .all(|module| module.expansion_state() == Some(false)));
        assert_eq!(properties.navigation.expansion_state(), Some(false));
    }
}
