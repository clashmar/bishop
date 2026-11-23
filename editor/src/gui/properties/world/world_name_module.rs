use super::super::PropertyModule;
use crate::gui::text_input::{committed_name_change, draw_labeled_text_input};
use crate::shared::scene_ui::inspector::{InspectorContext, InspectorHostAction};
use bishop::{Rect, WgpuContext};
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::world::World;
use widgets::{InputCommit, WidgetId};

const NAME_ROW_HEIGHT: f32 = 30.0;

/// Edits a world's name and returns the rename action to its host.
pub struct WorldNameModule {
    input_id: WidgetId,
    host_action: Option<InspectorHostAction>,
}

impl WorldNameModule {
    /// Creates a world-name property module.
    pub fn new() -> Self {
        Self {
            input_id: WidgetId::default(),
            host_action: None,
        }
    }
}

impl Default for WorldNameModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldNameModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        world: &mut World,
        _game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let name_rect = Rect::new(rect.x, rect.y, rect.w, NAME_ROW_HEIGHT);
        let (edited, commit) =
            draw_labeled_text_input(ctx, name_rect, "Name:", &world.name, self.input_id);
        self.host_action = committed_world_rename(&world.name, &edited, commit);
    }

    fn take_host_action(&mut self) -> Option<InspectorHostAction> {
        self.host_action.take()
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .padding(0.0, 0.0)
            .block(NAME_ROW_HEIGHT)
    }

    fn title(&self) -> &str {
        "World Name"
    }
}

fn committed_world_rename(
    current: &str,
    edited: &str,
    commit: InputCommit,
) -> Option<InspectorHostAction> {
    committed_name_change(current, edited, commit).map(InspectorHostAction::RenameWorld)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_world_name_edit_returns_the_existing_host_rename_action() {
        assert_eq!(
            committed_world_rename("Old", "New", InputCommit::Committed),
            Some(InspectorHostAction::RenameWorld("New".to_owned()))
        );
        assert_eq!(
            committed_world_rename("Old", "New", InputCommit::Previewing),
            None
        );
        assert_eq!(
            committed_world_rename("Old", "Old", InputCommit::Committed),
            None
        );
    }
}
