// editor/src/gui/inspector/camera_module.rs
use bishop::prelude::*;
use engine_core::prelude::*;
use strum::IntoEnumIterator;

pub const ROOM_CAMERA_MODULE_TITLE: &str = "Room Camera";

#[derive(Default)]
pub struct RoomCameraModule {
    pub mode_id: WidgetId,
    pub zoom_id: WidgetId,
    pub slider_id: WidgetId,
    pub cam_mode_id: WidgetId,
    current_zoom_mode: ZoomMode,
}

const BODY_TOP_PADDING: f32 = WIDGET_SPACING;
const ROOM_CAMERA_BOTTOM_GUTTER: f32 = 10.0;
const ROOM_CAMERA_VISUAL_GAP: f32 = WIDGET_SPACING;
const MODE_ROW_HEIGHT: f32 = 30.0;
const STEP_SECTION_HEIGHT: f32 = 40.0;
const FREE_SECTION_HEIGHT: f32 = 35.0;

impl InspectorModule for RoomCameraModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        None
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<RoomCamera>(entity).is_some()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(BODY_TOP_PADDING)
            .bottom_gutter(ROOM_CAMERA_BOTTOM_GUTTER)
            .block(MODE_ROW_HEIGHT)
            .gap(ROOM_CAMERA_VISUAL_GAP)
            .block(zoom_visual_height(self.current_zoom_mode))
            .gap(ROOM_CAMERA_VISUAL_GAP)
            .block(MODE_ROW_HEIGHT)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let grid_size = game_ctx.cur_world.grid_size;
        let ecs = &mut game_ctx.ecs;

        // Track pivot change to apply after cam borrow ends
        let mut new_pivot: Option<Pivot> = None;

        let cam = ecs
            .get_mut::<RoomCamera>(entity)
            .expect("Camera must exist");

        self.current_zoom_mode = cam.zoom_mode;

        let mut y = rect.y + BODY_TOP_PADDING;

        // Layout dropdown now but draw at the end
        let mode_label = "Zoom Mode: ";
        let label_width = measure_text(ctx, mode_label, FIELD_TEXT_SIZE_16).width;
        ctx.draw_text(
            mode_label,
            rect.x,
            y + 20.0,
            FIELD_TEXT_SIZE_16,
            FIELD_TEXT_COLOR,
        );

        let mode_rect = Rect::new(
            rect.x + label_width + WIDGET_SPACING,
            y,
            rect.w - label_width - WIDGET_SPACING,
            MODE_ROW_HEIGHT,
        );
        let current_mode = cam.zoom_mode;
        let current_label = format!("{current_mode}");
        let zoom_options: Vec<ZoomMode> = ZoomMode::iter().collect();

        y = zoom_section_y(y, cam.zoom_mode);

        match cam.zoom_mode {
            ZoomMode::Step => {
                // Fields to change the global virtual screen dimensions
                let scale_rect = Rect::new(rect.x, y, rect.w, STEP_SECTION_HEIGHT);

                const STEPS: &[f32; 5] = &[0.5_f32, 1.0, 2.0, 3.0, 4.0];

                let current_scalar = 2.0 / (cam.zoom.x * world_virtual_width(grid_size));
                let new_scalar =
                    gui_stepper(ctx, scale_rect, "Scale", STEPS, current_scalar, blocked);

                if !blocked && (new_scalar - current_scalar).abs() > f32::EPSILON {
                    let width = world_virtual_width(grid_size) * new_scalar;
                    let height = world_virtual_height(grid_size) * new_scalar;
                    cam.zoom = vec2(1.0 / width * 2.0, 1.0 / height * 2.0);
                    new_pivot = Some(pivot_for_zoom_scalar(new_scalar));
                }
            }
            ZoomMode::Free => {
                // Editable numeric field and slider for zoom
                let zoom_rect = Rect::new(rect.x, y, rect.w, FREE_SECTION_HEIGHT);

                self.draw_freeform_mode(ctx, zoom_rect, cam, blocked, grid_size);
            }
        }

        y = camera_mode_y(y, cam.zoom_mode);

        // Camera mode
        let cam_mode_label = "Camera Mode: ";
        let cam_label_width = measure_text(ctx, cam_mode_label, FIELD_TEXT_SIZE_16).width;
        ctx.draw_text(
            cam_mode_label,
            rect.x,
            y + 20.0,
            FIELD_TEXT_SIZE_16,
            FIELD_TEXT_COLOR,
        );

        let cam_mode_rect = Rect::new(
            rect.x + cam_label_width + WIDGET_SPACING,
            y,
            rect.w - cam_label_width - WIDGET_SPACING,
            MODE_ROW_HEIGHT,
        );

        // Current value & label
        let current_cam_mode = cam.camera_mode;
        let current_cam_label = format!("{current_cam_mode}");
        let cam_mode_options: Vec<CameraMode> = vec![
            CameraMode::Fixed,
            CameraMode::Follow(FollowRestriction::Free),
            CameraMode::Follow(FollowRestriction::ClampY),
            CameraMode::Follow(FollowRestriction::ClampX),
        ];

        // Render the dropdowns in reverse order
        if let Some(new_cam_mode) = Dropdown::new(
            self.cam_mode_id,
            cam_mode_rect,
            &current_cam_label,
            &cam_mode_options,
            |mode| mode.ui_label(),
        )
        .blocked(blocked)
        .show(ctx)
        {
            if new_cam_mode != current_cam_mode {
                cam.camera_mode = new_cam_mode;
            }
        }

        if let Some(new_mode) = Dropdown::new(
            self.mode_id,
            mode_rect,
            &current_label,
            &zoom_options,
            |mode| mode.ui_label(),
        )
        .blocked(blocked)
        .show(ctx)
        {
            if new_mode != current_mode {
                cam.zoom_mode = new_mode;
                self.current_zoom_mode = new_mode;
            }
        }

        // Apply deferred pivot change
        if let Some(pivot) = new_pivot {
            if let Some(transform) = ecs.get_mut::<Transform>(entity) {
                transform.pivot = pivot;
            }
        }
    }
}

fn stepper_top_bleed() -> f32 {
    22.5
}

fn stepper_visual_height() -> f32 {
    FIELD_TEXT_SIZE_16 * 1.2 + 15.0
}

fn free_zoom_top_bleed() -> f32 {
    FIELD_TEXT_SIZE_16
}

fn free_zoom_visual_height() -> f32 {
    FREE_SECTION_HEIGHT
}

fn zoom_top_bleed(mode: ZoomMode) -> f32 {
    match mode {
        ZoomMode::Step => stepper_top_bleed(),
        ZoomMode::Free => free_zoom_top_bleed(),
    }
}

fn zoom_visual_height(mode: ZoomMode) -> f32 {
    match mode {
        ZoomMode::Step => stepper_visual_height(),
        ZoomMode::Free => free_zoom_visual_height(),
    }
}

fn zoom_section_y(first_row_y: f32, mode: ZoomMode) -> f32 {
    first_row_y + MODE_ROW_HEIGHT + ROOM_CAMERA_VISUAL_GAP + zoom_top_bleed(mode)
}

fn camera_mode_y(zoom_row_y: f32, mode: ZoomMode) -> f32 {
    zoom_row_y - zoom_top_bleed(mode) + zoom_visual_height(mode) + ROOM_CAMERA_VISUAL_GAP
}

impl RoomCameraModule {
    /// Draw a single numeric field that edits the scalar zoom.
    fn draw_freeform_mode(
        &self,
        ctx: &mut WgpuContext,
        rect: Rect,
        cam: &mut RoomCamera,
        blocked: bool,
        grid_size: f32,
    ) {
        let scalar = 2.0 / (cam.zoom.x * world_virtual_width(grid_size));

        const MIN: f32 = 0.5;
        const MAX: f32 = 4.0;
        let scalar = scalar.clamp(MIN, MAX);

        // Label
        let label = "Scale: ";
        let label_width = measure_text(ctx, label, FIELD_TEXT_SIZE_16).width + 1.0;
        let num_width = measure_text(ctx, "0.00", FIELD_TEXT_SIZE_16).width;
        ctx.draw_text(label, rect.x, rect.y, FIELD_TEXT_SIZE_16, FIELD_TEXT_COLOR);

        // Numeric field
        let num_rect = Rect::new(
            rect.x + label_width,
            rect.y - FIELD_TEXT_SIZE_16,
            num_width + WIDGET_SPACING,
            rect.h,
        );

        // Slider
        let slider_rect = Rect::new(
            rect.x + label_width + num_width + 2.0 * WIDGET_SPACING,
            rect.y - FIELD_TEXT_SIZE_16,
            rect.w - (label_width + num_width + 2.0 * WIDGET_SPACING),
            rect.h,
        );

        // Numeric field
        let typed = NumberInput::new(self.zoom_id, num_rect, round_to_dp(scalar, 2))
            .blocked(blocked)
            .show(ctx);

        // Slider
        let (slider_val, slider_state) = gui_slider(
            ctx,
            self.slider_id,
            slider_rect,
            MIN, // min
            MAX, // max
            round_to_dp(scalar, 2),
        );

        // Resolve the new scalar
        let mut new_scalar = scalar;
        if (typed - scalar).abs() > f32::EPSILON {
            new_scalar = round_to_dp(typed, 2).clamp(MIN, MAX);
        }
        if !matches!(slider_state, SliderState::Unchanged) {
            new_scalar = round_to_dp(slider_val, 2).clamp(MIN, MAX);
        }

        // Write back if anything changed
        if (new_scalar - scalar).abs() > f32::EPSILON {
            let width = world_virtual_width(grid_size) * new_scalar;
            let height = world_virtual_height(grid_size) * new_scalar;
            cam.zoom = vec2(1.0 / width * 2.0, 1.0 / height * 2.0);
        }
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        title: ROOM_CAMERA_MODULE_TITLE,
        factory: || {
            Box::new(
                CollapsibleModule::new(
                    crate::gui::inspector::room_camera_module::RoomCameraModule::default()
                )
                .with_title(ROOM_CAMERA_MODULE_TITLE)
            )
        },
    }
}

/// Return `v` rounded to *exactly* `dp` decimal places and stripped of the
/// floating‑point noise that would otherwise appear as …99999 or …00001.
#[inline]
fn round_to_dp(v: f32, dp: u32) -> f32 {
    let factor = 10_f32.powi(dp as i32);

    let mut r = (v * factor).round() / factor;

    if (r - r.round()).abs() < 1e-6 {
        r = r.round();
    }
    r
}

/// Returns the optimal pivot for camera placement at a given zoom scalar.
fn pivot_for_zoom_scalar(scalar: f32) -> Pivot {
    if scalar < 1.0 {
        Pivot::Center
    } else if scalar.round() % 2.0 == 0.0 {
        Pivot::TopLeft
    } else {
        Pivot::CenterLeft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_camera_step_height_matches_visible_controls() {
        let module = RoomCameraModule::default();

        assert_eq!(module.body_layout().height(), 134.2);
    }

    #[test]
    fn room_camera_free_height_matches_visible_controls() {
        let module = RoomCameraModule {
            current_zoom_mode: ZoomMode::Free,
            ..Default::default()
        };

        assert_eq!(module.body_layout().height(), 135.0);
    }

    #[test]
    fn zoom_section_layout_uses_even_visual_gaps() {
        assert_eq!(
            zoom_section_y(0.0, ZoomMode::Step),
            MODE_ROW_HEIGHT + ROOM_CAMERA_VISUAL_GAP + stepper_top_bleed()
        );

        assert_eq!(
            camera_mode_y(zoom_section_y(0.0, ZoomMode::Step), ZoomMode::Step),
            MODE_ROW_HEIGHT
                + ROOM_CAMERA_VISUAL_GAP
                + stepper_visual_height()
                + ROOM_CAMERA_VISUAL_GAP
        );
    }
}
