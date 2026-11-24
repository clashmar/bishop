use std::cell::Cell;

use bishop::prelude::{Color, Rect, Vec3, WgpuContext};
use crate::gui::field_layout::{
    BOTTOM_GUTTER, FIELD_GAP, FIELD_HEIGHT, TOP_PADDING, draw_label, field_rect,
};
use engine_core::{
    ecs::{
        components::{PointLight, PointLightLayerScope, PointLightStyle},
        inspector::{
            factory::ModuleFactoryEntry,
            layout::InspectorBodyLayout,
            module::{CollapsibleComponentModule, InspectorModule},
        },
        Ecs, Entity,
    },
    game::GameCtxMut,
};
use widgets::{ColorInput, Dropdown, InputCommit, NumberInput, Widget, WidgetId};

/// Title for the point-light inspector module.
pub const POINT_LIGHT_MODULE_TITLE: &str = "Point Light";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct VisibleControls {
    pixel_size: bool,
}

/// Edits point-light settings in the entity inspector.
#[derive(Default)]
pub struct PointLightModule {
    color_id: WidgetId,
    intensity_id: WidgetId,
    radius_id: WidgetId,
    attenuation_id: WidgetId,
    scope_id: WidgetId,
    style_id: WidgetId,
    pixel_size_id: WidgetId,
    controls: Cell<VisibleControls>,
    was_editing: Cell<bool>,
}

impl InspectorModule for PointLightModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(PointLight::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        let Some(light) = ecs.get::<PointLight>(entity) else {
            self.controls.set(VisibleControls::default());
            return false;
        };
        self.controls.set(visible_controls(light.style));
        true
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        Ecs::remove_component::<PointLight>(game_ctx, entity);
    }

    fn was_input_active(&self) -> bool {
        self.was_editing.get()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let rows = 6 + usize::from(self.controls.get().pixel_size);
        InspectorBodyLayout::new()
            .top_padding(TOP_PADDING)
            .rows(rows, FIELD_GAP)
            .bottom_gutter(BOTTOM_GUTTER)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        self.was_editing.set(false);
        let Some(light) = game_ctx.ecs.get_mut::<PointLight>(entity) else {
            return;
        };

        let mut y = rect.y + TOP_PADDING;
        draw_label(ctx, "Color:", rect, y);
        let current_color = Color::new(light.color.x, light.color.y, light.color.z, 1.0);
        let (color, color_commit) = ColorInput::new(self.color_id, field_rect(rect, y), current_color)
            .blocked(blocked)
            .show_with_commit(ctx);
        if input_active(color_commit) {
            light.color = Vec3::new(color.r, color.g, color.b);
            self.was_editing.set(true);
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Intensity:", rect, y);
        let (intensity, intensity_commit) = NumberInput::new(
            self.intensity_id,
            field_rect(rect, y),
            light.intensity,
        )
        .min(0.0)
        .blocked(blocked)
        .show(ctx);
        if input_active(intensity_commit) {
            light.intensity = nonnegative_input(intensity, intensity_commit);
            self.was_editing.set(true);
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Radius:", rect, y);
        let (radius, radius_commit) = NumberInput::new(self.radius_id, field_rect(rect, y), light.radius)
            .min(0.0)
            .blocked(blocked)
            .show(ctx);
        if input_active(radius_commit) {
            light.radius = nonnegative_input(radius, radius_commit);
            self.was_editing.set(true);
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Attenuation:", rect, y);
        let (attenuation, attenuation_commit) = NumberInput::new(
            self.attenuation_id,
            field_rect(rect, y),
            light.attenuation,
        )
        .min(0.0)
        .blocked(blocked)
        .show(ctx);
        if input_active(attenuation_commit) {
            light.attenuation = nonnegative_input(attenuation, attenuation_commit);
            self.was_editing.set(true);
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Layer Scope:", rect, y);
        let scope_label = light.layer_scope.to_string();
        if let Some(scope) = Dropdown::new(
            self.scope_id,
            field_rect(rect, y),
            &scope_label,
            &PointLightLayerScope::ALL,
            |scope| scope.to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            light.layer_scope = scope;
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Style:", rect, y);
        let style_label = light.style.to_string();
        if let Some(style) = Dropdown::new(
            self.style_id,
            field_rect(rect, y),
            &style_label,
            &PointLightStyle::ALL,
            |style| style.to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            light.style = style;
            self.controls.set(visible_controls(style));
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        if light.style == PointLightStyle::Pixel {
            draw_label(ctx, "Pixel Size:", rect, y);
            let (pixel_size, pixel_size_commit) = NumberInput::new(
                self.pixel_size_id,
                field_rect(rect, y),
                light.pixel_size,
            )
            .min(1)
            .blocked(blocked)
            .show(ctx);
            if input_active(pixel_size_commit) {
                light.pixel_size = pixel_size.max(1);
                self.was_editing.set(true);
            }
        }
    }
}

fn visible_controls(style: PointLightStyle) -> VisibleControls {
    VisibleControls {
        pixel_size: style == PointLightStyle::Pixel,
    }
}

fn input_active(commit: InputCommit) -> bool {
    matches!(commit, InputCommit::Previewing | InputCommit::Committed)
}

fn nonnegative_input(value: f32, commit: InputCommit) -> f32 {
    if commit == InputCommit::Committed {
        value.max(0.0)
    } else {
        value
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: PointLight::TYPE_NAME,
        title: POINT_LIGHT_MODULE_TITLE,
        tooltip: "Configures color, intensity, radius, layer scope, and world-space pixel size.",
        factory: || Box::new(
            CollapsibleComponentModule::new(PointLightModule::default())
                .with_title(POINT_LIGHT_MODULE_TITLE)
        ),
        allowed_for: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{POINT_LIGHT_MODULE_TITLE, PointLightModule, visible_controls};
    use engine_core::ecs::PointLight;
    use engine_core::ecs::components::PointLightStyle;
    use engine_core::ecs::inspector::factory::MODULES;
    use engine_core::ecs::inspector::module::InspectorModule;

    #[test]
    fn pixel_size_control_is_visible_only_for_pixel_style() {
        assert!(!visible_controls(PointLightStyle::Smooth).pixel_size);
        assert!(visible_controls(PointLightStyle::Pixel).pixel_size);
    }

    #[test]
    fn point_light_uses_one_registered_custom_inspector() {
        let entries = MODULES
            .iter()
            .filter(|entry| entry.type_name == PointLight::TYPE_NAME)
            .collect::<Vec<_>>();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, POINT_LIGHT_MODULE_TITLE);
        assert_eq!((entries[0].factory)().body_layout(), PointLightModule::default().body_layout());
    }
}
