use std::cell::Cell;

use bishop::prelude::{Color, Rect, Vec3, WgpuContext};
use crate::gui::field_layout::{
    BOTTOM_GUTTER, FIELD_GAP, FIELD_HEIGHT, LABEL_WIDTH, TOP_PADDING, draw_float_slider,
    draw_integer_slider, draw_label, field_rect, field_rect_with_label_width, input_active,
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
use widgets::{ColorInput, ColorInputEdit, Dropdown, Widget, WidgetId};

/// Title for the point-light inspector module.
pub const POINT_LIGHT_MODULE_TITLE: &str = "Point Light";

const INTENSITY_MAX: f32 = 20.0;
const RADIUS_MAX: f32 = 256.0;
const ATTENUATION_MAX: f32 = 10.0;
const PIXEL_SIZE_MIN: i32 = 1;
const PIXEL_SIZE_MAX: i32 = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct VisibleControls {
    pixel_size: bool,
}

/// Edits point-light settings in the entity inspector.
#[derive(Default)]
pub struct PointLightModule {
    color_id: WidgetId,
    intensity_id: WidgetId,
    intensity_slider_id: WidgetId,
    radius_id: WidgetId,
    radius_slider_id: WidgetId,
    attenuation_id: WidgetId,
    attenuation_slider_id: WidgetId,
    scope_id: WidgetId,
    style_id: WidgetId,
    pixel_size_id: WidgetId,
    pixel_size_slider_id: WidgetId,
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
        let (color_response, color_commit) = ColorInput::new(
            self.color_id,
            field_rect(rect, y),
            current_color,
        )
            .opacity_enabled(false)
            .blocked(blocked)
            .show_with_commit(ctx);
        match color_response.edit {
            ColorInputEdit::Preview { .. } | ColorInputEdit::Commit { .. } => {
                light.color = Vec3::new(
                    color_response.color.r,
                    color_response.color.g,
                    color_response.color.b,
                );
                self.was_editing.set(true);
            }
            ColorInputEdit::Cancel { initial } => {
                light.color = Vec3::new(initial.r, initial.g, initial.b);
                self.was_editing.set(true);
            }
            ColorInputEdit::None if input_active(color_commit) => {
                let color = color_response.color;
                light.color = Vec3::new(color.r, color.g, color.b);
                self.was_editing.set(true);
            }
            ColorInputEdit::None => {}
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Intensity:", rect, y);
        let (intensity, intensity_active) = draw_float_slider(
            ctx,
            blocked,
            rect,
            y,
            self.intensity_id,
            self.intensity_slider_id,
            light.intensity,
            INTENSITY_MAX,
            2,
        );
        if intensity_active {
            light.intensity = intensity;
            self.was_editing.set(true);
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Radius:", rect, y);
        let (radius, radius_active) = draw_float_slider(
            ctx,
            blocked,
            rect,
            y,
            self.radius_id,
            self.radius_slider_id,
            light.radius,
            RADIUS_MAX,
            1,
        );
        if radius_active {
            light.radius = radius;
            self.was_editing.set(true);
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Attenuation:", rect, y);
        let (attenuation, attenuation_active) = draw_float_slider(
            ctx,
            blocked,
            rect,
            y,
            self.attenuation_id,
            self.attenuation_slider_id,
            light.attenuation,
            ATTENUATION_MAX,
            2,
        );
        if attenuation_active {
            light.attenuation = attenuation;
            self.was_editing.set(true);
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Layer Scope:", rect, y);
        let scope_label = light.layer_scope.to_string();
        if let Some(scope) = Dropdown::new(
            self.scope_id,
            field_rect_with_label_width(rect, y, LABEL_WIDTH + 8.0),
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
            let (pixel_size, pixel_size_active) = draw_integer_slider(
                ctx,
                blocked,
                rect,
                y,
                self.pixel_size_id,
                self.pixel_size_slider_id,
                light.pixel_size,
                PIXEL_SIZE_MIN,
                PIXEL_SIZE_MAX,
            );
            if pixel_size_active {
                light.pixel_size = pixel_size;
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
