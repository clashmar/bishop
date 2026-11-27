use std::cell::Cell;

use bishop::prelude::{Color, Rect, Vec3, WgpuContext};
use crate::gui::field_layout::{
    BOTTOM_GUTTER, FIELD_GAP, FIELD_HEIGHT, LABEL_WIDTH, TOP_PADDING, draw_float_slider,
    draw_integer_slider, draw_label, field_rect, field_rect_with_label_width, input_active,
};
use engine_core::ecs::{
    components::{Glow, PointLightLayerScope, PointLightStyle},
    inspector::{
        factory::ModuleFactoryEntry,
        layout::InspectorBodyLayout,
        module::{CollapsibleComponentModule, InspectorModule},
    },
    Ecs, Entity,
};
use engine_core::game::GameCtxMut;
use widgets::{ColorInput, ColorInputEdit, Dropdown, Widget, WidgetId};

/// Title shown for the Glow component inspector.
pub const GLOW_MODULE_TITLE: &str = "Glow";

const INTENSITY_MAX: f32 = 20.0;
const HALO_SPREAD_MAX: f32 = 256.0;
const PIXEL_SIZE_MIN: i32 = 1;
const PIXEL_SIZE_MAX: i32 = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct VisibleControls {
    pixel_size: bool,
}

/// Edits sprite-shaped emission settings.
#[derive(Default)]
pub struct GlowModule {
    color_id: WidgetId,
    intensity_id: WidgetId,
    intensity_slider_id: WidgetId,
    halo_spread_id: WidgetId,
    halo_spread_slider_id: WidgetId,
    scope_id: WidgetId,
    style_id: WidgetId,
    pixel_size_id: WidgetId,
    pixel_size_slider_id: WidgetId,
    controls: Cell<VisibleControls>,
    was_editing: Cell<bool>,
}

impl InspectorModule for GlowModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(Glow::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        let Some(glow) = ecs.get::<Glow>(entity) else {
            self.controls.set(VisibleControls::default());
            return false;
        };
        self.controls.set(visible_controls(glow));
        true
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        Ecs::remove_component::<Glow>(game_ctx, entity);
    }

    fn was_input_active(&self) -> bool {
        self.was_editing.get()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let controls = self.controls.get();
        let rows = 5 + usize::from(controls.pixel_size);
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
        let controls;
        {
            let Some(glow) = game_ctx.ecs.get_mut::<Glow>(entity) else {
                return;
            };
            let mut y = rect.y + TOP_PADDING;

            draw_label(ctx, "Color:", rect, y);
            let current_color = Color::new(glow.color.x, glow.color.y, glow.color.z, 1.0);
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
                    let color = color_response.color;
                    glow.color = Vec3::new(color.r, color.g, color.b);
                    self.was_editing.set(true);
                }
                ColorInputEdit::Cancel { initial } => {
                    glow.color = Vec3::new(initial.r, initial.g, initial.b);
                    self.was_editing.set(true);
                }
                ColorInputEdit::None if input_active(color_commit) => {
                    let color = color_response.color;
                    glow.color = Vec3::new(color.r, color.g, color.b);
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
                glow.intensity,
                INTENSITY_MAX,
                2,
            );
            if intensity_active {
                glow.intensity = intensity;
                self.was_editing.set(true);
            }
            y += FIELD_HEIGHT + FIELD_GAP;

            draw_label(ctx, "Halo Spread:", rect, y);
            let (halo_spread, spread_active) = draw_float_slider(
                ctx,
                blocked,
                rect,
                y,
                self.halo_spread_id,
                self.halo_spread_slider_id,
                glow.halo_spread,
                HALO_SPREAD_MAX,
                1,
            );
            if spread_active {
                glow.halo_spread = halo_spread;
                self.was_editing.set(true);
            }
            y += FIELD_HEIGHT + FIELD_GAP;

            draw_label(ctx, "Layer Scope:", rect, y);
            let scope_label = glow.layer_scope.to_string();
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
                glow.layer_scope = scope;
            }
            y += FIELD_HEIGHT + FIELD_GAP;

            draw_label(ctx, "Style:", rect, y);
            let style_label = glow.style.to_string();
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
                glow.style = style;
            }
            y += FIELD_HEIGHT + FIELD_GAP;

            if glow.style == PointLightStyle::Pixel {
                draw_label(ctx, "Pixel Size:", rect, y);
                let (pixel_size, pixel_size_active) = draw_integer_slider(
                    ctx,
                    blocked,
                    rect,
                    y,
                    self.pixel_size_id,
                    self.pixel_size_slider_id,
                    glow.pixel_size,
                    PIXEL_SIZE_MIN,
                    PIXEL_SIZE_MAX,
                );
                if pixel_size_active {
                    glow.pixel_size = pixel_size;
                    self.was_editing.set(true);
                }
            }

            glow.normalize_settings();
            controls = visible_controls(glow);
        }
        self.controls.set(controls);
    }
}

fn visible_controls(glow: &Glow) -> VisibleControls {
    VisibleControls {
        pixel_size: glow.style == PointLightStyle::Pixel,
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: Glow::TYPE_NAME,
        title: GLOW_MODULE_TITLE,
        tooltip: "Configures sprite-shaped emission.",
        factory: || Box::new(
            CollapsibleComponentModule::new(GlowModule::default())
                .with_title(GLOW_MODULE_TITLE)
        ),
        allowed_for: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{GLOW_MODULE_TITLE, GlowModule, visible_controls};
    use engine_core::ecs::components::{Glow, PointLightStyle};
    use engine_core::ecs::inspector::factory::MODULES;
    use engine_core::ecs::inspector::module::InspectorModule;

    #[test]
    fn pixel_size_control_is_visible_only_for_pixel_style() {
        let mut glow = Glow::default();
        assert!(!visible_controls(&glow).pixel_size);

        glow.style = PointLightStyle::Pixel;
        assert!(visible_controls(&glow).pixel_size);
    }

    #[test]
    fn glow_uses_one_registered_custom_inspector() {
        let entries = MODULES
            .iter()
            .filter(|entry| entry.type_name == Glow::TYPE_NAME)
            .collect::<Vec<_>>();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, GLOW_MODULE_TITLE);
        assert_eq!(
            (entries[0].factory)().body_layout(),
            GlowModule::default().body_layout()
        );
    }
}
