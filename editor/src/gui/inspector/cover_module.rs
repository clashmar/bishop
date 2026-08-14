use std::cell::Cell;

use bishop::{
    Text,
    prelude::{Rect, WgpuContext},
};
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::inspector::module::{CollapsibleComponentModule, InspectorModule};
use engine_core::ecs::{Cover, CoverMode, Ecs, Entity};
use engine_core::game::GameCtxMut;
use engine_core::ui::measure_text;
use widgets::constants::{colors, layout};
use widgets::{Dropdown, InputCommit, NumberInput, Widget, WidgetId};

const TITLE: &str = "Cover";
const BODY_TOP_PADDING: f32 = layout::WIDGET_SPACING;
const BODY_BOTTOM_GUTTER: f32 = 10.0;
const ROW_HEIGHT: f32 = 30.0;
const FIELD_GAP: f32 = layout::WIDGET_SPACING;
const LABEL_Y_OFFSET: f32 = 20.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CoverUiMode {
    #[default]
    Hide,
    Fade,
}

impl CoverUiMode {
    fn label(self) -> &'static str {
        match self {
            Self::Hide => "Hide",
            Self::Fade => "Fade",
        }
    }

    fn from_cover(cover: Cover) -> Self {
        match cover.mode() {
            CoverMode::Hide => Self::Hide,
            CoverMode::Fade { .. } => Self::Fade,
        }
    }
}

impl std::fmt::Display for CoverUiMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Default)]
pub struct CoverModule {
    mode_id: WidgetId,
    alpha_id: WidgetId,
    current_mode: Cell<CoverUiMode>,
    was_editing: bool,
}

impl InspectorModule for CoverModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(Cover::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        let Some(cover) = ecs.get::<Cover>(entity) else {
            return false;
        };

        self.current_mode.set(CoverUiMode::from_cover(*cover));
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
            .block(ROW_HEIGHT);

        if self.current_mode.get() == CoverUiMode::Fade {
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

        let Some(cover) = game_ctx.ecs.get_mut::<Cover>(entity) else {
            return;
        };

        let mut current_mode = CoverUiMode::from_cover(*cover);
        self.current_mode.set(current_mode);

        let mut y = rect.y + BODY_TOP_PADDING;
        draw_label(ctx, "Mode:", rect.x, y);

        let mode_rect = field_rect(ctx, rect, y);
        let mode_options = [CoverUiMode::Hide, CoverUiMode::Fade];

        if let Some(new_mode) = Dropdown::new(
            self.mode_id,
            mode_rect,
            current_mode.label(),
            &mode_options,
            |mode| mode.label().to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            if new_mode != current_mode {
                cover.hide = matches!(new_mode, CoverUiMode::Hide);
                current_mode = new_mode;
                self.current_mode.set(new_mode);
                self.was_editing = true;
            }
        }

        if current_mode == CoverUiMode::Fade {
            y += ROW_HEIGHT + FIELD_GAP;

            draw_label(ctx, "Alpha:", rect.x, y);
            let alpha_rect = field_rect(ctx, rect, y);
            let (new_alpha, commit): (f32, InputCommit) =
                NumberInput::new(self.alpha_id, alpha_rect, cover.fade_alpha)
                    .blocked(blocked)
                    .show(ctx);

            match commit {
                InputCommit::Previewing | InputCommit::Committed => {
                    self.was_editing = true;
                    if (new_alpha - cover.fade_alpha).abs() > f32::EPSILON {
                        cover.fade_alpha = new_alpha;
                    }
                }
                InputCommit::Unchanged => {}
            }
        }
    }
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
    measure_text(ctx, "Mode:", layout::FIELD_TEXT_SIZE_16)
        .width
        .max(measure_text(ctx, "Alpha:", layout::FIELD_TEXT_SIZE_16).width)
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: Cover::TYPE_NAME,
        title: TITLE,
        factory: || {
            Box::new(
                CollapsibleComponentModule::new(
                    crate::gui::inspector::cover_module::CoverModule::default()
                )
                .with_title(TITLE)
            )
        },
        allowed_for: None,
    }
}
