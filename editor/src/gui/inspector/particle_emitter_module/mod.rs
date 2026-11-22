use std::cell::{Cell, RefCell};

use bishop::prelude::{Rect, WgpuContext};
use engine_core::ecs::components::{
    CurrentRoom, ParticleConfigIssue, ParticleEmissionMode, ParticleEmitter, ParticleEmitterConfig,
    ParticleVisual,
};
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::inspector::module::{CollapsibleComponentModule, InspectorModule};
use engine_core::ecs::{Ecs, Entity};
use engine_core::game::GameCtxMut;
use widgets::constants::layout;

mod config_fields;

const TITLE: &str = "Particle Emitter";
const BODY_TOP_PADDING: f32 = layout::WIDGET_SPACING;
const BODY_BOTTOM_GUTTER: f32 = 10.0;
const FIELD_GAP: f32 = layout::WIDGET_SPACING;
const COMMON_FIELD_ROWS: usize = 12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct VisibleControls {
    burst_count: bool,
    emission_rate: bool,
    line_length: bool,
    sprite_frame: bool,
}

/// Inspector module for editing a particle emitter component.
#[derive(Default)]
pub struct ParticleEmitterModule {
    controls: Cell<VisibleControls>,
    validation_issues: RefCell<Vec<ParticleConfigIssue>>,
    validation_height: Cell<f32>,
    issue_entity: Cell<Option<Entity>>,
    preview_control_visible: Cell<bool>,
    was_editing: Cell<bool>,
    widget_ids: config_fields::WidgetIds,
}

impl ParticleEmitterModule {
    fn visible_controls(config: &ParticleEmitterConfig) -> VisibleControls {
        VisibleControls {
            burst_count: config.mode == ParticleEmissionMode::Burst,
            emission_rate: config.mode == ParticleEmissionMode::Continuous,
            line_length: config.visual == ParticleVisual::Line,
            sprite_frame: matches!(config.visual, ParticleVisual::Sprite { .. }),
        }
    }
}

impl InspectorModule for ParticleEmitterModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(ParticleEmitter::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        let Some(emitter) = ecs.get::<ParticleEmitter>(entity) else {
            self.issue_entity.set(None);
            self.preview_control_visible.set(false);
            self.validation_issues.borrow_mut().clear();
            self.validation_height.set(0.0);
            return false;
        };
        self.preview_control_visible.set(ecs.has::<CurrentRoom>(entity));
        let previous_entity = self.issue_entity.replace(Some(entity));
        if previous_entity.is_some_and(|previous| previous != entity) {
            self.validation_issues.borrow_mut().clear();
            self.validation_height.set(0.0);
        }
        self.controls.set(Self::visible_controls(&emitter.config));
        match emitter.config.validate() {
            Ok(()) => self.validation_issues.borrow_mut().clear(),
            Err(issues) => *self.validation_issues.borrow_mut() = issues,
        }
        true
    }

    fn removable(&self) -> bool {
        true
    }

    fn was_input_active(&self) -> bool {
        self.was_editing.get()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let controls = self.controls.get();
        let optional_rows = [
            controls.burst_count,
            controls.emission_rate,
            controls.line_length,
            controls.sprite_frame,
        ]
        .into_iter()
        .filter(|visible| *visible)
        .count();
        let row_count =
            COMMON_FIELD_ROWS + optional_rows + usize::from(self.preview_control_visible.get());
        let mut body_layout = InspectorBodyLayout::new()
            .top_padding(BODY_TOP_PADDING)
            .bottom_gutter(BODY_BOTTOM_GUTTER)
            .rows(row_count, FIELD_GAP);
        let validation_height = self.validation_height.get();
        if validation_height > 0.0 {
            body_layout = body_layout.gap(FIELD_GAP).block(validation_height);
        }
        body_layout
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        config_fields::draw(self, ctx, blocked, rect, game_ctx, entity);
    }
}

fn apply_config_candidate(
    emitter: &mut ParticleEmitter,
    candidate: ParticleEmitterConfig,
) -> Result<(), Vec<ParticleConfigIssue>> {
    candidate.validate()?;
    emitter.config = candidate;
    Ok(())
}

#[cfg(test)]
fn validation_messages(issues: &[ParticleConfigIssue]) -> Vec<&'static str> {
    issues.iter().map(|issue| issue.message()).collect()
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: ParticleEmitter::TYPE_NAME,
        title: TITLE,
        tooltip: "Configures particle appearance and emission behavior.",
        factory: || {
            Box::new(
                CollapsibleComponentModule::new(
                    crate::gui::inspector::particle_emitter_module::ParticleEmitterModule::default()
                )
                .with_title(TITLE)
            )
        },
        allowed_for: None,
    }
}

#[cfg(test)]
mod tests;
