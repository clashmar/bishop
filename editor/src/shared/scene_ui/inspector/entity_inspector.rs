use super::{
    is_scene_component_hidden_in_prefab, linked_prefab_instance_state_for_scene_inspector,
    SceneCreateRequest, SceneInspectorContext, SceneInspectorOutput, ScenePrefabAction,
    ScenePrefabActionRequest,
};
use crate::app::EditorMode;
use crate::commands::room::copy_entity;
use crate::commands::scene::{
    capture_component_transient_state, AddComponentCmd, ComponentTransientState,
    DeleteEntityCmd, RemoveComponentCmd, UpdateComponentCmd,
};
use crate::editor_global::push_command;
use crate::gui::gui_constants::*;
use crate::gui::inspector::player_module::PlayerModule;
use crate::gui::inspector::room_camera_module::ROOM_CAMERA_MODULE_TITLE;
use crate::gui::menu_bar::menu_button;
use crate::shared::input::shortcuts_blocked;
use crate::shared::scene_ui::inspector::InspectorContent;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use widgets::constants::layout;

const PREFAB_METADATA_HEIGHT: f32 = 66.0;
const PREFAB_ACTION_TOP_PADDING: f32 = 4.0;
const PREFAB_ACTION_ROW_SPACING: f32 = 8.0;
const PREFAB_ACTION_ROW_BUTTON_SCALE: f32 = 0.9;
const PREFAB_ACTION_ROW_BUTTON_GAP: f32 = layout::WIDGET_SPACING * 0.5;
const PREFAB_SECTION_BOTTOM_GAP: f32 = 8.0;

struct PrefabActionStripLayout {
    open_button_rect: Rect,
    unlink_rect: Rect,
    sync_rect: Rect,
    revert_rect: Rect,
}

/// Stateful scene-inspector for entities, used by room and prefab hosts.
pub struct EntityInspector {
    target: Option<Entity>,
    modules: Vec<Box<dyn InspectorModule>>,
    widget_ids: WidgetIds,
    component_edits: HashMap<(Entity, &'static str), ComponentEditState>,
    interactive_rects: Vec<Rect>,
    command_mode: Option<EditorMode>,
}

struct WidgetIds {
    add_component_dropdown_id: WidgetId,
}

struct ComponentChange {
    entity: Entity,
    type_name: &'static str,
    old_ron: String,
    new_ron: String,
    old_transient_state: ComponentTransientState,
    new_transient_state: ComponentTransientState,
}

struct ComponentEditState {
    old_ron: String,
    old_transient_state: ComponentTransientState,
    new_ron: String,
    new_transient_state: ComponentTransientState,
    changed_this_frame: bool,
}

#[derive(Clone, PartialEq)]
struct AddableComponent {
    type_name: &'static str,
    label: String,
}

impl Display for AddableComponent {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.label)
    }
}

impl EntityInspector {
    /// Creates a fresh entity inspector for the given entity.
    pub fn new(entity: Entity) -> Self {
        let mut modules: Vec<Box<dyn InspectorModule>> = Vec::new();
        modules.push(Box::new(PlayerModule::default()));

        let mut name_module: Option<Box<dyn InspectorModule>> = None;
        let mut transform_module: Option<Box<dyn InspectorModule>> = None;
        let mut other_modules: Vec<Box<dyn InspectorModule>> = Vec::new();

        for entry in MODULES.iter() {
            let module = (entry.factory)();
            if entry.title == comp_type_name::<Name>() {
                name_module = Some(module);
            } else if entry.title == comp_type_name::<Transform>() {
                transform_module = Some(module);
            } else {
                other_modules.push(module);
            }
        }

        if let Some(name_mod) = name_module {
            modules.insert(1, name_mod);
        }
        if let Some(transform_mod) = transform_module {
            modules.insert(2, transform_mod);
        }
        modules.extend(other_modules);

        Self {
            target: Some(entity),
            modules,
            widget_ids: WidgetIds {
                add_component_dropdown_id: WidgetId::default(),
            },
            component_edits: HashMap::new(),
            interactive_rects: Vec::new(),
            command_mode: None,
        }
    }


    fn build_addable_components(
        &self,
        ecs: &mut Ecs,
        entity: Entity,
        hide_room_only_components: bool,
    ) -> Vec<AddableComponent> {
        let comp_target = component_target(ecs, entity);
        let is_proxy = ecs.has::<PlayerProxy>(entity);
        let mut result = Vec::new();
        for entry in MODULES.iter() {
            let type_name = entry.title;
            if type_name == ROOM_CAMERA_MODULE_TITLE {
                continue;
            }
            if hide_room_only_components && is_scene_component_hidden_in_prefab(type_name) {
                continue;
            }
            if is_proxy_local_module(type_name) && is_proxy {
                continue;
            }
            let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) else {
                onscreen_error!("Module `{}` has no ComponentReg entry", type_name);
                continue;
            };
            if entity_has_component(ecs, comp_target, reg) {
                continue;
            }
            result.push(AddableComponent {
                type_name,
                label: prettify_component_label(type_name),
            });
        }
        result
    }

    fn compute_module_height(
        &self,
        ecs: &mut Ecs,
        prefab_manager: &PrefabManager,
        entity: Entity,
        show_linked_prefab_metadata: bool,
        hide_room_only_components: bool,
    ) -> f32 {
        let mut total_content_h = 0.0;
        let comp_target = component_target(ecs, entity);
        if linked_prefab_instance_state_for_scene_inspector(
            show_linked_prefab_metadata,
            ecs,
            prefab_manager,
            entity,
        )
        .is_some()
        {
            total_content_h += prefab_metadata_section_spacing();
        }
        for module in &self.modules {
            let module_entity = if is_proxy_local_module(module.title()) {
                entity
            } else {
                comp_target
            };
            if module.visible(ecs, module_entity)
                && !(hide_room_only_components
                    && is_scene_component_hidden_in_prefab(module.title()))
            {
                total_content_h += module.height() + layout::WIDGET_SPACING;
            }
        }
        if total_content_h > 0.0 {
            total_content_h -= layout::WIDGET_SPACING;
        }
        total_content_h + INSET * 2.0
    }
}

impl InspectorContent for EntityInspector {
    fn header_height(&self) -> f32 {
        INSPECTOR_HEADER_HEIGHT
    }

    fn draw_header(
        &mut self,
        ctx: &mut WgpuContext,
        header_rect: Rect,
        blocked: bool,
        game_ctx: &mut GameCtxMut,
        scene_ctx: &SceneInspectorContext,
    ) -> SceneInspectorOutput {
        self.interactive_rects.clear();
        let mut output = SceneInspectorOutput::default();
        self.command_mode = Some(scene_ctx.command_mode);

        let Some(entity) = self.target else {
            return output;
        };

        if Controls::copy(ctx) {
            copy_entity(game_ctx.ecs, entity);
        }

        let add_label = "Add Component";
        let txt_add = measure_text(ctx, add_label, layout::HEADER_FONT_SIZE_20);
        let btn_w_add = txt_add.width + layout::WIDGET_PADDING;
        let add_rect = register_rect(
            &mut self.interactive_rects,
            Rect::new(
                ctx.screen_width() - INSET - btn_w_add,
                header_rect.y + INSPECTOR_HEADER_BUTTON_Y,
                btn_w_add,
                BTN_HEIGHT,
            ),
        );

        let remove_label = "Remove";
        let txt_remove = measure_text(ctx, remove_label, layout::HEADER_FONT_SIZE_20);
        let btn_w_remove = txt_remove.width + layout::WIDGET_PADDING;
        let create_label = "+ Entity";
        let txt_create = measure_text(ctx, create_label, layout::HEADER_FONT_SIZE_20);
        let btn_w_create = txt_create.width + layout::WIDGET_PADDING;

        // Remove button
        if !game_ctx.ecs.get_store::<Player>().contains(entity) {
            let remove_rect = register_rect(
                &mut self.interactive_rects,
                Rect::new(
                    add_rect.x - layout::WIDGET_SPACING - btn_w_remove,
                    header_rect.y + INSPECTOR_HEADER_BUTTON_Y,
                    btn_w_remove,
                    BTN_HEIGHT,
                ),
            );
            if menu_button(ctx, remove_rect, remove_label, blocked)
                || Controls::delete(ctx) && !shortcuts_blocked()
            {
                let command = DeleteEntityCmd::new(entity, scene_ctx.command_mode);
                push_command(Box::new(command));
                self.target = None;
                return output;
            }
        }

        // +Entity button
        if let Some(parent) = scene_ctx.selected_create_parent {
            let create_rect = register_rect(
                &mut self.interactive_rects,
                Rect::new(
                    add_rect.x
                        - layout::WIDGET_SPACING
                        - btn_w_remove
                        - layout::WIDGET_SPACING
                        - btn_w_create,
                    header_rect.y + INSPECTOR_HEADER_BUTTON_Y,
                    btn_w_create,
                    BTN_HEIGHT,
                ),
            );
            if menu_button(ctx, create_rect, create_label, blocked) {
                output.create_request =
                    Some(SceneCreateRequest { parent: Some(parent) });
                return output;
            }
        }

        // Add Component dropdown
        let options = self.build_addable_components(
            game_ctx.ecs,
            entity,
            scene_ctx.hide_room_only_components,
        );
        if let Some(component) = Dropdown::new(
            self.widget_ids.add_component_dropdown_id,
            add_rect,
            add_label,
            &options,
            |component| component.to_string(),
        )
        .filterable()
        .menu_style()
        .blocked(options.is_empty() || blocked)
        .show(ctx)
        {
            let target = component_target(game_ctx.ecs, entity);
            if COMPONENTS
                .iter()
                .any(|r| r.type_name == component.type_name)
            {
                push_command(Box::new(AddComponentCmd::new(
                    target,
                    scene_ctx.command_mode,
                    component.type_name,
                )));
            } else {
                onscreen_error!("Component `{}` not found in registry", component.type_name);
            }
        }

        output
    }

    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        content_rect: Rect,
        blocked: bool,
        game_ctx: &mut GameCtxMut,
        scene_ctx: &SceneInspectorContext,
    ) -> SceneInspectorOutput {
        let mut output = SceneInspectorOutput::default();
        let Some(entity) = self.target else {
            return output;
        };
        let comp_target = component_target(game_ctx.ecs, entity);
        let linked_prefab = linked_prefab_instance_state_for_scene_inspector(
            scene_ctx.show_linked_prefab_metadata,
            game_ctx.ecs,
            game_ctx.prefab_manager,
            entity,
        );

        let mut y = content_rect.y + INSET;
        let mut comp_changes: Vec<ComponentChange> = Vec::new();
        let mut any_module_editing = false;

        if let Some(prefab_state) = linked_prefab.as_ref() {
            // is_visible check and y offset handled by the caller's ScrollableArea
            let metadata_rect = Rect::new(
                content_rect.x + INSET,
                y,
                content_rect.w - INSET * 2.0,
                PREFAB_METADATA_HEIGHT,
            );
            ctx.draw_rectangle(
                metadata_rect.x,
                metadata_rect.y,
                metadata_rect.w,
                metadata_rect.h,
                Color::new(0., 0., 0., 0.28),
            );
            let layout = prefab_action_strip_layout(metadata_rect);
            let open_button_label = prefab_open_button_label(
                ctx,
                &prefab_state.label,
                layout.open_button_rect.w,
            );
            register_rect(&mut self.interactive_rects, layout.open_button_rect);
            if Button::new(layout.open_button_rect, &open_button_label)
                .blocked(blocked)
                .show(ctx)
            {
                output.prefab_action = Some(ScenePrefabActionRequest {
                    action: ScenePrefabAction::OpenPrefabEditor,
                    selected_entity: prefab_state.selected_entity,
                    root_entity: prefab_state.root_entity,
                    prefab_id: prefab_state.prefab_id,
                });
                return output;
            }

            let actions_blocked = !prefab_state.has_local_changes;
            register_rect(&mut self.interactive_rects, layout.unlink_rect);
            if Button::new(layout.unlink_rect, "Unlink")
                .blocked(blocked)
                .show(ctx)
            {
                output.prefab_action = Some(ScenePrefabActionRequest {
                    action: ScenePrefabAction::UnlinkInstance,
                    selected_entity: prefab_state.selected_entity,
                    root_entity: prefab_state.root_entity,
                    prefab_id: prefab_state.prefab_id,
                });
                return output;
            }

            register_rect(&mut self.interactive_rects, layout.sync_rect);
            if Button::new(layout.sync_rect, "Sync")
                .blocked(blocked || actions_blocked)
                .show(ctx)
            {
                output.prefab_action = Some(ScenePrefabActionRequest {
                    action: ScenePrefabAction::ApplyInstanceToPrefab,
                    selected_entity: prefab_state.selected_entity,
                    root_entity: prefab_state.root_entity,
                    prefab_id: prefab_state.prefab_id,
                });
                return output;
            }

            register_rect(&mut self.interactive_rects, layout.revert_rect);
            if Button::new(layout.revert_rect, "Revert")
                .blocked(blocked || actions_blocked)
                .show(ctx)
            {
                output.prefab_action = Some(ScenePrefabActionRequest {
                    action: ScenePrefabAction::RevertInstanceToPrefab,
                    selected_entity: prefab_state.selected_entity,
                    root_entity: prefab_state.root_entity,
                    prefab_id: prefab_state.prefab_id,
                });
                return output;
            }

            y += prefab_metadata_section_spacing();
        }

        for module in &mut self.modules {
            let module_entity = if is_proxy_local_module(module.title()) {
                entity
            } else {
                comp_target
            };
            if module.visible(game_ctx.ecs, module_entity) {
                let h = module.height();
                let sub_rect = Rect::new(
                    content_rect.x + INSET,
                    y,
                    content_rect.w - INSET * 2.0,
                    h,
                );
                let pre_snapshot = module.undo_component_type().and_then(|type_name| {
                    let reg = COMPONENTS.iter().find(|r| r.type_name == type_name)?;
                    if (reg.has)(game_ctx.ecs, module_entity) {
                        let boxed = (reg.clone)(game_ctx.ecs, module_entity);
                        Some((
                            type_name,
                            (reg.to_ron_component)(boxed.as_ref()),
                            capture_component_transient_state(type_name, boxed.as_ref()),
                        ))
                    } else {
                        None
                    }
                });
                module.draw(ctx, blocked, sub_rect, game_ctx, module_entity);
                if module.was_input_active() {
                    any_module_editing = true;
                }
                if module.take_remove_request() {
                    if let Some((type_name, ron, _)) = pre_snapshot {
                        self.component_edits.remove(&(module_entity, type_name));
                        push_command(Box::new(RemoveComponentCmd::new(
                            module_entity,
                            scene_ctx.command_mode,
                            type_name,
                            ron,
                        )));
                    }
                } else if let Some((type_name, pre_ron, pre_transient_state)) = pre_snapshot {
                    if let Some(reg) =
                        COMPONENTS.iter().find(|r| r.type_name == type_name)
                    {
                        if (reg.has)(game_ctx.ecs, module_entity) {
                            let boxed = (reg.clone)(game_ctx.ecs, module_entity);
                            let post_ron = (reg.to_ron_component)(boxed.as_ref());
                            let post_transient_state = capture_component_transient_state(
                                type_name,
                                boxed.as_ref(),
                            );
                            if pre_ron != post_ron {
                                comp_changes.push(ComponentChange {
                                    entity: module_entity,
                                    type_name,
                                    old_ron: pre_ron,
                                    new_ron: post_ron,
                                    old_transient_state: pre_transient_state,
                                    new_transient_state: post_transient_state,
                                });
                            }
                        }
                    }
                }
                y += h + layout::WIDGET_SPACING;
            }
        }

        for change in comp_changes {
            let state = self
                .component_edits
                .entry((change.entity, change.type_name))
                .or_insert_with(|| ComponentEditState {
                    old_ron: change.old_ron,
                    old_transient_state: change.old_transient_state,
                    new_ron: change.new_ron.clone(),
                    new_transient_state: change.new_transient_state.clone(),
                    changed_this_frame: true,
                });
            state.new_ron = change.new_ron;
            state.new_transient_state = change.new_transient_state;
            state.changed_this_frame = true;
        }

        let completed: Vec<ComponentChange> = self
            .component_edits
            .iter_mut()
            .filter_map(|(&(entity, type_name), state)| {
                if !state.changed_this_frame
                    && !any_module_editing
                    && state.old_ron != state.new_ron
                {
                    Some(ComponentChange {
                        entity,
                        type_name,
                        old_ron: state.old_ron.clone(),
                        new_ron: state.new_ron.clone(),
                        old_transient_state: state.old_transient_state.clone(),
                        new_transient_state: state.new_transient_state.clone(),
                    })
                } else {
                    state.changed_this_frame = false;
                    None
                }
            })
            .collect();

        for change in completed {
            self.component_edits
                .remove(&(change.entity, change.type_name));
            push_command(Box::new(UpdateComponentCmd::new(
                change.entity,
                scene_ctx.command_mode,
                change.type_name,
                change.old_ron,
                change.new_ron,
                change.old_transient_state,
                change.new_transient_state,
            )));
        }

        output
    }

    fn total_content_height(
        &self,
        game_ctx: &mut GameCtxMut,
        scene_ctx: &SceneInspectorContext,
    ) -> f32 {
        let Some(entity) = self.target else {
            return 0.0;
        };
        self.compute_module_height(
            &mut game_ctx.ecs,
            &game_ctx.prefab_manager,
            entity,
            scene_ctx.show_linked_prefab_metadata,
            scene_ctx.hide_room_only_components,
        )
    }

    fn was_input_active(&self) -> bool {
        false
    }

    fn target(&self) -> Option<Entity> {
        self.target
    }

    fn interactive_rects(&self) -> Vec<Rect> {
        self.interactive_rects.clone()
    }
}

fn component_target(ecs: &Ecs, entity: Entity) -> Entity {
    if ecs.has::<PlayerProxy>(entity) {
        ecs.get_player_entity().unwrap_or(entity)
    } else {
        entity
    }
}

fn is_proxy_local_module(module_title: &str) -> bool {
    module_title == comp_type_name::<Transform>() || module_title == "PlayerModule"
}

fn register_rect(active_rects: &mut Vec<Rect>, rect: Rect) -> Rect {
    active_rects.push(rect);
    rect
}

fn entity_has_component(ecs: &Ecs, entity: Entity, reg: &ComponentRegistry) -> bool {
    (reg.has)(ecs, entity)
}

fn prettify_component_label(type_name: &str) -> String {
    match type_name {
        "AudioSource" => "Audio Source".to_string(),
        _ => type_name.to_string(),
    }
}

fn prefab_open_button_label<C: BishopContext>(
    ctx: &mut C,
    label: &str,
    button_width: f32,
) -> String {
    truncate_to_width(
        ctx,
        label,
        button_width - layout::WIDGET_PADDING * 2.0,
        layout::DEFAULT_FONT_SIZE_16,
    )
}

fn prefab_metadata_section_spacing() -> f32 {
    PREFAB_METADATA_HEIGHT + layout::WIDGET_SPACING + PREFAB_SECTION_BOTTOM_GAP
}

fn prefab_action_strip_layout(metadata_rect: Rect) -> PrefabActionStripLayout {
    let open_button_y = metadata_rect.y + PREFAB_ACTION_TOP_PADDING;
    let button_y = open_button_y + BTN_HEIGHT + PREFAB_ACTION_ROW_SPACING;
    let column_w = (metadata_rect.w - PREFAB_ACTION_ROW_BUTTON_GAP * 2.0) / 3.0;
    let button_w = column_w * PREFAB_ACTION_ROW_BUTTON_SCALE;
    let button_h = BTN_HEIGHT * PREFAB_ACTION_ROW_BUTTON_SCALE;
    let button_x_offset = (column_w - button_w) * 0.5;
    let button_y_offset = (BTN_HEIGHT - button_h) * 0.5;
    let unlink_rect = Rect::new(
        metadata_rect.x + button_x_offset,
        button_y + button_y_offset,
        button_w,
        button_h,
    );
    let sync_rect = Rect::new(
        metadata_rect.x + column_w + PREFAB_ACTION_ROW_BUTTON_GAP + button_x_offset,
        button_y + button_y_offset,
        button_w,
        button_h,
    );
    let revert_rect = Rect::new(
        metadata_rect.x + (column_w + PREFAB_ACTION_ROW_BUTTON_GAP) * 2.0 + button_x_offset,
        button_y + button_y_offset,
        button_w,
        button_h,
    );
    let open_button_rect = Rect::new(
        unlink_rect.x,
        open_button_y,
        revert_rect.x + revert_rect.w - unlink_rect.x,
        BTN_HEIGHT,
    );

    PrefabActionStripLayout {
        open_button_rect,
        unlink_rect,
        sync_rect,
        revert_rect,
    }
}
