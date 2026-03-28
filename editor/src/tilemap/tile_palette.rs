// editor/src/tilemap/tile_palette.rs
use bishop::prelude::*;
use engine_core::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::VecDeque;

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct TilePalette {
    pub tile_size: f32,
    pub columns: usize,
    pub rows: usize,
    pub selected_index: usize,
    pub entries: Vec<TileDefId>,
    #[serde(skip)]
    pub ui: TilePaletteUi,
    #[serde(skip)]
    command_queue: VecDeque<PaletteCmd>,
}

enum PaletteCmd {
    Create,
    Edit,
    Delete(usize),
}

#[derive(Clone, Default, PartialEq)]
pub enum TilePaletteUiMode {
    #[default]
    Create,
    Edit,
}

#[derive(Clone, Default)]
pub struct TilePaletteUi {
    pub open: bool,
    pub mode: TilePaletteUiMode,
    pub edit_initialized: bool,
    pub edit_index: usize,
    pub sprite_id: SpriteId,
    pub walkable: bool,
    pub solid: bool,
    pub damage: f32,
}

impl TilePalette {
    pub fn new() -> Self {
        Self {
            ui: TilePaletteUi::default(),
            tile_size: DEFAULT_GRID_SIZE,
            columns: 1,
            rows: 0,
            selected_index: 0,
            entries: Vec::new(),
            command_queue: VecDeque::new(),
        }
    }

    pub fn update(&mut self, asset_manager: &mut AssetManager) {
        while let Some(cmd) = self.command_queue.pop_front() {
            match cmd {
                PaletteCmd::Create => self.create_tile(asset_manager),
                PaletteCmd::Edit => self.edit_tile(asset_manager),
                PaletteCmd::Delete(i) => self.delete_tile(i, asset_manager),
            }
        }
    }

    /// Returns the currently selected TileDefId, or `None` when the palette
    /// is still empty.
    #[inline]
    pub fn selected_def_opt(&self) -> Option<TileDefId> {
        self.entries.get(self.selected_index).copied()
    }

    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        asset_manager: &mut AssetManager,
    ) {
        // Draw grid
        for i in 0..self.entries.len() {
            let col = i % self.columns;
            let row = i / self.columns;
            let y = rect.y + (row as f32 * self.tile_size);

            // Skip rows that are completely outside the visible area
            if y + self.tile_size < rect.y || y > rect.y + self.tile_size * 5.0 {
                continue;
            }

            let x = rect.x + col as f32 * self.tile_size;

            let sprite_id = asset_manager
                .tile_defs
                .get(&self.entries[i])
                .expect("Could not find tile definition.")
                .sprite_id;

            let tex = asset_manager.get_texture_from_id(ctx, sprite_id);

            ctx.draw_texture_ex(
                tex,
                x,
                y,
                Color::WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(self.tile_size, self.tile_size)),
                    ..Default::default()
                },
            );
            if i == self.selected_index {
                ctx.draw_rectangle_lines(x, y, self.tile_size, self.tile_size, 3.0, Color::RED);
            }
        }

        self.draw_tile_dialog(ctx, asset_manager);
    }

    /// Called from `TileMapEditor::handle_ui_click` when the mouse
    /// is over the palette area. Returns `true` if the click was
    /// consumed (i.e. user selected a tile).
    pub fn handle_click(&mut self, mouse_pos: Vec2, rect: Rect) -> bool {
        if !Rect::new(
            rect.x,
            rect.y,
            self.columns as f32 * self.tile_size,
            self.rows as f32 * self.tile_size,
        )
        .contains(mouse_pos)
        {
            return false;
        }

        let local_x = mouse_pos.x - rect.x;
        let local_y = mouse_pos.y - rect.y;
        let col = (local_x / self.tile_size) as usize;
        let row = (local_y / self.tile_size) as usize;
        let idx = row * self.columns + col;
        if idx < self.entries.len() {
            self.selected_index = idx;
            return true;
        }
        false
    }

    fn draw_tile_dialog(&mut self, ctx: &mut WgpuContext, asset_manager: &mut AssetManager) {
        if !self.ui.open {
            return;
        }

        if self.ui.edit_initialized {
            let entry = &self.entries[self.ui.edit_index];

            let tile_def = asset_manager
                .tile_defs
                .get(entry)
                .expect("Could not find tile definition.");

            self.ui.sprite_id = tile_def.sprite_id;

            // Walk through the component specs
            for spec in &tile_def.components {
                match spec {
                    TileComponent::Walkable(v) => self.ui.walkable = *v,
                    TileComponent::Solid(v) => self.ui.solid = *v,
                    TileComponent::Damage(d) => self.ui.damage = *d,
                }
            }
            self.ui.edit_initialized = false;
        }

        // Background panel
        let panel = Rect::new(100., 80., 300., 260.);
        ctx.draw_rectangle(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            Color::new(0., 0., 0., 0.6),
        );
        ctx.draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2., Color::WHITE);

        // Sprite selector
        let sprite_rect = Rect::new(panel.x + 10., panel.y + 60., panel.w - 20., 30.);
        if Button::new(sprite_rect, "Pick sprite").show(ctx) {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG images", &["png"])
                .pick_file()
            {
                let normalized_path = asset_manager.normalize_path(path);

                self.ui.sprite_id = asset_manager
                    .get_or_load(ctx, &normalized_path)
                    .expect("Could not get id for sprite path.");
            }
        }

        // Preview
        if !self.ui.sprite_id.0 != 0 {
            let tex = asset_manager.get_texture_from_id(ctx, self.ui.sprite_id);
            ctx.draw_texture_ex(
                tex,
                panel.x + panel.w - 50.,
                panel.y + 60.,
                Color::WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(40., 40.)),
                    ..Default::default()
                },
            );
        }

        // Component check‑boxes
        let mut walk = self.ui.walkable;
        let mut solid = self.ui.solid;

        let cb_walk = Rect::new(panel.x + 10., panel.y + 110., 20., 20.);
        if gui_checkbox(ctx, cb_walk, &mut walk) {
            self.ui.walkable = walk;
        }
        ctx.draw_text(
            "Walkable",
            cb_walk.x + 30.,
            cb_walk.y + 15.,
            18.,
            Color::WHITE,
        );

        let cb_solid = Rect::new(panel.x + 10., panel.y + 140., 20., 20.);
        if gui_checkbox(ctx, cb_solid, &mut solid) {
            self.ui.solid = solid;
        }
        ctx.draw_text(
            "Solid",
            cb_solid.x + 30.,
            cb_solid.y + 15.,
            18.,
            Color::WHITE,
        );

        let btn_label = match self.ui.mode {
            TilePaletteUiMode::Create => "Create",
            TilePaletteUiMode::Edit => "Update",
        };

        // Create/Update
        let btn_ok = Rect::new(panel.x + 30., panel.y + 220., 100., 30.);
        if Button::new(btn_ok, btn_label).show(ctx) {
            // Add the request to the queue, it will be excecuted next frame
            let cmd = match self.ui.mode {
                TilePaletteUiMode::Create => PaletteCmd::Create,
                TilePaletteUiMode::Edit => PaletteCmd::Edit,
            };
            self.command_queue.push_back(cmd);
            self.ui.open = false;
        }

        // Cancel
        let btn_cancel = Rect::new(panel.x + 170., panel.y + 220., 100., 30.);
        if Button::new(btn_cancel, "Cancel").show(ctx) {
            self.ui.open = false;
        }

        // Draw delete button if in edit mode
        if self.ui.mode == TilePaletteUiMode::Edit {
            let btn_del = Rect::new(panel.x + 30., panel.y + 260., 240., 30.);
            if Button::new(btn_del, "Delete").show(ctx) {
                //Add the request to the queue
                let cmd = PaletteCmd::Delete(self.ui.edit_index);
                self.command_queue.push_back(cmd);
                self.ui.open = false;
            }
        }
    }

    pub fn create_tile(&mut self, asset_manager: &mut AssetManager) {
        // Build TileDef
        let mut comps = vec![
            TileComponent::Walkable(self.ui.walkable),
            TileComponent::Solid(self.ui.solid),
        ];

        if self.ui.damage > 0.0 {
            comps.push(TileComponent::Damage(self.ui.damage));
        }

        let tile_def = TileDef {
            sprite_id: self.ui.sprite_id,
            components: comps,
        };

        // Insert the definition into the world ecs tile_def map
        let def_id = asset_manager.insert_tile_def(tile_def);

        // Persist the palette entry
        self.entries.push(def_id);

        // Auto‑select the newly created tile
        self.selected_index = self.entries.len() - 1;

        // Grow the UI grid
        let needed = self.entries.len();
        self.rows = needed.div_ceil(self.columns)
    }

    pub fn edit_tile(&mut self, asset_manager: &mut AssetManager) {
        // Build TileDef components
        let mut comps = vec![
            TileComponent::Walkable(self.ui.walkable),
            TileComponent::Solid(self.ui.solid),
        ];
        if self.ui.damage > 0.0 {
            comps.push(TileComponent::Damage(self.ui.damage));
        }

        // Get the existing entry id
        let entry = self.entries[self.ui.edit_index];

        // Update sprite ref if it changed
        asset_manager.update_tile_def_sprite(entry, self.ui.sprite_id);

        // Update non-sprite fields
        if let Some(def) = asset_manager.tile_defs.get_mut(&entry) {
            def.components = comps;
        }
    }

    pub fn delete_tile(&mut self, idx: usize, asset_manager: &mut AssetManager) {
        // Remove the definition and decrement sprite ref
        let def_id = self.entries[idx];
        asset_manager.delete_tile_def(def_id);

        // Remove palette entry
        self.entries.remove(idx);

        // Adjust selected index safely
        self.selected_index = self.entries.len().saturating_sub(1);

        // Re-compute rows
        self.rows = self.entries.len().div_ceil(self.columns);
    }

    /// The current height of the palette.
    pub fn height(&self) -> f32 {
        self.rows as f32 * self.tile_size
    }

    /// Called after self.columns changes.
    fn recompute_rows(&mut self) {
        self.rows = if self.columns == 0 {
            0
        } else {
            self.entries.len().div_ceil(self.columns)
        };
    }

    /// Set the column count based on an available width.
    pub fn set_columns_for_width(&mut self, available_width: f32) {
        let cols = (available_width / self.tile_size).floor() as usize;
        self.columns = cols.max(1); // at least one column
        self.recompute_rows();
    }
}
