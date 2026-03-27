// engine_core/src/ecs/generic_module.rs
use crate::ecs::component::{Component, comp_type_name};
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::inspector_layout::InspectorBodyLayout;
use crate::ecs::inspector_module::InspectorModule;
use crate::ecs::reflect_field::*;
use crate::ecs::transform::Pivot;
use crate::game::*;
use crate::ui::text::*;
use crate::ui::widgets::*;
use bishop::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;

const TOP_PADDING: f32 = 10.0;
const SPACING: f32 = 5.0;
const LABEL_PADDING: f32 = 10.0;
const MIN_WIDGET_WIDTH: f32 = 80.0;
const MIN_LABEL_WIDTH: f32 = 80.0;
const FONT_SIZE: f32 = DEFAULT_FONT_SIZE_16;

/// A thin wrapper that can draw *any* `T: Reflect`.
pub struct GenericModule<T> {
    _phantom: PhantomData<T>,
    field_ids: HashMap<String, WidgetId>,
    removable: bool,
}

impl<T> Default for GenericModule<T> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
            field_ids: HashMap::new(),
            removable: true,
        }
    }
}

impl<T> GenericModule<T> {
    pub fn new(removable: bool) -> Self {
        Self {
            _phantom: PhantomData,
            field_ids: HashMap::new(),
            removable,
        }
    }
}

impl<T> InspectorModule for GenericModule<T>
where
    T: Reflect + Component + Default + 'static,
{
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(comp_type_name::<T>())
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        // Use the new `get_store` helper
        ecs.get_store::<T>().contains(entity)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let ecs = &mut game_ctx.ecs;

        // Grab a mutable reference to the component instance
        let component = {
            match ecs.get_store_mut::<T>().get_mut(entity) {
                Some(c) => c,
                None => return,
            }
        };

        // Layout constants
        let mut y = rect.y + TOP_PADDING;

        // Iterate over the fields supplied by the `Reflect` impl
        for field in component.fields() {
            // Create the id for the widget
            let base_key = field.name.to_string();
            let base_id = *self.field_ids.entry(base_key.clone()).or_default();

            // Prepare the field label
            let display_name = parse_field_name(field.name);
            let label = format!("{} :", display_name);
            let label_w = measure_text(ctx, &label, FONT_SIZE)
                .width
                .max(MIN_LABEL_WIDTH);
            let widget_x = rect.x + label_w + LABEL_PADDING;

            ctx.draw_text(&label, rect.x, y + 22.0, FONT_SIZE, FIELD_TEXT_COLOR);

            // Widget rectangle
            let widget_x = if widget_x > rect.x + rect.w - MIN_WIDGET_WIDTH {
                // Clamp the widget size to the min length
                rect.x + rect.w - MIN_WIDGET_WIDTH
            } else {
                widget_x
            };

            let widget_w = (rect.x + rect.w) - widget_x - 10.0;
            let widget_rect = Rect::new(
                widget_x,
                y,
                widget_w.max(MIN_WIDGET_WIDTH),
                DEFAULT_FIELD_HEIGHT,
            );

            // Dispatch based on the enum variant
            match (field.value, field.widget_hint) {
                (FieldValue::SpriteId(id), _) => {
                    gui_sprite_picker(ctx, widget_rect, id, game_ctx.asset_manager, blocked);
                }
                (FieldValue::Text(txt), _) => {
                    let (new, _) = TextInput::new(base_id, widget_rect, txt.as_str())
                        .blocked(blocked)
                        .show(ctx);
                    if new != *txt {
                        *txt = new;
                    }
                }
                (FieldValue::Float(f), _) => {
                    let new = NumberInput::new(base_id, widget_rect, *f)
                        .blocked(blocked)
                        .show(ctx);
                    if (new - *f).abs() > f32::EPSILON {
                        *f = new;
                    }
                }
                (FieldValue::Int(i), _) => {
                    let new = NumberInput::new(base_id, widget_rect, *i)
                        .blocked(blocked)
                        .show(ctx);
                    if new != *i {
                        *i = new;
                    }
                }
                (FieldValue::Bool(b), _) => {
                    let cb_rect = Rect::new(
                        widget_rect.x,
                        widget_rect.y + 7.5,
                        DEFAULT_CHECKBOX_DIMS,
                        DEFAULT_CHECKBOX_DIMS,
                    );
                    let mut v = *b;
                    if gui_checkbox(ctx, cb_rect, &mut v) && !blocked {
                        *b = v;
                    }
                }
                (FieldValue::Vec2(v), _) => {
                    let id_x = *self
                        .field_ids
                        .entry(format!("{}.x", field.name))
                        .or_default();

                    let id_y = *self
                        .field_ids
                        .entry(format!("{}.y", field.name))
                        .or_default();

                    let half = widget_rect.w / 2.0;

                    // X
                    let rect_x = Rect::new(widget_rect.x, widget_rect.y, half - 2.0, widget_rect.h);
                    let new_x = NumberInput::new(id_x, rect_x, v.x)
                        .blocked(blocked)
                        .show(ctx);
                    if (new_x - v.x).abs() > f32::EPSILON {
                        v.x = new_x;
                    }
                    // Y
                    let rect_y = Rect::new(
                        widget_rect.x + half + 2.0,
                        widget_rect.y,
                        half - 2.0,
                        widget_rect.h,
                    );
                    let new_y = NumberInput::new(id_y, rect_y, v.y)
                        .blocked(blocked)
                        .show(ctx);
                    if (new_y - v.y).abs() > f32::EPSILON {
                        v.y = new_y;
                    }
                }
                (FieldValue::Vec3(v), _) => {
                    let id_x = *self
                        .field_ids
                        .entry(format!("{}.x", field.name))
                        .or_default();
                    let id_y = *self
                        .field_ids
                        .entry(format!("{}.y", field.name))
                        .or_default();
                    let id_z = *self
                        .field_ids
                        .entry(format!("{}.z", field.name))
                        .or_default();

                    let third = widget_rect.w / 3.0 - SPACING / 3.0;

                    // X
                    let rect_x = Rect::new(widget_rect.x, widget_rect.y, third, widget_rect.h);
                    let new_x = NumberInput::new(id_x, rect_x, v.x)
                        .blocked(blocked)
                        .show(ctx);
                    if (new_x - v.x).abs() > f32::EPSILON {
                        v.x = new_x;
                    }
                    // Y
                    let rect_y = Rect::new(
                        widget_rect.x + third + 2.0,
                        widget_rect.y,
                        third,
                        widget_rect.h,
                    );
                    let new_y = NumberInput::new(id_y, rect_y, v.y)
                        .blocked(blocked)
                        .show(ctx);
                    if (new_y - v.y).abs() > f32::EPSILON {
                        v.y = new_y;
                    }
                    // Z
                    let rect_z = Rect::new(
                        widget_rect.x + 2.0 * third + 4.0,
                        widget_rect.y,
                        third,
                        widget_rect.h,
                    );
                    let new_z = NumberInput::new(id_z, rect_z, v.z)
                        .blocked(blocked)
                        .show(ctx);
                    if (new_z - v.z).abs() > f32::EPSILON {
                        v.z = new_z;
                    }
                }
                (FieldValue::Pivot(pivot), _) => {
                    if let Some(selected) =
                        Dropdown::new(base_id, widget_rect, pivot.label(), Pivot::all(), |p| {
                            p.label().to_string()
                        })
                        .blocked(blocked)
                        .show(ctx)
                    {
                        *pivot = selected;
                    }
                }
            }

            y += widget_rect.h + SPACING;
        }
    }

    /// Compute the body layout from the number of reflected fields.
    fn body_layout(&self) -> InspectorBodyLayout {
        // Create a temporary default instance of `T` only to query its fields
        let mut temp = T::default();
        let field_count = temp.fields().len();

        InspectorBodyLayout::new().rows(field_count, SPACING)
    }

    fn removable(&self) -> bool {
        self.removable
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        Ecs::remove_component::<T>(game_ctx, entity);
    }
}
