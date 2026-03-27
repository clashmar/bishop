// editor/src/gui/inspector/inspector_panel.rs
use crate::gui::inspector::audio_source_module::clear_active_audio_preview;
use crate::gui::inspector::room_camera_module::ROOM_CAMERA_MODULE_TITLE;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::gui::inspector::player_module::PlayerModule;
use crate::editor_global::push_command;
use crate::gui::menu_bar::menu_button;
use crate::commands::room::*;
use crate::gui::gui_constants::*;
use engine_core::prelude::*;
use bishop::prelude::*;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};

const SCROLL_SPEED: f32 = 5.0;

/// A component change captured during a single inspector draw pass.
struct ComponentChange {
    entity: Entity,
    type_name: &'static str,
    old_ron: String,
    new_ron: String,
    old_transient_state: ComponentTransientState,
    new_transient_state: ComponentTransientState,
}

struct ComponentEditState {
    /// Snapshot before editing began; never updated after initial capture.
    old_ron: String,
    /// Snapshot of non-persisted editor state before editing began.
    old_transient_state: ComponentTransientState,
    /// Most recent serialised state of the component.
    new_ron: String,
    /// Most recent non-persisted editor state for the component.
    new_transient_state: ComponentTransientState,
    /// Set to `true` each frame the component changes; cleared at end-of-frame.
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

/// Returns the entity that should be used for component operations.
fn component_target(ecs: &Ecs, entity: Entity) -> Entity {
    if ecs.has::<PlayerProxy>(entity) {
        ecs.get_player_entity().unwrap_or(entity)
    } else {
        entity
    }
}

/// Returns true if this module should use the proxy entity directly.
fn is_proxy_local_module(module_title: &str) -> bool {
    module_title == comp_type_name::<Transform>()
        || module_title == "PlayerModule"
}

/// The panel that lives on the right‑hand side of the room editor.
pub struct InspectorPanel {
    /// Geometry of the panel.
    rect: Rect,
    /// Currently inspected entity.
    pub target: Option<Entity>,
    /// All sub‑modules that can draw UI.
    modules: Vec<Box<dyn InspectorModule>>,
    /// Rectangles that were drawn this frame and are therefore active.
    active_rects: Vec<Rect>,
    /// Scroll state for the module list.
    scroll_state: ScrollState,
    /// Ids of the widgets at the top level of the inspector.
    widget_ids: WidgetIds,
    /// In-progress component edits awaiting undo command generation.
    component_edits: HashMap<(Entity, &'static str), ComponentEditState>,
}

pub struct WidgetIds {
    pub darkness_slider_id: WidgetId,
    pub add_component_dropdown_id: WidgetId,
}

impl InspectorPanel {
    /// Create a fresh panel with the default set of modules.
    pub fn new() -> Self {
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

        let widget_ids = WidgetIds {
            darkness_slider_id: WidgetId::default(),
            add_component_dropdown_id: WidgetId::default(),
        };

        Self {
            rect: Rect::new(0., 0., 0., 0.),
            target: None,
            modules,
            active_rects: Vec::new(),
            scroll_state: ScrollState::new(),
            widget_ids,
            component_edits: HashMap::new(),
        }
    }

    /// Called by the editor each frame to place the panel.
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Tell the inspector which entity is currently selected.
    pub fn set_target(&mut self, entity: Option<Entity>) {
        if self.target != entity {
            clear_active_audio_preview();
            self.target = entity;
            self.scroll_state = ScrollState::new();
            self.component_edits.clear();
        }
    }

    /// Render the panel and any visible sub‑modules.
    /// Returns true if 'Create' was pressed.
    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        game_ctx: &mut GameCtxMut
    ) -> bool {
        self.active_rects.clear();

        const BTN_MARGIN: f32 = 10.0;

        if let Some(entity) = self.target {
            let Some(room_id) = game_ctx.cur_world.current_room_id else {
                return false;
            };

            if Controls::copy(ctx) {
                copy_entity(game_ctx.ecs, entity);
            }

            // Layout: Add Component dropdown (right-aligned)
            let add_label = "Add Component";
            let txt_add = measure_text(ctx, add_label, HEADER_FONT_SIZE_20);
            let btn_w_add = txt_add.width + WIDGET_PADDING;
            let add_rect = self.register_rect(Rect::new(
                ctx.screen_width() - INSET - btn_w_add,
                self.rect.y + INSET,
                btn_w_add,
                BTN_HEIGHT,
            ));

            // Layout: Remove button (left of Add Component)
            let remove_label = "Remove";
            let txt_remove = measure_text(ctx, remove_label, HEADER_FONT_SIZE_20);
            let btn_w_remove = txt_remove.width + WIDGET_PADDING;

            // Inspector body
            let top_offset = MENU_PANEL_HEIGHT + INSET;
            let inner = Rect::new(
                self.rect.x,
                top_offset,
                self.rect.w - INSET,
                self.rect.h - (top_offset - self.rect.y) - INSET,
            );

            ctx.draw_rectangle(
                inner.x,
                inner.y,
                inner.w,
                inner.h,
                Color::new(0., 0., 0., 0.6),
            );

            let total_content_h = self.total_content_height(game_ctx.ecs, entity);

            let area = ScrollableArea::new(inner, total_content_h)
                .scroll_speed(SCROLL_SPEED)
                .blocked(is_mouse_over_dropdown_list(ctx))
                .begin(ctx, &mut self.scroll_state);
            let content_rect = area.content_rect();

            ctx.push_clip_rect(inner);

            let mut y = content_rect.y + INSET + self.scroll_state.scroll_y;
            let blocked = self.is_blocked(ctx);
            let comp_target = component_target(game_ctx.ecs, entity);

            // Collect changes this frame.
            let mut comp_changes: Vec<ComponentChange> = Vec::new();

            for module in &mut self.modules {
                let module_entity = if is_proxy_local_module(module.title()) {
                    entity
                } else {
                    comp_target
                };

                if module.visible(game_ctx.ecs, module_entity) {
                    let h = module.height();

                    if area.is_visible(y, h) {
                        let sub_rect = Rect::new(content_rect.x + INSET, y, content_rect.w - INSET * 2.0, h);

                        let pre_snapshot = module.undo_component_type().and_then(|type_name| {
                            let reg = COMPONENTS.iter().find(|r| r.type_name == type_name)?;
                            if (reg.has)(&game_ctx.ecs, module_entity) {
                                let boxed = (reg.clone)(&game_ctx.ecs, module_entity);
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

                        if module.take_remove_request() {
                            if let Some((type_name, ron, _)) = pre_snapshot {
                                self.component_edits.remove(&(module_entity, type_name));
                                push_command(Box::new(RemoveComponentCmd::new(
                                    module_entity,
                                    room_id,
                                    type_name,
                                    ron,
                                )));
                            }
                        } else if let Some((type_name, pre_ron, pre_transient_state)) = pre_snapshot {
                            if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) {
                                if (reg.has)(&game_ctx.ecs, module_entity) {
                                    let boxed = (reg.clone)(&game_ctx.ecs, module_entity);
                                    let post_ron = (reg.to_ron_component)(boxed.as_ref());
                                    let post_transient_state =
                                        capture_component_transient_state(type_name, boxed.as_ref());
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
                    }

                    y += h + WIDGET_SPACING;
                }
            }

            // Merge frame changes into component_edits tracking state.
            for change in comp_changes {
                let state = self.component_edits
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

            // Flush completed edits (no change this frame) as undo commands.
            let completed: Vec<ComponentChange> = self.component_edits
                .iter_mut()
                .filter_map(|(&(entity, type_name), state)| {
                    if !state.changed_this_frame {
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
                self.component_edits.remove(&(change.entity, change.type_name));
                push_command(Box::new(UpdateComponentCmd::new(
                    change.entity,
                    room_id,
                    change.type_name,
                    change.old_ron,
                    change.new_ron,
                    change.old_transient_state,
                    change.new_transient_state,
                )));
            }

            area.draw_scrollbar(ctx, self.scroll_state.scroll_y);
            ctx.pop_clip_rect();
            flush_dropdown_lists(ctx);
            ctx.draw_rectangle_lines(inner.x, inner.y, inner.w, inner.h, 2., Color::WHITE);

            // Add Component dropdown (filterable, menu style)
            let options = self.build_addable_components(game_ctx.ecs, entity);
            if let Some(component) = Dropdown::new(
                self.widget_ids.add_component_dropdown_id,
                add_rect,
                add_label,
                &options,
                |component| component.to_string(),
            )
            .filterable()
            .menu_style()
            .blocked(options.is_empty())
            .show(ctx)
            {
                let target = component_target(game_ctx.ecs, entity);
                if COMPONENTS.iter().any(|r| r.type_name == component.type_name) {
                    push_command(Box::new(AddComponentCmd::new(
                        target,
                        room_id,
                        component.type_name,
                    )));
                } else {
                    onscreen_error!(
                        "Component `{}` not found in registry",
                        component.type_name,
                    );
                }
            }

            // Remove button (not shown for the player entity)
            if !(game_ctx.ecs.get_store::<Player>().contains(entity)) {
                let remove_rect = self.register_rect(Rect::new(
                    add_rect.x - WIDGET_SPACING - btn_w_remove,
                    self.rect.y + INSET,
                    btn_w_remove,
                    BTN_HEIGHT,
                ));

                if menu_button(ctx, remove_rect, remove_label, false) || Controls::delete(ctx) && !input_is_focused() {
                    let room_id = game_ctx.cur_world.current_room_id.unwrap_or_default();
                    let command = DeleteEntityCmd {
                        entity,
                        room_id,
                        saved: None,
                    };
                    push_command(Box::new(command));

                    self.target = None;
                    return false;
                }
            }
        } else {
            // No entity selected
            let create_label = "+ Entity";
            let txt_create = measure_text(ctx, create_label, HEADER_FONT_SIZE_20);
            let create_btn = Rect::new(
                self.rect.x + self.rect.w - txt_create.width - BTN_MARGIN - (WIDGET_PADDING * 2.0),
                self.rect.y + BTN_MARGIN,
                txt_create.width + WIDGET_PADDING * 2.0,
                BTN_HEIGHT,
            );

            let add_cam_label = "+ Camera";
            let txt_cam = measure_text(ctx, add_cam_label, HEADER_FONT_SIZE_20);
            let cam_btn_w = txt_cam.width + WIDGET_PADDING * 2.0;
            let cam_btn = Rect::new(
                create_btn.x - WIDGET_SPACING - cam_btn_w,
                create_btn.y,
                cam_btn_w,
                BTN_HEIGHT,
            );

            if menu_button(ctx, cam_btn, add_cam_label, false) {
                let ecs = &mut game_ctx.ecs;
                let cur_room = game_ctx.cur_world.current_room().unwrap();
                cur_room.create_room_camera(ecs, cur_room.id, game_ctx.cur_world.grid_size);
            }

            let cur_room = game_ctx.cur_world.current_room_mut().unwrap();

            // Darkness slider
            let slider_width = 150.0;
            let slider_rect = self.register_rect(Rect::new(
                create_btn.x + create_btn.w - slider_width,
                create_btn.y + BTN_HEIGHT + 20.0,
                slider_width,
                BTN_HEIGHT,
            ));

            let (new_val, state) = gui_slider(
                ctx,
                self.widget_ids.darkness_slider_id,
                slider_rect,
                0.0,
                1.0,
                cur_room.darkness,
            );

            if !matches!(state, SliderState::Unchanged) {
                cur_room.darkness = new_val.clamp(0.0, 1.0);
            }

            let txt_val = format!("{:.2}", cur_room.darkness);
            let txt_measure = measure_text(ctx, &txt_val, DEFAULT_FONT_SIZE_16);
            let txt_x = slider_rect.x - txt_measure.width - WIDGET_SPACING;
            let txt_y = slider_rect.y + 20.;
            ctx.draw_text(&txt_val, txt_x, txt_y, 20.0, Color::WHITE);

            return menu_button(ctx, create_btn, create_label, false);
        }

        false
    }

    fn is_blocked(&self, ctx: &mut WgpuContext) -> bool {
        is_mouse_over_panel(ctx)
    }

    /// Returns the list of component type names that can be added to the entity.
    /// Excludes room cameras, proxy-local components for proxies, and already-present components.
    fn build_addable_components(&self, ecs: &Ecs, entity: Entity) -> Vec<AddableComponent> {
        let comp_target = component_target(ecs, entity);
        let is_proxy = ecs.has::<PlayerProxy>(entity);
        let mut result = Vec::new();
        for entry in MODULES.iter() {
            let type_name = entry.title;
            if type_name == ROOM_CAMERA_MODULE_TITLE {
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

    #[inline]
    fn register_rect(&mut self, rect: Rect) -> Rect {
        self.active_rects.push(rect);
        rect
    }

    pub fn is_mouse_over(&self, ctx: &WgpuContext) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects.iter().any(|r| r.contains(mouse_screen))
        || (self.rect.contains(mouse_screen) && self.target.is_some())
    }

    fn total_content_height(&self, ecs: &Ecs, entity: Entity) -> f32 {
        let mut total_content_h = 0.0;
        let comp_target = component_target(ecs, entity);
        for module in &self.modules {
            let module_entity = if is_proxy_local_module(module.title()) {
                entity
            } else {
                comp_target
            };
            if module.visible(ecs, module_entity) {
                total_content_h += module.height() + WIDGET_SPACING;
            }
        }
        if total_content_h > 0.0 {
            total_content_h -= WIDGET_SPACING;
        }
        total_content_h += INSET * 2.0;
        total_content_h
    }
}

/// Utility function used by both the panel and the menu
fn entity_has_component(
    ecs: &Ecs,
    entity: Entity,
    reg: &ComponentRegistry,
) -> bool {
    (reg.has)(ecs, entity)
}

fn prettify_component_label(type_name: &str) -> String {
    match type_name {
        "AudioSource" => "Audio Source".to_string(),
        _ => type_name.to_string(),
    }
}
