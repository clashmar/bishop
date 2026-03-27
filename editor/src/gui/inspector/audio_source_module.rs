// editor/src/gui/inspector/audio_source_module.rs
use engine_core::prelude::*;
use bishop::prelude::*;

const TOP_PADDING: f32 = 10.0;
const SPACING: f32 = 5.0;
const ROW_HEIGHT: f32 = DEFAULT_FIELD_HEIGHT;
const LABEL_W: f32 = 120.0;

/// Editor inspector module for the `AudioSource` component.
///
/// Renders group selection controls plus sounds, volume, pitch variation,
/// volume variation, and looping controls for the currently selected group.
#[derive(Default)]
pub struct AudioSourceModule {
    group_id: WidgetId,
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
        // group selector + sounds rows + Add Sound button +
        // volume + pitch_var + volume_var + looping
        let rows = self.sounds_len + 6;
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

        ensure_selected_group(source);

        let mut y = rect.y + TOP_PADDING;
        let x = rect.x + WIDGET_PADDING;
        let w = rect.w - 2.0 * WIDGET_PADDING;

        // --- Group selector ---
        let group_row = Rect::new(x, y, w, ROW_HEIGHT);
        let group_options = group_options(source);
        let current_group_label = source
            .current
            .as_ref()
            .map_or_else(|| "No Group".to_string(), SoundGroupId::ui_label);

        let dropdown_width = (w - ROW_HEIGHT - SPACING).max(0.0);
        let dropdown_rect = Rect::new(group_row.x, group_row.y, dropdown_width, ROW_HEIGHT);
        let remove_rect = Rect::new(
            dropdown_rect.x + dropdown_rect.w + SPACING,
            group_row.y,
            ROW_HEIGHT,
            ROW_HEIGHT,
        );

        if let Some(selected) = Dropdown::new(
            self.group_id,
            dropdown_rect,
            &current_group_label,
            &group_options,
            SoundGroupId::ui_label,
        )
        .blocked(blocked)
        .show(ctx)
        {
            match selected {
                SoundGroupId::New => {
                    let new_group_id = SoundGroupId::Custom(next_group_name(source));
                    source.groups.insert(
                        new_group_id.clone(),
                        AudioGroup {
                            volume: 1.0,
                            ..Default::default()
                        },
                    );
                    source.current = Some(new_group_id);
                }
                group_id => {
                    source.current = Some(group_id);
                }
            }
        }

        if Button::new(remove_rect, "x")
            .blocked(blocked || source.current.is_none())
            .show(ctx)
        {
            if let Some(current) = source.current.take() {
                source.groups.remove(&current);
                source.current = first_group_id(source);
            }
        }

        y += ROW_HEIGHT + SPACING;

        let Some(group_id) = source.current.clone() else {
            self.sounds_len = 0;
            return;
        };

        let Some(group) = source.groups.get_mut(&group_id) else {
            source.current = first_group_id(source);
            self.sounds_len = 0;
            return;
        };

        self.sounds_len = group.sounds.len();

        // --- Sounds list ---
        let mut remove_idx: Option<usize> = None;
        for (i, sound) in group.sounds.iter().enumerate() {
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
            group.sounds.remove(idx);
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
                group.sounds.push(id);
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
        let (new_vol, state) = gui_slider(ctx, self.volume_id, slider_rect, 0.0, 1.0, group.volume);
        if !blocked && !matches!(state, SliderState::Unchanged) {
            group.volume = new_vol;
        }
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
        let (new_pitch, state) =
            gui_slider(ctx, self.pitch_id, slider_rect, 0.0, 1.0, group.pitch_variation);
        if !blocked && !matches!(state, SliderState::Unchanged) {
            group.pitch_variation = new_pitch;
        }
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
        let (new_vol_var, state) = gui_slider(
            ctx,
            self.volume_var_id,
            slider_rect,
            0.0,
            1.0,
            group.volume_variation,
        );
        if !blocked && !matches!(state, SliderState::Unchanged) {
            group.volume_variation = new_vol_var;
        }
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
            gui_checkbox(ctx, cb_rect, &mut group.looping);
        }
    }
}

fn group_options(source: &AudioSource) -> Vec<SoundGroupId> {
    let mut groups = source
        .groups
        .keys()
        .filter(|group_id| !matches!(group_id, SoundGroupId::New))
        .cloned()
        .collect::<Vec<_>>();
    groups.sort();
    groups.push(SoundGroupId::New);
    groups
}

fn ensure_selected_group(source: &mut AudioSource) {
    if matches!(source.current, Some(SoundGroupId::New)) {
        source.current = None;
    }

    if source.current.as_ref().is_some_and(|group_id| source.groups.contains_key(group_id)) {
        return;
    }

    source.current = first_group_id(source);
}

fn first_group_id(source: &AudioSource) -> Option<SoundGroupId> {
    source
        .groups
        .keys()
        .filter(|group_id| !matches!(group_id, SoundGroupId::New))
        .cloned()
        .min()
}

fn next_group_name(source: &AudioSource) -> String {
    let mut index = 1;
    loop {
        let candidate = format!("Group {}", index);
        let candidate_id = SoundGroupId::Custom(candidate.clone());
        if !source.groups.contains_key(&candidate_id) {
            return candidate;
        }
        index += 1;
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
                .with_title("Audio Source")
            )
        },
    }
}
