use std::cell::Cell;

use bishop::prelude::*;
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::inspector::module::{CollapsibleComponentModule, InspectorModule};
use engine_core::ecs::*;
use engine_core::game::GameCtxMut;
use engine_core::ui::measure_text;
use widgets::constants::{colors, layout};
use widgets::{Dropdown, InputCommit, NumberInput, Widget, WidgetId};

const TITLE: &str = "Kinematic";
const BODY_TOP_PADDING: f32 = layout::WIDGET_SPACING;
const BODY_BOTTOM_GUTTER: f32 = 10.0;
const ROW_HEIGHT: f32 = 30.0;
const FIELD_GAP: f32 = layout::WIDGET_SPACING;
const LABEL_Y_OFFSET: f32 = 20.0;

#[derive(Default)]
pub struct KinematicModule {
    contact_behavior_id: WidgetId,
    mode_id: WidgetId,
    axis_id: WidgetId,
    direction_id: WidgetId,
    speed_id: WidgetId,
    distance_id: WidgetId,
    was_editing: bool,
    current_mode: Cell<KinematicMotionMode>,
}

impl InspectorModule for KinematicModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(Kinematic::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        let Some(kinematic) = ecs.get::<Kinematic>(entity) else {
            return false;
        };

        self.current_mode.set(kinematic.motion.mode);
        true
    }

    fn removable(&self) -> bool {
        true
    }

    fn was_input_active(&self) -> bool {
        self.was_editing
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let mut layout = InspectorBodyLayout::new()
            .top_padding(BODY_TOP_PADDING)
            .bottom_gutter(BODY_BOTTOM_GUTTER)
            .block(ROW_HEIGHT)
            .gap(FIELD_GAP)
            .block(ROW_HEIGHT);

        if self.current_mode.get() != KinematicMotionMode::None {
            layout = layout
                .gap(FIELD_GAP)
                .block(ROW_HEIGHT)
                .gap(FIELD_GAP)
                .block(ROW_HEIGHT)
                .gap(FIELD_GAP)
                .block(ROW_HEIGHT);
        }

        if self.current_mode.get() == KinematicMotionMode::PingPong {
            layout = layout.gap(FIELD_GAP).block(ROW_HEIGHT);
        }

        layout
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        self.was_editing = false;

        let Some(kinematic) = game_ctx.ecs.get_mut::<Kinematic>(entity) else {
            return;
        };

        self.current_mode.set(kinematic.motion.mode);
        let mut y = rect.y + BODY_TOP_PADDING;

        draw_label(ctx, "Contact:", rect.x, y);
        if let Some(contact_behavior) = Dropdown::new(
            self.contact_behavior_id,
            field_rect(ctx, rect, y),
            kinematic.contact_behavior.label(),
            &contact_behavior_options(kinematic.motion.mode),
            |value| value.label().to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            if contact_behavior != kinematic.contact_behavior {
                kinematic.contact_behavior = contact_behavior;
                self.was_editing = true;
            }
        }

        y += ROW_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Motion:", rect.x, y);
        if let Some(mode) = Dropdown::new(
            self.mode_id,
            field_rect(ctx, rect, y),
            kinematic.motion.mode.label(),
            &motion_mode_options(),
            |value| value.label().to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            if mode != kinematic.motion.mode {
                kinematic.motion.mode = mode;
                self.current_mode.set(mode);
                if kinematic.contact_behavior.requires_ping_pong() && mode != KinematicMotionMode::PingPong {
                    kinematic.contact_behavior = KinematicContactBehavior::Stop;
                }
                kinematic.clear_runtime_state();
                self.was_editing = true;
            }
        }

        if kinematic.motion.mode == KinematicMotionMode::None {
            return;
        }

        y += ROW_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Axis:", rect.x, y);
        if let Some(axis) = Dropdown::new(
            self.axis_id,
            field_rect(ctx, rect, y),
            kinematic.motion.axis.label(),
            &axis_options(),
            |value| value.label().to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            if axis != kinematic.motion.axis {
                kinematic.motion.axis = axis;
                kinematic.clear_runtime_state();
                self.was_editing = true;
            }
        }

        y += ROW_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Direction:", rect.x, y);
        if let Some(direction) = Dropdown::new(
            self.direction_id,
            field_rect(ctx, rect, y),
            kinematic.motion.direction.label(),
            &direction_options(),
            |value| value.label().to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            if direction != kinematic.motion.direction {
                kinematic.motion.direction = direction;
                kinematic.clear_runtime_state();
                self.was_editing = true;
            }
        }

        y += ROW_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Speed:", rect.x, y);
        let (speed, speed_commit) = NumberInput::new(self.speed_id, field_rect(ctx, rect, y), kinematic.motion.speed)
            .min(0.0)
            .blocked(blocked)
            .show(ctx);
        if is_input_active(speed_commit) {
            self.was_editing = true;
        }
        if (speed - kinematic.motion.speed).abs() > f32::EPSILON {
            kinematic.motion.speed = speed.max(0.0);
            kinematic.clear_runtime_state();
            self.was_editing = true;
        }

        if kinematic.motion.mode != KinematicMotionMode::PingPong {
            return;
        }

        y += ROW_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Distance:", rect.x, y);
        let (distance, distance_commit) = NumberInput::new(
            self.distance_id,
            field_rect(ctx, rect, y),
            kinematic.motion.travel_distance,
        )
        .min(0.0)
        .blocked(blocked)
        .show(ctx);
        if is_input_active(distance_commit) {
            self.was_editing = true;
        }
        if (distance - kinematic.motion.travel_distance).abs() > f32::EPSILON {
            kinematic.motion.travel_distance = distance.max(0.0);
            kinematic.clear_runtime_state();
            self.was_editing = true;
        }
    }
}

fn is_input_active(commit: InputCommit) -> bool {
    matches!(commit, InputCommit::Previewing | InputCommit::Committed)
}

fn contact_behavior_options(mode: KinematicMotionMode) -> Vec<KinematicContactBehavior> {
    let mut options = vec![
        KinematicContactBehavior::Stop,
        KinematicContactBehavior::Crush,
        KinematicContactBehavior::Eject,
        KinematicContactBehavior::Trigger,
    ];
    if mode == KinematicMotionMode::PingPong {
        options.push(KinematicContactBehavior::Reverse);
    }
    options
}

fn motion_mode_options() -> [KinematicMotionMode; 3] {
    [
        KinematicMotionMode::None,
        KinematicMotionMode::Constant,
        KinematicMotionMode::PingPong,
    ]
}

fn axis_options() -> [KinematicAxis; 2] {
    [KinematicAxis::Horizontal, KinematicAxis::Vertical]
}

fn direction_options() -> [KinematicDirection; 2] {
    [KinematicDirection::Positive, KinematicDirection::Negative]
}

fn draw_label(ctx: &mut WgpuContext, label: &str, x: f32, y: f32) {
    ctx.draw_text(
        label,
        x,
        y + LABEL_Y_OFFSET,
        layout::FIELD_TEXT_SIZE_16,
        colors::DEFAULT_TEXT_COLOR,
    );
}

fn field_rect(ctx: &mut WgpuContext, rect: Rect, y: f32) -> Rect {
    let label_width = shared_label_width(ctx);
    Rect::new(
        rect.x + label_width + layout::WIDGET_SPACING,
        y,
        rect.w - label_width - layout::WIDGET_SPACING,
        ROW_HEIGHT,
    )
}

fn shared_label_width(ctx: &mut WgpuContext) -> f32 {
    ["Contact:", "Motion:", "Axis:", "Direction:", "Speed:", "Distance:"]
        .into_iter()
        .map(|label| measure_text(ctx, label, layout::FIELD_TEXT_SIZE_16).width)
        .fold(0.0, f32::max)
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: Kinematic::TYPE_NAME,
        title: TITLE,
        factory: || {
            Box::new(
                CollapsibleComponentModule::new(
                    crate::gui::inspector::kinematic_module::KinematicModule::default()
                )
                .with_title(TITLE)
            )
        },
        allowed_for: None,
    }
}
