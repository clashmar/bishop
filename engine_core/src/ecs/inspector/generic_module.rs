// engine_core/src/ecs/inspector/generic_module.rs
use crate::ecs::component::{comp_type_name, Component};
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::Entity;
use crate::ecs::inspector::layout::InspectorBodyLayout;
use crate::ecs::inspector::module::InspectorModule;
use crate::ecs::reflect_field::*;
use crate::ecs::Pivot;
use crate::game::*;
use crate::ui::text::*;
use crate::ui::widgets::*;
use bishop::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;
use widgets::constants::{colors, layout};

const TOP_PADDING: f32 = 10.0;
const SPACING: f32 = 5.0;
const LABEL_PADDING: f32 = 10.0;
const MIN_WIDGET_WIDTH: f32 = 80.0;
const MIN_LABEL_WIDTH: f32 = 80.0;
const FONT_SIZE: f32 = layout::DEFAULT_FONT_SIZE_16;

/// A thin wrapper that can draw *any* `T: Reflect`.
pub struct GenericModule<T> {
    _phantom: PhantomData<T>,
    field_ids: HashMap<String, WidgetId>,
    removable: bool,
    was_editing: bool,
    field_snapshots: HashMap<WidgetId, FieldSnapshot>,
}

enum FieldSnapshot {
    Text(String),
    Float(f32),
    Int(i32),
}

impl<T> Default for GenericModule<T> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
            field_ids: HashMap::new(),
            field_snapshots: HashMap::new(),
            removable: true,
            was_editing: false,
        }
    }
}

impl<T> GenericModule<T> {
    fn show_number_with_snapshot(
        &mut self,
        id: WidgetId,
        rect: Rect,
        current: f32,
        blocked: bool,
        assign: &mut dyn FnMut(f32),
        ctx: &mut WgpuContext,
    ) {
        let snapshot = self
            .field_snapshots
            .entry(id)
            .or_insert_with(|| FieldSnapshot::Float(current));
        let orig = match *snapshot {
            FieldSnapshot::Float(v) => v,
            _ => unreachable!(),
        };
        let (new, commit) = NumberInput::new(id, rect, orig).blocked(blocked).show(ctx);
        match commit {
            InputCommit::Previewing => {
                self.was_editing = true;
                if (new - current).abs() > f32::EPSILON {
                    assign(new);
                }
            }
            InputCommit::Committed => {
                self.field_snapshots.remove(&id);
                self.was_editing = true;
                if (new - current).abs() > f32::EPSILON {
                    assign(new);
                }
            }
            InputCommit::Unchanged => {
                if let Some(FieldSnapshot::Float(original)) = self.field_snapshots.remove(&id) {
                    if (original - current).abs() > f32::EPSILON {
                        assign(original);
                    }
                }
            }
        }
    }

    pub fn new(removable: bool) -> Self {
        Self {
            _phantom: PhantomData,
            field_ids: HashMap::new(),
            field_snapshots: HashMap::new(),
            removable,
            was_editing: false,
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
        self.was_editing = false;

        let component = {
            match ecs.get_store_mut::<T>().get_mut(entity) {
                Some(c) => c,
                None => return,
            }
        };

        let mut y = rect.y + TOP_PADDING;

        for field in component.fields() {
            let base_key = field.name.to_string();
            let base_id = *self.field_ids.entry(base_key.clone()).or_default();

            let display_name = parse_field_name(field.name);
            let label = format!("{} :", display_name);
            let label_w = measure_text(ctx, &label, FONT_SIZE)
                .width
                .max(MIN_LABEL_WIDTH);
            let widget_x = rect.x + label_w + LABEL_PADDING;

            ctx.draw_text(
                &label,
                rect.x,
                y + 22.0,
                FONT_SIZE,
                colors::DEFAULT_TEXT_COLOR,
            );

            let widget_x = if widget_x > rect.x + rect.w - MIN_WIDGET_WIDTH {
                rect.x + rect.w - MIN_WIDGET_WIDTH
            } else {
                widget_x
            };

            let widget_w = (rect.x + rect.w) - widget_x - 10.0;
            let widget_rect = Rect::new(
                widget_x,
                y,
                widget_w.max(MIN_WIDGET_WIDTH),
                layout::DEFAULT_FIELD_HEIGHT,
            );

            match (field.value, field.widget_hint) {
                (FieldValue::SpriteId(id), _) => {
                    gui_sprite_picker(
                        ctx,
                        widget_rect,
                        base_id,
                        id,
                        game_ctx.asset_registry,
                        game_ctx.sprite_manager,
                        blocked,
                    );
                }
                (FieldValue::Text(txt), _) => {
                    let snapshot = self
                        .field_snapshots
                        .entry(base_id)
                        .or_insert_with(|| FieldSnapshot::Text(txt.clone()));
                    let current = match snapshot {
                        FieldSnapshot::Text(s) => s.as_str(),
                        _ => unreachable!(),
                    };
                    let (new, commit) = TextInput::new(base_id, widget_rect, current)
                        .blocked(blocked)
                        .show(ctx);
                    match commit {
                        InputCommit::Previewing => {
                            self.was_editing = true;
                            if new != *txt {
                                *txt = new;
                            }
                        }
                        InputCommit::Committed => {
                            self.field_snapshots.remove(&base_id);
                            self.was_editing = true;
                            if new != *txt {
                                *txt = new;
                            }
                        }
                        InputCommit::Unchanged => {
                            if let Some(FieldSnapshot::Text(original)) =
                                self.field_snapshots.remove(&base_id)
                            {
                                if original != *txt {
                                    *txt = original;
                                }
                            }
                        }
                    }
                }
                (FieldValue::Float(f), _) => {
                    let snapshot = self
                        .field_snapshots
                        .entry(base_id)
                        .or_insert_with(|| FieldSnapshot::Float(*f));
                    let current = match *snapshot {
                        FieldSnapshot::Float(v) => v,
                        _ => unreachable!(),
                    };
                    let (new, commit) = NumberInput::new(base_id, widget_rect, current)
                        .blocked(blocked)
                        .show(ctx);
                    match commit {
                        InputCommit::Previewing => {
                            self.was_editing = true;
                            if (new - *f).abs() > f32::EPSILON {
                                *f = new;
                            }
                        }
                        InputCommit::Committed => {
                            self.field_snapshots.remove(&base_id);
                            self.was_editing = true;
                            if (new - *f).abs() > f32::EPSILON {
                                *f = new;
                            }
                        }
                        InputCommit::Unchanged => {
                            if let Some(FieldSnapshot::Float(original)) =
                                self.field_snapshots.remove(&base_id)
                            {
                                if (original - *f).abs() > f32::EPSILON {
                                    *f = original;
                                }
                            }
                        }
                    }
                }
                (FieldValue::Int(i), _) => {
                    let snapshot = self
                        .field_snapshots
                        .entry(base_id)
                        .or_insert_with(|| FieldSnapshot::Int(*i));
                    let current = match *snapshot {
                        FieldSnapshot::Int(v) => v,
                        _ => unreachable!(),
                    };
                    let (new, commit) = NumberInput::new(base_id, widget_rect, current)
                        .blocked(blocked)
                        .show(ctx);
                    match commit {
                        InputCommit::Previewing => {
                            self.was_editing = true;
                            if new != *i {
                                *i = new;
                            }
                        }
                        InputCommit::Committed => {
                            self.field_snapshots.remove(&base_id);
                            self.was_editing = true;
                            if new != *i {
                                *i = new;
                            }
                        }
                        InputCommit::Unchanged => {
                            if let Some(FieldSnapshot::Int(original)) =
                                self.field_snapshots.remove(&base_id)
                            {
                                if original != *i {
                                    *i = original;
                                }
                            }
                        }
                    }
                }
                (FieldValue::Bool(b), _) => {
                    let cb_rect = Rect::new(
                        widget_rect.x,
                        widget_rect.y + 7.5,
                        layout::DEFAULT_CHECKBOX_DIMS,
                        layout::DEFAULT_CHECKBOX_DIMS,
                    );
                    Checkbox::new(cb_rect, b).blocked(blocked).show(ctx);
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

                    let rect_x = Rect::new(widget_rect.x, widget_rect.y, half - 2.0, widget_rect.h);
                    self.show_number_with_snapshot(
                        id_x,
                        rect_x,
                        v.x,
                        blocked,
                        &mut |val| v.x = val,
                        ctx,
                    );

                    let rect_y = Rect::new(
                        widget_rect.x + half + 2.0,
                        widget_rect.y,
                        half - 2.0,
                        widget_rect.h,
                    );
                    self.show_number_with_snapshot(
                        id_y,
                        rect_y,
                        v.y,
                        blocked,
                        &mut |val| v.y = val,
                        ctx,
                    );
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

                    let rect_x = Rect::new(widget_rect.x, widget_rect.y, third, widget_rect.h);
                    self.show_number_with_snapshot(
                        id_x,
                        rect_x,
                        v.x,
                        blocked,
                        &mut |val| v.x = val,
                        ctx,
                    );

                    let rect_y = Rect::new(
                        widget_rect.x + third + 2.0,
                        widget_rect.y,
                        third,
                        widget_rect.h,
                    );
                    self.show_number_with_snapshot(
                        id_y,
                        rect_y,
                        v.y,
                        blocked,
                        &mut |val| v.y = val,
                        ctx,
                    );

                    let rect_z = Rect::new(
                        widget_rect.x + 2.0 * third + 4.0,
                        widget_rect.y,
                        third,
                        widget_rect.h,
                    );
                    self.show_number_with_snapshot(
                        id_z,
                        rect_z,
                        v.z,
                        blocked,
                        &mut |val| v.z = val,
                        ctx,
                    );
                }
                (FieldValue::Pivot(pivot), _) => {
                    if let Some(selected) =
                        Dropdown::new(base_id, widget_rect, pivot.label(), Pivot::all(), |p| {
                            p.label().to_string()
                        })
                        .suppressed(blocked)
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

    fn was_input_active(&self) -> bool {
        self.was_editing
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        Ecs::remove_component::<T>(game_ctx, entity);
    }
}
