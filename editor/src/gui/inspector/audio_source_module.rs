// editor/src/gui/inspector/audio_source_module.rs
use engine_core::prelude::*;
use bishop::prelude::*;

const TOP_PADDING: f32 = 10.0;
const SPACING: f32 = 5.0;
const ROW_HEIGHT: f32 = DEFAULT_FIELD_HEIGHT;
const LABEL_W: f32 = 120.0;

/// Editor inspector module for the `AudioSource` component.
///
/// Renders a sounds list, add/remove controls, and sliders for volume,
/// pitch variation, volume variation, and a looping checkbox.
#[derive(Default)]
pub struct AudioSourceModule {
    volume_id: WidgetId,
    pitch_id: WidgetId,
    volume_var_id: WidgetId,
    /// Cached sound count used for height computation the following frame.
    sounds_len: usize,
}

impl InspectorModule for AudioSourceModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(AudioSource::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<AudioSource>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        Ecs::remove_component::<AudioSource>(game_ctx, entity);
    }

    fn height(&self) -> f32 {
        // sounds rows + Add Sound button + volume + pitch_var + volume_var + looping
        let rows = self.sounds_len + 5;
        TOP_PADDING + rows as f32 * (ROW_HEIGHT + SPACING) + SPACING
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let Some(source) = game_ctx.ecs.get_mut::<AudioSource>(entity) else {
            return;
        };

        self.sounds_len = source.sounds.len();

        let mut y = rect.y + TOP_PADDING;
        let x = rect.x + WIDGET_PADDING;
        let w = rect.w - 2.0 * WIDGET_PADDING;

        // --- Sounds list ---
        let mut remove_idx: Option<usize> = None;
        for (i, sound) in source.sounds.iter().enumerate() {
            let btn_w = ROW_HEIGHT;
            let label_rect = Rect::new(x, y, w - btn_w - SPACING, ROW_HEIGHT);
            let btn_rect = Rect::new(x + w - btn_w, y, btn_w, ROW_HEIGHT);

            ctx.draw_text(
                sound,
                label_rect.x,
                label_rect.y + 20.0,
                DEFAULT_FONT_SIZE_16,
                FIELD_TEXT_COLOR,
            );

            if Button::new(btn_rect, "x").blocked(blocked).show(ctx) {
                remove_idx = Some(i);
            }
            y += ROW_HEIGHT + SPACING;
        }
        if let Some(idx) = remove_idx {
            source.sounds.remove(idx);
        }

        // --- Add Sound button ---
        let add_rect = Rect::new(x, y, w, ROW_HEIGHT);
        if Button::new(add_rect, "Add Sound").blocked(blocked).show(ctx) {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Audio", &["wav"])
                .set_directory(engine_core::storage::path_utils::audio_folder())
                .pick_file()
            {
                let base = engine_core::storage::path_utils::audio_folder();
                let relative = path.strip_prefix(&base).unwrap_or(&path);
                let id = relative
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");
                source.sounds.push(id);
            }
        }
        y += ROW_HEIGHT + SPACING;

        // --- Volume ---
        ctx.draw_text(
            "Volume:",
            x,
            y + 20.0,
            DEFAULT_FONT_SIZE_16,
            FIELD_TEXT_COLOR,
        );
        let slider_rect = Rect::new(x + LABEL_W + SPACING, y, w - LABEL_W - SPACING, ROW_HEIGHT);
        let (new_vol, _) = gui_slider(ctx, self.volume_id, slider_rect, 0.0, 1.0, source.volume);
        source.volume = new_vol;
        y += ROW_HEIGHT + SPACING;

        // --- Pitch Variation ---
        ctx.draw_text(
            "Pitch Var:",
            x,
            y + 20.0,
            DEFAULT_FONT_SIZE_16,
            FIELD_TEXT_COLOR,
        );
        let slider_rect = Rect::new(x + LABEL_W + SPACING, y, w - LABEL_W - SPACING, ROW_HEIGHT);
        let (new_pitch, _) =
            gui_slider(ctx, self.pitch_id, slider_rect, 0.0, 1.0, source.pitch_variation);
        source.pitch_variation = new_pitch;
        y += ROW_HEIGHT + SPACING;

        // --- Volume Variation ---
        ctx.draw_text(
            "Vol Var:",
            x,
            y + 20.0,
            DEFAULT_FONT_SIZE_16,
            FIELD_TEXT_COLOR,
        );
        let slider_rect = Rect::new(x + LABEL_W + SPACING, y, w - LABEL_W - SPACING, ROW_HEIGHT);
        let (new_vol_var, _) = gui_slider(
            ctx,
            self.volume_var_id,
            slider_rect,
            0.0,
            1.0,
            source.volume_variation,
        );
        source.volume_variation = new_vol_var;
        y += ROW_HEIGHT + SPACING;

        // --- Looping checkbox ---
        ctx.draw_text(
            "Looping:",
            x,
            y + 20.0,
            DEFAULT_FONT_SIZE_16,
            FIELD_TEXT_COLOR,
        );
        let cb_rect = Rect::new(
            x + LABEL_W + SPACING,
            y + (ROW_HEIGHT - DEFAULT_CHECKBOX_DIMS) / 2.0,
            DEFAULT_CHECKBOX_DIMS,
            DEFAULT_CHECKBOX_DIMS,
        );
        if !blocked {
            gui_checkbox(ctx, cb_rect, &mut source.looping);
        }
    }
}

inventory::submit! {
    ModuleFactoryEntry {
        title: <AudioSource>::TYPE_NAME,
        factory: || {
            Box::new(
                CollapsibleModule::new(
                    crate::gui::inspector::audio_source_module::AudioSourceModule::default()
                )
                .with_title(<AudioSource>::TYPE_NAME)
            )
        },
    }
}
