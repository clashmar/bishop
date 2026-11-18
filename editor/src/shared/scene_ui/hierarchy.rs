use crate::app::EditorMode;
use crate::commands::scene::{RemoveParentCmd, SetParentCmd};
use crate::editor_global::push_command;
use bishop::prelude::*;
use engine_core::ecs::*;
use ::widgets::*;
use std::collections::HashSet;

pub(crate) const HIERARCHY_HORIZONTAL_PADDING: f32 = 10.0;

const ROW_HEIGHT: f32 = 22.0;
const ENTITY_NAME_TEXT_OFFSET: f32 = 18.0;
pub(crate) const ENTITY_NAME_FONT_SIZE: f32 = 14.0;

/// Read-only scene UI state shared by hierarchy hosts.
pub trait SceneUiHost {
    /// Returns the command mode for hierarchy-triggered commands.
    fn command_mode(&self) -> EditorMode;
}

/// Selection behavior required by the shared hierarchy row renderer.
pub trait SceneHierarchyHost: SceneUiHost {
    /// Returns whether the entity is currently selected by the host.
    fn is_selected(&self, entity: Entity) -> bool;

    /// Applies a selection action to the given entity.
    fn apply_selection_action(&mut self, entity: Entity, action: SceneHierarchySelectionAction);
}

/// Selection actions emitted by the shared hierarchy UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneHierarchySelectionAction {
    /// Replaces the current selection with the entity.
    Replace,
    /// Toggles membership of the entity in the current selection.
    Toggle,
}

/// Mutable per-frame hierarchy row drawing state.
pub struct SceneHierarchyFrame<'a> {
    /// Drawing context.
    pub ctx: &'a mut WgpuContext,
    /// Hierarchy panel bounds.
    pub panel_rect: Rect,
    /// Active scroll area for row visibility checks.
    pub area: &'a ActiveScrollArea,
    /// Whether hierarchy interaction is currently blocked.
    pub blocked: bool,
    /// Expanded tree nodes.
    pub expanded: &'a mut HashSet<Entity>,
    /// Optional entity set restricting rows during a search.
    pub visible_entities: Option<&'a HashSet<Entity>>,
    /// Candidate entity and pointer origin awaiting drag-threshold movement.
    pub drag_candidate: &'a mut Option<(Entity, Vec2)>,
    /// Currently dragged entity, if any.
    pub dragging: &'a mut Option<Entity>,
}

/// Draws a scene entity row and its descendants using host-provided selection and policy.
pub fn draw_scene_entity_tree(
    entity: Entity,
    depth: usize,
    y: &mut f32,
    frame: &mut SceneHierarchyFrame<'_>,
    host: &mut dyn SceneHierarchyHost,
    ecs: &mut Ecs,
) {
    if frame
        .visible_entities
        .is_some_and(|visible| !visible.contains(&entity))
    {
        return;
    }

    let content_rect = frame.area.content_rect();
    let content_w = content_rect.w - HIERARCHY_HORIZONTAL_PADDING * 2.0;
    let indent = depth as f32 * 16.0;
    let row_rect = Rect::new(
        content_rect.x + HIERARCHY_HORIZONTAL_PADDING + indent,
        *y,
        content_w - indent,
        ROW_HEIGHT,
    );

    let mut pending_set_parent: Option<(Entity, Entity)> = None;

    if frame.area.is_fully_visible(row_rect.y, row_rect.h) {
        let has_children = has_children(ecs, entity);
        let is_expanded = frame.expanded.contains(&entity);
        let mouse: Vec2 = frame.ctx.mouse_position().into();
        let mouse_over = row_rect.contains(mouse);
        let entity_name = get_entity_name(ecs, entity);
        let expand_button_rect = Rect::new(row_rect.x, row_rect.y, 14.0, ROW_HEIGHT);
        let show_expand_button = has_children && frame.visible_entities.is_none();
        let expand_button_hovered = show_expand_button && expand_button_rect.contains(mouse);

        if host.is_selected(entity) {
            frame.ctx.draw_rectangle(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                Color::new(0.25, 0.45, 0.85, 0.35),
            );
        }

        if show_expand_button {
            let symbol = if is_expanded { "-" } else { "+" };
            let clicked = Button::new(expand_button_rect, symbol)
                .plain()
                .text_color(Color::WHITE)
                .hover_color(Color::GREY)
                .suppressed(frame.blocked)
                .show(frame.ctx);
            if !frame.blocked && clicked {
                if is_expanded {
                    frame.expanded.remove(&entity);
                } else {
                    frame.expanded.insert(entity);
                }
            }
        }

        if !frame.blocked
            && mouse_over
            && !expand_button_hovered
            && frame.ctx.is_mouse_button_pressed(MouseButton::Left)
            && frame.dragging.is_none()
        {
            let action = if frame.ctx.is_key_down(KeyCode::LeftShift)
                || frame.ctx.is_key_down(KeyCode::RightShift)
            {
                SceneHierarchySelectionAction::Toggle
            } else {
                SceneHierarchySelectionAction::Replace
            };
            host.apply_selection_action(entity, action);
        }

        if !frame.blocked
            && mouse_over
            && !expand_button_hovered
            && frame.ctx.is_mouse_button_pressed(MouseButton::Left)
            && frame.dragging.is_none()
            && frame.drag_candidate.is_none()
        {
            *frame.drag_candidate = Some((entity, mouse));
        }

        if !frame.blocked {
            if let Some(dragged) = *frame.dragging {
                if dragged != entity && mouse_over && !is_ancestor(ecs, dragged, entity) {
                    frame.ctx.draw_rectangle(
                        row_rect.x,
                        row_rect.y,
                        row_rect.w,
                        row_rect.h,
                        Color::new(0.3, 0.7, 0.3, 0.3),
                    );
                    if frame.ctx.is_mouse_button_released(MouseButton::Left) {
                        pending_set_parent = Some((dragged, entity));
                        frame.expanded.insert(entity);
                        *frame.dragging = None;
                    }
                }
            }
        }

        frame.ctx.draw_text(
            &entity_name,
            row_rect.x + ENTITY_NAME_TEXT_OFFSET,
            row_rect.y + 16.0,
            ENTITY_NAME_FONT_SIZE,
            Color::WHITE,
        );
    }

    if let Some((child, new_parent)) = pending_set_parent {
        let old_parent = get_parent(ecs, child);
        push_command(Box::new(SetParentCmd::new(
            child,
            new_parent,
            old_parent,
            host.command_mode(),
        )));
    }

    *y += ROW_HEIGHT;

    if frame.expanded.contains(&entity) && has_children(ecs, entity) {
        for child in get_children(ecs, entity) {
            draw_scene_entity_tree(child, depth + 1, y, frame, host, ecs);
        }
    }

    if !frame.blocked {
        if let Some(dragged) = *frame.dragging {
            if dragged == entity {
                let mouse: Vec2 = frame.ctx.mouse_position().into();
                if !frame.panel_rect.contains(mouse)
                    && frame.ctx.is_mouse_button_released(MouseButton::Left)
                {
                    let old_parent = get_parent(ecs, dragged);
                    push_command(Box::new(RemoveParentCmd::new(
                        dragged,
                        old_parent,
                        host.command_mode(),
                    )));
                    *frame.dragging = None;
                }
            }
        }
    }
}

fn get_entity_name(ecs: &Ecs, entity: Entity) -> String {
    ecs.get::<Name>(entity)
        .map(|name| name.to_string())
        .unwrap_or_else(|| format!("{entity:?}"))
}
