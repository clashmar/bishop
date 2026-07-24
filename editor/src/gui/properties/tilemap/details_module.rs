use super::TilemapPaneState;
use crate::commands::tilemap::UpdateTileDefinitionCmd;
use crate::editor_global::{push_command, push_toast};
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::component_registry::{
    COMPONENTS, COMPONENT_CONFLICT_GROUPS, component_removal_blocked_by,
};
use engine_core::ecs::inspector::factory::{MODULES, ModuleFactoryEntry, module_title};
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::inspector::module::InspectorModule;
use engine_core::ecs::{Ecs, Entity, SpriteId};
use engine_core::game::GameCtxMut;
use engine_core::storage::assets_folder;
use engine_core::tiles::{
    TileDef, TileDefId, add_tile_definition_component_by_type_name,
    capture_tile_definition_components, remove_tile_definition_component_by_type_name,
    replace_tile_definition_components, tile_definition_component_allowed,
};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use widgets::constants::{colors, layout};
use widgets::*;

const TOP_LABEL_HEIGHT: f32 = 22.0;
const ROW_GAP: f32 = layout::WIDGET_SPACING;
const PREVIEW_SIZE: f32 = 40.0;
const SPRITE_SECTION_HEIGHT: f32 = TOP_LABEL_HEIGHT + PREVIEW_SIZE + ROW_GAP * 2.0;
const ACTIONS_HEIGHT: f32 = layout::DEFAULT_FIELD_HEIGHT;
const ACTIONS_GAP: f32 = layout::WIDGET_SPACING;
const EMPTY_HEIGHT: f32 = layout::DEFAULT_FIELD_HEIGHT;

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

struct TileDefinitionComponentEditor {
    modules: Vec<Box<dyn InspectorModule>>,
    temp_ecs: Ecs,
    temp_entity: Entity,
    add_component_dropdown_id: WidgetId,
    synced_tile_id: Option<TileDefId>,
    pending_before: Option<TileDef>,
    awaiting_apply: Option<TileDef>,
    body_height: f32,
}

impl TileDefinitionComponentEditor {
    fn new() -> Self {
        let mut entries: Vec<&'static ModuleFactoryEntry> = MODULES
            .iter()
            .copied()
            .filter(|entry| tile_definition_component_allowed(entry.type_name))
            .collect();
        entries.sort_by(|left, right| left.title.cmp(right.title));

        Self {
            modules: entries.into_iter().map(|entry| (entry.factory)()).collect(),
            temp_ecs: Ecs::default(),
            temp_entity: Entity::null(),
            add_component_dropdown_id: WidgetId::default(),
            synced_tile_id: None,
            pending_before: None,
            awaiting_apply: None,
            body_height: EMPTY_HEIGHT,
        }
    }

    fn body_height(&self) -> f32 {
        self.body_height
    }

    fn clear_selection(&mut self) {
        self.synced_tile_id = None;
        self.pending_before = None;
        self.awaiting_apply = None;
        self.body_height = EMPTY_HEIGHT;
        self.temp_ecs = Ecs::default();
        self.temp_entity = Entity::null();
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        tile_id: TileDefId,
        tile_def: &TileDef,
        game_ctx: &mut GameCtxMut,
    ) {
        self.sync_to_definition(tile_id, tile_def, game_ctx);

        let before_frame = self.capture_temp_definition(tile_def.sprite_id);
        let addable = self.build_addable_components();
        let add_rect = Rect::new(rect.x, rect.y, rect.w, ACTIONS_HEIGHT);
        if let Some(component) = Dropdown::new(
            self.add_component_dropdown_id,
            add_rect,
            "+Component",
            &addable,
            |component| component.to_string(),
        )
        .filterable()
        .menu_style()
        .button_text_color(colors::DEFAULT_TEXT_COLOR)
        .blocked(addable.is_empty())
        .show(ctx)
        {
            self.with_temp_game_ctx(game_ctx, |temp_ctx, entity| {
                add_tile_definition_component_by_type_name(temp_ctx, entity, component.type_name);
            });
        }

        let mut any_module_editing = false;
        let mut pending_removals = Vec::new();
        let mut y = rect.y + ACTIONS_HEIGHT + ACTIONS_GAP;

        for module in &mut self.modules {
            if !module.visible(&self.temp_ecs, self.temp_entity) {
                continue;
            }

            let module_rect = Rect::new(rect.x, y, rect.w, module.height());
            {
                let mut temp_ctx = make_temp_game_ctx(&mut self.temp_ecs, game_ctx);
                module.draw(ctx, false, module_rect, &mut temp_ctx, self.temp_entity);
            }
            if module.was_input_active() {
                any_module_editing = true;
            }
            if module.take_remove_request() {
                if let Some(type_name) = module.undo_component_type() {
                    let blocker =
                        component_removal_blocked_by(type_name, self.temp_entity, &self.temp_ecs);
                    if let Some(blocker) = blocker {
                        push_toast(
                            format!(
                                "Cannot remove {}: required by {}",
                                module_title(type_name),
                                module_title(blocker),
                            ),
                            3.0,
                        );
                    } else {
                        pending_removals.push(type_name);
                    }
                }
            }
            y += module.height() + layout::WIDGET_SPACING;
        }

        for type_name in pending_removals {
            self.with_temp_game_ctx(game_ctx, |temp_ctx, entity| {
                remove_tile_definition_component_by_type_name(temp_ctx, entity, type_name);
            });
        }

        let after_frame = self.capture_temp_definition(tile_def.sprite_id);
        if before_frame != after_frame && self.pending_before.is_none() {
            self.pending_before = Some(tile_def.clone());
        }

        if !any_module_editing {
            if let Some(before) = self.pending_before.take() {
                if before != after_frame {
                    self.awaiting_apply = Some(after_frame.clone());
                    push_command(Box::new(UpdateTileDefinitionCmd::new(tile_id, before, after_frame)));
                }
            }
        }

        self.body_height = (y - rect.y - layout::WIDGET_SPACING).max(ACTIONS_HEIGHT);
    }

    fn sync_to_definition(
        &mut self,
        tile_id: TileDefId,
        tile_def: &TileDef,
        game_ctx: &mut GameCtxMut,
    ) {
        if self.synced_tile_id != Some(tile_id) {
            self.rebuild_temp_entity(tile_id, tile_def, game_ctx);
            return;
        }

        if self.pending_before.is_some() {
            return;
        }

        if let Some(awaiting_apply) = &self.awaiting_apply {
            if awaiting_apply == tile_def {
                self.awaiting_apply = None;
            } else {
                return;
            }
        }

        if self.capture_temp_definition(tile_def.sprite_id) != *tile_def {
            self.rebuild_temp_entity(tile_id, tile_def, game_ctx);
        }
    }

    fn rebuild_temp_entity(
        &mut self,
        tile_id: TileDefId,
        tile_def: &TileDef,
        game_ctx: &mut GameCtxMut,
    ) {
        self.temp_ecs = Ecs::default();
        self.temp_entity = self.temp_ecs.create_entity().finish();
        self.pending_before = None;
        self.awaiting_apply = None;
        self.synced_tile_id = Some(tile_id);
        self.body_height = EMPTY_HEIGHT;

        self.with_temp_game_ctx(game_ctx, |temp_ctx, entity| {
            replace_tile_definition_components(temp_ctx, entity, &tile_def.components);
        });
    }

    fn build_addable_components(&self) -> Vec<AddableComponent> {
        let mut result = Vec::new();

        for entry in MODULES.iter() {
            if !tile_definition_component_allowed(entry.type_name) {
                continue;
            }
            let Some(reg) = COMPONENTS.iter().find(|reg| reg.type_name == entry.type_name) else {
                continue;
            };
            if (reg.has)(&self.temp_ecs, self.temp_entity) {
                continue;
            }
            if let Some(predicate) = entry.allowed_for {
                if !predicate(self.temp_entity, &self.temp_ecs) {
                    continue;
                }
            }
            if COMPONENT_CONFLICT_GROUPS.iter().any(|group| {
                group.contains(&entry.type_name)
                    && group.iter().any(|&other| {
                        other != entry.type_name
                            && COMPONENTS
                                .iter()
                                .find(|reg| reg.type_name == other)
                                .is_some_and(|reg| (reg.has)(&self.temp_ecs, self.temp_entity))
                    })
            }) {
                continue;
            }
            result.push(AddableComponent {
                type_name: entry.type_name,
                label: entry.title.to_string(),
            });
        }

        result.sort_by(|left, right| left.label.cmp(&right.label));
        result
    }

    fn capture_temp_definition(&mut self, sprite_id: SpriteId) -> TileDef {
        TileDef {
            sprite_id,
            components: capture_tile_definition_components(&mut self.temp_ecs, self.temp_entity),
        }
    }

    fn with_temp_game_ctx<R>(
        &mut self,
        game_ctx: &mut GameCtxMut<'_>,
        f: impl FnOnce(&mut GameCtxMut<'_>, Entity) -> R,
    ) -> R {
        let mut temp_ctx = make_temp_game_ctx(&mut self.temp_ecs, game_ctx);
        f(&mut temp_ctx, self.temp_entity)
    }
}

/// Tile-definition sprite and component details for the selected tile definition.
pub struct TilemapDetailsModule {
    sprite_picker_id: WidgetId,
    has_selection: bool,
    component_editor: TileDefinitionComponentEditor,
}

impl TilemapDetailsModule {
    pub fn new() -> Self {
        Self {
            sprite_picker_id: WidgetId::default(),
            has_selection: false,
            component_editor: TileDefinitionComponentEditor::new(),
        }
    }

    fn push_update_if_changed(tile_id: TileDefId, before: TileDef, after: TileDef) {
        if before != after {
            push_command(Box::new(UpdateTileDefinitionCmd::new(tile_id, before, after)));
        }
    }
}

impl super::super::PropertyModule<TilemapPaneState> for TilemapDetailsModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        state: &mut TilemapPaneState,
        game_ctx: &mut GameCtxMut,
        _: &InspectorContext,
    ) {
        let Some(tile_id) = state.selected_tile_id else {
            self.has_selection = false;
            self.component_editor.clear_selection();
            ctx.draw_text(
                "Select a tile definition to edit",
                rect.x,
                rect.y + 20.0,
                layout::DEFAULT_FONT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            return;
        };

        let Some(before) = game_ctx.tile_registry.get(tile_id).cloned() else {
            self.has_selection = false;
            self.component_editor.clear_selection();
            ctx.draw_text(
                "Selected tile definition no longer exists",
                rect.x,
                rect.y + 20.0,
                layout::DEFAULT_FONT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            return;
        };

        self.has_selection = true;

        ctx.draw_text(
            &format!("Editing Tile {}", tile_id.0),
            rect.x,
            rect.y + 18.0,
            layout::DEFAULT_FONT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        let sprite_row_y = rect.y + TOP_LABEL_HEIGHT + ROW_GAP;
        let sprite_button_rect = Rect::new(
            rect.x,
            sprite_row_y,
            rect.w - PREVIEW_SIZE - ROW_GAP,
            layout::DEFAULT_FIELD_HEIGHT,
        );
        if Button::new(sprite_button_rect, "Pick Sprite")
            .interaction_id(self.sprite_picker_id)
            .show_native_dialog(ctx)
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG images", &["png"])
                .set_directory(assets_folder())
                .pick_file()
            {
                let asset_registry = &mut *game_ctx.asset_registry;
                let sprite_manager = &mut *game_ctx.sprite_manager;
                let normalized_path = sprite_manager.normalize_path(path);
                if let Some(sprite_id) =
                    sprite_manager.get_or_load(asset_registry, ctx, &normalized_path)
                {
                    let mut after = before.clone();
                    after.sprite_id = sprite_id;
                    Self::push_update_if_changed(tile_id, before.clone(), after);
                }
            }
        }

        let preview_rect = Rect::new(
            rect.x + rect.w - PREVIEW_SIZE,
            sprite_row_y,
            PREVIEW_SIZE,
            PREVIEW_SIZE,
        );
        let texture = game_ctx
            .sprite_manager
            .get_texture_from_id(ctx, before.sprite_id);
        ctx.draw_texture_ex(
            texture,
            preview_rect.x,
            preview_rect.y,
            Color::WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(preview_rect.w, preview_rect.h)),
                ..Default::default()
            },
        );
        ctx.draw_rectangle_lines(
            preview_rect.x,
            preview_rect.y,
            preview_rect.w,
            preview_rect.h,
            2.0,
            Color::WHITE,
        );

        let components_rect = Rect::new(
            rect.x,
            rect.y + SPRITE_SECTION_HEIGHT,
            rect.w,
            rect.h - SPRITE_SECTION_HEIGHT,
        );
        self.component_editor
            .draw(ctx, components_rect, tile_id, &before, game_ctx);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        if !self.has_selection {
            return InspectorBodyLayout::new().block(layout::DEFAULT_FIELD_HEIGHT);
        }

        InspectorBodyLayout::new().block(SPRITE_SECTION_HEIGHT + self.component_editor.body_height())
    }

    fn title(&self) -> &str {
        "Details"
    }
}

fn make_temp_game_ctx<'a>(
    temp_ecs: &'a mut Ecs,
    game_ctx: &'a mut GameCtxMut<'_>,
) -> GameCtxMut<'a> {
    GameCtxMut {
        ecs: temp_ecs,
        world: None,
        world_directory: Vec::new(),
        room_world_map: HashMap::new(),
        asset_registry: &mut *game_ctx.asset_registry,
        tile_registry: &mut *game_ctx.tile_registry,
        sprite_manager: &mut *game_ctx.sprite_manager,
        script_manager: &mut *game_ctx.script_manager,
        prefab_manager: game_ctx.prefab_manager,
    }
}
